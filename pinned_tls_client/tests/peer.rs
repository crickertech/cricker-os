//! **The client against a TLS implementation that is not ours**, on the host, in milliseconds.
//!
//! Each test starts `helpers/tls-peer` (Python's `ssl`, which is OpenSSL) as a child process and
//! speaks TLS over its stdin and stdout, which is exactly how QEMU's slirp runs the same script per
//! connection for the in-guest test. So the peer a guest meets and the peer these tests meet are
//! one program, and a refusal here is a refusal there.
//!
//! What each test proves that nothing else would:
//!
//! - the handshake completes against a real server, through this tree's provider, and a body
//!   arrives intact (the provider had never completed a handshake before this milestone);
//! - a well-formed chain from another root is refused, which is the pin;
//! - a valid chain from the pinned root for another name is refused, which is the name check;
//! - the production pin is the real ISRG Root X1 and not a substituted file.

use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use pinned_tls_client::{Error, PinnedPeer, Session};

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn pinned_root() -> Vec<u8> {
    std::fs::read(repository().join("pinned_tls_client/fixtures/pinned-root.der")).unwrap()
}

/// `helpers/tls-peer` on a pipe: one connection, as slirp would make it.
struct Peer {
    child: Child,
    to: ChildStdin,
    from: ChildStdout,
}

impl Peer {
    fn start() -> Self {
        let mut child = Command::new("python3")
            .arg(repository().join("helpers/tls-peer"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("python3 is needed to run helpers/tls-peer");
        let to = child.stdin.take().unwrap();
        let from = child.stdout.take().unwrap();
        Self { child, to, from }
    }
}

impl Read for Peer {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.from.read(buf)
    }
}

impl Write for Peer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.to.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.to.flush()
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn the_pinned_peer_answers_over_tls_1_3() {
    let peer = PinnedPeer::new("basalt.test", &pinned_root()).unwrap();
    let session = Session::handshake(&peer, Peer::start()).expect("the handshake was refused");
    // X25519 is the provider's first group, so it is the key share the client sends and the one a
    // server that supports it takes in one round trip.
    assert_eq!(
        session.key_exchange_group(),
        Some(rustls::NamedGroup::X25519)
    );
    assert!(session.cipher_suite().is_some());
    let mut body = Vec::new();
    let status = session
        .get("/index", |part| body.extend_from_slice(part))
        .unwrap();
    assert_eq!(status, 200);
    assert_eq!(
        body,
        b"basalt test index: served over TLS 1.3 by helpers/tls-peer\n"
    );
}

#[test]
fn a_large_body_arrives_intact_across_many_records() {
    // 1 MiB is 64 TLS records of 16 KiB, so this crosses every record boundary the reader has.
    const N: usize = 1 << 20;
    let peer = PinnedPeer::new("basalt.test", &pinned_root()).unwrap();
    let session = Session::handshake(&peer, Peer::start()).unwrap();
    let mut body = Vec::with_capacity(N);
    let status = session
        .get(&format!("/bytes/{N}"), |part| body.extend_from_slice(part))
        .unwrap();
    assert_eq!(status, 200);
    assert_eq!(body.len(), N);
    assert!(body.iter().enumerate().all(|(i, &b)| b == i as u8));
}

#[test]
fn a_chain_from_another_root_is_refused() {
    // The peer presents a well-formed chain for exactly the name asked for, from an authority this
    // client was never given. That it is refused is the pin.
    let peer = PinnedPeer::new("stranger.test", &pinned_root()).unwrap();
    match Session::handshake(&peer, Peer::start()) {
        Err(Error::Tls(rustls::Error::InvalidCertificate(
            rustls::CertificateError::UnknownIssuer,
        ))) => {}
        Err(other) => panic!("refused for the wrong reason: {other:?}"),
        Ok(_) => panic!("a chain from an unpinned root was accepted"),
    }
}

#[test]
fn the_stranger_chain_is_well_formed_so_its_refusal_is_the_pin() {
    // The control for the test above. Pinning the stranger's own root admits the same chain, so
    // that refusal was about which root, and not a malformed fixture refused for any reason.
    let root =
        std::fs::read(repository().join("pinned_tls_client/fixtures/stranger-root.der")).unwrap();
    let peer = PinnedPeer::new("stranger.test", &root).unwrap();
    let session =
        Session::handshake(&peer, Peer::start()).expect("the stranger's own root refused it");
    assert_eq!(session.get("/index", |_| {}).unwrap(), 200);
}

#[test]
fn a_valid_chain_for_another_name_is_refused() {
    // The peer presents the pinned authority's chain for basalt.test to a client that asked for
    // another name. Every signature is good; only the name is wrong.
    let peer = PinnedPeer::new("elsewhere.test", &pinned_root()).unwrap();
    match Session::handshake(&peer, Peer::start()) {
        Err(Error::Tls(rustls::Error::InvalidCertificate(
            rustls::CertificateError::NotValidForName
            | rustls::CertificateError::NotValidForNameContext { .. },
        ))) => {}
        Err(other) => panic!("refused for the wrong reason: {other:?}"),
        Ok(_) => panic!("a certificate for basalt.test was accepted for elsewhere.test"),
    }
}

#[test]
fn the_production_pin_is_isrg_root_x1() {
    use sha2::Digest;
    let digest = sha2::Sha256::digest(pinned_tls_client::ISRG_ROOT_X1);
    assert_eq!(digest.as_slice(), pinned_tls_client::ISRG_ROOT_X1_SHA256);
    let peer = PinnedPeer::basalt_index();
    assert_eq!(peer.host_name(), "basalt.nifeos.org");
}

#[test]
fn a_pin_names_a_host_not_an_address() {
    assert!(matches!(
        PinnedPeer::new("10.0.2.9", &pinned_root()),
        Err(Error::HostName)
    ));
    assert!(matches!(
        PinnedPeer::new("basalt.test", b"not a certificate"),
        Err(Error::Root)
    ));
}

/// **Measurement, not a gate: real Let's Encrypt hosts reach the production root.** Ignored by
/// default because it needs the internet; `cargo test -- --ignored` runs it.
///
/// basalt.nifeos.org does not exist yet (§250 (an image names its distribution's package index)'s `BUGS`: creating it is calef's act), so this pins
/// ISRG Root X1 for two hosts that stand in for it. `letsencrypt.org` serves the ECDSA hierarchy
/// (`YE2`, `Root YE`, `ISRG Root X2` cross-signed by X1); `pages.github.com` is GitHub Pages (its certificate is `*.github.io`), which
/// is what §250 says will serve basalt's index, and served the RSA one (`YR1`, `Root YR`
/// cross-signed by X1) when measured 2026-10-06 (UTC). Each run reports which suite it spoke.
#[test]
#[ignore = "needs the internet"]
fn real_lets_encrypt_hosts_chain_to_isrg_root_x1() {
    for host in ["letsencrypt.org", "pages.github.com"] {
        let peer = PinnedPeer::new(host, pinned_tls_client::ISRG_ROOT_X1).unwrap();
        let tcp = std::net::TcpStream::connect((host, 443)).unwrap();
        let started = std::time::Instant::now();
        let session = Session::handshake(&peer, tcp)
            .unwrap_or_else(|e| panic!("{host} did not chain to ISRG Root X1: {e:?}"));
        eprintln!(
            "{host}: {:?} over {:?}, handshake {:?}",
            session.cipher_suite(),
            session.key_exchange_group(),
            started.elapsed()
        );
    }
}
