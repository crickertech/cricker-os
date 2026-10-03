//! **The version set** (milestone 614 (two installed versions of one program, each runnable, and a
//! caller granted the one it needs), ruling 4; the module's name is provisional, as the ruling says
//! the capability's is): a committed file of `<program> <version>` lines a project carries, which
//! selects, for each program it names, among the versions already installed.
//!
//! The file holds version strings, because people write it; it is resolved to digests through the
//! live table at activation, so enforcement stays digest-authoritative one layer down (ruling 1).
//! That is also why nothing here validates much: a version set vouches for nothing and can run
//! nothing. It selects among rows the table already answers for, so a cloned repository can *ask*
//! and still cannot install or run uninstalled bytes. The shell consults the nearest set at or
//! above the working directory, asdf-style, on calef's usability ruling; that walk is over
//! directories for one file name, not over program names, and DECISIONS §229 (how a bare name at
//! the prompt reaches an installed program) stands: one answer per name, no search order.
//!
//! # EXAMPLES
//!
//! ```
//! const SET: &str = "# this project pins\nuptime 0.1.0\n\nwc 2.4\n";
//! assert_eq!(swish::versions::entry(SET, "uptime"), Some("0.1.0"));
//! assert_eq!(swish::versions::entry(SET, "date"), None);
//! ```
//!
//! # BUGS
//!
//! - **First line wins, silently.** A set that names a program twice is a project's own
//!   contradiction, and the resolution is order, which a reader may not expect. Nothing here
//!   refuses, because a version set vouches for nothing: the worst a contradictory line can do is
//!   select a version the table answers for anyway.
//! - **The dev tool that installs what a set asks for is not built** (ruling 4's follow-on): a
//!   set naming an uninstalled version can only ever produce the divergence notice.
//!
//! Name: provisional, milestone 614's build lane, 2026-09-29.

/// **The file a version set lives in**, found at or above the working directory. Provisional, as
/// the capability and the module are.
pub const FILE: &str = "versions";

/// **The version the set asks for `program`**: its first `<program> <version>` line. Blank lines
/// and `#` comments are skipped, as in the activation set; a line that is not two words is skipped,
/// because a set that cannot be read selects nothing rather than refusing what the table already
/// answers for.
pub fn entry<'a>(set: &'a str, program: &str) -> Option<&'a str> {
    set.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .find_map(|line| {
            let mut words = line.split_whitespace();
            match (words.next(), words.next(), words.next()) {
                (Some(found), Some(version), None) if found == program => Some(version),
                _ => None,
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_set_answers_its_first_line_for_a_program() {
        const SET: &str = "# pinned by the project\nuptime 0.1.0\n\nwc 2.4\n";
        assert_eq!(entry(SET, "uptime"), Some("0.1.0"));
        assert_eq!(entry(SET, "wc"), Some("2.4"));
        assert_eq!(entry(SET, "date"), None);
        assert_eq!(entry(SET, "pinned"), None, "a comment is not a line");
    }

    #[test]
    fn the_first_line_wins_and_malformed_lines_are_skipped() {
        const SET: &str = "uptime 0.1.0\nuptime 0.2.0\n\ngarbage\nuptime 0.3.0 extra\n";
        assert_eq!(entry(SET, "uptime"), Some("0.1.0"));
    }

    #[test]
    fn an_empty_set_selects_nothing() {
        assert_eq!(entry("", "uptime"), None);
        assert_eq!(entry("# nothing yet\n", "uptime"), None);
    }
}
