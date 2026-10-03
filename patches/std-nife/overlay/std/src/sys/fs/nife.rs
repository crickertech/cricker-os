//! `std::fs` for nife (milestone 27 phase two): `File` bound to the FS-service contract
//! (DECISIONS §27, notes/fs-server.md, `crates/filesystem_protocol`).
//!
//! # The interesting part: there is no global namespace
//!
//! `File::open` takes a path, and this system has no filesystem root to resolve one against. Per
//! §27, open-by-path exists **only inside the FS server**, resolved relative to the one directory
//! node the client's endpoint is bound to. So the honest mapping is:
//!
//! > a std program holds a **directory capability** (the FS-service endpoint, slot 4 of the std
//! > slot convention in `pal/nife/rt.rs`), and `File::open("foo")` means *"foo, under the
//! > directory I was granted"*, not *"foo somewhere in a global filesystem"*.
//!
//! Three consequences, and they are the design, not a limitation to apologise for:
//!
//! 1. **A path that tries to leave the granted directory is refused here, before the wire.** An
//!    absolute path, or any `..`, names something this process holds no capability for, and there
//!    is no namespace in which to express it. That is [`io::ErrorKind::InvalidFilename`] with a
//!    message saying so, deliberately **not** `PermissionDenied`: nothing checked a permission,
//!    and pretending otherwise would be a Unix EPERM fiction over a capability refusal.
//! 2. **A program granted no directory capability gets `Unsupported` from everything.** Not an
//!    empty filesystem, not `NotFound`: the platform cannot do it, because no capability reaches a
//!    filesystem. The same shape `std::net` uses when the `Stack` endpoint is absent.
//! 3. **A nested path is a chain of descents, not a lookup** (milestone 122). `File::open("a/b/c")`
//!    is `OPENDIR a`, `OPENDIR b`, then `OPEN c` against `b`'s handle, each hop attenuated by the
//!    last. This used to be refused outright, which meant a program could list a subdirectory and
//!    then not open what it found there. **The grant is exactly as tight as it was**: every hop
//!    still resolves under the capability this process holds, rights only ever narrow, and there
//!    are no symlinks to escape through. What changed is only whether the program has to spell the
//!    descent itself.
//!
//! # What binds, and what stays honestly Unsupported
//!
//! Bound: [`File::open`] (`OPEN`), `read` (`READ`), `write` (`WRITE`), `seek`/`tell` (an offset
//! this side keeps, since the contract's read and write are both positional), `file_attr`/`size`
//! (`FSTAT`), close on `Drop` (`CLOSE`), plus [`stat`]/[`exists`] built from open + fstat + close.
//!
//! **Also bound since milestone 31 phase 2:** `create` and `create_new` (`CREATE`) and `truncate`
//! (`TRUNCATE`), which means **`File::create` and `std::fs::write` work now** rather than being
//! Unsupported by construction. Two things this corrects. Creating a *file* was never what §27 kept
//! host-side; that was creating a *filesystem*, which needs uuid and getrandom, and
//! `Transaction::create_node` is not std-gated, so a file can be made on-device without entropy
//! becoming a userspace dependency. And the previous refusal was the right call at the time for a
//! reason worth remembering: without `TRUNCATE`, `std::fs::write` would have left a tail of the old
//! contents behind, so a write that half-worked would have read as a write that failed, which is
//! precisely the confusion DECISIONS §27 records being corrected four times in one day.
//!
//! **Also bound since milestone 64:** `read_dir`, `create_dir`, `remove_file`, `remove_dir` and
//! `rename`, on the verbs milestones 47 and 48 added (`OPENDIR`, `READDIR`, `MKDIR`, `UNLINK`,
//! `RMDIR`, `RENAME`). Every one of them was refused here for a reason that had stopped being true:
//! this module said "no verb in the contract backs it" long after the server started dispatching
//! all six. Milestone 64's measurement found them by asking fifty crates.io crates what they
//! needed; see notes/crates-io-on-nife.md.
//!
//! **And since milestone 64's second pass:** `File::set_len` (`TRUNCATE` again, with the size in the
//! second word instead of 0) and [`copy`] (no verb, and none needed: it is an open, a read/write
//! loop and two closes).
//!
//! **And since milestone 122:** nested paths anywhere a name is taken, [`Dir`] (std's own
//! `openat`-shaped API, unstable as `#![feature(dirfd)]`), and `remove_dir_all`, which needed no
//! code at all: std's generic implementation is written in terms of `read_dir` and `remove_file`
//! on paths it composes itself, so it started working the moment those paths resolved.
//!
//! **And since milestone 64's last pass:** `Metadata::modified` (`GETMTIME`) and
//! `std::fs::set_times`/`set_times_nofollow` (`SETMTIME_AT`), on the verbs milestone 47's `touch`
//! added (DECISIONS §112). This header said "no verb" for both until 2026-09-19, three weeks after
//! the verbs landed, which is the fifth refusal in this module found to have outlived its reason.
//! Both verbs take a **name under a directory handle**, not an open file, and that decides the
//! shape of the binding: a path's metadata carries an mtime, and an open `File`'s does not. See
//! [`Mtime`] for why the PAL refuses rather than re-resolving a name it no longer holds.
//!
//! Still Unsupported, each because no verb in the contract backs it: symlinks and hard links,
//! `canonicalize`, permissions, access and creation times, setting a time through an open `File`,
//! locks, and duplicating a handle (`File::try_clone`, and `Dir::try_clone` for anything but
//! the granted directory: a handle is a token the server minted; there is no dup verb).
//!
//! See notes/std.md for the full list with reasons.

use crate::ffi::OsString;
use crate::fmt;
use crate::fs::TryLockError;
use crate::io::{self, BorrowedCursor, IoSlice, IoSliceMut, SeekFrom};
use crate::path::{Component, Path, PathBuf};
use crate::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
use crate::sys::pal::nife::fsproto::{self, fs as proto};
use crate::sys::pal::nife::rt;
use crate::sys::time::{SystemTime, UNIX_EPOCH};
use crate::sys::{unsupported, unsupported_err};

// The pieces of the phase-one backend that stay exactly as honest as they were: nothing in the
// contract links or canonicalizes, so those keep the `unsupported` implementations rather than
// gaining a nife-shaped copy of the same refusal. `FileTimes` used to come from there too, and its
// setters discarded what they were given; it is this module's own now, because `set_times` has a
// verb to carry a modification time to.
#[expect(dead_code)]
#[path = "unsupported.rs"]
mod unsupported_fs;
pub use unsupported_fs::{canonicalize, link, readlink, symlink};

/// **`remove_dir_all` needed no code** (milestone 122), and that is the whole finding.
///
/// It used to come from `unsupported.rs` with a note saying the recursion has to descend and a
/// nested path is refused, so the loop belonged in `components/src/rm.rs` where a program can hold a
/// directory capability per level. The second half of that was right and is now this module's
/// business: [`walk`] holds one per level. std's own generic implementation is written entirely in
/// terms of `read_dir`, `remove_file` and `remove_dir` on paths it composes with `DirEntry::path`,
/// which is exactly the shape that started working when descent did.
///
/// It is the same finding milestone 64 recorded twice and this module records a fourth time: **a
/// refusal outlived the reason for it**, and looked correct the whole while. The rights it needs at
/// every level are `dir::REMOVE_TREE`'s (`ENUMERATE` to see, `DESCEND` to walk in, `REMOVE` to take
/// the name out), asked for one verb at a time by the walk rather than all at once.
pub use crate::sys::fs::common::remove_dir_all;

/// No nife-specific fields behind `std::fs::{HomeDirs, MediaDirs}` (rust-lang/rust#158936): the
/// empty structs every non-Unix platform takes. Nothing in this system names a home or a media
/// directory, so there is nothing for an extension trait to carry; `home_dir` is `None` for the
/// same reason (`sys/paths/nife.rs`).
pub use crate::sys::fs::common::{ExtraHomeDirs, ExtraMediaDirs};

/// The FS-service endpoint: this process's entire authority over files. Naming a file over it is a
/// request the server resolves under the one directory the endpoint is bound to.
const FS: u64 = rt::FS_DIR_SLOT;

/// The page shared with the FS server, mapped by the loader alongside the grant.
const PAGE_VA: u64 = rt::FS_PAGE;

/// The contract's transfer unit: one page, so one request never moves more than this.
const PAGE: usize = fsproto::PAGE;

// --- Is a filesystem reachable at all? -------------------------------------------------------
//
// This has to be answerable WITHOUT touching the shared page, because a program that was not
// granted a directory capability has no page mapped and a probe that wrote a name into it would
// fault instead of returning an error. So the probe is a request that carries no payload: `FSTAT`
// on a handle number the server's table can never contain.
//
//   - no capability in the slot: the kernel refuses the invoke itself and the first reply word is
//     one of its own small negatives (`NoSuchSlot` = -1, `WrongObject` = -2, `NotPermitted` = -3).
//   - a real server: it answers `-EBADF` (-9) for the impossible handle, which is a *reply*, so a
//     filesystem is reachable.
//
// Cached, because the answer cannot change: a capability table slot's contents are fixed at spawn on this
// ABI (0 = not yet asked, 1 = granted, 2 = not granted).

static REACHABLE: AtomicU8 = AtomicU8::new(0);

/// True if this process holds a directory capability. See the note above for why the probe is an
/// `FSTAT` on an impossible handle rather than anything that touches the shared page.
///
/// `pub(crate)` for `sys/paths/nife.rs`, which needs the same fact for the same reason: naming a
/// place a process has no capability for is the answer-instead-of-refusing failure this platform's
/// own notes keep recording.
pub(crate) fn is_reachable() -> bool {
    match REACHABLE.load(Ordering::Relaxed) {
        1 => true,
        2 => false,
        _ => {
            let (r0, _) = rt::call(FS, proto::req(proto::FSTAT, proto::MAX_HANDLE, 0), 0);
            // A payload-free probe cannot draw `ENOENT` (whose -2 collides with the kernel's
            // `WrongObject`), so all three of the kernel's refusals are unambiguous here.
            let ok = !matches!(r0 as i64, -1 | -2 | -3);
            REACHABLE.store(if ok { 1 } else { 2 }, Ordering::Relaxed);
            ok
        }
    }
}

/// Whether a reply word is the *kernel* refusing the invoke rather than the server answering.
///
/// The kernel's own errors are -1..-8 (`abi::Error`); the server's are negated errnos. The two
/// spaces overlap, which is a wart of the contract recorded in notes/std.md.
///
/// **This used to read -1 as "you hold no such capability" too, and that was wrong** (milestone
/// 122). The comment justifying it said `EPERM` (1) is not in the FS server's vocabulary; it has
/// been since milestone 47, where `dir::EPERM` is what a directory capability answers when the
/// right a verb needs is withheld and when a descent asks for more than the parent holds. So the
/// one reply that says "this capability does not carry that right" was being reported to a std
/// program as "this platform cannot do that", which is the silent-degradation shape DECISIONS §42
/// forbids and which would have made a narrowed grant indistinguishable from no grant at all.
///
/// What makes -1 unambiguous here is [`is_reachable`]: every entry point in this module checks it
/// first and returns `Unsupported` when the slot is empty, so by the time a `request` is issued a
/// server has already answered one message. A kernel `NoSuchSlot` cannot follow that. -3 stays,
/// because `ESRCH` really is not in the FS server's vocabulary and `NotPermitted` from the kernel
/// is what a WRITE-less endpoint answers.
///
/// The clean fix is the contract's, not this function's: a tag or an offset in the reply word so
/// the two error spaces stop overlapping. See the BUGS section of notes/std.md.
///
/// Name: provisional, flagged 2026-09-24 by the boolean-predicate pass
/// (design/naming/boolean-predicates-worklist.md). It does not yet follow the Rust predicate rule
/// calef ratified 2026-09-24; recommended `is_missing_capability`, because `is_no_capability` does
/// not parse.
fn no_capability(r0: u64) -> bool {
    matches!(r0 as i64, -3)
}

// --- The shared page -------------------------------------------------------------------------
//
// One page for the whole process, so two concurrent operations would trample each other. A
// spinlock guards it: uncontended on this single-threaded target, correct if threads arrive, the
// same discipline the heap and the net PAL's registry use. Every request that touches the page
// holds the guard across its `CALL`, which is also what makes the reply's bytes ours to read.

static LOCKED: AtomicBool = AtomicBool::new(false);

struct Page;

impl Drop for Page {
    fn drop(&mut self) {
        LOCKED.store(false, Ordering::Release);
    }
}

/// Take the shared page. Held across the `CALL` that uses it.
fn page() -> Page {
    while LOCKED
        .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
        .is_err()
    {
        crate::hint::spin_loop();
    }
    Page
}

impl Page {
    /// Put `bytes` in the page: a name to open, or data to write.
    ///
    /// **A plain copy, not a volatile byte loop** (milestone 606 (a directory walk costs what it
    /// does on Linux), provisional number). The page is shared with another address space, and
    /// what keeps the compiler from caching or eliding an access to it is the `CALL` itself: every
    /// exchange is `put`, then `rt::call`, then `get`, and `rt::call` is an `asm!` block without
    /// `nomem`, which the compiler must assume reads and writes any memory. So no access moves
    /// across it or is assumed unchanged by it. The volatile loop bought nothing on top of that
    /// and cost a load or store per byte: 334 KB of them in a walk that reads the fixture tree.
    fn put(&mut self, bytes: &[u8]) {
        // SAFETY: PAGE_VA is a mapped, writable page of `PAGE` bytes; callers clamp to it, and it
        // cannot overlap `bytes`, which is this process's own memory.
        unsafe { core::ptr::copy_nonoverlapping(bytes.as_ptr(), PAGE_VA as *mut u8, bytes.len()) };
    }

    /// Put `bytes` at `off` in the page. Only `RENAME` needs this: it is the one verb that carries
    /// two names, back to back with the source first, because one request word cannot hold two
    /// lengths and the page is where the contract puts what does not fit in a word.
    fn put_at(&mut self, off: usize, bytes: &[u8]) {
        // SAFETY: PAGE_VA is a mapped, writable page of `PAGE` bytes; the caller checks that
        // `off + bytes.len()` is within it before calling. [`Page::put`] says why a plain copy.
        unsafe {
            core::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                (PAGE_VA as usize + off) as *mut u8,
                bytes.len(),
            )
        };
    }

    /// Take `out.len()` bytes out of the page (a completed read landed there).
    fn get(&mut self, out: &mut [u8]) {
        // SAFETY: as [`Page::put`]; callers clamp `out` to the page.
        unsafe {
            core::ptr::copy_nonoverlapping(PAGE_VA as *const u8, out.as_mut_ptr(), out.len())
        };
    }
}

// --- Errors, mapped by meaning ---------------------------------------------------------------

/// One request on the FS service. Returns the reply's first word as a count/handle, or an
/// `io::Error`. The wire convention is `filesystem_protocol`'s: non-negative is a result, negative is a
/// negated errno ([`fsproto::reply_errno`]).
fn request(w0: u64, w1: u64) -> io::Result<u64> {
    let (r0, _) = rt::call(FS, w0, w1);
    reply(r0)
}

/// A reply word as a result: [`request`]'s mapping, split out so a reply can be kept and turned into
/// an `io::Error` later. [`FileAttr`] needs that, because `Metadata` is a value std hands back
/// whole and `modified()` is where a refused `GETMTIME` has to surface, not `metadata()`.
fn reply(r0: u64) -> io::Result<u64> {
    match fsproto::reply_errno(r0 as i64) {
        None => Ok(r0),
        // The kernel refusing the invoke (a revoked or missing endpoint) is the same answer a
        // program with no directory capability gets: the platform cannot do this.
        Some(_) if no_capability(r0) => Err(unsupported_err()),
        Some(errno) => Err(from_errno(errno)),
    }
}

/// The server's errno into an `io::ErrorKind`, by meaning. There is no errno anywhere else in this
/// PAL: the FS service is the one place nife speaks one, because the component behind it
/// (RedoxFS) does, and §27 maps it at the server boundary and nowhere deeper.
fn from_errno(errno: i32) -> io::Error {
    match errno {
        // `EPERM` is milestone 47's, and milestone 122 is what made it reachable from here: a
        // directory capability that does not carry `ENUMERATE`, and a descent that asked for more
        // than the parent holds, both answer it. It is the second errno in this map that is about
        // *you* rather than about the name, so it gets the kind that says so.
        //
        // **`PermissionDenied` here is not the EPERM fiction `count_names`' refusals avoid**, and the
        // difference is which side of the wire decided. Those refusals are this client saying "no
        // capability designates that name", where no permission was ever consulted. This is the
        // server saying "the capability you hold lacks the right that verb needs", which is a fact
        // about authority and is exactly what `dir::EPERM` was specified to mean loudly rather than
        // conceal. `ReadOnlyFilesystem` is its sibling for the mutating rights; std has no third
        // kind that would fit better.
        1 => io::const_error!(
            io::ErrorKind::PermissionDenied,
            "this directory capability does not carry the right that verb needs"
        ),
        2 => io::const_error!(io::ErrorKind::NotFound, "no such name in the granted directory"),
        9 => io::const_error!(
            io::ErrorKind::InvalidInput,
            "the FS server does not honor that handle"
        ),
        // The four below arrived with milestone 64's bindings, and each one exists because the
        // verbs it belongs to distinguish cases the read/write half never had to. Mapping them all
        // to `Other` would have made `remove_dir` on a full directory indistinguishable from a disk
        // error, which is exactly what a caller loops on.
        17 => io::const_error!(io::ErrorKind::AlreadyExists, "that name is already taken"),
        20 => io::const_error!(io::ErrorKind::NotADirectory, "that name is not a directory"),
        21 => io::const_error!(io::ErrorKind::IsADirectory, "that name is a directory"),
        22 => io::const_error!(io::ErrorKind::InvalidInput, "the FS server refused the request"),
        28 => io::const_error!(io::ErrorKind::StorageFull, "the filesystem is full"),
        // `EROFS` here is a **capability** answer, not a mode bit: the directory capability this
        // process holds does not carry the right this verb needs (§47's ladder). It is the one
        // errno in this map that means "you", rather than "that name".
        30 => io::const_error!(
            io::ErrorKind::ReadOnlyFilesystem,
            "this directory capability does not carry the right that verb needs"
        ),
        39 => io::const_error!(io::ErrorKind::DirectoryNotEmpty, "that directory is not empty"),
        _ => io::const_error!(io::ErrorKind::Other, "the FS server reported a failure"),
    }
}

// --- Paths: a chain of names under the granted directory --------------------------------------

/// Check every component of `path` and count the names that survive, or refuse the path.
///
/// This is where "no global namespace" is enforced, on the client side, before a byte reaches the
/// server. The server enforces it again (it resolves a single component under the handle it is
/// given and nothing else); doing it here as well is not redundant, it is what turns a would-be
/// escape into a legible `io::Error` instead of an `ENOENT` that reads like a missing file.
///
/// Every refusal is [`io::ErrorKind::InvalidFilename`], never `PermissionDenied`: no permission was
/// consulted, and there is no name for what was asked, because this process holds no capability
/// that designates it. The message says which case it was.
///
/// **A nested path is no longer one of them** (milestone 122). It is a chain of names, and [`walk`]
/// spends one attenuated `OPENDIR` per link to reach the last one. **Nor is a leading `/`**
/// (milestone 47's namespace half, 2026-08-18): `/` is the root of *this process's* namespace,
/// which is the directory it was granted, so `/motd` and `motd` name one file and neither can
/// reach anything the grant does not. That is Plan 9's answer rather than DOS's, and here it is
/// less than Plan 9's: a nife process has exactly one root, so there is nothing for the leading
/// slash to select between. What is still refused is what still names nothing: `..` at any
/// position, and a Windows-shaped prefix.
fn count_names(path: &Path) -> io::Result<usize> {
    let mut n = 0;
    for c in path.components() {
        match c {
            // "./motd" is "motd", and "a/./b" is "a/b": the current directory IS the one the walk
            // has reached.
            Component::CurDir => {}
            // `/motd` is `motd`, for the same reason `./motd` is: this process has exactly one
            // root and it is the directory it was granted. The slash selects nothing, because
            // there is nothing else to select; what it buys is that a path a program built from
            // `current_dir()` resolves instead of being refused.
            //
            // **A leading slash is relative to the base the walk was given, and that is a
            // deliberate divergence from `openat`.** POSIX makes an absolute path ignore the
            // `dirfd`; here `Dir::open_file("/x")` resolves under that `Dir` rather than jumping
            // back to the granted root. The Unix rule exists because `/` names one global thing
            // and a `dirfd` is a shortcut into it. Neither half is true here: a handle is a
            // namespace, `/` is the root of whichever one you are resolving in, and a rule that
            // let a name climb out of the handle it was asked of would make a `Dir` useless for
            // the one thing programs reach for it for. It also cannot widen anything either way,
            // since a process that holds a `Dir` holds the root it descended from.
            Component::RootDir => {}
            // A Windows-shaped prefix (`C:`, `\\?\`) is still nothing here, and unlike a leading
            // slash there is no root it could name.
            Component::Prefix(_) => {
                return Err(io::const_error!(
                    io::ErrorKind::InvalidFilename,
                    "a path prefix names nothing here: this process holds a directory \
                     capability, not a drive or a share"
                ));
            }
            // Refused at every position, not just the first. `sub/../other` would be a descent
            // followed by an ascent, and there is no verb for the ascent and no capability that
            // would designate what it reached: a handle names a directory, and nothing on the wire
            // names its parent. This is the property that makes the walk safe by construction
            // rather than by checking, and it is why `cap-primitives` has to work so much harder on
            // Unix (see notes/std/fs-descent.md).
            Component::ParentDir => {
                return Err(io::const_error!(
                    io::ErrorKind::InvalidFilename,
                    "`..` would leave the granted directory, which no capability designates"
                ));
            }
            Component::Normal(part) => {
                let name = part.to_str().ok_or_else(|| {
                    io::const_error!(
                        io::ErrorKind::InvalidFilename,
                        "a file name must be UTF-8 to cross this contract"
                    )
                })?;
                if name.len() > PAGE {
                    return Err(io::const_error!(
                        io::ErrorKind::InvalidFilename,
                        "a name is longer than the page shared with the FS server"
                    ));
                }
                n += 1;
            }
        }
    }
    Ok(n)
}

/// The names in `path`, in order. Only ever called after [`count_names`] has agreed there is
/// nothing else in there, so the `to_str` that cannot fail is dropped rather than unwrapped.
fn names(path: &Path) -> impl Iterator<Item = &str> {
    path.components().filter_map(|c| match c {
        Component::Normal(part) => part.to_str(),
        _ => None,
    })
}

/// A directory handle a request is issued against: [`proto::ROOT`] for the granted directory, or
/// one a descent minted.
///
/// It closes what it minted and never closes `ROOT`, which is not this process's to close: `ROOT`
/// is the directory the endpoint is bound to rather than a handle anybody handed out. The server's
/// table has the bound directory in slot 0 and mints from 1 up, so `handle != ROOT` is exactly
/// "we made this".
struct At(u64);

impl Drop for At {
    fn drop(&mut self) {
        if self.0 != proto::ROOT {
            // A failure has nowhere to go: we are done with the handle either way, and a leaked
            // server-side slot is the cost of a reply we could not have acted on.
            let _ = request(proto::req(proto::CLOSE, self.0, 0), 0);
        }
    }
}

/// One hop: `OPENDIR name` under `parent`, asking for `want` on the child.
///
/// **`want` is the whole of the authority question** and the trap in this half of the PAL. The
/// server intersects the request with the parent's rights and answers `EPERM` when the result comes
/// up *short of the request* (§47's monotonicity is the intersection; the refusal is the server
/// refusing to hand back less than was asked for without saying so). A client cannot read its own
/// capability, so asking for more than the operation needs breaks under every narrowed grant while
/// passing every test that grants the root's full rights, which is the mistake `readdir` records
/// having nearly shipped.
fn descend(p: &mut Page, parent: &At, name: &str, want: u64) -> io::Result<At> {
    p.put(name.as_bytes());
    let handle = request(proto::req(proto::OPENDIR, parent.0, name.len() as u64), want)?;
    Ok(At(handle))
}

/// **Resolve `path` to the directory its last name lives in, plus that name** (milestone 122).
///
/// `File::open("a/b/c")` is `OPENDIR a`, `OPENDIR b`, then `OPEN c` against `b`'s handle. This is
/// `openat` semantics, which is what the contract already was; what this function adds is that a
/// path-shaped standard library can sit on top of it, which is the whole of milestone 122's option
/// A. Unix answered the same question the same way and kept both interfaces, which is why [`Dir`]
/// exists beside this.
///
/// **The rights story, because it looks like a widening and is not.** Every hop asks for
/// `DESCEND | needs`, where `needs` is what the *final* verb requires on the directory it lands in
/// (`filesystem_protocol::verb::TABLE` is the list, and `OPEN`'s "READ or WRITE" is the one row a caller has
/// to resolve from its own `OpenOptions`). Carrying `needs` down the whole chain rather than only
/// asking for it at the end costs nothing that was ever available: a child's rights are its
/// parent's intersected with the request, so a right an ancestor lacks is a right no descendant of
/// it could have had either. The walk therefore asks for exactly the maximum the grant could give
/// at the depth it is going to, and no more.
///
/// **At most two handles are open at once**, because the assignment in the loop drops the previous
/// hop, which closes it. A deep path costs round trips rather than handle-table slots.
fn walk<'a>(p: &mut Page, from: u64, path: &'a Path, needs: u64) -> io::Result<(At, &'a str)> {
    let total = count_names(path)?;
    if total == 0 {
        return Err(io::const_error!(
            io::ErrorKind::InvalidFilename,
            "the granted directory itself is not a file this contract can open"
        ));
    }
    let mut at = At(from);
    let mut seen = 0;
    for name in names(path) {
        seen += 1;
        if seen == total {
            return Ok((at, name));
        }
        at = descend(p, &at, name, fsproto::dir::DESCEND | needs)?;
    }
    // `count_names` counted the same components this loop walks, so the return above always fires.
    Err(io::const_error!(
        io::ErrorKind::InvalidFilename,
        "the granted directory itself is not a file this contract can open"
    ))
}

/// Resolve `path` to a **directory** handle, asking for `want` on it.
///
/// The empty path and `"."` name the granted directory itself, which is [`proto::ROOT`] and costs
/// no message: it is the directory the endpoint is bound to rather than a name inside it. Anything
/// else is [`walk`] followed by one more descent, since the last component of a path a caller wants
/// listed is itself the directory rather than a name inside one.
fn dir_at(p: &mut Page, path: &Path, want: u64) -> io::Result<At> {
    let names_in_path = count_names(path)?;
    if names_in_path == 0 {
        return Ok(At(proto::ROOT));
    }
    // One `OPENDIR` for the whole path when the server takes paths (milestone 606, ruling A); the
    // server descends each step asking for `DESCEND | want`, which is what the walk below sends.
    if names_in_path > 1 && paths_offered() {
        let joined = joined(path);
        match descend(p, &At(proto::ROOT), &joined, want) {
            Err(e) if refused_as_a_path(&e) => {}
            done => return done.inspect(|_| paths_seen()),
        }
        let (parent, name) = walk(p, proto::ROOT, path, want)?;
        let at = descend(p, &parent, name, want)?;
        paths_refused();
        return Ok(at);
    }
    let (parent, name) = walk(p, proto::ROOT, path, want)?;
    descend(p, &parent, name, want)
}

// --- Paths in one request (milestone 606, ruling A) -----------------------------------------
//
// A server since milestone 606 (a directory walk costs what it does on Linux) resolves a relative
// path in one `OPEN` or `OPENDIR`, with exactly the rights the hop-by-hop walk would get
// (`filesystem_protocol::fs::OPEN` has the rule). One that predates it answers `EINVAL` to any
// name with a `/`. So the PAL sends the path, and on `EINVAL` walks it hop by hop instead; if the
// walk then succeeds, the `EINVAL` was the server's age rather than the name, and every later
// request walks without asking again. An `EINVAL` the walk repeats was the name's, and says
// nothing about the server.
//
// 0 = not yet known, 1 = paths work, 2 = this server walks hop by hop.
static PATHS: AtomicU8 = AtomicU8::new(0);

fn paths_offered() -> bool {
    PATHS.load(Ordering::Relaxed) != 2
}

fn paths_seen() {
    PATHS.store(1, Ordering::Relaxed);
}

fn paths_refused() {
    if PATHS.load(Ordering::Relaxed) == 0 {
        PATHS.store(2, Ordering::Relaxed);
    }
}

/// Whether an error to a one-request path could be an older server's refusal of the `/`.
fn refused_as_a_path(e: &io::Error) -> bool {
    PATHS.load(Ordering::Relaxed) == 0 && e.kind() == io::ErrorKind::InvalidInput
}

/// The names in `path` joined by `/`: what one request carries. `count_names` has already refused
/// anything that is not a plain name.
fn joined(path: &Path) -> String {
    let mut out = String::new();
    for (i, name) in names(path).enumerate() {
        if i > 0 {
            out.push('/');
        }
        out.push_str(name);
    }
    out
}

/// The [`dir`] rights an `OPEN` needs on the directory holding the name.
///
/// `filesystem_protocol::verb::TABLE` records `OPEN` as the contract's one "any of" row (`READ` *or* `WRITE`
/// will do), so this is the one place a caller has to say which, and it says it from the options it
/// was handed. Asking for both when the caller only reads would refuse a read through a read-only
/// grant, which is the over-asking trap one level down from the one [`descend`] describes.
///
/// [`dir`]: fsproto::dir
fn open_rights(opts: &OpenOptions) -> u64 {
    let mut want = 0;
    if opts.read {
        want |= fsproto::dir::READ;
    }
    if opts.write || opts.append || opts.truncate {
        want |= fsproto::dir::WRITE;
    }
    want
}

/// `OPEN` the last name in `path`. Returns the server's handle and the size its reply carried (0
/// when it carried none; `filesystem_protocol::fs::OPEN` says why 0 means "not given").
///
/// Since ruling A of milestone 606 (a directory walk costs what it does on Linux) a multi-name
/// path is one `OPEN` with the rights each step asks for in the second word, and a server that
/// refuses it gets the hop-by-hop walk ([`paths_offered`] and its note).
fn open_handle(path: &Path, opts: &OpenOptions) -> io::Result<(u64, u64)> {
    if !is_reachable() {
        return Err(unsupported_err());
    }
    let mut p = page();
    let want = open_rights(opts);
    if count_names(path)? > 1 && paths_offered() {
        let joined = joined(path);
        p.put(joined.as_bytes());
        match open_reply(proto::req(proto::OPEN, proto::ROOT, joined.len() as u64), want) {
            Err(e) if refused_as_a_path(&e) => {}
            done => return done.inspect(|_| paths_seen()),
        }
        let (at, name) = walk(&mut p, proto::ROOT, path, want)?;
        p.put(name.as_bytes());
        let opened = open_reply(proto::req(proto::OPEN, at.0, name.len() as u64), want)?;
        paths_refused();
        return Ok(opened);
    }
    let (at, name) = walk(&mut p, proto::ROOT, path, want)?;
    p.put(name.as_bytes());
    open_reply(proto::req(proto::OPEN, at.0, name.len() as u64), want)
}

/// One `OPEN`, keeping both reply words: the handle, and the size ruling B put in the second.
fn open_reply(w0: u64, want: u64) -> io::Result<(u64, u64)> {
    let (r0, r1) = rt::call(FS, w0, want);
    Ok((reply(r0)?, r1))
}

/// Create the last name in `path` and return the handle. [`open_handle`]'s twin: the wire shape is
/// identical, only the verb and the rights the walk asks for differ, which is why `CREATE` was
/// specified to match `OPEN`.
///
/// **The walk asks for `CREATE` *and* what the caller will do with the file**, and leaving the
/// second half out is a bug the nested case finds and the flat case cannot. A file handle carries
/// `parent & (READ | WRITE)` (`redoxfs_server`'s `create_file_at`, the same arithmetic as `open_file_at`),
/// so a descent that asked only for `CREATE` hands back a directory handle with no `WRITE` in it,
/// which mints a file nobody can write. `std::fs::write("a/b")` created the file and then failed
/// with `EROFS` on its own first write. Against the granted directory it never showed, because
/// nothing is requested there: `ROOT` carries whatever the endpoint carries.
///
/// The server answers `EEXIST` if the name is already there, and this is only ever called after an
/// open reported `NotFound`, so that reply means somebody else created it in between. It surfaces as
/// `AlreadyExists` rather than being retried, because a silent retry would turn a lost race into a
/// caller writing over a file it believes it just made.
fn create_handle(path: &Path, opts: &OpenOptions) -> io::Result<u64> {
    if !is_reachable() {
        return Err(unsupported_err());
    }
    let mut p = page();
    let (at, name) = walk(
        &mut p,
        proto::ROOT,
        path,
        fsproto::dir::CREATE | open_rights(opts),
    )?;
    p.put(name.as_bytes());
    request(proto::req(proto::CREATE, at.0, name.len() as u64), 0)
}

// --- File ------------------------------------------------------------------------------------

/// An open file: a handle the server minted, plus the offset std's positional API keeps on this
/// side (the contract's `READ`/`WRITE` are both explicitly positional, so there is no cursor in
/// the server to get out of step with).
///
/// The offset is an atomic rather than a `Cell` because `std::fs::File` is `Sync` on every
/// platform and this target should not be the exception; on a single-threaded target the atomic
/// costs nothing.
pub struct File {
    handle: u64,
    pos: AtomicU64,
    /// `append` mode: every write goes at the current end of file, so the position is refreshed
    /// from `FSTAT` before each one. Not atomic against another writer (no lock verb in the
    /// contract), which is the same caveat every non-POSIX backend carries.
    append: bool,
    /// The size `OPEN`'s reply carried (milestone 606, ruling B), for sizing a buffer and for
    /// nothing else: [`File::open_size_hint`] is the only reader, and `metadata()` always asks the
    /// server. 0 is "no hint". A write or a truncate through this handle clears it.
    size_hint: AtomicU64,
}

/// A file's metadata, as much of it as the contract carries: the size, whether the name was a
/// directory, and (when it was reached by name) the modification time. Everything else std asks for
/// either does not exist on this service or is not on the wire.
///
/// **A directory's `size` is 0 and that is a placeholder, not a measurement.** `FSTAT` reports one
/// number and the only handles that reach it are files; the directory case is reached from a
/// listing, where `READDIR` said "this one is a directory" and nothing said how big it is.
#[derive(Clone)]
pub struct FileAttr {
    size: u64,
    dir: bool,
    mtime: Mtime,
}

/// **What the contract said about a modification time**, kept until `modified()` asks.
///
/// `GETMTIME` (milestone 47, DECISIONS §112) resolves a **name under a directory handle**; it never
/// takes an open file. So whether a `Metadata` can carry an mtime depends on how it was reached:
///
/// - `std::fs::metadata(path)` and `DirEntry::metadata` walk to the directory holding the name, so
///   they ask, and keep the answer: [`Mtime::Seconds`], or [`Mtime::Refused`] with the reply word.
/// - `File::metadata` and `Dir::metadata` hold a handle and **no name**, so there is nothing to ask
///   by: [`Mtime::Unnamed`].
///
/// **Why not remember the name the `File` was opened by and ask with that?** Because the answer
/// would be about whatever holds that name *now*. A file renamed or unlinked since it was opened
/// would report another file's time, or a `NotFound` for a file the caller is holding open, and
/// `set_times` through the same trick would stamp the wrong file. Unix's `fstat` and `futimens`
/// act on the inode, not the name, and a binding that quietly acted on a name instead would be the
/// silent-degradation shape DECISIONS §42 forbids. The honest fix is a handle-taking form on the
/// contract, which two programs agree on and is therefore calef's; it is proposed in
/// design/roadmap/proposals/an-mtime-for-an-open-file.md.
///
/// A refused `GETMTIME` does **not** fail `metadata()`: the size and the kind are still true, and
/// a `metadata()` that failed because one field could not be read would break every caller that
/// only wanted `is_dir()`. The refusal is kept and reported by `modified()`, which is the call that
/// asked for the fact.
#[derive(Clone, Copy)]
enum Mtime {
    /// Unix seconds, as the server reported them. See `modified()` for what the number means on an
    /// image this system wrote, which is not always a wall-clock time.
    Seconds(u64),
    /// The server (or a caretaker in front of it) refused `GETMTIME`; the reply word, so the error
    /// can be rebuilt with [`reply`] when it is asked for.
    Refused(u64),
    /// Reached through a handle rather than a name, so there was nothing to ask by.
    Unnamed,
}

/// A directory listing, **read whole at `read_dir` time** rather than streamed.
///
/// std hands the caller an iterator it may hold across arbitrary other work, including opening the
/// files it just listed. The listing arrives through the one page shared with the FS server, and
/// that page is behind a lock every other operation also wants, so an iterator that fetched a page
/// per `next()` would either hold the lock across user code or have to reacquire it and cope with
/// the directory having moved under its cursor. Draining it up front costs the names' bytes and
/// makes both problems go away.
///
/// The cost is stated rather than hidden: a huge directory is a `Vec` of every name in it, and the
/// snapshot is from `read_dir` time, so an entry removed before the caller reaches it opens as
/// `NotFound`. That is the ordinary readdir caveat (`filesystem_protocol::fs::READDIR` records the same one
/// for its cursor) rather than something this choice introduced.
pub struct ReadDir {
    /// The path the caller passed, so `DirEntry::path()` can rebuild what they would expect.
    root: PathBuf,
    entries: crate::vec::IntoIter<(OsString, bool)>,
}

pub struct DirEntry {
    root: PathBuf,
    name: OsString,
    dir: bool,
}

#[derive(Clone, Debug)]
pub struct OpenOptions {
    read: bool,
    write: bool,
    append: bool,
    truncate: bool,
    create: bool,
    create_new: bool,
}

/// **The times a `set_times` call asks for** (milestone 64, rank 28). Its own type now rather than
/// the unsupported backend's, whose setters threw the value away, because `SETMTIME_AT` has
/// somewhere to carry a modification time. The access time is kept too, and kept only so
/// [`set_times`] can refuse it loudly: no verb sets one, and dropping it would report success for
/// half of what the caller asked.
#[derive(Copy, Clone, Debug, Default)]
pub struct FileTimes {
    accessed: Option<SystemTime>,
    modified: Option<SystemTime>,
}

impl FileTimes {
    pub fn set_accessed(&mut self, t: SystemTime) {
        self.accessed = Some(t);
    }

    pub fn set_modified(&mut self, t: SystemTime) {
        self.modified = Some(t);
    }
}

/// Permissions do not exist on this service. `readonly` is honestly false (a granted directory
/// endpoint carries WRITE or it does not, and that is a capability, not a mode bit).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FilePermissions {}

/// A name is a file or a directory. Symlinks are not exposed over this contract at all, so
/// `is_symlink` is not "we did not look", it is "there is no such thing here".
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct FileType {
    dir: bool,
}

/// Directories are created with `MKDIR`, which needs no options: the contract has no mode bits to
/// set, and `recursive` is std's own loop over this.
#[derive(Copy, Clone, Debug)]
pub struct DirBuilder {}

impl FileAttr {
    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn perm(&self) -> FilePermissions {
        FilePermissions {}
    }

    pub fn file_type(&self) -> FileType {
        FileType { dir: self.dir }
    }

    /// **`Metadata::modified`** (milestone 64, rank 19), from the `GETMTIME` [`stat`] issued.
    ///
    /// The value is the contract's, Unix seconds, and it is only as true as the server that stamped
    /// it. **A file written on this system carries the FS server's logical clock, not a wall-clock
    /// time**: `redoxfs_server` has no clock grant, so every mutation stamps a counter that starts
    /// at 1 on each mount (notes/touch.md's `BUGS`). Such a file reads as a moment in early 1970,
    /// orders correctly against other writes in the same boot, and orders wrongly against a file
    /// the host tool made. That is recorded where a reader of this PAL meets it (notes/std.md) and
    /// proposed as its own piece of work; it is not something this function can detect, because
    /// `SETMTIME_AT` may legitimately assert a small number too.
    pub fn modified(&self) -> io::Result<SystemTime> {
        match self.mtime {
            Mtime::Seconds(s) => {
                UNIX_EPOCH.checked_add_duration(&crate::time::Duration::from_secs(s)).ok_or_else(
                    || {
                        io::const_error!(
                            io::ErrorKind::InvalidData,
                            "the FS server reported an mtime past what SystemTime can hold"
                        )
                    },
                )
            }
            // `ENOTDIR` is the one refusal that is about the grant's *shape*: a per-file caretaker
            // answers it for all three mtime verbs because it has no directory to resolve a name
            // under (`filesystem_protocol::grant::POLICY`). Reported as what it is rather than as
            // `NotADirectory`, which would read as a statement about the file.
            Mtime::Refused(r0) if fsproto::reply_errno(r0 as i64) == Some(20) => {
                Err(io::const_error!(
                    io::ErrorKind::Unsupported,
                    "this process holds a single-file grant, and the contract reads an mtime by \
                     name under a directory"
                ))
            }
            Mtime::Refused(r0) => match reply(r0) {
                Err(e) => Err(e),
                // Unreachable by construction (`stat` keeps only negative replies here), and answered
                // as an error rather than a time so a future mistake cannot become a fabricated one.
                Ok(_) => Err(io::const_error!(
                    io::ErrorKind::InvalidData,
                    "an mtime refusal was recorded for a reply that was not a refusal"
                )),
            },
            Mtime::Unnamed => Err(io::const_error!(
                io::ErrorKind::Unsupported,
                "the contract reads an mtime by name, and this metadata came from a handle: use \
                 std::fs::metadata(path)"
            )),
        }
    }

    /// No verb reports an access time.
    pub fn accessed(&self) -> io::Result<SystemTime> {
        unsupported()
    }

    /// RedoxFS keeps a creation time, but no verb carries it across the contract.
    pub fn created(&self) -> io::Result<SystemTime> {
        unsupported()
    }
}

impl FilePermissions {
    pub fn readonly(&self) -> bool {
        false
    }

    pub fn set_readonly(&mut self, _readonly: bool) {
        // std's API cannot report a failure here, and this service has no permission bits, so the
        // only honest thing is to do nothing; `set_permissions` (which CAN fail) refuses.
    }
}

impl FileType {
    pub fn is_dir(&self) -> bool {
        self.dir
    }

    pub fn is_file(&self) -> bool {
        !self.dir
    }

    pub fn is_symlink(&self) -> bool {
        false
    }
}

impl DirBuilder {
    pub fn new() -> DirBuilder {
        DirBuilder {}
    }

    /// `MKDIR` under the granted directory. The verb hands back a **capability to the new
    /// directory** (it is descend-with-creation, §47), and std's `create_dir` returns `()`, so the
    /// handle is closed immediately. Nothing is lost by that: a later `read_dir` of the same name
    /// mints another one, and holding this one would be a handle-table slot leaked per `mkdir`.
    pub fn mkdir(&self, p: &Path) -> io::Result<()> {
        if !is_reachable() {
            return Err(unsupported_err());
        }
        let mut page = page();
        let (at, name) = walk(&mut page, proto::ROOT, p, fsproto::dir::CREATE | fsproto::dir::DESCEND)?;
        page.put(name.as_bytes());
        // **The second word asks for rights on the child, and asking for too much is a refusal
        // rather than an attenuation.** The server intersects the request with the parent's rights
        // and answers `EPERM` if the result is smaller than what was asked (§47's monotonicity is
        // the intersection; the refusal is it telling the truth about it). A PAL cannot know what
        // its own directory capability carries, so it must ask for the *minimum the operation
        // needs* or it breaks under every narrowed grant. `create_dir` needs nothing from the
        // handle it gets back, because it closes it.
        let handle = request(proto::req(proto::MKDIR, at.0, name.len() as u64), 0)?;
        let _ = request(proto::req(proto::CLOSE, handle, 0), 0);
        Ok(())
    }
}

impl fmt::Debug for ReadDir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReadDir").field("root", &self.root).finish()
    }
}

impl Iterator for ReadDir {
    type Item = io::Result<DirEntry>;

    fn next(&mut self) -> Option<io::Result<DirEntry>> {
        // The whole listing was fetched by `readdir`, so nothing here can fail. The `io::Result`
        // is std's shape for platforms that read one entry at a time, not a claim that this one
        // does.
        let (name, dir) = self.entries.next()?;
        Some(Ok(DirEntry { root: self.root.clone(), name, dir }))
    }
}

impl DirEntry {
    /// The path a caller would use to reach this entry, which is the path they passed to
    /// `read_dir` with the name appended, so feeding an entry's `path()` straight back to
    /// `File::open` works. It would not if this returned a bare name, because `PathBuf` joining is
    /// std's job and std joins against what it was given.
    ///
    /// **This is the call milestone 122 exists for.** `read_dir(".")` yields `./name` and always
    /// opened; `read_dir("sub")` yields `sub/name`, which is two components, which was refused. A
    /// std program could list a subdirectory and then not open one thing it had just been told was
    /// there, and every walker in the ecosystem is built out of exactly this pair.
    pub fn path(&self) -> PathBuf {
        self.root.join(&self.name)
    }

    pub fn file_name(&self) -> OsString {
        self.name.clone()
    }

    /// **From the listing when it can be, from the name when it must be.** `READDIR` said whether
    /// the entry is a directory, so `file_type` is free and costs no message. Metadata is not on the
    /// listing, so it is [`stat`] of the entry's path, for a directory as for a file: a
    /// directory's size is still the placeholder [`FileAttr`] describes, but its mtime is real, and
    /// `GETMTIME` answers for a directory exactly as for a file. That made a directory entry's
    /// metadata cost a walk where it used to cost nothing, which is the price of a `modified()`
    /// that answers; a caller that only wants the kind should ask `file_type`, as it would anywhere.
    pub fn metadata(&self) -> io::Result<FileAttr> {
        stat(&self.path())
    }

    pub fn file_type(&self) -> io::Result<FileType> {
        Ok(FileType { dir: self.dir })
    }
}

impl OpenOptions {
    pub fn new() -> OpenOptions {
        OpenOptions {
            read: false,
            write: false,
            append: false,
            truncate: false,
            create: false,
            create_new: false,
        }
    }

    pub fn read(&mut self, read: bool) {
        self.read = read;
    }
    pub fn write(&mut self, write: bool) {
        self.write = write;
    }
    pub fn append(&mut self, append: bool) {
        self.append = append;
    }
    pub fn truncate(&mut self, truncate: bool) {
        self.truncate = truncate;
    }
    pub fn create(&mut self, create: bool) {
        self.create = create;
    }
    pub fn create_new(&mut self, create_new: bool) {
        self.create_new = create_new;
    }
}

impl File {
    pub fn open(path: &Path, opts: &OpenOptions) -> io::Result<File> {
        if !opts.read && !opts.write && !opts.append {
            return Err(io::const_error!(
                io::ErrorKind::InvalidInput,
                "an open must ask for read, write, or append"
            ));
        }
        // `CREATE` and `TRUNCATE` exist now (milestone 31 phase 2), so this whole block used to be a
        // refusal and is now a mapping. Creating a *file* was never the thing §27 kept host-side:
        // that was creating a *filesystem*, which needs uuid and getrandom. `Transaction::create_node`
        // is not std-gated, so the server can make a file without entropy ever becoming a userspace
        // dependency, and the read of §27 that conflated the two is corrected there.
        //
        // The order below is POSIX's, and it matters: create-then-truncate, with truncate applied
        // after a successful open of an existing file. `std::fs::write` is
        // `create(true).truncate(true)`, so getting the order wrong would leave the old tail behind
        // on exactly the path that exists to replace a file's contents, which is the day-costing bug
        // this milestone is here to remove.
        let mut opened_size = 0;
        let handle = match open_handle(path, opts) {
            Ok((h, _)) if opts.create_new => {
                // `create_new` means "must not already exist", and it does. Close the handle the open
                // just minted rather than leaking it for the life of the process: the error path is
                // the one nobody exercises, so it is the one that leaks.
                let _ = request(proto::req(proto::CLOSE, h, 0), 0);
                return Err(io::const_error!(
                    io::ErrorKind::AlreadyExists,
                    "the file already exists"
                ));
            }
            Ok((h, size)) => {
                opened_size = size;
                h
            }
            // Not there, and the caller asked for it to be made. This is the case that used to be
            // Unsupported.
            Err(e) if (opts.create || opts.create_new) && e.kind() == io::ErrorKind::NotFound => {
                create_handle(path, opts)?
            }
            Err(e) => return Err(e),
        };

        // Truncate after the open, so a fresh file (already empty) pays nothing and an existing one
        // is emptied before the first write rather than after it.
        if opts.truncate {
            if let Err(e) = request(proto::req(proto::TRUNCATE, handle, 0), 0) {
                let _ = request(proto::req(proto::CLOSE, handle, 0), 0);
                return Err(e);
            }
        }

        // A truncated file's size is 0 whatever the open said, and 0 already means "no hint".
        let hint = if opts.truncate { 0 } else { opened_size };
        let file = File {
            handle,
            pos: AtomicU64::new(0),
            append: opts.append,
            size_hint: AtomicU64::new(hint),
        };
        if opts.append {
            let end = file.file_attr()?.size;
            file.pos.store(end, Ordering::Relaxed);
        }
        Ok(file)
    }

    pub fn file_attr(&self) -> io::Result<FileAttr> {
        let size = request(proto::req(proto::FSTAT, self.handle, 0), 0)?;
        // Anything that got as far as a handle came through `OPEN`, which refuses a directory
        // (`EISDIR`), so this is a file by construction. No mtime: `GETMTIME` takes a name and a
        // handle has none ([`Mtime`] says why the PAL does not re-resolve the one it was opened by).
        Ok(FileAttr { size, dir: false, mtime: Mtime::Unnamed })
    }

    pub fn fsync(&self) -> io::Result<()> {
        // Nothing is buffered on this side, and the server commits a RedoxFS transaction per write
        // (that is what makes a mid-write kill recoverable), so a successful write is already
        // durable and there is nothing left for a sync verb to do.
        Ok(())
    }

    pub fn datasync(&self) -> io::Result<()> {
        self.fsync()
    }

    pub fn lock(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn lock_shared(&self) -> io::Result<()> {
        unsupported()
    }

    pub fn try_lock(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(unsupported_err()))
    }

    pub fn try_lock_shared(&self) -> Result<(), TryLockError> {
        Err(TryLockError::Error(unsupported_err()))
    }

    pub fn unlock(&self) -> io::Result<()> {
        unsupported()
    }

    /// **`File::set_len`** (milestone 64, rank 8). The contract's `TRUNCATE` is POSIX `ftruncate` in
    /// both directions already (`filesystem_protocol::fs::TRUNCATE` says so and the server implements it), and
    /// the size rides in the second word. This was refused here for the same reason the five verbs
    /// in milestone 64's first pass were: the binding was never written, not the verb missing.
    ///
    /// `File::open` already sends this verb with a size of 0 for `OpenOptions::truncate`, so the one
    /// difference between "the file was emptied" and "the file is now n bytes" was the word this
    /// function passes.
    pub fn truncate(&self, size: u64) -> io::Result<()> {
        self.size_hint.store(0, Ordering::Relaxed);
        request(proto::req(proto::TRUNCATE, self.handle, 0), size)?;
        Ok(())
    }

    pub fn read(&self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let pos = self.pos.load(Ordering::Relaxed);
        let want = buf.len().min(PAGE);
        let mut p = page();
        let n = (request(proto::req(proto::READ, self.handle, want as u64), pos)? as usize).min(want);
        p.get(&mut buf[..n]);
        drop(p);
        self.pos.store(pos + n as u64, Ordering::Relaxed);
        Ok(n)
    }

    pub fn read_vectored(&self, bufs: &mut [IoSliceMut<'_>]) -> io::Result<usize> {
        crate::io::default_read_vectored(|b| self.read(b), bufs)
    }

    #[inline]
    pub fn is_read_vectored(&self) -> bool {
        false
    }

    pub fn read_buf(&self, cursor: BorrowedCursor<'_, u8>) -> io::Result<()> {
        crate::io::default_read_buf(|b| self.read(b), cursor)
    }

    pub fn write(&self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        self.size_hint.store(0, Ordering::Relaxed);
        if self.append {
            let end = self.file_attr()?.size;
            self.pos.store(end, Ordering::Relaxed);
        }
        let pos = self.pos.load(Ordering::Relaxed);
        let chunk = buf.len().min(PAGE);
        let mut p = page();
        p.put(&buf[..chunk]);
        let n =
            (request(proto::req(proto::WRITE, self.handle, chunk as u64), pos)? as usize).min(chunk);
        drop(p);
        self.pos.store(pos + n as u64, Ordering::Relaxed);
        Ok(n)
    }

    pub fn write_vectored(&self, bufs: &[IoSlice<'_>]) -> io::Result<usize> {
        crate::io::default_write_vectored(|b| self.write(b), bufs)
    }

    #[inline]
    pub fn is_write_vectored(&self) -> bool {
        false
    }

    pub fn flush(&self) -> io::Result<()> {
        Ok(())
    }

    /// The position is this side's, since the contract's `READ` and `WRITE` both carry an explicit
    /// offset: there is no cursor in the server to get out of step with, and a seek costs no
    /// message at all except `SeekFrom::End`, which needs the size.
    pub fn seek(&self, pos: SeekFrom) -> io::Result<u64> {
        let (base, delta) = match pos {
            SeekFrom::Start(off) => {
                self.pos.store(off, Ordering::Relaxed);
                return Ok(off);
            }
            SeekFrom::Current(off) => (self.pos.load(Ordering::Relaxed), off),
            SeekFrom::End(off) => (self.file_attr()?.size, off),
        };
        // A resulting offset before the start is `InvalidInput`, std's contract, not a wrap-around.
        let target = base.checked_add_signed(delta).ok_or_else(|| {
            io::const_error!(io::ErrorKind::InvalidInput, "cannot seek before the start of a file")
        })?;
        self.pos.store(target, Ordering::Relaxed);
        Ok(target)
    }

    /// **The size `OPEN` reported, as a buffer-sizing hint** (milestone 606, calef's ruling B, form
    /// 2, 2026-09-27). The overlay's patch to `std::fs` asks this before `metadata()` when it sizes
    /// the buffer for `fs::read`, `fs::read_to_string` and `read_to_end`, which is what saves the
    /// `FSTAT` in front of every whole-file read. It is never used to answer `metadata()`: calef's
    /// reason is that a stale hint costs at most one resize, while a stale `metadata()` lies.
    pub fn open_size_hint(&self) -> Option<u64> {
        match self.size_hint.load(Ordering::Relaxed) {
            0 => None,
            size => Some(size),
        }
    }

    pub fn size(&self) -> Option<io::Result<u64>> {
        // `FSTAT` is one message, so answering the size hint is cheaper than the read-until-EOF
        // loop `read_to_string`/`read_to_end` fall back to.
        Some(self.file_attr().map(|a| a.size))
    }

    pub fn tell(&self) -> io::Result<u64> {
        Ok(self.pos.load(Ordering::Relaxed))
    }

    pub fn duplicate(&self) -> io::Result<File> {
        // A handle is a token the server minted for one session; there is no dup verb, and copying
        // the number would forge a second owner of the same handle, including its close.
        unsupported()
    }

    pub fn set_permissions(&self, _perm: FilePermissions) -> io::Result<()> {
        unsupported()
    }

    /// **Refused, and the reason changed on 2026-09-19.** It used to be "no verb sets an mtime",
    /// which stopped being true when milestone 47's `touch` added `SETMTIME_AT`. What is true now is
    /// narrower: that verb takes a **name under a directory**, and an open file has a handle and no
    /// name. Stamping whatever holds the name this file was opened by would set the wrong file's
    /// time after a rename, which is a silent wrong answer rather than a refusal ([`Mtime`] has the
    /// whole argument). `std::fs::set_times(path, ..)` is bound and is what a caller can use; a
    /// handle-taking verb is proposed in design/roadmap/proposals/an-mtime-for-an-open-file.md.
    pub fn set_times(&self, _times: FileTimes) -> io::Result<()> {
        Err(io::const_error!(
            io::ErrorKind::Unsupported,
            "the contract sets an mtime by name, and an open file carries none: use \
             std::fs::set_times(path, ..)"
        ))
    }
}

/// **A directory a program holds** (milestone 122): std's own `openat`-shaped API, `Dir::open` and
/// `Dir::open_file`, bound to the capability it already is.
///
/// This is the half of milestone 122 that matters more than nested paths, and the reason is the
/// mistake it avoids rather than the software it runs. If path composition were the only interface,
/// no program would ever be written to hold a directory and the system's model would be invisible
/// to everything running on it, which is a capability system quietly becoming an ambient one with
/// extra steps.
///
/// **It is not an interface this project invented**, which was the surprise: `std::fs::Dir` exists
/// upstream behind `#![feature(dirfd)]` (rust-lang/rust#120426) and is exactly `openat`. Until this
/// milestone nife got std's generic fallback, which stores a `canonicalize`d path, so `Dir::open`
/// on this system failed at the first call in the first line, and the type most aligned with what
/// this OS *is* was the one type it could not offer. It is also what `cap-std` binds to, which is
/// DECISIONS §84's first preference for porting real software.
///
/// # The rights it asks for, which is the whole design
///
/// Everywhere else in this module the rule is **ask for the minimum the verb needs**, because
/// over-asking is `EPERM` rather than attenuation and a PAL cannot read its own capability. A held
/// directory has no single verb to be the minimum of: it is an object, and what will be asked of it
/// later is not knowable when it is minted.
///
/// So a `Dir` asks for **everything its parent carries**, which is what `openat(dirfd, name,
/// O_DIRECTORY)` hands back on Unix and is the honest analogue. It cannot widen anything: the
/// server's answer is `parent & requested`, so the result is the projection of an authority this
/// process already holds onto one directory inside it.
///
/// Knowing what the parent carries is the awkward part, because **the contract has no way to say
/// "attenuate to whatever you have"** and no verb that reports a handle's rights. The common case
/// costs one message: ask for `dir::ALL`, and a full-rights grant answers with the handle. A
/// narrowed grant answers `EPERM`, and then [`Dir::held_rights`] finds out exactly what is there by
/// asking for one right at a time, one message per bit of `dir::ALL` (seven today), once per
/// `Dir::open` at the first hop only, since every hop after that descends from a parent whose
/// rights are now known.
///
/// That probe is a workaround and is recorded as one: the fix is a sentinel in `OPENDIR`'s rights
/// word meaning "the parent's, whatever they are", which is a contract change and therefore not a
/// lane's to make. See the BUGS section of notes/std.md.
pub struct Dir {
    /// The server's handle, `proto::ROOT` for the granted directory itself.
    at: At,
    /// What [`Self::at`] is known to carry. [`Dir::UNKNOWN`] for the granted directory, whose
    /// rights are the endpoint's and which nothing on the wire reports.
    rights: u64,
}

impl Dir {
    /// The rights of the granted directory: not a mask, a statement that we have not been told.
    /// `u64::MAX` cannot be confused with a real one, since `dir::ALL` is seven bits.
    const UNKNOWN: u64 = u64::MAX;

    /// The granted directory itself, which costs no message: it is what the endpoint is bound to
    /// rather than a name inside anything.
    fn root() -> Dir {
        Dir { at: At(proto::ROOT), rights: Dir::UNKNOWN }
    }

    /// Descend one name, asking for everything this directory carries. See the type's header for
    /// why "everything it carries" is the right ask for an object and the wrong ask for a verb.
    fn child(&self, p: &mut Page, name: &str) -> io::Result<Dir> {
        let want = if self.rights == Dir::UNKNOWN { fsproto::dir::ALL } else { self.rights };
        match descend(p, &self.at, name, want) {
            Ok(at) => Ok(Dir { at, rights: want }),
            // The one recoverable refusal: we asked for more than this capability holds, and the
            // server told us so rather than quietly handing back less (§42). Find out what is
            // actually there and ask again. Only reachable from a directory whose rights we were
            // never told, so it happens at most once per `Dir::open`.
            Err(e) if e.kind() == io::ErrorKind::PermissionDenied && self.rights == Dir::UNKNOWN => {
                let held = self.held_rights(p, name)?;
                let at = descend(p, &self.at, name, held)?;
                Ok(Dir { at, rights: held })
            }
            Err(e) => Err(e),
        }
    }

    /// Which of the rights in `dir::ALL` this directory carries, found by asking for one at a time.
    ///
    /// Each `OPENDIR` here succeeds exactly when the parent holds that one right, because the
    /// server's test is `parent & requested == requested`. One message and one close per bit, and
    /// the handles are released as they are minted rather than held, so this never approaches the
    /// contract's per-session handle budget.
    ///
    /// A parent that cannot be descended at all answers `ENOENT` rather than `EPERM` (§47 withholds
    /// a naming right by denying the name, so a holder cannot map what it may not reach), and that
    /// propagates out of the first probe as an ordinary `NotFound`.
    fn held_rights(&self, p: &mut Page, name: &str) -> io::Result<u64> {
        let mut held = 0;
        // Every bit of `dir::ALL`, lowest first, rather than a list of names. The list this
        // replaced named six rights and `ALL` had carried a seventh (`SETTIME`, milestone 47)
        // since 2026-08-24, so a `Dir` under a narrowed grant silently dropped it. Harmless so far,
        // because no `Dir` operation asks for `SETTIME`; walking the mask is what keeps the next
        // right from being missed the same way when one does.
        let mut rest = fsproto::dir::ALL;
        while rest != 0 {
            let right = rest & rest.wrapping_neg();
            rest &= !right;
            match descend(p, &self.at, name, right) {
                // Dropping the `At` closes it: this was a question, not a capability we wanted.
                Ok(_) => held |= right,
                Err(e) if e.kind() == io::ErrorKind::PermissionDenied => {}
                Err(e) => return Err(e),
            }
        }
        Ok(held)
    }

    /// **Open a directory.** `""` and `"."` are the granted directory and cost nothing; anything
    /// else is one descent per component, each carrying the last one's rights forward.
    ///
    /// The `OpenOptions` are upstream's and are ignored, which is the same answer Unix gives: a
    /// `dirfd` opened `O_RDONLY` still unlinks and still opens a file for writing, because what it
    /// carries is the authority over the directory rather than a mode. Here that authority is
    /// literal.
    pub fn open(path: &Path, _opts: &OpenOptions) -> io::Result<Dir> {
        if !is_reachable() {
            return Err(unsupported_err());
        }
        if count_names(path)? == 0 {
            return Ok(Dir::root());
        }
        let mut p = page();
        let mut cur = Dir::root();
        for name in names(path) {
            cur = cur.child(&mut p, name)?;
        }
        Ok(cur)
    }

    /// **Open a directory with the minimum permission for traversal** (`#![feature(dirfd)]`,
    /// rust-lang/rust#120426, added to the trait every backend implements the day this overlay
    /// was bumped past `nightly-2026-08-26`). Unix answers this with `O_PATH`/`O_SEARCH`: a
    /// handle that can be descended through but not necessarily enumerated.
    ///
    /// **This contract has no such right to ask for.** [`fsproto::dir`] has seven bits
    /// (`ENUMERATE`, `READ`, `WRITE`, `CREATE`, `REMOVE`, `DESCEND`, `SETTIME`) and [`Dir`]'s own
    /// header explains why every open here asks for all of them anyway: a held directory is an
    /// object, not a verb, and there is no wire sentinel meaning "the least you'll let me have". So
    /// the honest answer is [`Dir::open`] itself, which already asks for everything the parent
    /// carries, a superset of "enough to traverse".
    pub fn open_for_traversal(path: &Path) -> io::Result<Dir> {
        let mut opts = OpenOptions::new();
        opts.read(true);
        Dir::open(path, &opts)
    }

    /// **A second handle to the directory this one already holds**, which upstream added to every
    /// backend on `nightly-2026-09-22` (`std::fs::Dir::try_clone`, still `#![feature(dirfd)]`).
    /// Unix answers it with `dup`, Windows with `DuplicateHandle`, and both are cheap because a
    /// descriptor is a copyable reference to an object the kernel owns.
    ///
    /// **Here it is the gap [`Dir::open_dir`] and [`File::duplicate`] already record**, arrived at
    /// from a third direction: a handle is a token the FS server minted for one session, the
    /// contract has no dup verb, and copying the number would forge a second owner of the same
    /// handle, including its close. So this refuses for anything the server minted.
    ///
    /// The granted directory is the exception and costs nothing, for the reason [`Dir::root`]
    /// gives: `proto::ROOT` is a sentinel rather than a minted handle, any number of `Dir`s may
    /// hold it at once, and its `Drop` closes nothing. That is the same split `open_dir("")`
    /// already makes, so the two answers cannot disagree.
    pub fn duplicate(&self) -> io::Result<Dir> {
        if self.at.0 == proto::ROOT { Ok(Dir::root()) } else { Err(unsupported_err()) }
    }

    /// Open a file **under this directory**, which is the call the whole type exists for: the name
    /// is resolved against a capability the program is holding, not against anything ambient. A
    /// nested `path` walks from here exactly as [`walk`] walks from the granted directory.
    pub fn open_file(&self, path: &Path, opts: &OpenOptions) -> io::Result<File> {
        if !opts.read && !opts.write && !opts.append {
            return Err(io::const_error!(
                io::ErrorKind::InvalidInput,
                "an open must ask for read, write, or append"
            ));
        }
        let handle = {
            let mut p = page();
            let (at, name) = walk(&mut p, self.at.0, path, open_rights(opts))?;
            p.put(name.as_bytes());
            request(proto::req(proto::OPEN, at.0, name.len() as u64), 0)?
        };
        if opts.truncate {
            if let Err(e) = request(proto::req(proto::TRUNCATE, handle, 0), 0) {
                let _ = request(proto::req(proto::CLOSE, handle, 0), 0);
                return Err(e);
            }
        }
        let file = File {
            handle,
            pos: AtomicU64::new(0),
            append: opts.append,
            size_hint: AtomicU64::new(0),
        };
        if opts.append {
            let end = file.file_attr()?.size;
            file.pos.store(end, Ordering::Relaxed);
        }
        Ok(file)
    }

    /// What there is to know about a directory over this contract, which is that it is one. The
    /// size is the placeholder [`FileAttr`] describes: no verb reports a directory's size, and
    /// inventing one would be a fact the contract does not carry. `modified()` refuses for
    /// [`Mtime`]'s reason: a held directory is a handle, and `GETMTIME` asks by name. Its parent can
    /// answer for it (`std::fs::metadata` of the path it was opened by).
    pub fn self_metadata(&self) -> io::Result<FileAttr> {
        Ok(FileAttr { size: 0, dir: true, mtime: Mtime::Unnamed })
    }

    /// A name's metadata relative to this directory, the dirfd shape upstream std grew on
    /// 2026-09-30 (`Dir::metadata(path)` in `fs.rs` now reaches here). The same four messages
    /// [`stat`] issues, walked from this handle rather than the root: the authority is identical
    /// and only the starting point differs. An empty relative path is the directory itself, the
    /// same placeholder [`self_metadata`] returns.
    pub fn metadata(&self, path: &Path) -> io::Result<FileAttr> {
        if !is_reachable() {
            return Err(unsupported_err());
        }
        if count_names(path)? == 0 {
            return Ok(FileAttr { size: 0, dir: true, mtime: Mtime::Unnamed });
        }
        stat_under(self.at.0, path)
    }

    /// `symlink_metadata` relative to this directory. No symlink crosses this contract, so
    /// following one and not following one are the same thing, the same line [`lstat`] draws.
    pub fn symlink_metadata(&self, path: &Path) -> io::Result<FileAttr> {
        self.metadata(path)
    }

    /// `UNLINK` a name under this directory. A directory is refused (`IsADirectory`), the same line
    /// the path-shaped [`unlink`] draws.
    pub fn remove_file(&self, path: &Path) -> io::Result<()> {
        let mut p = page();
        let (at, name) = walk(&mut p, self.at.0, path, fsproto::dir::REMOVE)?;
        p.put(name.as_bytes());
        request(proto::req(proto::UNLINK, at.0, name.len() as u64), 0)?;
        Ok(())
    }

    /// **Rename across two held directories**, which is the shape `RENAME` was specified for: the
    /// source handle rides in the request word and the destination's in the second, so this is one
    /// atomic message rather than a copy and an unlink.
    pub fn rename(&self, from: &Path, to_dir: &Dir, to: &Path) -> io::Result<()> {
        let mut p = page();
        let (src_at, src) = walk(&mut p, self.at.0, from, fsproto::dir::REMOVE)?;
        let (dst_at, dst) = walk(&mut p, to_dir.at.0, to, fsproto::dir::CREATE)?;
        if src.len() + dst.len() > PAGE {
            return Err(io::const_error!(
                io::ErrorKind::InvalidFilename,
                "the two names together are longer than the page shared with the FS server"
            ));
        }
        p.put(src.as_bytes());
        p.put_at(src.len(), dst.as_bytes());
        request(
            proto::req(proto::RENAME, src_at.0, src.len() as u64),
            proto::rename_dst(dst_at.0, dst.len() as u64),
        )?;
        Ok(())
    }

    /// `MKDIR` under this directory, the `self`-relative twin of [`DirBuilder::mkdir`] (which
    /// walks from the granted root). Same verb, same close-what-you-don't-keep reasoning: the
    /// handle `MKDIR` hands back is closed immediately, since std's `create_dir` returns `()`
    /// and a later `open_dir`/`read_dir` of the same name mints another one.
    pub fn create_dir(&self, path: &Path) -> io::Result<()> {
        let mut p = page();
        let (at, name) =
            walk(&mut p, self.at.0, path, fsproto::dir::CREATE | fsproto::dir::DESCEND)?;
        p.put(name.as_bytes());
        let handle = request(proto::req(proto::MKDIR, at.0, name.len() as u64), 0)?;
        let _ = request(proto::req(proto::CLOSE, handle, 0), 0);
        Ok(())
    }

    /// **Open a directory relative to this one.** One descent per component, exactly
    /// [`Dir::open`]'s walk, just seeded from `self` instead of the granted root.
    ///
    /// `""`/`"."` is the one case that walk cannot express: it names `self` again, and this
    /// contract has no verb to mint a second handle to a directory `self` already holds (the
    /// same gap [`File::duplicate`] documents). Under the granted directory itself that costs
    /// nothing, because [`Dir::root`] is the `ROOT` sentinel and any number of `Dir`s may hold
    /// it at once (its `Drop` never closes it); anywhere else, honestly, this refuses rather
    /// than inventing a duplicate the server was never asked to mint.
    pub fn open_dir(&self, path: &Path, _opts: &OpenOptions) -> io::Result<Dir> {
        if count_names(path)? == 0 {
            return if self.at.0 == proto::ROOT { Ok(Dir::root()) } else { Err(unsupported_err()) };
        }
        let mut p = page();
        let mut names = names(path);
        let mut cur = self.child(&mut p, names.next().expect("count_names checked above"))?;
        for name in names {
            cur = cur.child(&mut p, name)?;
        }
        Ok(cur)
    }

    /// `RMDIR` under this directory, the `self`-relative twin of the path-shaped [`rmdir`].
    /// Empty-only, same as that one: `DirectoryNotEmpty` is the answer for anything else, and a
    /// file underneath is refused with `NotADirectory`.
    pub fn remove_dir(&self, path: &Path) -> io::Result<()> {
        let mut p = page();
        let (at, name) = walk(&mut p, self.at.0, path, fsproto::dir::REMOVE)?;
        p.put(name.as_bytes());
        request(proto::req(proto::RMDIR, at.0, name.len() as u64), 0)?;
        Ok(())
    }
}

impl fmt::Debug for Dir {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Dir").field("handle", &self.at.0).finish()
    }
}

impl fmt::Debug for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("File").field("handle", &self.handle).finish()
    }
}

impl Drop for File {
    fn drop(&mut self) {
        // `CLOSE` frees the server's handle-table slot. A failure here has nowhere to go and
        // nothing to fix: the process is done with the file either way.
        let _ = request(proto::req(proto::CLOSE, self.handle, 0), 0);
    }
}

// --- Path-level operations --------------------------------------------------------------------

/// A name's metadata: open, `FSTAT`, close, then `GETMTIME`. Four messages instead of one, because
/// the contract has no stat-by-name verb; the effect is the same and the authority is identical (the
/// name still resolves only under the granted directory, and every one of the four needs the same
/// `dir::READ` the walk asks for).
///
/// **A directory answers too** (milestone 64). `OPEN` refuses a directory with `EISDIR`, and that
/// refusal *is* the answer to "what kind of thing is this name", so it is read as one rather than
/// propagated. Without this, `Path::is_dir()` was false for every directory, and
/// `std::fs::create_dir_all` was not idempotent: it recovers from `AlreadyExists` by asking whether
/// the name is already a directory, and got told no.
///
/// **The mtime is asked for by name, after the kind is known** (milestone 64's last pass). The walk
/// has already reached the directory holding the name, so `GETMTIME` against that handle is one
/// more message and needs no right the walk did not already ask for. It is issued after `OPEN`
/// rather than before so that every *error* this function returns is the one it returned before
/// the mtime existed: a refused `GETMTIME` is kept in the [`FileAttr`] and surfaces from
/// `modified()`, never from here (see [`Mtime`]). The four messages are not atomic, so a name
/// replaced between `OPEN` and `GETMTIME` reports the new node's time with the old node's size,
/// which is the same window a Unix program that calls `stat` twice has.
///
/// The size it reports for a directory is 0 and that is a placeholder rather than a measurement,
/// for the reason [`FileAttr`] gives. `accessed`/`created` still refuse, so nothing here invents a
/// fact the contract does not carry.
pub fn stat(path: &Path) -> io::Result<FileAttr> {
    if !is_reachable() {
        return Err(unsupported_err());
    }
    // ``, `.` and `/` name the granted directory, which no `OPEN` can reach: it is the directory
    // the endpoint is bound to rather than a name inside it. Holding the capability at all is the
    // whole of what there is to know about it, and it has no name to ask an mtime by. (`/` used
    // to fall through to the walk and come back `InvalidFilename`, a leftover from before a
    // leading slash named this process's root; `count_names` is the same test `readdir` uses.)
    if count_names(path)? == 0 {
        return Ok(FileAttr { size: 0, dir: true, mtime: Mtime::Unnamed });
    }
    stat_under(proto::ROOT, path)
}

/// The name-shaped stat against whichever directory handle the caller holds: the root endpoint for
/// the free functions above, a [`Dir`]'s handle for the dirfd-shaped ones below. One mechanism,
/// because upstream's `fstatat(dirfd, ...)` and `stat(path)` differ in exactly the starting handle
/// and nothing else, and two copies of the four-message dance would be one too many places to
/// learn that `EISDIR` is an answer.
fn stat_under(at: u64, path: &Path) -> io::Result<FileAttr> {
    let mut p = page();
    let (at, name) = walk(&mut p, at, path, fsproto::dir::READ)?;
    p.put(name.as_bytes());
    let (size, dir) = match request(proto::req(proto::OPEN, at.0, name.len() as u64), 0) {
        Ok(handle) => {
            let size = request(proto::req(proto::FSTAT, handle, 0), 0);
            // Closed before `size?` can return, so an `FSTAT` failure does not leak the handle.
            let _ = request(proto::req(proto::CLOSE, handle, 0), 0);
            (size?, false)
        }
        Err(e) if e.kind() == io::ErrorKind::IsADirectory => (0, true),
        Err(e) => return Err(e),
    };
    // The name again: nothing promises `OPEN`'s reply left the page as it found it.
    p.put(name.as_bytes());
    let (r0, _) = rt::call(FS, proto::req(proto::GETMTIME, at.0, name.len() as u64), 0);
    let mtime = match fsproto::reply_errno(r0 as i64) {
        None => Mtime::Seconds(r0),
        Some(_) => Mtime::Refused(r0),
    };
    Ok(FileAttr { size, dir, mtime })
}

/// No symlinks cross this contract, so following one and not following one are the same thing.
pub fn lstat(path: &Path) -> io::Result<FileAttr> {
    stat(path)
}

pub fn exists(path: &Path) -> io::Result<bool> {
    match stat(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// **List a directory** (`OPENDIR` + `READDIR`, milestone 64).
///
/// Two shapes, and the difference is which directory capability answers:
///
/// - `read_dir(".")` (or `read_dir("")`) lists **the granted directory itself**, which is handle
///   `fs::ROOT` and costs no `OPENDIR`.
/// - `read_dir("sub")` asks the server for a capability to `sub` first, lists through it, and
///   closes it. That is descend-then-enumerate, and it is the only way to reach `sub`: this
///   process holds no name for it other than through its parent.
///
/// The listing is drained here rather than streamed; [`ReadDir`] says why.
pub fn readdir(p: &Path) -> io::Result<ReadDir> {
    if !is_reachable() {
        return Err(unsupported_err());
    }
    // One guard for the whole exchange, because `page()` is not reentrant: every OPENDIR name,
    // every READDIR page, and the CLOSE all run under it. That also makes the listing a snapshot
    // with no other operation of ours interleaved.
    let mut page = page();

    // `ENUMERATE` and nothing more, for the reason `DirBuilder::mkdir` spells out: over-asking is
    // `EPERM`, not attenuation, so a PAL that asked for `dir::ALL` here would work through the
    // test's full-rights grant and fail through every narrowed one. Every hop on the way to a
    // nested path asks for `DESCEND | ENUMERATE`, which is the same minimum one level up.
    let at = dir_at(&mut page, p, fsproto::dir::ENUMERATE)?;
    let handle = at.0;

    let mut entries: Vec<(OsString, bool)> = Vec::new();
    let mut buf = [0u8; PAGE];
    let mut cursor: u64 = 0;
    let result = loop {
        // `req_len` is ignored by READDIR; the cursor rides in the second word.
        let filled = match request(proto::req(proto::READDIR, handle, 0), cursor) {
            Ok(n) => n as usize,
            Err(e) => break Err(e),
        };
        // Zero bytes means the cursor is past the end, which is how the contract tells "the
        // directory ended" apart from "the page was full" without the client guessing.
        if filled == 0 {
            break Ok(());
        }
        let filled = filled.min(PAGE);
        page.get(&mut buf[..filled]);
        let mut this_page = 0u64;
        for (name, dir) in fsproto::dirent::iter(&buf[..filled]) {
            // A name is bytes on the wire and an `OsString` in std, and on this target an
            // `OsString` is UTF-8. A name the FS server holds that is not UTF-8 cannot be
            // represented, so it is skipped rather than lossily renamed: handing back a name that
            // does not open is worse than not listing it.
            if let Ok(s) = core::str::from_utf8(name) {
                entries.push((OsString::from(s), dir));
            }
            this_page += 1;
        }
        if this_page == 0 {
            // A non-empty reply that decoded to no records is a malformed page, and looping on it
            // would spin forever against a server that keeps sending it.
            break Err(io::const_error!(
                io::ErrorKind::InvalidData,
                "the FS server returned a directory page with no readable entries"
            ));
        }
        cursor += this_page;
    };

    // `at` closes what it minted, and never closes `ROOT`.
    drop(at);
    result?;

    Ok(ReadDir { root: p.to_path_buf(), entries: entries.into_iter() })
}

/// **Remove a name** (`UNLINK`, milestone 64). Not revocation: a handle already open on the file
/// keeps reading, exactly as POSIX promises, and `filesystem_protocol::fs::UNLINK` records why the contract
/// draws that line. A directory is refused (`IsADirectory`); [`rmdir`] is its verb.
pub fn unlink(p: &Path) -> io::Result<()> {
    if !is_reachable() {
        return Err(unsupported_err());
    }
    let mut page = page();
    let (at, name) = walk(&mut page, proto::ROOT, p, fsproto::dir::REMOVE)?;
    page.put(name.as_bytes());
    request(proto::req(proto::UNLINK, at.0, name.len() as u64), 0)?;
    Ok(())
}

/// **Remove an empty directory** (`RMDIR`, milestone 64). Empty-only is the safety property, not a
/// missing feature: `DirectoryNotEmpty` is the answer for anything else, and the recursion that
/// Unix's `rm -r` does belongs in userspace where each step can be checked against the capability
/// for that level. A file is refused (`NotADirectory`), the mirror of [`unlink`]'s refusal.
pub fn rmdir(p: &Path) -> io::Result<()> {
    if !is_reachable() {
        return Err(unsupported_err());
    }
    let mut page = page();
    let (at, name) = walk(&mut page, proto::ROOT, p, fsproto::dir::REMOVE)?;
    page.put(name.as_bytes());
    request(proto::req(proto::RMDIR, at.0, name.len() as u64), 0)?;
    Ok(())
}

/// **Move a name** (`RENAME`, milestone 64), and the reason this one matters out of proportion to
/// how often crates name it: write-a-temp-then-rename is how every careful program replaces a file,
/// and `filesystem_protocol::fs::RENAME` is both concurrency-atomic and crash-atomic, which is what that idiom
/// actually needs.
///
/// The two names travel in the shared page back to back, source first, because one request word
/// cannot carry two lengths.
///
/// **`RENAME` is the one verb that names two directories**, and since milestone 122 both of them
/// can be somewhere other than the granted root: each side is walked separately and its handle goes
/// into the request, which is what makes `rename("a/tmp", "b/final")` a single atomic message
/// rather than something a caller has to build out of copy and unlink. The rights asked for differ
/// by side, and that is the contract's rather than a nicety: the source directory loses a name
/// (`REMOVE`) and the destination gains one (`CREATE`).
pub fn rename(old: &Path, new: &Path) -> io::Result<()> {
    if !is_reachable() {
        return Err(unsupported_err());
    }
    let mut page = page();
    let (src_at, src) = walk(&mut page, proto::ROOT, old, fsproto::dir::REMOVE)?;
    let (dst_at, dst) = walk(&mut page, proto::ROOT, new, fsproto::dir::CREATE)?;
    if src.len() + dst.len() > PAGE {
        return Err(io::const_error!(
            io::ErrorKind::InvalidFilename,
            "the two names together are longer than the page shared with the FS server"
        ));
    }
    page.put(src.as_bytes());
    page.put_at(src.len(), dst.as_bytes());
    request(
        proto::req(proto::RENAME, src_at.0, src.len() as u64),
        proto::rename_dst(dst_at.0, dst.len() as u64),
    )?;
    Ok(())
}

/// **Copy a file's contents** (milestone 64, rank 26), as an open, a read/write loop, and two
/// closes. No verb backs this and none needs to: unlike `rename`, which has to be atomic and
/// therefore has to be the server's job, a copy is exactly "read all of one, write all of the
/// other", and doing it here costs the same messages doing it in the caller would.
///
/// Both names resolve under the granted directory, so this cannot copy across a capability boundary;
/// a program holding two directories needs the namespace milestone 47 owns.
///
/// **What it does not copy is permissions**, which every other platform's `fs::copy` does. There are
/// none to copy (§27 has no mode bits), so the usual caveat is inverted: nothing is silently lost,
/// because there was never anything there.
pub fn copy(from: &Path, to: &Path) -> io::Result<u64> {
    let mut reader = OpenOptions::new();
    reader.read(true);
    let src = File::open(from, &reader)?;

    // Refuse a directory before creating anything, so a mistaken `copy("dir", "x")` does not leave
    // an empty `x` behind. `File::open` already answers `EISDIR` for a directory source, so reaching
    // here means the source is a file.
    let mut writer = OpenOptions::new();
    writer.write(true);
    writer.create(true);
    writer.truncate(true);
    let dst = File::open(to, &writer)?;

    let mut buf = [0u8; PAGE];
    let mut copied: u64 = 0;
    loop {
        let n = src.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let mut written = 0;
        while written < n {
            // A short write is not an error and not a loop-exit: the contract's `WRITE` is capped at
            // one page and may return less, so the only correct read of `Ok(0)` here is a full disk,
            // which would otherwise spin.
            match dst.write(&buf[written..n])? {
                0 => {
                    return Err(io::const_error!(
                        io::ErrorKind::WriteZero,
                        "the FS server accepted no bytes; the destination cannot grow"
                    ));
                }
                k => written += k,
            }
        }
        copied += n as u64;
    }
    Ok(copied)
}

pub fn set_perm(_p: &Path, _perm: FilePermissions) -> io::Result<()> {
    unsupported()
}

/// The `nofollow` twin of [`set_perm`], added upstream in the 2026-07-31 nightly.
///
/// Cannot be re-exported from `unsupported_fs` the way `rename` and friends are: that version takes
/// `unsupported_fs::FilePermissions`, and this module defines its own, so the signature does not
/// match. It refuses for the same reason `set_perm` does — permissions are not a thing this contract
/// has (§27), so there is nothing to set and nothing to follow or not follow.
pub fn set_perm_nofollow(_p: &Path, _perm: FilePermissions) -> io::Result<()> {
    unsupported()
}

/// **`std::fs::set_times`** (milestone 64, rank 28): `SETMTIME_AT`, the caller's time, by name.
///
/// **Always `SETMTIME_AT`, never `SETMTIME`, and the rights follow from that.** std hands this
/// function a `SystemTime` the caller chose, which is an assertion about history even when the
/// caller computed it from `SystemTime::now()`; DECISIONS §112 put exactly that authority behind
/// `dir::SETTIME`, separate from `dir::WRITE`. So the walk asks for `WRITE | SETTIME` and a grant
/// without `SETTIME` answers `ReadOnlyFilesystem`. Falling back to `SETMTIME` there would write the
/// server's own "now" in place of the time the caller asked for and report success, which is the
/// one thing this function must not do; and the server's "now" is a logical counter, not a time
/// (see `FileAttr::modified`).
///
/// Three things are refused before any message is sent, so a refusal never leaves half a change
/// behind:
///
/// - **An access time.** No verb sets one. Refusing the whole call is the honest answer to a
///   request this contract can only half carry; setting the mtime and dropping the atime would
///   report success for something that did not happen.
/// - **A time past `i64::MAX` seconds.** The contract's reply word is signed, so `GETMTIME` could
///   never report such a value back; it would read as an error. The server accepts it (§112 does
///   not referee plausibility), and this PAL declines to write a time it cannot read.
/// - **The granted directory itself** (``, `.`, `/`), which has no name to ask by.
///
/// **Whole seconds.** The wire carries seconds and the server stores a zero nanosecond part, so a
/// sub-second part is truncated. That is the same answer a second-granularity filesystem gives on
/// Unix, and std documents timestamp precision as platform-dependent; it is recorded in
/// notes/std/file-times.md's BUGS rather than refused, because refusing every `SystemTime::now()` would make
/// the call unusable for its commonest caller.
///
/// A `FileTimes` with nothing set changes nothing, and still resolves the name, so a missing file
/// is `NotFound` as it is on Unix rather than a vacuous success.
pub fn set_times(p: &Path, times: FileTimes) -> io::Result<()> {
    if !is_reachable() {
        return Err(unsupported_err());
    }
    if times.accessed.is_some() {
        return Err(io::const_error!(
            io::ErrorKind::Unsupported,
            "no verb in the file contract sets an access time; nothing was changed"
        ));
    }
    let Some(modified) = times.modified else {
        return stat(p).map(|_| ());
    };
    if count_names(p)? == 0 {
        return Err(io::const_error!(
            io::ErrorKind::Unsupported,
            "the granted directory has no name to set its mtime by"
        ));
    }
    // `SystemTime` on this target is a `Duration` since the epoch, so this cannot fail; a time
    // before 1970 is not representable here at all, which is the time PAL's own limit.
    let since = modified.sub_time(&UNIX_EPOCH).map_err(|_| {
        io::const_error!(io::ErrorKind::InvalidInput, "an mtime before 1970 cannot cross this contract")
    })?;
    let secs = since.as_secs();
    if secs > i64::MAX as u64 {
        return Err(io::const_error!(
            io::ErrorKind::InvalidInput,
            "an mtime this far out could be written but never read back over this contract"
        ));
    }
    let mut page = page();
    let (at, name) =
        walk(&mut page, proto::ROOT, p, fsproto::dir::WRITE | fsproto::dir::SETTIME)?;
    page.put(name.as_bytes());
    request(proto::req(proto::SETMTIME_AT, at.0, name.len() as u64), secs)?;
    Ok(())
}

/// No symlinks cross this contract, so not following one is [`set_times`] itself.
pub fn set_times_nofollow(p: &Path, times: FileTimes) -> io::Result<()> {
    set_times(p, times)
}
