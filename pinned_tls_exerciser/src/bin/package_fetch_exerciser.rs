//! **A package fetched through the distribution's index, under calef's five rulings of 2026-10-10
//! (UTC) on #1884**, for milestone 801 (packages over the internet), items 1, 3 and 4.
//!
//! `system_tests/src/user/package_index_tests.rs` starts it holding the network, a clock, entropy
//! and a resolver badge granted the root zone, as `jig`'s is (Q5). It then:
//!
//! | line | what it proves |
//! |---|---|
//! | `index unreachable at gone.basalt.test` | the image's first address is lost, so it tries the second (Q2) |
//! | `index ok ... from basalt.test` | the backup resolved by name, TLS 1.3 to the pinned root, the index read from `/<channel>/metadata/` (Q4) |
//! | `passed over https://packages.basalt.test...` | a listed location resolving to a private address is refused (Q1) |
//! | `fetched greeting-... from basalt.test's targets` | the repository's own copy is the fallback, admitted by the index's digest |
//! | `refused uptime-...: NotCataloged` | a repository copy with one byte flipped: only the digest can tell |
//! | `refused nosuch: not in the index` | nothing is fetched for a name the index does not list |
//! | `passed over https://rebind.basalt.test...` then `fetched rebound-... from basalt.test's targets` | the twin listed under the rebinding name: the client dials only the public address its check passed, never the private one a second resolution would name (milestone 871 (a sixth outsider pass attacks the confinement claim)) |
//!
//! It installs nothing: installing from an index is milestone 809 (the package client becomes a
//! program)'s `jig`. Under slirp every listed location is private, so taking bytes from one is
//! proved only by `package_index`'s host tests.
//!
//! Name: provisional 2026-10-09 (UTC), milestone 801's lane, `pinned_tls_exerciser`'s shape.
//!
//! # BUGS
//!
//! - The fix is a type at the listed path and a test at the rest. A listed location's addresses
//!   come only from `package_index::Location::check`, but nothing stops a future caller from
//!   resolving the name again and handing [`get`] the fresh answer. What catches that is
//!   `package_index_tests`' rebinding assertion, falsified on all three architectures.
//! - Under slirp the fixed client dials the checked public address, 192.0.2.1, for real: a SYN
//!   leaves the emulator for RFC 5737 documentation space, and the net server's connect gives up
//!   within its 15 s bound. That is the honest cost of connecting where the check approved.

use std::net::{SocketAddr, TcpStream, ToSocketAddrs};

use entropy_backend as _;
use package_index::fixture::{
    ABSENT, BACKUP_HOST, GENUINE, INDEX_PORT, PRIMARY_HOST, REBOUND, TAMPERED,
};
use package_index::{
    AddressRefusal, Entry, Index, Location, Miss, PATH_MAX, PROVISIONAL_CHANNEL, Repository,
    STAND_IN_INDEX_FILE, Source, accept,
};
use pinned_tls_client::{PinnedPeer, Session};

/// The test authority `helpers/tls-peer` chains `basalt.test` to. Test-only. Listed locations are
/// pinned to it too: §196 (nife carries TLS) holds one root per source, and which root a listed
/// location's host must chain to is a question this lane put back on #1884.
const PINNED_ROOT: &[u8] = include_bytes!("../../../pinned_tls_client/fixtures/pinned-root.der");

/// One `GET` over TLS, connected to one of `to` and speaking to `server_name`, pinned to the test
/// root. The body, or why not.
///
/// It takes addresses, never a name to resolve: `server_name` is for TLS alone (the server name
/// sent and the name the certificate must carry). Connecting by name resolved a second time, and
/// that second resolution is the rebinding escape milestone 871 (a sixth outsider pass attacks the
/// confinement claim) booted; `TcpStream::connect` on a slice of `SocketAddr` asks no resolver.
fn get(server_name: &str, to: &[SocketAddr], path: &str) -> Result<Vec<u8>, String> {
    let peer = PinnedPeer::new(server_name, PINNED_ROOT).map_err(|e| format!("{e:?}"))?;
    let tcp = TcpStream::connect(to).map_err(|e| format!("{:?}", e.kind()))?;
    let session = Session::handshake(&peer, tcp).map_err(|e| format!("{e:?}"))?;
    let mut body = Vec::new();
    match session.get(path, |part| body.extend_from_slice(part)) {
        Ok(200) => Ok(body),
        Ok(status) => Err(format!("status {status}")),
        Err(e) => Err(format!("{e:?}")),
    }
}

/// Resolve `host` once. A repository's address is the owner's or the image's choice and is not
/// held to Q1, so nothing is checked here; it still resolves once, so [`get`] never resolves.
fn resolve(host: &str, port: u16) -> Result<Vec<SocketAddr>, String> {
    Ok((host, port)
        .to_socket_addrs()
        .map_err(|e| format!("{:?}", e.kind()))?
        .collect())
}

/// `GET` from a repository the owner or the image chose.
fn from_repository(repo: &Repository<'_>, path: &str) -> Result<Vec<u8>, String> {
    get(repo.host, &resolve(repo.host, repo.port)?, path)
}

/// The index from the first of the image's addresses that answers (Q2).
fn fetch_index<'r>(addresses: &'r [Repository<'r>]) -> (String, &'r Repository<'r>) {
    for repo in addresses {
        let mut buf = [0u8; PATH_MAX];
        let path = repo.metadata_path(STAND_IN_INDEX_FILE, &mut buf).unwrap();
        match from_repository(repo, path) {
            Ok(body) => {
                return (
                    String::from_utf8(body).expect("the index is not text"),
                    repo,
                );
            }
            Err(why) => println!("index unreachable at {}: {why}", repo.host),
        }
    }
    panic!("no index address answered");
}

/// A listed location, if every address its host resolves to is public (Q1's safeguard), fetched
/// from exactly those addresses. One resolution is checked and the same one is dialed:
/// `Location::check` hands back the slice it passed, the only addresses this connects to.
fn from_location(at: &Location<'_>) -> Result<Vec<u8>, String> {
    let resolved = resolve(at.host, at.port)?;
    let checked = at.check(&resolved).map_err(|why| match why {
        AddressRefusal::Private(ip) => format!("a private address ({ip})"),
        other => format!("{other:?}"),
    })?;
    get(checked.host(), checked.addresses(), checked.path())
}

/// Ask the index for `name`, try its sources in order, and say what happened.
fn install_check(index: &Index<'_>, from: &Repository<'_>, name: &str, architecture: &str) {
    let entry: Entry<'_> = match index.find(name, architecture) {
        Ok(entry) => entry,
        Err(Miss::NoSuchPackage) => return println!("refused {name}: not in the index"),
        Err(Miss::SeveralVersions) => return println!("refused {name}: several versions"),
    };
    for source in Index::sources(&entry, from, None) {
        let (bytes, whence) = match source {
            Source::Listed(at) => match from_location(&at) {
                Ok(bytes) => (bytes, format!("{}:{}", at.host, at.port)),
                Err(why) => {
                    println!(
                        "passed over https://{}:{}{}: {why}",
                        at.host, at.port, at.path
                    );
                    continue;
                }
            },
            Source::Repository(repo) => {
                let mut buf = [0u8; PATH_MAX];
                let path = repo.target_path(entry.stem, &mut buf).unwrap();
                match from_repository(repo, path) {
                    Ok(bytes) => (bytes, format!("{}'s targets", repo.host)),
                    Err(why) => return println!("refused {}: fetch failed: {why}", entry.stem),
                }
            }
        };
        return match accept(&entry, &bytes) {
            Ok(ok) => println!(
                "fetched {} from {whence}, {} bytes, program {}",
                entry.stem,
                bytes.len(),
                ok.program
            ),
            Err(why) => println!("refused {}: {why:?}", entry.stem),
        };
    }
}

fn main() {
    // The panic-to-exit hook `pinned_tls_exerciser` carries, for its reason.
    std::panic::set_hook(Box::new(|info| {
        println!("PANIC {info}");
        std::process::exit(101);
    }));
    println!("package_fetch_exerciser start");
    let addresses = [
        Repository::new(PRIMARY_HOST, INDEX_PORT, PROVISIONAL_CHANNEL).unwrap(),
        Repository::new(BACKUP_HOST, INDEX_PORT, PROVISIONAL_CHANNEL).unwrap(),
    ];
    let (text, from) = fetch_index(&addresses);
    let index = Index::parse(&text).unwrap_or_else(|bad| panic!("index line refused: {bad:?}"));
    println!(
        "index ok {} packages from {} over TLS",
        index.entries().count(),
        from.host
    );
    let architecture = std::env::consts::ARCH;
    for name in [GENUINE, TAMPERED, ABSENT, REBOUND] {
        install_check(&index, from, name, architecture);
    }
    println!("package_fetch_exerciser done");
}
