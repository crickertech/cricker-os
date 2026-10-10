//! **A distribution's package index: which packages exist, the digest each must hash to, and where
//! its bytes live**, for milestone 801 (packages over the internet), item 1.
//!
//! §250 (an image names its distribution's package index, and the bytes may live anywhere) splits
//! what the image's catalog holds together. The index comes from one pinned host and says what may
//! be installed; a package's bytes come from wherever the index says, over any transport, and are
//! believed only if they hash to the index's digest (§195 (a reviewed recipe vouches for a package)).
//!
//! **The encoding here is a stand-in, not the format.** §250's amendment makes the index a TUF
//! repository, and where basalt's fields and the "moved to" field sit inside TUF's metadata is
//! unruled (`notes/packages/the-index-format.md`). Until then [`Index::parse`] reads the catalog's
//! line with a location after it, and a ruling replaces that function and nothing above it.
//!
//! [`accept`] hands fetched bytes to `package_archive::installable_as` with the entry as a one-line
//! catalog: the progenitor installer's own check, so an index and the image's catalog cannot judge
//! bytes differently. What the index is believed for is how it arrived, over TLS from the one host
//! the image pins (§250 clause 3). The whole account and the BUGS are
//! `notes/packages/over-the-internet.md`.
//!
//! # EXAMPLES
//!
//! ```
//! use package_index::{Index, Scheme};
//!
//! let text = "greeting-0.1.0-aarch64 \
//!     sha256:6d1a1e0cafa7f2bbcf4a5c4c3ff6b6a8b6bd1e0c50f8c64fa2ce1c4bf0c2d8a1 \
//!     https://mirror.example/greeting-0.1.0-aarch64.nifepkg\n";
//! let entry = Index::parse(text).unwrap().find("greeting", "aarch64").unwrap();
//! assert_eq!(entry.stem, "greeting-0.1.0-aarch64");
//! assert_eq!(entry.location.scheme, Scheme::Https);
//! assert_eq!((entry.location.host, entry.location.port), ("mirror.example", 443));
//! ```
//!
//! Name: provisional 2026-10-09 (UTC), milestone 801's lane, with every public item here.

#![cfg_attr(not(test), no_std)]

use measured_boot::{DIGEST_TEXT_LEN, Digest, digest_text, parse_digest};
use package_archive::{CatalogMiss, Installable, Refusal, STEM_LEN, installable_as, matching_stem};

/// **Where the test host serves the stand-in index.** Not a ruling: the index's path and file name
/// on `basalt.nifeos.org` are calef's to rule (§250's unwritten part), and under TUF they would be
/// a repository prefix with TUF's own file names under it. Every caller reaches the path through this
/// constant, so the ruling is one line.
pub const STAND_IN_INDEX_PATH: &str = "/stand-in/index";

/// The longest path a location may carry. `http_response::get_request` writes the request line
/// into a 512-byte buffer beside the host header, so a path near that would not fit a request.
pub const PATH_MAX: usize = 256;

/// How a location is fetched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    /// Plain HTTP. Safe for a package's bytes, because the digest decides.
    Http,
    /// HTTPS.
    Https,
}

/// **Where one package's bytes live**: `http://` or `https://`, a host, an optional port, a path.
/// No user, no fragment. A query is part of the path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location<'a> {
    /// `http` or `https`.
    pub scheme: Scheme,
    /// A DNS name or a dotted IPv4 address, as written.
    pub host: &'a str,
    /// The port written, or the scheme's own (80, 443).
    pub port: u16,
    /// Starts with `/`, at most [`PATH_MAX`] bytes, printable ASCII.
    pub path: &'a str,
}

impl<'a> Location<'a> {
    /// Read one location. `None` for anything that is not exactly the shape above.
    pub fn parse(text: &'a str) -> Option<Location<'a>> {
        let (scheme, rest) = if let Some(rest) = text.strip_prefix("https://") {
            (Scheme::Https, rest)
        } else {
            (Scheme::Http, text.strip_prefix("http://")?)
        };
        let slash = rest.find('/')?;
        let (authority, path) = rest.split_at(slash);
        let (host, port) = match authority.split_once(':') {
            Some((host, port)) => (host, parse_port(port)?),
            None => (
                authority,
                match scheme {
                    Scheme::Http => 80,
                    Scheme::Https => 443,
                },
            ),
        };
        if !is_host(host) || path.len() > PATH_MAX || !path.bytes().all(|b| b.is_ascii_graphic()) {
            return None;
        }
        // A fragment is the browser's and never reaches a server; a location carrying one was not
        // written for a client like this.
        if path.contains('#') {
            return None;
        }
        Some(Location {
            scheme,
            host,
            port,
            path,
        })
    }
}

fn parse_port(text: &str) -> Option<u16> {
    if text.is_empty() || text.len() > 5 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok().filter(|&p| p != 0)
}

/// Letters, digits, hyphens and dots, with no empty label: a DNS name or a dotted IPv4 address.
/// Whether it resolves is the resolver's question, and whether a dotted quad is in range is the
/// connecting side's.
fn is_host(host: &str) -> bool {
    !host.is_empty()
        && host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
}

/// **One package the index lists**: the stem the image catalog would key it by, the digest its
/// whole file must hash to, and where to fetch it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry<'a> {
    /// `name-version-architecture`, as in a catalog line and a package's own header.
    pub stem: &'a str,
    /// What the whole file must hash to.
    pub digest: Digest,
    /// Where its bytes are fetched from.
    pub location: Location<'a>,
}

/// A line [`Index::parse`] refused, and its number from 1, so a client can say which.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BadLine<'a> {
    /// Counted from 1.
    pub number: usize,
    /// The line as it arrived.
    pub line: &'a str,
}

/// Why [`Index::find`] found no one entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Miss {
    /// The index lists no package of that name (at that version) for this architecture.
    NoSuchPackage,
    /// A bare name, and the index lists several versions of it: the rule of milestone 614 (two
    /// installed versions of one program, each runnable).
    SeveralVersions,
}

/// **An index whose every line was read.** One unreadable line refuses the whole index: a client
/// that skipped it would be installing from an index it only partly understood, which is the
/// failure a newer format meeting an older client looks like.
#[derive(Clone, Copy, Debug)]
pub struct Index<'a> {
    text: &'a str,
}

impl<'a> Index<'a> {
    /// Read `text`, refusing it whole at the first line that is not `<stem> <digest> <location>`.
    pub fn parse(text: &'a str) -> Result<Index<'a>, BadLine<'a>> {
        let index = Index { text };
        for (number, line) in index.lines() {
            if entry(line).is_none() {
                return Err(BadLine { number, line });
            }
        }
        Ok(index)
    }

    /// Every entry, in the order the index lists them.
    pub fn entries(&self) -> impl Iterator<Item = Entry<'a>> + 'a {
        let text = self.text;
        Index { text }.lines().filter_map(|(_, line)| entry(line))
    }

    /// **The one entry `name` (or `name@version`) asks for on `architecture`**, by the same rule the
    /// image's catalog lookup uses (`package_archive::matching_stem`).
    pub fn find(&self, name: &str, architecture: &str) -> Result<Entry<'a>, Miss> {
        let stem =
            matching_stem(self.entries().map(|e| e.stem), name, architecture).map_err(|miss| {
                match miss {
                    CatalogMiss::NoSuchPackage => Miss::NoSuchPackage,
                    CatalogMiss::SeveralVersions => Miss::SeveralVersions,
                }
            })?;
        // The same stem twice is one package to `matching_stem`; here it must also be one digest
        // and one location, so the first is the one, and a second line that disagrees is refused.
        let mut found = None;
        for e in self.entries().filter(|e| e.stem == stem) {
            match found {
                None => found = Some(e),
                Some(first) if first == e => {}
                Some(_) => return Err(Miss::SeveralVersions),
            }
        }
        found.ok_or(Miss::NoSuchPackage)
    }

    fn lines(&self) -> impl Iterator<Item = (usize, &'a str)> + 'a {
        self.text
            .split('\n')
            .enumerate()
            .map(|(i, line)| (i + 1, line))
            .filter(|(_, line)| !line.is_empty())
    }
}

fn entry(line: &str) -> Option<Entry<'_>> {
    let mut fields = line.split(' ');
    let (stem, digest, location) = (fields.next()?, fields.next()?, fields.next()?);
    if fields.next().is_some() || stem.is_empty() || stem.len() > STEM_LEN {
        return None;
    }
    Some(Entry {
        stem,
        digest: parse_digest(digest).ok()?,
        location: Location::parse(location)?,
    })
}

/// **May these fetched bytes be installed as `entry`?** The package must be the one the entry names
/// and its whole file must hash to the entry's digest; then its program member must match its own
/// table of contents. That is `package_archive::installable_as`, handed the entry as a one-line
/// catalog, so the decision is the installer's own and not a second copy of it.
pub fn accept<'b>(entry: &Entry<'_>, bytes: &'b [u8]) -> Result<Installable<'b>, Refusal> {
    let mut line = [0u8; STEM_LEN + 1 + DIGEST_TEXT_LEN];
    let stem = entry.stem.as_bytes();
    line[..stem.len()].copy_from_slice(stem);
    line[stem.len()] = b' ';
    let end = stem.len() + 1 + DIGEST_TEXT_LEN;
    line[stem.len() + 1..end].copy_from_slice(&digest_text(&entry.digest));
    // ASCII by construction: a stem `Index::parse` admitted and a digest's text form.
    let catalog = core::str::from_utf8(&line[..end]).map_err(|_| Refusal::NotCataloged)?;
    installable_as(catalog, entry.stem, bytes)
}

/// **The boot test's names, in one place for the programs that must agree on them**:
/// `package_fetch_exerciser` asks, `system_tests/src/user/package_index_tests.rs` grants and judges.
/// The hosts that answer, `helpers/name-server-peer` and `helpers/tls-peer`, spell these again in
/// Python on purpose, as `name_resolution_protocol::fixture` explains.
pub mod fixture {
    /// The stand-in for `basalt.nifeos.org`. `helpers/tls-peer` presents the pinned test
    /// authority's certificate for this name.
    pub const INDEX_HOST: &str = "basalt.test";
    /// `helpers/tls-peer`'s port in every runner. Production would be 443.
    pub const INDEX_PORT: u16 = 8443;
    /// The zone the test grants the client's resolver badge: the index host and the package host.
    pub const ZONE: &str = "basalt.test";
    /// Where the stand-in index says every package's bytes live: plain HTTP, another port, another
    /// name, `helpers/package-http-peer`.
    pub const PACKAGE_HOST: &str = "packages.basalt.test";
    /// A package the index lists and its location serves intact.
    pub const GENUINE: &str = "greeting@0.1.0";
    /// A package the index lists whose location serves a copy with one byte flipped.
    pub const TAMPERED: &str = "uptime@0.1.0";
    /// A package the index does not list, so nothing is fetched.
    pub const ABSENT: &str = "nosuch";
}

#[cfg(test)]
mod tests {
    use measured_boot::sha256;
    use package_archive::{package_size, write_package};

    use super::*;

    fn package(name: &str, version: &str, program: &[u8]) -> Vec<u8> {
        let members: [(&str, &[u8]); 1] = [(name, program)];
        let mut out = vec![0u8; package_size(&members)];
        let attributes = package_archive::Attributes {
            name,
            version,
            architecture: "aarch64",
        };
        write_package(&attributes, &members, &mut out).unwrap();
        out
    }

    fn line(stem: &str, bytes: &[u8], location: &str) -> String {
        let digest = digest_text(&sha256(bytes));
        format!(
            "{stem} {} {location}\n",
            core::str::from_utf8(&digest).unwrap()
        )
    }

    #[test]
    fn a_location_is_a_scheme_a_host_a_port_and_a_path() {
        let l = Location::parse("http://packages.basalt.test:8080/a/b.nifepkg?x=1").unwrap();
        assert_eq!(
            (l.scheme, l.host, l.port, l.path),
            (
                Scheme::Http,
                "packages.basalt.test",
                8080,
                "/a/b.nifepkg?x=1"
            )
        );
        assert_eq!(Location::parse("http://10.0.2.9/p").unwrap().port, 80);
        assert_eq!(Location::parse("https://h/p").unwrap().port, 443);
    }

    #[test]
    fn a_location_that_is_not_that_shape_is_refused() {
        for bad in [
            "ftp://h/p",
            "http://h",
            "http:///p",
            "http://h:0/p",
            "http://h:65536/p",
            "http://h:/p",
            "http://user@h/p",
            "http://-h/p",
            "http://h..x/p",
            "http://h/p q",
            "http://h/p#frag",
            "HTTP://h/p",
        ] {
            assert_eq!(Location::parse(bad), None, "{bad}");
        }
        let long = format!("http://h/{}", "a".repeat(PATH_MAX));
        assert_eq!(Location::parse(&long), None);
    }

    #[test]
    fn find_reads_name_and_version_as_the_catalog_does() {
        let text = format!(
            "{}{}{}",
            line("greeting-0.1.0-aarch64", b"a", "http://h/1"),
            line("greeting-0.2.0-aarch64", b"b", "http://h/2"),
            line("greeting-0.1.0-riscv64", b"c", "http://h/3"),
        );
        let index = Index::parse(&text).unwrap();
        assert_eq!(
            index.find("greeting", "aarch64"),
            Err(Miss::SeveralVersions)
        );
        let e = index.find("greeting@0.2.0", "aarch64").unwrap();
        assert_eq!((e.stem, e.location.path), ("greeting-0.2.0-aarch64", "/2"));
        assert_eq!(
            index.find("greeting", "riscv64").unwrap().location.path,
            "/3"
        );
        assert_eq!(index.find("greeting", "x86_64"), Err(Miss::NoSuchPackage));
        assert_eq!(index.find("nosuch", "aarch64"), Err(Miss::NoSuchPackage));
    }

    #[test]
    fn one_unreadable_line_refuses_the_whole_index() {
        let good = line("greeting-0.1.0-aarch64", b"a", "http://h/1");
        for bad in [
            "greeting-0.1.0-aarch64 sha256:00 http://h/1",
            "greeting-0.1.0-aarch64 http://h/1",
            "greeting-0.1.0-aarch64 sha256:{} http://h/1 extra",
            "moved-to https://elsewhere/index",
        ] {
            let text = format!("{good}{bad}\n");
            let refused = Index::parse(&text).unwrap_err();
            assert_eq!((refused.number, refused.line), (2, bad));
        }
    }

    #[test]
    fn two_lines_for_one_stem_that_disagree_are_refused() {
        let text = format!(
            "{}{}",
            line("greeting-0.1.0-aarch64", b"a", "http://h/1"),
            line("greeting-0.1.0-aarch64", b"a", "http://elsewhere/1"),
        );
        let index = Index::parse(&text).unwrap();
        assert_eq!(
            index.find("greeting", "aarch64"),
            Err(Miss::SeveralVersions)
        );
    }

    /// The property item 1 exists for: the bytes are believed by the index's digest and by nothing
    /// about where they came from.
    #[test]
    fn fetched_bytes_are_accepted_only_if_they_are_the_entry_named() {
        let greeting = package("greeting", "0.1.0", b"\x7fELF greeting");
        let uptime = package("uptime", "0.1.0", b"\x7fELF uptime");
        let text = format!(
            "{}{}",
            line("greeting-0.1.0-aarch64", &greeting, "http://anywhere/g"),
            line("uptime-0.1.0-aarch64", &uptime, "http://anywhere/u"),
        );
        let index = Index::parse(&text).unwrap();
        let entry = index.find("greeting", "aarch64").unwrap();

        let ok = accept(&entry, &greeting).unwrap();
        assert_eq!(ok.program, "greeting");

        let mut tampered = greeting.clone();
        let mid = tampered.len() / 2;
        tampered[mid] ^= 1;
        assert!(matches!(
            accept(&entry, &tampered),
            Err(Refusal::NotCataloged | Refusal::Unreadable)
        ));
        // A package the index also vouches for, served in place of the one asked for.
        assert_eq!(
            accept(&entry, &uptime).map(|i| i.program),
            Err(Refusal::NotRequested)
        );
        assert_eq!(
            accept(&entry, b"not a package").map(|i| i.program),
            Err(Refusal::Unreadable)
        );
    }
}
