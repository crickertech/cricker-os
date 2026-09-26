//! **What a bare word at the prompt runs** (milestone 47 (navigation and naming), DECISIONS §229
//! (how a bare name at the prompt reaches an installed program), B2).
//!
//! A word with a `/` in it is a path and always means that file; that is decided before anything
//! here is asked. A word without one is a builtin, a program the image carries, or the live
//! activation set's entry of that name, found with `activation_set::lookup_name`, which never
//! matches an owner's vouch. There is no search order. A name that is both the image's and an
//! installed package's is refused, naming both, because either choice would be a guess.
//!
//! The shell reads the live table on each line that needs it, rather than caching it. It is two
//! small file reads, against a spawn that builds a whole process, and a table the shell never
//! caches cannot be stale.
//!
//! # EXAMPLES
//!
//! ```
//! use swish::bare::{self, Bare};
//! let table = "greeting greeting-0.1.0-aarch64 \
//!     0000000000000000000000000000000000000000000000000000000000000001\n";
//! let Bare::Installed(path) = bare::resolve(b"greeting", false, Some(table)) else { panic!() };
//! assert_eq!(path.as_bytes(), b"/packages/greeting/0.1.0/greeting");
//! assert!(matches!(bare::resolve(b"uptime", true, Some(table)), Bare::Image));
//! ```
//!
//! # BUGS
//!
//! - Only a plain line runs an installed program by its bare name, as only a plain line runs one by
//!   its path (notes/packages.md's BUGS). A bare installed name in a pipeline is refused as a
//!   program this image does not carry.
//! - A table that cannot be read resolves nothing, so an installed program's bare name is then
//!   refused as unknown and an image program runs without the collision check. The progenitor's
//!   rule is the same: a table it cannot read vouches for nothing.
//!
//! Name: provisional, milestone 47's bare-name lane, 2026-09-26.

/// The longest installed path this builds: `/packages/` and three fields of at most
/// `package_archive::NAME_LEN` (32) bytes each, with their slashes. A path is walked a component at
/// a time, and a component longer than the shell can name fails there, loudly.
pub const PATH_MAX: usize = 10 + 3 * 32 + 2;

/// An installed program's path, from the shell's root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Path {
    bytes: [u8; PATH_MAX],
    len: usize,
}

impl Path {
    /// The path's bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.len]
    }

    fn of(name: &str, version: &str, program: &str) -> Option<Path> {
        let mut p = Path {
            bytes: [0; PATH_MAX],
            len: 0,
        };
        for part in ["/packages/", name, "/", version, "/", program] {
            let end = p.len + part.len();
            p.bytes
                .get_mut(p.len..end)?
                .copy_from_slice(part.as_bytes());
            p.len = end;
        }
        Some(p)
    }
}

/// What a bare word names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bare {
    /// A program the image carries, and no installed program has the name.
    Image,
    /// An installed program, at this path.
    Installed(Path),
    /// Both, which is refused.
    Both(Path),
    /// Neither: the planner refuses it as a program this shell cannot run.
    Unknown,
}

/// **Resolve a bare word.** `image` is whether the image carries a program of that name, and
/// `table` the live generation, when the shell could read one.
pub fn resolve(word: &[u8], image: bool, table: Option<&str>) -> Bare {
    let installed = core::str::from_utf8(word)
        .ok()
        .zip(table)
        .and_then(|(name, table)| activation_set::lookup_name(table, name).ok().flatten())
        .and_then(|entry| {
            let (name, version, _) = activation_set::stem_parts(entry.package)?;
            Path::of(name, version, entry.program)
        });
    match (image, installed) {
        (true, Some(p)) => Bare::Both(p),
        (true, None) => Bare::Image,
        (false, Some(p)) => Bare::Installed(p),
        (false, None) => Bare::Unknown,
    }
}

/// **The refusal for a name that is both**, naming both, and the way to run each.
pub fn write_both(word: &[u8], path: &Path, out: &mut dyn FnMut(&[u8])) {
    out(b"  refused: ");
    out(word);
    out(b" is both a program the image carries and an installed one, at ");
    out(path.as_bytes());
    out(b"\n  run the installed one by that path; the image's is not reachable by this name\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate std;
    use std::vec::Vec;

    const TABLE: &str = "\
greeting greeting-0.1.0-aarch64 0000000000000000000000000000000000000000000000000000000000000001
uptime uptime-0.1.0-aarch64 0000000000000000000000000000000000000000000000000000000000000002
a.out owner 0000000000000000000000000000000000000000000000000000000000000003
";

    /// Each of the four answers, and the one that must never happen: a vouch claiming a name.
    #[test]
    fn a_bare_word_has_one_meaning_or_is_refused() {
        let Bare::Installed(p) = resolve(b"greeting", false, Some(TABLE)) else {
            panic!("an installed name resolves")
        };
        assert_eq!(p.as_bytes(), b"/packages/greeting/0.1.0/greeting");
        let Bare::Both(p) = resolve(b"uptime", true, Some(TABLE)) else {
            panic!("an image name that is also installed is both")
        };
        assert_eq!(p.as_bytes(), b"/packages/uptime/0.1.0/uptime");
        assert_eq!(resolve(b"wc", true, Some(TABLE)), Bare::Image);
        assert_eq!(resolve(b"nope", false, Some(TABLE)), Bare::Unknown);
        // The owner's vouch is found by digest only: its name reaches nothing.
        assert_eq!(resolve(b"a.out", false, Some(TABLE)), Bare::Unknown);
        // No table, no installed names, and the image's run unchecked.
        assert_eq!(resolve(b"uptime", true, None), Bare::Image);
    }

    #[test]
    fn the_refusal_names_both() {
        let Bare::Both(p) = resolve(b"uptime", true, Some(TABLE)) else {
            panic!()
        };
        let mut out = Vec::new();
        write_both(b"uptime", &p, &mut |b| out.extend_from_slice(b));
        let text = std::string::String::from_utf8(out).unwrap();
        assert!(text.contains("uptime is both"), "{text}");
        assert!(text.contains("/packages/uptime/0.1.0/uptime"), "{text}");
    }
}
