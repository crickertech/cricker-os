//! **A TLS client that trusts one root for one host name, and nothing else**, for milestone 501
//! (a TLS client that speaks to one pinned peer).
//!
//! DECISIONS §196 (nife carries TLS) clause 4 rules the shape: *"the package client holds one root
//! or one pinned key for the one source it talks to"*, not a system trust store, because a store
//! has to be updated independently of the system and the thing that updates it is the package
//! manager. §250 (an image names its distribution's package index) names the pair: ISRG Root X1,
//! for `basalt.nifeos.org`. This crate is that shape in types.
//!
//! # The pin is the only way in
//!
//! A [`Session`] is made from a [`PinnedPeer`] and nothing else, and a `PinnedPeer` is exactly one
//! host name and exactly one root. There is no root store in this API to add a second root to, no
//! verifier to swap, and no "accept anyway" switch, so a caller cannot widen what the client
//! trusts without changing this crate. That is the strongest rung this tree has (CLAUDE.md's
//! ladder: make the wrong state unrepresentable), and it is cheap here because `rustls` already
//! refuses everything outside the roots it is given.
//!
//! What the client checks, and where each check lives:
//!
//! | check | done by |
//! |---|---|
//! | the chain ends at the pinned root | `rustls-webpki`, against a store holding one anchor |
//! | each signature in the chain | `cryptography_provider`'s verification algorithms |
//! | the leaf names the host asked for | `rustls-webpki` |
//! | the server holds the leaf's key | `rustls`, over the handshake transcript |
//! | the certificates are in date | `rustls-webpki`, against `std`'s clock: see `BUGS` |
//! | TLS 1.3 and nothing older | `cryptography_provider` offers no TLS 1.2 suite at all |
//!
//! # What it does not buy
//!
//! Integrity of a package. Under §195 (a reviewed recipe vouches for a package) a package's digest
//! decides whether its bytes may run, over any transport. TLS here protects the index, which is
//! where the digests come from, and keeps the path from learning what was asked for. §196 says so
//! in its own words and so does milestone 501's block.
//!
//! # EXAMPLES
//!
//! ```no_run
//! use pinned_tls_client::{PinnedPeer, Session};
//! use std::net::TcpStream;
//!
//! // The production pin: basalt's index host and the one root it must chain to (§250).
//! let peer = PinnedPeer::basalt_index();
//! // No resolver yet (milestone 384), so the caller supplies the address.
//! let tcp = TcpStream::connect("192.0.2.1:443").unwrap();
//! let mut session = Session::handshake(&peer, tcp).unwrap();
//! let mut index = Vec::new();
//! let status = session.get("/index", |body| index.extend_from_slice(body)).unwrap();
//! assert_eq!(status, 200);
//! ```
//!
//! A test pins its own root instead, which is how `tests/peer.rs` and `pinned_tls_exerciser` reach
//! `helpers/tls-peer`:
//!
//! ```
//! let root = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/pinned-root.der")).unwrap();
//! let peer = pinned_tls_client::PinnedPeer::new("basalt.test", &root).unwrap();
//! assert_eq!(peer.host_name(), "basalt.test");
//! ```
//!
//! # BUGS
//!
//! - **Certificate expiry is checked against whatever `std` says the time is**, and on nife that
//!   is the clock service's page, which no stranger's machine can vouch for (milestone 501's block).
//!   A machine whose clock is far wrong refuses a good chain as not yet valid or expired, and one
//!   whose clock is held back accepts an expired one. Failing closed on the first is right; the
//!   second is the open half.
//! - **No rotation story.** When the pinned root is retired, every image carrying it stops
//!   reaching its index. ISRG Root X1's certificate says it is valid until 2035-06-04 (read from
//!   the certificate in `roots/`, 2026-10-06), and the chains Let's Encrypt serves today reach it
//!   only through cross-signatures that end sooner: see `PinnedPeer::basalt_index`.
//! - **One request per connection**, HTTP/1.0, `Content-Length` required, no redirects. Those are
//!   `http_response`'s limits, taken whole.
//! - **No resolver.** The caller connects the stream; milestone 384 (in a capability system the
//!   resolver is a grant) owns turning a name into an
//!   address, and the name verified here is whatever the pin says, never what the resolver used.

use std::io::{self, Read, Write};
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, ServerName};
use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

/// ISRG Root X1, the root §250 (an image names its distribution's package index) pins for basalt's
/// index host, as DER.
///
/// Fetched 2026-10-06 (UTC) from `https://letsencrypt.org/certs/isrgrootx1.der` and compared byte
/// for byte with the copy in macOS's `SystemRootCertificates.keychain`: identical. Its SHA-256 is
/// [`ISRG_ROOT_X1_SHA256`], which a test recomputes, so a substituted file fails a test rather than
/// a handshake on somebody's machine.
pub const ISRG_ROOT_X1: &[u8] = include_bytes!("../roots/isrg-root-x1.der");

/// The SHA-256 of [`ISRG_ROOT_X1`]: `96:BC:EC:06:...:08:C6`, the fingerprint Let's Encrypt
/// publishes for it.
pub const ISRG_ROOT_X1_SHA256: [u8; 32] = [
    0x96, 0xbc, 0xec, 0x06, 0x26, 0x49, 0x76, 0xf3, 0x74, 0x60, 0x77, 0x9a, 0xcf, 0x28, 0xc5, 0xa7,
    0xcf, 0xe8, 0xa3, 0xc0, 0xaa, 0xe1, 0x1a, 0x8f, 0xfc, 0xee, 0x05, 0xc0, 0xbd, 0xdf, 0x08, 0xc6,
];

/// basalt's index host, from §250.
pub const BASALT_INDEX_HOST: &str = "basalt.nifeos.org";

/// One host name and the one root its certificate must chain to. The whole of what a [`Session`]
/// trusts.
#[derive(Clone, Debug)]
pub struct PinnedPeer {
    name: ServerName<'static>,
    roots: Arc<RootCertStore>,
}

impl PinnedPeer {
    /// Pin `root_der` (one certificate, DER) as the only root for `host_name`.
    ///
    /// Refuses a host name that is not a DNS name (an IP address is legal for TLS and is not what
    /// a pin names here) and a root that does not parse as a trust anchor.
    pub fn new(host_name: &str, root_der: &[u8]) -> Result<Self, Error> {
        let name = ServerName::try_from(host_name.to_owned()).map_err(|_| Error::HostName)?;
        if !matches!(name, ServerName::DnsName(_)) {
            return Err(Error::HostName);
        }
        let mut roots = RootCertStore::empty();
        roots
            .add(CertificateDer::from(root_der.to_vec()))
            .map_err(|_| Error::Root)?;
        Ok(Self {
            name,
            roots: Arc::new(roots),
        })
    }

    /// **The production pin**: [`BASALT_INDEX_HOST`] and [`ISRG_ROOT_X1`], §250's clause 3.
    ///
    /// Measured 2026-10-06 (UTC), and it changes what pinning X1 means. Let's Encrypt now issues
    /// from its newer roots: `letsencrypt.org` presented leaf, `YE2`, `Root YE`, then `ISRG Root X2`
    /// cross-signed by X1; a GitHub Pages host (`*.github.io`, the host §250 names for
    /// `nifeos/basalt`) presented leaf, `YR1`, then `Root YR` cross-signed by X1. Both reach X1,
    /// so the pin works today, but only because the server sends the cross-signatures, and both
    /// cross-signatures say they end on 2032-09-02. Past that, or on a host that stops sending
    /// them, this pin refuses a correct server. Recorded in milestone 501's `BUGS`.
    pub fn basalt_index() -> Self {
        Self::new(BASALT_INDEX_HOST, ISRG_ROOT_X1).expect("ISRG Root X1 is a valid trust anchor")
    }

    /// The host name the certificate must carry, which is also the name sent as SNI.
    pub fn host_name(&self) -> &str {
        match &self.name {
            ServerName::DnsName(n) => n.as_ref(),
            _ => unreachable!("PinnedPeer::new admits DNS names only"),
        }
    }

    /// The `rustls` configuration this pin makes: TLS 1.3, this tree's provider, one root, and no
    /// client certificate.
    fn config(&self) -> Arc<ClientConfig> {
        let provider = Arc::new(cryptography_provider::provider());
        let config = ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .expect("cryptography_provider offers TLS 1.3 suites")
            .with_root_certificates(self.roots.clone())
            .with_no_client_auth();
        Arc::new(config)
    }
}

/// Why a fetch did not produce a response. Every variant fails closed: nothing a caller receives
/// came from a peer this pin did not admit.
#[derive(Debug)]
pub enum Error {
    /// The host name is not a DNS name.
    HostName,
    /// The root is not a certificate `rustls-webpki` accepts as a trust anchor.
    Root,
    /// The handshake or a record was refused. A certificate from another root, for another
    /// name, or out of date lands here as `rustls::Error::InvalidCertificate`.
    Tls(rustls::Error),
    /// The transport failed underneath TLS.
    Io(io::Error),
    /// The response was not one `http_response` reads.
    Http(http_response::Error),
    /// The connection ended before the body `Content-Length` promised.
    Truncated,
    /// The request line did not fit (a path longer than the request buffer).
    RequestTooLong,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        // `rustls::Stream` reports a TLS failure as an `io::Error` wrapping the `rustls::Error`;
        // unwrap it, so a refused certificate reads as a refused certificate and not as a socket
        // that misbehaved.
        match e
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<rustls::Error>())
        {
            Some(tls) => Error::Tls(tls.clone()),
            None => Error::Io(e),
        }
    }
}

/// An established TLS 1.3 session with a pinned peer, over any byte stream.
pub struct Session<S: Read + Write> {
    host: String,
    tls: StreamOwned<ClientConnection, S>,
}

impl<S: Read + Write> Session<S> {
    /// Run the handshake to completion over `stream`, refusing any peer the pin does not admit.
    ///
    /// Completing here, rather than lazily on the first write as `rustls::Stream` would, is what
    /// lets a caller time the handshake apart from the transfer and see a refusal as a refusal.
    pub fn handshake(peer: &PinnedPeer, stream: S) -> Result<Self, Error> {
        let conn = ClientConnection::new(peer.config(), peer.name.clone()).map_err(Error::Tls)?;
        let mut tls = StreamOwned::new(conn, stream);
        while tls.conn.is_handshaking() {
            tls.conn.complete_io(&mut tls.sock)?;
        }
        Ok(Self {
            host: peer.host_name().to_owned(),
            tls,
        })
    }

    /// The negotiated cipher suite, for a caller that reports what it spoke.
    pub fn cipher_suite(&self) -> Option<rustls::CipherSuite> {
        self.tls.conn.negotiated_cipher_suite().map(|s| s.suite())
    }

    /// The negotiated key exchange group.
    pub fn key_exchange_group(&self) -> Option<rustls::NamedGroup> {
        self.tls
            .conn
            .negotiated_key_exchange_group()
            .map(|g| g.name())
    }

    /// Send one `GET path` and hand the body to `body` as it arrives, never holding it. Returns
    /// the status; the caller decides what a status other than 200 means. Consumes the session,
    /// because `http_response` speaks one request per connection.
    pub fn get(mut self, path: &str, mut body: impl FnMut(&[u8])) -> Result<u16, Error> {
        let mut request = [0u8; 512];
        let n = http_response::get_request(&self.host, path, &mut request)
            .ok_or(Error::RequestTooLong)?;
        self.tls.write_all(&request[..n])?;
        self.tls.flush()?;

        let mut response = http_response::Response::new();
        let mut buf = vec![0u8; 16 * 1024];
        while !response.is_complete() {
            let got = match self.tls.read(&mut buf) {
                Ok(0) => return Err(Error::Truncated),
                Ok(got) => got,
                // A peer that hangs up without close_notify after a complete body is harmless,
                // but this loop only gets here before the body is complete, so it is a truncation.
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Err(Error::Truncated),
                Err(e) => return Err(e.into()),
            };
            let part = response.feed(&buf[..got]).map_err(Error::Http)?;
            body(part);
        }
        let status = response.status().ok_or(Error::Truncated)?;
        // Say goodbye properly; a peer that is already gone does not change the answer.
        self.tls.conn.send_close_notify();
        let _ = self.tls.conn.complete_io(&mut self.tls.sock);
        Ok(status)
    }
}
