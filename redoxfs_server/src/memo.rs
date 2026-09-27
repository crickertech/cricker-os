//! **What the server remembers between requests, for as long as nothing has changed**
//! (milestone 606 (a directory walk costs what it does on Linux), provisional number).
//!
//! A walk asks the same questions of the same directories over and over: list `wide`, then open
//! `wide/n000` (find `wide` in the grant, find `n000` in `wide`), then `wide/n001`, and so on. Before
//! this, every one of those went to RedoxFS, and RedoxFS answers each by walking its node tree from
//! the root, five blocks deep, hashing every block it touches. `walk_model` measured it: 1,259
//! requests and 6.4 ms of server time per warm walk on an M-series core, of which seahash was 40%
//! and block copies most of the rest. A Unix kernel answers the same questions from its dentry and
//! page caches. This is those two caches, sized for a server with an 8 MiB heap:
//!
//! - **a directory's listing**, sorted and filtered exactly as `READDIR` sends it, with each
//!   child's node and whether it is a directory. `READDIR` pages come from it, and `OPEN` and
//!   `OPENDIR` look names up in it by binary search instead of `find_node`. A listing is only
//!   memoized whole, so a name it does not hold is a name the directory does not hold.
//! - **a name found one at a time**, in a directory nobody listed: what `OPEN` and `OPENDIR`
//!   learned from `find_node`, so opening `etc/motd` twice asks RedoxFS once. Only names that
//!   exist are kept; a miss asks again.
//! - **a small file's bytes**, read whole on first `READ` and served from memory after. `FSTAT`
//!   answers from it too.
//!
//! # The one rule that keeps it correct
//!
//! The server is the only writer of its image, and every verb that changes the image goes through
//! [`crate::Server`]'s `change` (which forgets everything) or `change_file` (which forgets one file's
//! bytes). Read-only verbs go through `look`, and a verb added later has to say which it is. The
//! one exception is `close`, whose comment says why freeing an unlinked node needs no forgetting. Forgetting everything on a namespace change is coarse
//! on purpose: it makes invalidation one line with nothing to get wrong, and a namespace change is
//! rare next to the reads a walk issues.
//!
//! # BUGS
//!
//! - **Coarse.** One `mkdir` anywhere forgets every listing everywhere. A workload that creates
//!   files while it walks (a build writing objects beside the sources it reads) gets the uncached
//!   cost back after every create. Per-directory invalidation is the refinement, and it is only
//!   worth its extra rules once something measures that workload.
//! - **No eviction policy.** When either budget is full the whole of that half is dropped and
//!   refilled. That is correct and bounded; it is not an LRU, and a working set larger than the
//!   budget thrashes.
//! - **Bytes, not pages.** A file is memoized whole or not at all, up to [`FILE_MAX`]. A large file
//!   is read through RedoxFS every time, as before.
//!
//! Name: provisional 2026-09-26 (milestone 606's lane); calef names modules.

use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

use redoxfs::{Node, TreePtr};
use subtree_scope::Kind;

/// The largest file whose bytes are kept. The walk fixture's largest is 256 KiB, and a source
/// tree's files are almost all smaller.
pub const FILE_MAX: usize = 256 * 1024;

/// The most file bytes kept at once. A quarter of the server's 8 MiB heap; the walk fixture is
/// 334,000 bytes.
pub const FILE_BUDGET: usize = 2 * 1024 * 1024;

/// The most names kept across every listing, about 64 bytes each with their node: roughly 1 MiB.
pub const NAME_BUDGET: usize = 16 * 1024;

/// One child in a memoized listing.
pub struct Child {
    pub name: Box<str>,
    pub ptr: TreePtr<Node>,
    /// What the node is, as `subtree_scope` needs to know it: a symbolic link is kept apart from
    /// a file so that no walk steps through one and no open lands on one (ruling D).
    pub kind: Kind,
}

/// The names found in one directory: name to its node and what it is.
type Found = BTreeMap<Box<str>, (TreePtr<Node>, Kind)>;

/// See the module header.
#[derive(Default)]
pub struct Memo {
    /// Directory node id to its listing, sorted by name.
    listings: BTreeMap<u32, Vec<Child>>,
    /// Names found one at a time in directories that were never listed: what `OPEN` and
    /// `OPENDIR` learn from `find_node`, kept so the next open of the same name does not ask again.
    /// Only names that exist; a miss is asked every time.
    found: BTreeMap<u32, Found>,
    /// Names held across `listings` and `found`, against [`NAME_BUDGET`].
    names: usize,
    /// File node id to its whole contents.
    files: BTreeMap<u32, Vec<u8>>,
    file_bytes: usize,
    /// Files known to be larger than [`FILE_MAX`], so a read of one does not first walk the tree
    /// to learn its size again.
    large: BTreeSet<u32>,
}

impl Memo {
    /// Forget everything: a namespace changed.
    pub fn forget(&mut self) {
        self.listings.clear();
        self.found.clear();
        self.names = 0;
        self.forget_files();
    }

    fn forget_files(&mut self) {
        self.files.clear();
        self.file_bytes = 0;
        self.large.clear();
    }

    /// Forget one file's bytes: it was written, truncated or closed for the last time.
    pub fn forget_file(&mut self, ptr: TreePtr<Node>) {
        if let Some(body) = self.files.remove(&ptr.id()) {
            self.file_bytes -= body.len();
        }
        self.large.remove(&ptr.id());
    }

    /// The listing of the directory `dir`, if it is memoized.
    pub fn listing(&self, dir: TreePtr<Node>) -> Option<&[Child]> {
        self.listings.get(&dir.id()).map(Vec::as_slice)
    }

    /// Keep `children` (already sorted) as `dir`'s listing, dropping every listing first if it
    /// would not fit.
    pub fn keep_listing(&mut self, dir: TreePtr<Node>, children: Vec<Child>) {
        if children.len() > NAME_BUDGET {
            return;
        }
        if self.names + children.len() > NAME_BUDGET {
            self.listings.clear();
            self.found.clear();
            self.names = 0;
        }
        // A listing answers every name, so the names found one at a time in it are dropped.
        if let Some(found) = self.found.remove(&dir.id()) {
            self.names -= found.len();
        }
        self.names += children.len();
        if let Some(old) = self.listings.insert(dir.id(), children) {
            self.names -= old.len();
        }
    }

    /// Keep one name `find_node` found in `dir`.
    pub fn keep_found(&mut self, dir: TreePtr<Node>, name: &str, child: TreePtr<Node>, kind: Kind) {
        if self.listings.contains_key(&dir.id()) {
            return; // the listing already answers it
        }
        if self.names + 1 > NAME_BUDGET {
            self.listings.clear();
            self.found.clear();
            self.names = 0;
        }
        if self
            .found
            .entry(dir.id())
            .or_default()
            .insert(name.into(), (child, kind))
            .is_none()
        {
            self.names += 1;
        }
    }

    /// `name` in `dir`: `None` if nothing is memoized that answers it, `Some(None)` if `dir`'s
    /// listing is memoized and the name is not in it, which is as good as `find_node`'s `ENOENT`.
    pub fn lookup(&self, dir: TreePtr<Node>, name: &str) -> Option<Option<(TreePtr<Node>, Kind)>> {
        let Some(list) = self.listing(dir) else {
            let found = self.found.get(&dir.id())?.get(name)?;
            return Some(Some(*found));
        };
        Some(
            list.binary_search_by(|c| (*c.name).cmp(name))
                .ok()
                .map(|i| (list[i].ptr, list[i].kind)),
        )
    }

    /// A file's memoized bytes.
    pub fn file(&self, ptr: TreePtr<Node>) -> Option<&[u8]> {
        self.files.get(&ptr.id()).map(Vec::as_slice)
    }

    /// Whether a file of `size` bytes is one this would keep.
    pub fn would_keep(size: u64) -> bool {
        size as usize <= FILE_MAX
    }

    /// Whether the file is known to be one this will not keep.
    pub fn is_large(&self, ptr: TreePtr<Node>) -> bool {
        self.large.contains(&ptr.id())
    }

    /// Remember that the file is too large to keep, until it changes.
    pub fn mark_large(&mut self, ptr: TreePtr<Node>) {
        self.large.insert(ptr.id());
    }

    /// Keep `body` as the file's bytes, dropping every file first if it would not fit.
    pub fn keep_file(&mut self, ptr: TreePtr<Node>, body: Vec<u8>) {
        if body.len() > FILE_MAX {
            return;
        }
        if self.file_bytes + body.len() > FILE_BUDGET {
            self.forget_files();
        }
        self.file_bytes += body.len();
        if let Some(old) = self.files.insert(ptr.id(), body) {
            self.file_bytes -= old.len();
        }
    }
}
