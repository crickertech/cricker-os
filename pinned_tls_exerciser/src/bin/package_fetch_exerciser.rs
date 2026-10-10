//! **A package fetched through the distribution's index: the index by host name over pinned TLS,
//! the bytes from wherever it says over plain HTTP, believed only by the index's digest**, for
//! milestone 801 (packages over the internet), items 1, 3 and 4 together.
//!
//! `system_tests/src/user/package_index_tests.rs` starts it holding the network, a clock, entropy
//! and a resolver badge granted `package_index::fixture::ZONE`. It then:
//!
//! | line | what it proves |
//! |---|---|
//! | `index ok ...` | `basalt.test` resolved through the grant, TLS 1.3 to the pinned root, the stand-in index read |
//! | `fetched greeting-...` | the bytes came from another host the index named, and the index's digest admitted them |
//! | `refused uptime-...: NotCataloged` | a location serving a tampered copy: only the digest can tell |
//! | `refused nosuch: not in the index` | nothing is fetched for a name the index does not list |
//!
//! It installs nothing. The installer is the progenitor's, and taking an index's word for what may
//! be installed is milestone 809 (the package client becomes a program)'s `jig`.
//!
//! Name: provisional 2026-10-09 (UTC), milestone 801's lane, `pinned_tls_exerciser`'s shape.

use std::io::{Read, Write};
use std::net::TcpStream;

use entropy_backend as _;
use package_index::fixture::{ABSENT, GENUINE, INDEX_HOST, INDEX_PORT, TAMPERED};
use package_index::{Entry, Index, Miss, STAND_IN_INDEX_PATH, Scheme, accept};
use pinned_tls_client::{PinnedPeer, Session};

/// The test authority `helpers/tls-peer` chains `basalt.test` to. Test-only.
const PINNED_ROOT: &[u8] = include_bytes!("../../../pinned_tls_client/fixtures/pinned-root.der");

/// The index, over TLS from the pinned host, reached by name.
fn fetch_index() -> String {
    let peer = PinnedPeer::new(INDEX_HOST, PINNED_ROOT).expect("the test pin is well formed");
    let tcp = TcpStream::connect((INDEX_HOST, INDEX_PORT))
        .unwrap_or_else(|e| panic!("could not reach {INDEX_HOST}:{INDEX_PORT} by name: {e}"));
    let session = Session::handshake(&peer, tcp)
        .unwrap_or_else(|e| panic!("the index host's handshake was refused: {e:?}"));
    let mut body = Vec::new();
    let status = session
        .get(STAND_IN_INDEX_PATH, |part| body.extend_from_slice(part))
        .unwrap_or_else(|e| panic!("the index request failed: {e:?}"));
    assert_eq!(status, 200, "the index host answered {status}");
    String::from_utf8(body).expect("the index is not text")
}

/// One package's bytes from where its entry says, over plain HTTP. An HTTPS location would need a
/// pin for that host, and §250 (an image names its distribution's package index) pins only the
/// index host, so it is refused here (see BUGS in
/// `notes/packages/over-the-internet.md`).
fn fetch_bytes(entry: &Entry<'_>) -> Result<Vec<u8>, String> {
    let at = entry.location;
    if at.scheme == Scheme::Https {
        return Err("an https location, and this client pins only the index host".into());
    }
    let mut tcp = TcpStream::connect((at.host, at.port)).map_err(|e| format!("{:?}", e.kind()))?;
    let mut request = [0u8; 512];
    let n = http_response::get_request(at.host, at.path, &mut request)
        .ok_or("the request did not fit")?;
    tcp.write_all(&request[..n]).map_err(|e| format!("{:?}", e.kind()))?;
    let mut response = http_response::Response::new();
    let mut body = Vec::new();
    let mut buf = vec![0u8; 16 * 1024];
    while !response.is_complete() {
        let got = tcp.read(&mut buf).map_err(|e| format!("{:?}", e.kind()))?;
        if got == 0 {
            return Err("the connection ended before the body did".into());
        }
        let part = response.feed(&buf[..got]).map_err(|e| format!("{e:?}"))?;
        body.extend_from_slice(part);
    }
    match response.status() {
        Some(200) => Ok(body),
        other => Err(format!("status {other:?}")),
    }
}

/// Ask the index for `name` on this architecture, fetch it, and say what happened.
fn install_check(index: &Index<'_>, name: &str, architecture: &str) {
    let entry = match index.find(name, architecture) {
        Ok(entry) => entry,
        Err(Miss::NoSuchPackage) => return println!("refused {name}: not in the index"),
        Err(Miss::SeveralVersions) => return println!("refused {name}: several versions"),
    };
    let bytes = match fetch_bytes(&entry) {
        Ok(bytes) => bytes,
        Err(why) => return println!("refused {}: fetch failed: {why}", entry.stem),
    };
    match accept(&entry, &bytes) {
        Ok(ok) => println!(
            "fetched {} from {}:{}, {} bytes, program {}",
            entry.stem,
            entry.location.host,
            entry.location.port,
            bytes.len(),
            ok.program
        ),
        Err(why) => println!("refused {}: {why:?}", entry.stem),
    }
}

fn main() {
    // The panic-to-exit hook `pinned_tls_exerciser` carries, for its reason.
    std::panic::set_hook(Box::new(|info| {
        println!("PANIC {info}");
        std::process::exit(101);
    }));
    println!("package_fetch_exerciser start");
    let text = fetch_index();
    let index = Index::parse(&text).unwrap_or_else(|bad| panic!("index line refused: {bad:?}"));
    println!(
        "index ok {} packages from {INDEX_HOST} over TLS",
        index.entries().count()
    );
    let architecture = std::env::consts::ARCH;
    for name in [GENUINE, TAMPERED, ABSENT] {
        install_check(&index, name, architecture);
    }
    println!("package_fetch_exerciser done");
}
