//! **A `rustls` crypto provider assembled in this tree, over primitives chosen deliberately**
//! for milestone 442 (a crypto provider `rustls` can use on all three bare-metal targets).
//!
//! # Why this exists rather than a dependency
//!
//! DECISIONS §196 (nife carries TLS: `rustls` for the protocol, and a crypto provider we make
//! work) ruled that nife carries TLS and that the provider under it is work rather than a
//! purchase. The one pure-Rust provider on crates.io is `rustls-rustcrypto`, and **calef refused
//! it** on 2026-09-20: *"rustls-rustcrypto doesn't seem like a high quality dependency."* Version
//! 0.0.2-alpha, published 2024-04-24, three versions ever and two of them yanked, pinning majors
//! of `sha2`, `aes-gcm` and `p256` that were all superseded during 2026.
//!
//! **That refusal is about the glue, not about the primitives**, which is why this crate is glue
//! and nothing else.
//!
//! # The §46 line, drawn out loud
//!
//! DECISIONS §46 (thin primitives or whole subsystems; we write everything in between) says this tree writes what is on the verification path and **takes** what is won
//! by exposure rather than by reading a specification, and it names cryptography as the second
//! kind in so many words: *"take it, do not write it."* So:
//!
//! - **Every cryptographic operation is taken.** AES-GCM, ChaCha20-Poly1305, SHA-2, HMAC, X25519,
//!   P-256 and P-384 ECDSA, and Ed25519 all live in crates this package depends on. Their
//!   correctness includes constant-time behaviour and resistance to attacks no specification
//!   states, and `aes-gcm` and `chacha20poly1305` carry NCC Group's 2020 audit in their own README.
//! - **The glue is written, and glue is not cryptography.** A [`rustls::crypto::CryptoProvider`]
//!   is five fields: a cipher suite table, a key exchange group table, a signature verification
//!   algorithm table, a random source, and a private key loader. This crate fills them. It selects,
//!   names and plumbs; it computes nothing, and no operation in it is secret-dependent in a way a
//!   timing attack could read, because every secret-dependent operation is inside a primitive.
//! - **The one place the line could have been crossed is RSA**, and it was not. See `BUGS`.
//!
//! The honest accounting, which is in notes/cryptography-provider.md in full: measured the same
//! way, `rustls-rustcrypto`'s graph is **73 crates** and this one's is **53**, and the full
//! like-for-like comparison at equal algorithm coverage is 73 against 67. **The case for writing
//! this was never size.** It is that the glue is the piece with the least exposure in the whole
//! chain, so it is the piece worth owning.
//!
//! # What this provider supports
//!
//! **TLS 1.3 only.** `rustls`' `tls12` feature is off in this crate's manifest. A package client
//! talks to one host it was configured for, over a protocol version every such host has spoken for
//! years, and TLS 1.2 would add a second record layer, a second key schedule and RSA key transport
//! to a surface DECISIONS §196 already describes as the part that must stay small.
//!
//! | field | what this provides |
//! |---|---|
//! | `cipher_suites` | `TLS13_AES_128_GCM_SHA256`, `TLS13_AES_256_GCM_SHA384`, `TLS13_CHACHA20_POLY1305_SHA256` |
//! | `kx_groups` | X25519, NIST P-256 |
//! | `signature_verification_algorithms` | ECDSA P-256/SHA-256, ECDSA P-384/SHA-384, Ed25519, and RSA PKCS#1 v1.5 and PSS with SHA-256, SHA-384 and SHA-512 |
//! | `secure_random` | the entropy service, through `entropy_backend` and `getrandom` |
//! | `key_provider` | refuses: this provider loads no private keys |
//!
//! # EXAMPLES
//!
//! ```no_run
//! // The provider, ready to hand to `rustls`.
//! let provider = cryptography_provider::provider();
//! assert_eq!(provider.cipher_suites.len(), 3);
//! assert_eq!(provider.kx_groups.len(), 2);
//! ```
//!
//! On nife the consuming binary must also carry `use entropy_backend as _;` and the build
//! configuration in `cryptography_exerciser/.cargo/config.toml`, or the random source has no
//! backend and, on `x86_64-unknown-nife`, a primitive will execute an instruction the target has
//! no state for. Both are recorded where a reader meets them.
//!
//! # BUGS
//!
//! - **An RSA key whose SPKI says `RSASSA-PSS` rather than `rsaEncryption` is not verified.** Both
//!   are legal; the second is what every certificate authority in the public web PKI issues, and
//!   the first is rare enough that `rustls`' own ring-backed table calls its PSS entries
//!   `..._LEGACY_KEY` for taking the same position. A chain using one fails closed.
//! - **A PKCS#1 v1.5 `AlgorithmIdentifier` with absent parameters is not verified.** The encoding
//!   should carry an explicit `NULL`, and some older certificates omit it; `rustls`' ring table
//!   carries separate `..._ABSENT_PARAMS` entries for exactly that. Both GitHub chains this
//!   milestone verified use the explicit form, so nothing here needed it yet.
//! - **No TLS 1.2**, by the choice above. A peer that offers nothing newer will fail to negotiate.
//! - **No client certificates.** `key_provider` refuses every private key, which is correct for a
//!   package client and wrong for anything that must authenticate itself.
//! - **No FIPS claim anywhere.** Every `fips()` is left at its `false` default, which is the
//!   truthful answer and is stated so nobody reads the silence as a claim.

#![no_std]

extern crate alloc;

use alloc::{sync::Arc, vec};

use rustls::crypto::{CryptoProvider, KeyProvider, SecureRandom, WebPkiSupportedAlgorithms};
use rustls::sign::SigningKey;
use rustls::{CipherSuite, Error, SignatureScheme, SupportedCipherSuite};

mod aead;
mod hash;
mod hmac;
mod kx;
mod verify;

/// **The provider.** Hand it to `rustls`' config builder, or install it as the process default.
pub fn provider() -> CryptoProvider {
    CryptoProvider {
        cipher_suites: vec![
            TLS13_AES_128_GCM_SHA256,
            TLS13_AES_256_GCM_SHA384,
            TLS13_CHACHA20_POLY1305_SHA256,
        ],
        // X25519 first: it is the default key share a client sends, and it is what almost every
        // modern peer prefers, so putting it first is the difference between one round trip and
        // two on the common path.
        kx_groups: vec![&kx::X25519, &kx::P256],
        signature_verification_algorithms: SIGNATURE_ALGORITHMS,
        secure_random: &Entropy,
        key_provider: &NoPrivateKeys,
    }
}

/// `TLS13_AES_128_GCM_SHA256`.
pub static TLS13_AES_128_GCM_SHA256: SupportedCipherSuite =
    SupportedCipherSuite::Tls13(&rustls::Tls13CipherSuite {
        common: rustls::crypto::CipherSuiteCommon {
            suite: CipherSuite::TLS13_AES_128_GCM_SHA256,
            hash_provider: &hash::SHA256,
            // 2^24, which is `rustls`' own documented figure for AES-GCM: past it an attacker
            // gains an advantage distinguishing the record stream from a random permutation.
            // Copied from that documentation rather than chosen, because choosing it is a
            // cryptographic judgement and this crate makes none.
            confidentiality_limit: 1 << 24,
        },
        hkdf_provider: &rustls::crypto::tls13::HkdfUsingHmac(&hmac::SHA256),
        aead_alg: &aead::AES128_GCM,
        quic: None,
    });

/// `TLS13_AES_256_GCM_SHA384`.
pub static TLS13_AES_256_GCM_SHA384: SupportedCipherSuite =
    SupportedCipherSuite::Tls13(&rustls::Tls13CipherSuite {
        common: rustls::crypto::CipherSuiteCommon {
            suite: CipherSuite::TLS13_AES_256_GCM_SHA384,
            hash_provider: &hash::SHA384,
            confidentiality_limit: 1 << 24,
        },
        hkdf_provider: &rustls::crypto::tls13::HkdfUsingHmac(&hmac::SHA384),
        aead_alg: &aead::AES256_GCM,
        quic: None,
    });

/// `TLS13_CHACHA20_POLY1305_SHA256`.
pub static TLS13_CHACHA20_POLY1305_SHA256: SupportedCipherSuite =
    SupportedCipherSuite::Tls13(&rustls::Tls13CipherSuite {
        common: rustls::crypto::CipherSuiteCommon {
            suite: CipherSuite::TLS13_CHACHA20_POLY1305_SHA256,
            hash_provider: &hash::SHA256,
            // `u64::MAX` for ChaCha20-Poly1305, again `rustls`' own documented figure: the bound
            // that forces rekeying for AES-GCM does not apply to a stream cipher this way.
            confidentiality_limit: u64::MAX,
        },
        hkdf_provider: &rustls::crypto::tls13::HkdfUsingHmac(&hmac::SHA256),
        aead_alg: &aead::CHACHA20_POLY1305,
        quic: None,
    });

/// Every signature algorithm this provider will verify, and the map from TLS's own scheme numbers
/// to them.
///
/// **The two halves are not redundant.** `all` is what `rustls-webpki` walks a certificate chain
/// with; `mapping` is what `rustls` uses for the peer's handshake signature, where the scheme is
/// named on the wire. A scheme absent from `mapping` is one this client will not offer.
static SIGNATURE_ALGORITHMS: WebPkiSupportedAlgorithms = WebPkiSupportedAlgorithms {
    all: &[
        &verify::EcdsaP256Sha256,
        &verify::EcdsaP384Sha384,
        &verify::Ed25519,
        &verify::RSA_PKCS1_SHA256,
        &verify::RSA_PKCS1_SHA384,
        &verify::RSA_PKCS1_SHA512,
        &verify::RSA_PSS_SHA256,
        &verify::RSA_PSS_SHA384,
        &verify::RSA_PSS_SHA512,
    ],
    mapping: &[
        (
            SignatureScheme::ECDSA_NISTP256_SHA256,
            &[&verify::EcdsaP256Sha256],
        ),
        (
            SignatureScheme::ECDSA_NISTP384_SHA384,
            &[&verify::EcdsaP384Sha384],
        ),
        (SignatureScheme::ED25519, &[&verify::Ed25519]),
        // **PSS only in the mapping, and PKCS#1 v1.5 deliberately absent from it.** This table is
        // what verifies the peer's *handshake* signature, and RFC 8446 section 4.4.3 forbids
        // PKCS#1 v1.5 there: a TLS 1.3 server signing with an RSA key must use PSS. The `all`
        // list above is the other question, certificate chains, where v1.5 is what almost every
        // certificate authority actually issues.
        (SignatureScheme::RSA_PSS_SHA256, &[&verify::RSA_PSS_SHA256]),
        (SignatureScheme::RSA_PSS_SHA384, &[&verify::RSA_PSS_SHA384]),
        (SignatureScheme::RSA_PSS_SHA512, &[&verify::RSA_PSS_SHA512]),
    ],
};

/// `rustls`' randomness, which on nife is the entropy service.
///
/// The chain is `rustls` to here, to `rand_core`'s `OsRng`, to `getrandom`, to
/// `entropy_backend`'s hook, to `std::random::SystemRng`, to the entropy service's one endpoint,
/// to the device. Every link but `getrandom`'s is one this project owns.
#[derive(Debug)]
struct Entropy;

impl SecureRandom for Entropy {
    fn fill(&self, buf: &mut [u8]) -> Result<(), rustls::crypto::GetRandomFailed> {
        use rand_core::RngCore;
        // `try_fill_bytes` rather than `fill_bytes`: the infallible one panics on a failing
        // source, and a process that cannot get randomness should fail the handshake rather than
        // die. On nife a draw with no entropy capability granted panics inside `SystemRng`
        // regardless, which is the deliberate choice of milestone 56 (secrets, credentials, and
        // the entropy to make them safe) and not something this layer can soften.
        rand_core::OsRng
            .try_fill_bytes(buf)
            .map_err(|_| rustls::crypto::GetRandomFailed)
    }
}

/// A key provider that refuses every private key, because this provider is for a client that
/// presents no certificate.
///
/// **A refusal rather than an omission.** `rustls` requires the field, so the choice is between
/// answering honestly and pulling in PKCS#8 parsing and a signing path that nothing here would
/// ever exercise. Anything that needs client authentication needs a different provider, and will
/// find out at the call that loads the key rather than at the handshake.
#[derive(Debug)]
struct NoPrivateKeys;

impl KeyProvider for NoPrivateKeys {
    fn load_private_key(
        &self,
        _key_der: pki_types::PrivateKeyDer<'static>,
    ) -> Result<Arc<dyn SigningKey>, Error> {
        Err(Error::General(alloc::string::String::from(
            "this provider loads no private keys: it is for a client that presents no certificate",
        )))
    }
}
