//! **Milestone 494 (a driver for the network card a PC actually has)'s bench boot**, behind the
//! `network_bench` feature (name provisional; notes/e1000e.md has the evening it is for).
//!
//! Exit criterion 2 asks for three things on xenon, photographed: the PCI survey line naming the
//! card, a DHCP lease from the house router, and a measured transfer from a host on the LAN. The
//! tour already prints the survey. This boot replaces the hand-over with the other two, through
//! exactly the code the QEMU gates prove (`e1000e_service`, `net_stack`, `socket_test_client`), and
//! prints them in the order they can fail, every line prefixed `network-bench:` so a photograph and
//! a serial capture read the same way:
//!
//! 1. `nic`: the device id and model, its requester id, whether the link came up, the MAC address;
//! 2. `preflight dmar scope : PASS|FAIL`, which IOMMU unit owns the NIC, the condition milestone
//!    261 (the NVMe driver leaves the kernel)'s bench evening named for the NVMe applied to this
//!    device;
//! 3. `dhcp`: the leased address, after a line saying it is waiting, so a boot that never gets one
//!    ends on that line rather than on silence;
//! 4. `transfer`: bytes, time and rate from the peer the image was built for;
//! 5. `verdict LEASED-AND-MEASURED | UNCONFINED | NO-PEER | FAILED`, and `done, halting.`
//!
//! # What the number counts
//!
//! One TCP stream into `net_stack` through a 2 KiB socket buffer, received by a client process one
//! `RECEIVE` call per chunk, with the NIC polled every millisecond. **That is a measurement of this
//! stack as built, not of the I219**: the window is 2 KiB, so the rate is bounded by round trips
//! rather than by the wire, and the poll interval sits inside every round trip. It is the number
//! a package download gets today, which is what rung 3 needs to know.
//!
//! # BUGS
//!
//! - **The DHCP wait has no bound.** `net_stack` retries DISCOVER forever and the kernel blocks on
//!   its report. A boot with no lease ends on the `waiting` line, which is the diagnosis.
//! - **The peer is baked into the image** at build time (`NIFE_NETWORK_BENCH_PEER`), because this
//!   boot has no way to be told one. A different peer is a rebuild.

use crate::user::e1000e_service::{self as service, Absent};
use crate::{arch, println};

/// The peer, `a.b.c.d:port`, from the build's environment; `None` builds a boot that leases and
/// stops.
const PEER: Option<&str> = option_env!("NIFE_NETWORK_BENCH_PEER");

/// Run the bench and never come back.
pub fn run() -> ! {
    println!();
    println!(
        "network-bench: milestone 494 bench boot. {} build, {} Hz counter, peer {}.",
        if cfg!(debug_assertions) {
            "DEBUG"
        } else {
            "release"
        },
        arch::timer::frequency(),
        PEER.unwrap_or("none"),
    );
    let verdict = measure();
    println!("network-bench: verdict {verdict}");
    println!("network-bench: done, halting.");
    arch::halt(arch::HaltReason::measurement_boot());
}

fn measure() -> &'static str {
    let Some(image) = crate::trust::require_program("net_stack") else {
        return "FAILED: no net_stack in the initrd, or its bytes are not the measured ones";
    };
    let peer = match PEER.map(parse_peer) {
        Some(Some(p)) => Some(p),
        Some(None) => return "FAILED: NIFE_NETWORK_BENCH_PEER is not a.b.c.d:port",
        None => None,
    };
    let started = match peer {
        Some(word) => service::start_drain(image, word).map(|(cli, w)| (Some(cli), w)),
        None => {
            service::start_net_server(image, socket_protocol::NO_LISTEN_GRANT).map(|w| (None, w))
        }
    };
    let (client, w) = match started {
        Ok(v) => v,
        Err(Absent::NoController) => return "FAILED: no NIC this driver claims is on the bus",
        Err(Absent::Refused { rid, why }) => {
            println!("network-bench: nic       : rid {rid:#06x} refused at bring-up: {why:?}");
            return "FAILED: the NIC is on the bus and bring-up refused it";
        }
        Err(Absent::Busy) => return "FAILED: a server already holds the NIC",
    };
    let m = w.mac;
    println!(
        "network-bench: nic       : 8086:{:04x} {}, link {}, mac {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        w.device,
        ::e1000e::model(w.device).unwrap_or("?"),
        if w.link_up { "up" } else { "NOT UP" },
        m[0],
        m[1],
        m[2],
        m[3],
        m[4],
        m[5],
    );
    println!(
        "network-bench: preflight dmar scope : {}",
        if w.confined_by_iommu { "PASS" } else { "FAIL" }
    );
    // With a client, `start_drain` drained the lease before returning; without one, take it here.
    let Some(cli) = client else {
        println!(
            "network-bench: dhcp      : waiting for a lease (no bound: a last line here is the answer)"
        );
        let o = (crate::sched::ipc_receive(w.report)[0] as u32).to_be_bytes();
        println!(
            "network-bench: dhcp      : leased {}.{}.{}.{}",
            o[0], o[1], o[2], o[3]
        );
        return if w.confined_by_iommu {
            "NO-PEER: leased, nothing to measure (build with NIFE_NETWORK_BENCH_PEER)"
        } else {
            "UNCONFINED: leased, but no IOMMU unit this kernel programmed owns the NIC"
        };
    };
    let o = w.lease.to_be_bytes();
    println!(
        "network-bench: dhcp      : leased {}.{}.{}.{}",
        o[0], o[1], o[2], o[3]
    );
    let [code, bytes, ticks, ..] = crate::sched::ipc_receive(cli);
    if code != 1 {
        println!("network-bench: transfer  : client stopped with code {code:#x}");
        return "FAILED: the transfer did not complete (see socket_test_client for the code)";
    }
    let hz = arch::timer::frequency();
    let micros = ticks.saturating_mul(1_000_000) / hz.max(1);
    let rate = bytes.saturating_mul(8).saturating_mul(1_000_000) / micros.max(1);
    println!(
        "network-bench: transfer  : {bytes} bytes in {}.{:03} s = {}.{:03} Mbit/s",
        micros / 1_000_000,
        (micros / 1000) % 1000,
        rate / 1_000_000,
        (rate / 1000) % 1000,
    );
    if bytes == 0 {
        "FAILED: connected and received nothing"
    } else if w.confined_by_iommu {
        "LEASED-AND-MEASURED"
    } else {
        "UNCONFINED: measured, but no IOMMU unit this kernel programmed owns the NIC"
    }
}

/// `a.b.c.d:port` as the drain client's word, `ipv4 << 16 | port`.
fn parse_peer(s: &str) -> Option<u64> {
    let (ip, port) = s.split_once(':')?;
    let port: u16 = port.parse().ok()?;
    let mut word: u64 = 0;
    let mut n = 0;
    for part in ip.split('.') {
        word = word << 8 | u64::from(part.parse::<u8>().ok()?);
        n += 1;
    }
    (n == 4).then_some(word << 16 | u64::from(port))
}
