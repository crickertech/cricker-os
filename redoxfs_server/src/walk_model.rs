//! **A directory walk, replayed against the server core on the host** (milestone 606 (a directory
//! walk costs what it does on Linux), provisional number).
//!
//! `walk_pricing` times a walk from inside a confined `std` program, where every figure carries
//! the kernel, the caretaker hop and the device. This replays the same walk one layer down: the
//! request sequence `std`'s nife PAL sends for `walk_pricing::walk` (`OPENDIR` per component,
//! `READDIR` pages, `OPEN`, `FSTAT`, `READ` to end of file, a `CLOSE` per handle), issued straight
//! at [`Server`] over a [`Counting`] disk. So it answers two questions the on-target figure cannot
//! separate:
//!
//! - **how many device reads a walk costs**, exactly and deterministically, which is what a test
//!   can gate on (`tests/walk_cost.rs`: a warm walk reads nothing from the device);
//! - **what the server's own work costs**, on the same core the HVF guest runs on, with no IPC and
//!   no device in the number (`examples/walk_replay.rs`).
//!
//! # BUGS
//!
//! - **The request sequence is a model of the PAL, not the PAL.** It is written from
//!   `patches/std-nife/overlay/std/src/sys/fs/nife.rs` (`walk`, `dir_at`, `readdir`, `File::open`,
//!   `read_to_end`'s size hint and its probe read at end of file) and nothing checks that the two
//!   agree. If the PAL changes how many requests an operation takes, this goes on replaying the
//!   old count until someone edits it. The on-target `walk split` line is the check on it.
//! - **Staged through the server, not by `redoxfs_host`.** The on-target image is built by the
//!   host tool from `walk_pricing::stage`'s directory, so its block layout differs from the one
//!   this builds. Counts of requests are the same; counts of device reads can differ.
//!
//! Name: provisional 2026-09-26 (milestone 606's lane); calef names modules.

use alloc::vec::Vec;

use filesystem_protocol::dir;
use filesystem_protocol::fixture::walk as tree;
use redoxfs::Disk;
use syscall::error::Result;

use crate::{BLOCK, Server};

/// A [`Disk`] that counts what reaches it: the calls, and the blocks they moved. Put it
/// *under* [`crate::CachedDisk`] and it counts the device reads a cache did not answer.
pub struct Counting<D> {
    inner: D,
    /// `read_at` calls that reached this disk.
    pub reads: u64,
    /// Blocks those calls read.
    pub blocks_read: u64,
}

impl<D> Counting<D> {
    /// Wrap `inner` with both counters at zero.
    pub fn new(inner: D) -> Self {
        Self {
            inner,
            reads: 0,
            blocks_read: 0,
        }
    }
}

impl<D: Disk> Disk for Counting<D> {
    unsafe fn read_at(&mut self, block: u64, buffer: &mut [u8]) -> Result<usize> {
        self.reads += 1;
        self.blocks_read += buffer.len().div_ceil(BLOCK) as u64;
        // SAFETY: forwarded from this method's own caller, on the same buffer.
        unsafe { self.inner.read_at(block, buffer) }
    }

    unsafe fn write_at(&mut self, block: u64, buffer: &[u8]) -> Result<usize> {
        // SAFETY: as above.
        unsafe { self.inner.write_at(block, buffer) }
    }

    fn size(&mut self) -> Result<u64> {
        self.inner.size()
    }
}

/// The requests one walk sent, by verb. Their sum is the walk's round trips through the FS
/// server, and on nife each is two IPC round trips (client to caretaker, caretaker to server).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Requests {
    pub opendir: u64,
    pub readdir: u64,
    pub open: u64,
    pub fstat: u64,
    pub read: u64,
    pub close: u64,
}

impl Requests {
    /// Every request, summed.
    pub fn total(&self) -> u64 {
        self.opendir + self.readdir + self.open + self.fstat + self.read + self.close
    }
}

/// What a replayed walk found: the same counts `walk_pricing::Totals` reports on target, so the
/// two can be checked against each other and against the fixture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Walked {
    pub entries: usize,
    pub files: usize,
    pub bytes: usize,
    pub components: usize,
    pub requests: Requests,
}

/// **Build [`tree`] under `parent`**, named [`tree::ROOT`], through the server's own verbs, and
/// return nothing: a walk opens it the way the caretaker does, by name.
pub fn stage<D: Disk>(srv: &mut Server<D>, parent: u32) -> Result<()> {
    let root = srv.make_dir(parent, tree::ROOT, dir::ALL)?;
    let file = |srv: &mut Server<D>, at: u32, name: &str, body: &[u8]| -> Result<()> {
        let h = srv.create_file_at(at, name)?;
        let mut off = 0;
        while off < body.len() {
            let end = (off + BLOCK).min(body.len());
            srv.write(h, off as u64, &body[off..end])?;
            off = end;
        }
        srv.close(h)
    };
    let chain = srv.make_dir(root, tree::CHAIN, dir::ALL)?;
    let mut level = chain;
    for i in 0..=tree::DEPTH {
        if i > 0 {
            let next = srv.make_dir(level, tree::LEVEL, dir::ALL)?;
            if level != chain {
                srv.close(level)?;
            }
            level = next;
        }
        file(srv, level, tree::LEVEL_FILE, tree::SMALL_BODY)?;
    }
    srv.close(level)?;
    if level != chain {
        srv.close(chain)?;
    }
    let wide = srv.make_dir(root, tree::WIDE, dir::ALL)?;
    for i in 0..tree::WIDE_COUNT {
        let name = tree::wide_name(i);
        file(
            srv,
            wide,
            core::str::from_utf8(&name).unwrap_or("?"),
            tree::SMALL_BODY,
        )?;
    }
    srv.close(wide)?;
    let narrow = srv.make_dir(root, tree::NARROW, dir::ALL)?;
    let first = tree::wide_name(0);
    file(
        srv,
        narrow,
        core::str::from_utf8(&first).unwrap_or("?"),
        tree::SMALL_BODY,
    )?;
    srv.close(narrow)?;
    let sizes = srv.make_dir(root, tree::SIZES, dir::ALL)?;
    for (name, len) in tree::SIZE_FILES {
        let body: Vec<u8> = (0..len).map(tree::sized_byte).collect();
        file(srv, sizes, name, &body)?;
    }
    srv.close(sizes)?;
    srv.close(root)
}

/// The PAL's rights for a descent: `DESCEND` plus what the final verb needs (`walk` in the PAL).
const LIST: u64 = dir::DESCEND | dir::ENUMERATE;
const OPEN: u64 = dir::DESCEND | dir::READ;

/// **Replay `walk_pricing::walk` over the tree at `root`**, a directory handle holding at least
/// `ENUMERATE | READ | DESCEND`, as the PAL would send it through a caretaker bound there.
pub fn walk<D: Disk>(srv: &mut Server<D>, root: u32) -> Result<Walked> {
    let mut w = Walked::default();
    let mut path: Vec<Vec<u8>> = Vec::new();
    walk_from(srv, root, &mut path, &mut w)?;
    Ok(w)
}

/// `dir_at` in the PAL: one `OPENDIR` per component asking for `want`, closing each hop as the
/// next one lands, so at most two handles are open. Returns the handle, or `root` for the grant.
fn descend<D: Disk>(
    srv: &mut Server<D>,
    root: u32,
    path: &[Vec<u8>],
    want: u64,
    r: &mut Requests,
) -> Result<u32> {
    let mut at = root;
    for name in path {
        r.opendir += 1;
        let next = srv.open_dir(at, core::str::from_utf8(name).unwrap_or("?"), want)?;
        if at != root {
            r.close += 1;
            srv.close(at)?;
        }
        at = next;
    }
    Ok(at)
}

fn walk_from<D: Disk>(
    srv: &mut Server<D>,
    root: u32,
    path: &mut Vec<Vec<u8>>,
    w: &mut Walked,
) -> Result<()> {
    // `readdir`: the whole listing drained up front, then the handle closed.
    let at = descend(srv, root, path, LIST, &mut w.requests)?;
    let mut entries: Vec<(Vec<u8>, bool)> = Vec::new();
    let mut page = [0u8; filesystem_protocol::PAGE];
    loop {
        w.requests.readdir += 1;
        let n = srv.read_dir(at, entries.len() as u32, &mut page)?;
        if n == 0 {
            break;
        }
        for (name, is_dir) in filesystem_protocol::dirent::iter(&page[..n]) {
            entries.push((name.to_vec(), is_dir));
        }
    }
    if at != root {
        w.requests.close += 1;
        srv.close(at)?;
    }

    for (name, is_dir) in entries {
        w.entries += 1;
        w.components += path.len() + 1;
        path.push(name);
        if is_dir {
            walk_from(srv, root, path, w)?;
        } else {
            // `File::open`: descend to the parent, `OPEN` the last name, close the parent.
            let (last, dirs) = path.split_last().expect("just pushed");
            let parent = descend(srv, root, dirs, OPEN, &mut w.requests)?;
            w.requests.open += 1;
            let h = srv.open_file_at(parent, core::str::from_utf8(last).unwrap_or("?"))?;
            if parent != root {
                w.requests.close += 1;
                srv.close(parent)?;
            }
            // `read_to_end`: the size hint, then page-sized reads until one comes back empty.
            w.requests.fstat += 1;
            let _size = srv.fstat(h)?;
            let mut off = 0u64;
            loop {
                w.requests.read += 1;
                let got = srv.read(h, off, &mut page)?;
                if got == 0 {
                    break;
                }
                off += got as u64;
            }
            w.requests.close += 1;
            srv.close(h)?;
            w.bytes += off as usize;
            w.files += 1;
        }
        path.pop();
    }
    Ok(())
}
