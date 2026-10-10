//! **A package fetched through the distribution's index, by host name, over pinned TLS, and
//! believed only by the index's digest** (milestone 801 (packages over the internet), items 1, 3
//! and 4).
//!
//! The kernel plays the spawner: `net_stack` over the `e1000e`, `name_resolver` told to ask the
//! runners' name server, a badge granted `package_index::fixture::ZONE`, and `package_fetch_exerciser`
//! holding that badge, the network, a clock and entropy. The index host is `helpers/tls-peer` as
//! `basalt.test`, standing in for `basalt.nifeos.org`; the package host is
//! `helpers/package-http-peer` as `packages.basalt.test`. Nothing leaves slirp.
//!
//! Skips unless somebody built the program, on `pinned_tls_tests`' terms and for its reason:
//! `helpers/build-pinned-tls-exerciser.sh` fetches `rustls` and the provider's graph, which no gate
//! does yet (milestone 855 (the TLS graph enters the gated build)).

use super::name_resolver_tests::{entropy, grant, start_resolver};
use super::*;

/// The badge the program's resolver capability carries.
const GRANTED: u32 = 0x8010;

const NO_PROGRAM: &str = "no package_fetch_exerciser in this archive: build it with \
     helpers/build-pinned-tls-exerciser.sh (milestone 801), which fetches the TLS graph from crates.io";

/// **The index arrives over TLS from the host the client resolved by name, a package it names is
/// fetched from another host and admitted by the index's digest, a location serving altered bytes
/// is refused, and a name the index does not list fetches nothing.**
///
/// The refusal is what gives the fetch its meaning: the tampered response is a complete, correct
/// HTTP exchange of a well-formed package, so only the digest the index carried can refuse it.
///
/// Falsification: attested 2026-10-09. Not replayable: no sweep builds the TLS graph. With
/// `package_index::accept` hashing the fetched bytes instead of reading the entry's digest (on
/// aarch64), `uptime` was refused as `MemberMismatch` rather than `NotCataloged`, and the test went
/// red on that line. The peer's flipped byte is also caught by the package's own
/// table of contents, which is why the test names the refusal and not merely that one happened.
#[test_case]
fn a_package_is_fetched_through_the_index_by_name_over_tls_and_judged_by_its_digest() {
    let Some(exerciser) = program("package_fetch_exerciser") else {
        crate::testing::skip!(NO_PROGRAM);
    };
    use core::sync::atomic::Ordering;

    use crate::arch::exceptions::USER_FAULTS;

    let Some(entropy) = entropy() else {
        crate::testing::skip!("no entropy source on this machine, and the resolver needs one");
    };
    let image = |name: &str| program(name).unwrap_or_else(|| panic!("no {name} in the archive"));
    let mut w = match e1000e_service::start_net_server(
        image("net_stack"),
        socket_protocol::NO_LISTEN_GRANT,
    ) {
        Ok(w) => w,
        Err(e1000e_service::Absent::NoController) => {
            crate::testing::skip!("no e1000e NIC attached");
        }
        Err(why) => panic!("the e1000e NIC is on the bus and its bring-up refused: {why:?}"),
    };
    let _ = crate::sched::ipc_receive(w.report);
    let service = start_resolver(w.stack, entropy, &mut w.held);
    grant(service, GRANTED, package_index::fixture::ZONE);

    let faults_before = USER_FAULTS.load(Ordering::Relaxed);
    let spawned = std_service::start_networked_resolving(
        exerciser,
        image("clock"),
        image("entropy"),
        w.stack,
        Some((service, GRANTED)),
    );
    let run = &spawned.run;
    let mut got = [0u8; 2048];
    let len = super::std_tests::drain_sink(run.report, &mut got, "package_fetch_exerciser");
    let text = core::str::from_utf8(&got[..len]).unwrap_or("<not utf-8>");
    crate::println!("    package_fetch_exerciser printed {len} bytes:\n{text}");

    let arch = crate::arch::NAME;
    let mut fetched = [0u8; 64];
    let mut refused = [0u8; 64];
    let lines: [&str; 6] = [
        "package_fetch_exerciser start",
        "index ok ",
        format_into(
            &mut fetched,
            &[
                "fetched greeting-0.1.0-",
                arch,
                " from packages.basalt.test:8080",
            ],
        ),
        format_into(
            &mut refused,
            &["refused uptime-0.1.0-", arch, ": NotCataloged"],
        ),
        "refused nosuch: not in the index",
        "package_fetch_exerciser done",
    ];
    let mut from = 0;
    for line in lines {
        match text[from..].find(line) {
            Some(at) => from += at + line.len(),
            None => {
                panic!("package_fetch_exerciser never printed `{line}` (after what came before it)")
            }
        }
    }
    assert!(
        super::wait_for(|| !crate::sched::is_thread_present(run.thread)),
        "package_fetch_exerciser never left",
    );
    assert_eq!(
        USER_FAULTS.load(Ordering::Relaxed),
        faults_before,
        "package_fetch_exerciser trapped instead of exiting",
    );
    let _ = crate::sched::reclaim_region(spawned.frames);
    run.give_back("package_fetch_exerciser");
    w.held
        .release_or_fail("net_stack and name_resolver, for the package index client");
}

/// `parts` joined into `buf`, which a `no_std` test has instead of `format!`.
fn format_into<'b>(buf: &'b mut [u8], parts: &[&str]) -> &'b str {
    let mut at = 0;
    for part in parts {
        buf[at..at + part.len()].copy_from_slice(part.as_bytes());
        at += part.len();
    }
    core::str::from_utf8(&buf[..at]).unwrap_or("")
}
