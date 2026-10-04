//! **One file-service request, decoded and answered** ([`Server::handle`]).
//!
//! This was the body of the EL0 binary's serve loop until the fuzz lane moved it here, unchanged, so
//! a host fuzz target (`fuzz/fuzz_targets/redoxfs_server_session.rs`) and a host test can drive the
//! dispatch the running server runs rather than a copy of its admit-then-dispatch order. The binary
//! keeps only what is IO: receiving the request, finding the client's window, and replying.
//!
//! The two pieces of the dispatch that are IO and not decoding are reached through [`ServeEdges`],
//! so the binary supplies the block-IPC versions and the host supplies its own: the device flush
//! behind [`filesystem_protocol::fs::SYNC`], and the crash injector's count of `WRITE` requests
//! from milestone 37 (prove RedoxFS's crash consistency).
//!
//! Names: `handle`, `ServeEdges` and this module's are provisional, minted by the fuzz lane
//! (`lane/fuzz-service-handlers`) on 2026-10-04 (UTC); an architect names things.

use filesystem_protocol::{blk, fs, op, reply_err, xattr};
use redoxfs::Disk;
use syscall::error::{EINVAL, Error, Result};

use crate::Server;

/// One filesystem block, in bytes: the size of every reply whose length the server chooses.
const BLOCK: usize = blk::BLOCK_SIZE;

/// **The serve loop's IO, as the dispatch reaches it.** Everything else a request does is the
/// [`Server`]'s own logic.
pub trait ServeEdges {
    /// Called as an admitted [`fs::WRITE`] begins, before the server writes. The EL0 binary arms its
    /// crash injector here (milestone 37); a host caller usually does nothing.
    fn note_write(&mut self) {}

    /// Make the device durable, and return the block server's answer untouched: a count of
    /// completed flushes, or a negative errno passed through unmapped (`fs::SYNC` documents why).
    fn sync(&mut self) -> i64;
}

impl<D: Disk> Server<D> {
    /// **Answer one request**: `w0` and `w1` as the client sent them, `badge` as the kernel
    /// delivered it, and `window` the client's channel, which the request's names and bytes are
    /// read from and its reply's bytes written into (milestone 599 (a frame per filesystem client
    /// channel)). Returns the two reply words.
    ///
    /// `window` must be at least [`fs::TRANSFER_MAX`] bytes, which the EL0 binary's mapped window
    /// is exactly. A shorter slice panics on the first verb that reaches past it.
    ///
    /// This is the **only** place a RedoxFS error becomes a wire value ([`reply_err`]); everything
    /// below it speaks `syscall::error::Result`.
    pub fn handle(
        &mut self,
        badge: u64,
        w0: u64,
        w1: u64,
        window: &mut [u8],
        edges: &mut impl ServeEdges,
    ) -> (i64, u64) {
        let code = op(w0);
        // **A bound badge's handles go through `subtree_scope`** (milestone 606 (a directory walk
        // costs what it does on Linux), ruling D). Its `ROOT` is its grant's directory, and any
        // other handle must be one it minted; an unbound badge passes through as it always has.
        // `BIND` and `UNBIND` name a handle of the binder's own, which is unbound by their rule.
        // Closing `ROOT` is refused for a bound badge, as a caretaker refuses it: the grant's root
        // is not the client's to close.
        let raw = fs::req_handle(w0);
        let admitted = if code == fs::BIND || code == fs::UNBIND {
            Ok(raw as u32)
        } else if code == fs::CLOSE && raw == fs::ROOT && self.scoped(badge) {
            Err(Error::new(EINVAL))
        } else {
            self.admit(badge, raw)
        };
        let handle = match admitted {
            Ok(h) => h,
            Err(e) => return (reply_err(e.errno), 0),
        };
        // **Two clamps, and which one a verb gets is the compatibility property** (milestone 138
        // step 3). The channel is `fs::TRANSFER_MAX` bytes now, but a client maps only as much of
        // it as it intends to use, so a reply whose length THIS SERVER chooses must stay inside the
        // one page every client has always mapped: a `READDIR` that filled 64 KiB would be written
        // into a single-page client's unmapped second page. `READ` and `WRITE` are the two verbs
        // whose length the CLIENT chooses, so they are the two that may use the whole channel, and
        // a client asking for one page still gets exactly one page. See `filesystem_protocol::fs::TRANSFER_PAGES`.
        let len = fs::req_len(w0).min(BLOCK);
        let bulk_len = fs::req_len(w0).min(fs::TRANSFER_MAX);
        let offset = w1;

        let result: Result<i64> = match code {
            // The handle field is the **parent directory**, not a file: `fs::ROOT` is the endpoint's
            // bound directory, which is what every client that predates directory handles sends.
            fs::OPEN => match core::str::from_utf8(&window[..len]) {
                // The second word is the rights each directory on a path asks for (milestone
                // 606, ruling A); `fs::OPEN` has the whole rule.
                Ok(name) => self.open_file_path(handle, name, offset).map(|h| h as i64),
                Err(_) => Err(Error::new(EINVAL)),
            },
            // Read straight into the channel, up to the whole of it.
            fs::READ => self
                .read(handle, offset, &mut window[..bulk_len])
                .map(|n| n as i64),
            fs::WRITE => {
                edges.note_write(); // milestone 37: arm the crash if this is the named request
                self.write(handle, offset, &window[..bulk_len])
                    .map(|n| n as i64)
            }
            fs::FSTAT => self.fstat(handle).map(|s| s as i64),
            fs::CLOSE => self.close(handle).map(|()| 0),
            // Same shape as OPEN, deliberately: the name is `len` bytes at the start of the
            // channel, and the reply is a handle. A client that can open can create.
            fs::CREATE => match core::str::from_utf8(&window[..len]) {
                Ok(name) => self.create_file_at(handle, name).map(|h| h as i64),
                Err(_) => Err(Error::new(EINVAL)),
            },
            // **The verbs that hand back authority** (milestone 47). Both share OPEN's shape: the
            // name is `len` bytes at the start of the channel and the reply is a handle. What
            // differs is the second word, which carries the rights the caller is asking the child
            // to have rather than an offset. It is `offset` here only because that is what the
            // wire's second word is called; the server intersects it with the parent's rights and
            // refuses if the answer is smaller than the request.
            fs::OPENDIR | fs::MKDIR => match core::str::from_utf8(&window[..len]) {
                Ok(name) if code == fs::OPENDIR => {
                    self.open_dir(handle, name, offset).map(|h| h as i64)
                }
                Ok(name) => self.make_dir(handle, name, offset).map(|h| h as i64),
                Err(_) => Err(Error::new(EINVAL)),
            },
            // The cursor rides in the second word for TRUNCATE's reason: `len` is clamped to one
            // page above, and a cursor is an index into a directory rather than a payload length.
            // The listing goes into the channel and `r0` says how much of it was filled.
            fs::READDIR => self
                .read_dir(handle, offset as u32, &mut window[..BLOCK])
                .map(|n| n as i64),
            // **The only verb that names two directories**, so the second word is a packed pair
            // (handle, length) rather than a scalar and both names ride in the channel, source
            // first. The page is the bound: a pair of names longer than it is EINVAL here rather
            // than a clamp, because clamping a name is renaming something else.
            fs::RENAME => {
                let dst_len = fs::dst_len(offset);
                if len + dst_len > BLOCK {
                    Err(Error::new(EINVAL))
                } else {
                    let (src, dst) = window[..len + dst_len].split_at(len);
                    match (core::str::from_utf8(src), core::str::from_utf8(dst)) {
                        (Ok(src), Ok(dst)) => self
                            .admit(badge, fs::dst_handle(offset))
                            .and_then(|to| self.rename(handle, src, to, dst))
                            .map(|()| 0),
                        _ => Err(Error::new(EINVAL)),
                    }
                }
            }
            // `rm`'s two verbs. OPEN's shape again (a name at the start of the channel, resolved
            // under the handle), and the reply is 0 rather than a handle: they hand nothing back,
            // which is the whole difference between removing a name and destroying an object.
            // They are one arm because they differ only in the kind they will remove, and that
            // difference is the safety property: `UNLINK` refuses a directory, `RMDIR` refuses a
            // non-empty one, and neither spelling removes whatever it finds.
            fs::UNLINK | fs::RMDIR => match core::str::from_utf8(&window[..len]) {
                Ok(name) if code == fs::UNLINK => self.unlink(handle, name).map(|()| 0),
                Ok(name) => self.rmdir(handle, name).map(|()| 0),
                Err(_) => Err(Error::new(EINVAL)),
            },
            // **Extended attributes** (milestone 57). The handle field is the file or directory the
            // attribute is on rather than a parent directory, which is the one shape difference from
            // OPEN: an attribute has no name in any namespace, so there is nothing to resolve it
            // under.
            fs::GETXATTR => {
                // The name comes in on the page and the value goes back out on it, so the name is
                // copied to the stack before the reply is written over it. 255 bytes against a
                // measured 127 KiB high-water on a 397 KiB grant (notes/fs-server.md).
                let mut name = [0u8; xattr::MAX_NAME];
                if len > name.len() {
                    Err(Error::new(xattr::ERANGE))
                } else {
                    name[..len].copy_from_slice(&window[..len]);
                    // The server refuses a value that will not fit rather than writing past it.
                    self.get_xattr(handle, &name[..len], &mut window[..BLOCK])
                        .map(|(kind, n)| xattr::reply(kind, n))
                }
            }
            // The only verb here carrying two payloads: the name is `len` bytes at the start of the
            // page and the value follows it, with the value's length and its type code packed into
            // the second word. The page is the bound, and a pair that overruns it is EINVAL rather
            // than a clamp, for RENAME's reason: clipping a value stores something nobody wrote.
            fs::SETXATTR => {
                let value_len = xattr::spec_value_len(offset);
                if len + value_len > BLOCK {
                    Err(Error::new(EINVAL))
                } else {
                    let (name, value) = window[..len + value_len].split_at(len);
                    self.set_xattr(handle, name, xattr::spec_kind(offset), value)
                        .map(|()| 0)
                }
            }
            fs::LISTXATTR => self
                .list_xattr(handle, &mut window[..BLOCK])
                .map(|n| n as i64),
            fs::REMOVEXATTR => self.remove_xattr(handle, &window[..len]).map(|()| 0),
            // The new size rides in the second word, NOT in the length field, because it is an
            // offset-shaped quantity: `len` is clamped to one page above, which would silently cap a
            // truncate at 4096 bytes. Reading it from `offset` is what lets a file be truncated to
            // any size the filesystem can hold.
            fs::TRUNCATE => self.truncate(handle, offset).map(|()| 0),
            // **The durability verb** (milestone 55). Two halves in two places on purpose: the
            // rights check is logic and lives in the core, and the device flush is IO and is the
            // caller's [`ServeEdges::sync`]. The reply is the block server's own word (a count of
            // completed device flushes), passed through rather than reduced to a 0, so a client can
            // prove each sync was a fresh round trip; a negative is likewise passed through
            // unmapped, which is the one exception to this function's "map the error once" rule and
            // is argued at both ends.
            fs::SYNC => self.sync_permitted(handle).map(|()| edges.sync()),
            // The reply is a record in the channel and `r0` is its length, [`fs::READDIR`]'s shape:
            // a reply word carries one i64 and this answer is three u64s. Encoding here rather than
            // in the core's verbs keeps them free of the wire's layout, which is the same boundary
            // the errno mapping below sits on.
            fs::STATFS => self.statfs(handle).and_then(|(block, total, free)| {
                filesystem_protocol::statfs::encode(&mut window[..BLOCK], block, total, free)
                    .map(|n| n as i64)
                    .ok_or(Error::new(EINVAL))
            }),
            // **The mtime verbs** (milestone 47's `touch`; DECISIONS §112). All three name-taking,
            // [`fs::UNLINK`]'s shape: the name is `len` bytes at the start of the channel, resolved
            // under the directory `handle` names. The rights split lives in
            // [`Server::mtime`]/[`Server::set_mtime_now`]/[`Server::set_mtime_at`]; this arm is only
            // where the page is cut up, [`fs::GETXATTR`]'s boundary.
            fs::GETMTIME => match core::str::from_utf8(&window[..len]) {
                Ok(name) => self.mtime(handle, name).map(|t| t as i64),
                Err(_) => Err(Error::new(EINVAL)),
            },
            fs::SETMTIME => match core::str::from_utf8(&window[..len]) {
                Ok(name) => self.set_mtime_now(handle, name).map(|()| 0),
                Err(_) => Err(Error::new(EINVAL)),
            },
            // The asserted seconds value rides in the second word, TRUNCATE's reason: `len` is
            // clamped to one page above, which would silently cap a caller's timestamp at a
            // nonsensical range if it rode in the length field instead.
            fs::SETMTIME_AT => match core::str::from_utf8(&window[..len]) {
                Ok(name) => self.set_mtime_at(handle, name, offset).map(|()| 0),
                Err(_) => Err(Error::new(EINVAL)),
            },
            // Ruling D's two control verbs; `fs::BIND` has the rules, `subtree_scope` enforces them.
            fs::BIND => self.bind(badge, handle, offset).map(|()| 0),
            fs::UNBIND => self.unbind(badge, offset).map(|()| 0),
            _ => Err(Error::new(EINVAL)),
        };

        // A handle a bound badge minted is that badge's, and no other badge can name it.
        if let (fs::OPEN | fs::OPENDIR | fs::CREATE | fs::MKDIR, Ok(h)) = (code, &result) {
            self.claim(*h as u32, badge);
        }
        // `OPEN`'s reply carries the file's size in its second word (milestone 606, ruling B), so
        // a client reading the whole file needs no `FSTAT` to size its buffer. Every other verb's
        // second reply word is 0, as it always was.
        let size = match (code, &result) {
            (fs::OPEN, Ok(h)) => self.fstat(*h as u32).unwrap_or(0),
            _ => 0,
        };
        // The one error-mapping site: RedoxFS's Error -> the negated-errno wire value.
        (result.unwrap_or_else(|e| reply_err(e.errno)), size)
    }
}
