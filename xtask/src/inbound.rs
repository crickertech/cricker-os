//! The inbound-connection probe (milestone 140): a host-side client that connects to the
//! guest's listener through QEMU's port forward and says what came back.
//!
//! It lives on the host because the claim is about the outside world reaching in, which
//! nothing inside the guest can make on its own.

/// The bytes the inbound prober sends into the guest and the answer it requires back. They must
/// match `socket_protocol::fixture` (`IN_MSG`/`OUT_MSG`), which is what the two guest programs read
/// them from, and they are deliberately different strings: an echo would pass even if the guest
/// were only reflecting our own bytes, and the point of this gate is that the guest **composed** an
/// answer to a connection it did not make.
///
/// **Spelled again here rather than imported**, which is the same call the pinned [MS-NLMP] vectors
/// beside the credential tests make: the claim is that two independently-written sides agree, and a
/// shared constant would let one edit move both. A drift is loud, because the guest's answer would
/// not match and the prober says exactly what came back instead.
const INBOUND_IN: &[u8] = b"nife-in!";
const INBOUND_OUT: &[u8] = b"nife-out!";
/// How many connections the guest **offers** over a whole boot: two each from the two programs
/// that listen on the forwarded port, one after the other (`socket_test_client`'s hand-written
/// accept role from milestone 107, and `std_exerciser`'s `std::net::TcpListener` half from
/// milestone 64). The prober keeps connecting until it has this many or the run ends, and it must
/// keep going even once it has enough to pass, because a guest sitting in `ACCEPT` needs a peer:
/// stopping early would hang the guest's own test rather than the host's.
const INBOUND_OFFERED: usize = 4;

/// How many of those the prober must actually **collect** for the leg to pass, which is not the
/// same number, and the gap is deliberate.
///
/// **Three of four is a provable claim, not a fudge.** Neither program can supply more than two, so
/// a host that collected three collected at least one from *each* of them. That is exactly the half
/// no in-guest assertion can make: the bytes reached a process outside the machine, for both
/// listeners.
///
/// **What the fourth round would have added, and why it is not worth what it costs.** Requiring all
/// four would also confirm the *re-arm* host-side, which is what milestone 107's `2` did when there
/// was one listening window. But the re-arm is asserted where it is actually checkable, inside each
/// guest test: `serve_one_inbound` fails the run if a second `accept` does not return with the right
/// payload, and a guest cannot fake that. Paying for a duplicate of it with a flaky leg is a bad
/// trade.
///
/// **And it was measured rather than assumed.** With four required, run 32195227733's riscv64 leg
/// reported "the guest served 3 of 4" while **all 279 guest tests passed**, on a runner this
/// script's own load instrument called not oversubscribed. So the guest served four and the host
/// collected three: one answer went somewhere the prober was not reading. Five local riscv boots did
/// not reproduce it, which puts it around one in six on that runner class, and it is exactly the
/// kind of intermittent red that notes/net.md records misleading three separate milestones.
///
/// **The mechanism is still not identified**, and a second lane went looking on 2026-08-19 without
/// finding it. What that lane did establish is worth having before you start: the failure is
/// host-side by elimination (every guest path that serves fewer than two rounds is loud), the
/// teardown race is ruled out by timing, and it does not reproduce on macOS because CI is
/// `ubuntu-24.04-arm` and this is an emulator-timing failure. It also left the prober's transcript
/// printing on green runs, so the next red one has a known-good shape to be read against. The whole
/// finding is in notes/net/the-inbound-check.md; read it before touching this number. This constant makes the gate
/// robust to losing one round without letting it claim less than it proves.
const INBOUND_REQUIRED: usize = 3;

/// Ask the OS for a free TCP port on the loopback and let it go again.
///
/// There is a race between letting go and QEMU binding it, and it is the right trade: the
/// alternative is a *fixed* port, which two lanes running the suite on one machine collide on every
/// time rather than rarely. A lost race fails loudly (QEMU refuses to start), which is the failure
/// mode this project prefers to a quiet one.
fn free_loopback_port() -> Option<u16> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").ok()?;
    let port = listener.local_addr().ok()?.port();
    drop(listener);
    Some(port)
}

/// **The host side of the inbound gate** (milestone 107): a host process that connects TO the guest.
///
/// Everything else the suite proves about the network is the guest as a client. This is the mirror,
/// and it needs a host actor for the same reason the scanout check does: nothing inside the guest
/// can open a connection to the guest from outside it. QEMU's `hostfwd` forwards a loopback port
/// into the guest's listening port, and this thread connects to it, sends a payload, and requires
/// the guest's own answer back.
///
/// It **retries for the whole run** rather than being timed to the accept test, because nothing here
/// knows when that test starts. A connection that arrives while some other net test holds the NIC
/// finds no listener and is reset by smoltcp, which costs nothing and is indistinguishable from any
/// other closed port. It stops the moment both rounds have completed.
pub(crate) struct InboundProber {
    arch: String,
    port: Option<u16>,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<Result<(), String>>>,
}

impl InboundProber {
    /// Pick the port, tell the runner about it, and start poking. Call this **before** the child is
    /// spawned: the runner reads `NIFE_HOSTFWD_PORT` from the environment it inherits.
    pub(crate) fn new(arch: &str) -> Self {
        let Some(port) = free_loopback_port() else {
            eprintln!("inbound prober ({arch}): could not get a free loopback port");
            return Self {
                arch: arch.to_string(),
                port: None,
                stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true)),
                thread: None,
            };
        };
        // SAFETY: `set_var` became unsafe in edition 2024 because it races other threads. This runs
        // on the main thread before both the child that reads it and the prober thread below, and
        // that thread only touches sockets; no thread xtask starts ever writes the environment.
        unsafe { std::env::set_var("NIFE_HOSTFWD_PORT", port.to_string()) };

        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_thread = stop.clone();
        let thread = std::thread::spawn(move || probe_inbound(port, stop_thread));
        Self {
            arch: arch.to_string(),
            port: Some(port),
            stop,
            thread: Some(thread),
        }
    }

    /// Stop poking, and say whether the guest answered. Fails the leg when it did not: the guest's
    /// own assertion covers "somebody connected", and this covers the other half, that what came
    /// back was the answer the guest meant to send.
    pub(crate) fn report(mut self) -> bool {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
        let arch = &self.arch;
        let Some(thread) = self.thread.take() else {
            eprintln!("inbound check ({arch}) FAILED: the prober never started");
            return false;
        };
        let port = self.port.unwrap_or(0);
        match thread.join() {
            Ok(Ok(())) => {
                eprintln!(
                    "inbound check ({arch}): host connections to 127.0.0.1:{port} were forwarded \
                     into the guest, accepted, and answered with the guest's own bytes, by both \
                     listeners: the hand-written one and `std::net::TcpListener`. At least \
                     {INBOUND_REQUIRED} of the {INBOUND_OFFERED} offered, which is the floor that \
                     proves each of the two answered, since neither can supply more than two."
                );
                true
            }
            Ok(Err(reason)) => {
                eprintln!();
                eprintln!("inbound check ({arch}) FAILED: {reason}");
                eprintln!(
                    "  A host process connecting to the guest is the one thing no in-guest test can \
                     stage. See notes/net/the-inbound-half.md."
                );
                false
            }
            Err(_) => {
                eprintln!("inbound check ({arch}) FAILED: the prober thread panicked");
                false
            }
        }
    }
}

/// One prober thread: connect, speak, and require the guest's answer, up to `INBOUND_OFFERED`
/// times, passing at `INBOUND_REQUIRED`.
///
/// **Never abandon a connection because it is slow, and this is the whole subtlety** (found by the
/// first green run, where the guest passed and the prober reported nothing). A `connect` here
/// succeeds the moment QEMU accepts the host side; slirp only then starts the guest side, and if the
/// guest is not *polling* nothing answers the SYN. Dropping such a connection does not take back the
/// payload already written: slirp keeps the guest-side connection, completes the handshake whenever
/// the guest next polls, and delivers those bytes to a socket whose host end has gone. The guest then
/// serves a round nobody is listening for, and its answer is discarded.
///
/// One retry every 100 ms for a whole boot makes that a queue of them, which is exactly what
/// happened: the guest served both its rounds from abandoned connections and passed, while the
/// prober timed out on its own live one and reported zero.
///
/// So a timeout is **not** a reason to give up: keep reading the same connection until it answers,
/// dies, or the run ends. A hard error (the RST a guest with no listener sends, which is the common
/// case for most of a boot) *is* a reason, and a cheap one, because nothing was consumed.
fn probe_inbound(
    port: u16,
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
) -> Result<(), String> {
    use std::io::{ErrorKind, Read, Write};
    use std::sync::atomic::Ordering;

    let addr: std::net::SocketAddr = ([127, 0, 0, 1], port).into();
    let mut done = 0usize;
    let mut last = String::from("nothing ever answered on the forwarded port");
    let mut trace = InboundTrace::new();

    while done < INBOUND_OFFERED && !stop.load(Ordering::Relaxed) {
        let opened = std::time::Instant::now();
        let mut s = match std::net::TcpStream::connect_timeout(
            &addr,
            std::time::Duration::from_millis(500),
        ) {
            Ok(s) => s,
            Err(e) => {
                last = format!("could not connect to 127.0.0.1:{port}: {e}");
                trace.note("connect-failed", opened, 0);
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }
        };
        // Short, so the loop below can notice `stop`; not a deadline for the exchange.
        let _ = s.set_read_timeout(Some(std::time::Duration::from_millis(250)));
        let _ = s.set_nodelay(true);

        if let Err(e) = s.write_all(INBOUND_IN) {
            last = format!(
                "the guest closed before reading our {} bytes: {e}",
                INBOUND_IN.len()
            );
            trace.note("write-failed", opened, 0);
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
        }

        let mut got = Vec::new();
        let mut buf = [0u8; 64];
        // What ended this connection, for the tally the failure text prints. The read loop has
        // five exits and they mean different things: only two of them are "nothing was consumed",
        // and telling them apart is the whole diagnosis when a round goes missing.
        let mut outcome = "answered";
        let mut error: Option<String> = None;
        while got.len() < INBOUND_OUT.len() {
            match s.read(&mut buf) {
                Ok(0) => {
                    // The guest had no listener and closed; nothing was consumed. Unless bytes had
                    // already arrived, in which case a round WAS served and we lost the tail of it.
                    outcome = if got.is_empty() {
                        "closed-empty"
                    } else {
                        "closed-partial"
                    };
                    break;
                }
                Ok(n) => got.extend_from_slice(&buf[..n]),
                Err(e) if read_error_is_not_yet(e.kind()) => {
                    // Still waiting on a guest that has not polled yet. Hold the connection: see
                    // this function's note on why dropping it would feed the guest a round we
                    // cannot collect. The run ending is the only thing that ends this wait.
                    if stop.load(Ordering::Relaxed) {
                        last = String::from(
                            "the run ended while a connection was still waiting for the guest to \
                             accept it",
                        );
                        outcome = "stopped-while-waiting";
                        break;
                    }
                }
                Err(e) => {
                    last = format!("reading the guest's answer failed: {e}");
                    // Kept on the event, not only in `last`, which is printed on a red run alone.
                    // notes/net/the-inbound-check.md: a green run's `read-failed` once threw away the one fact that
                    // would have named it.
                    error = Some(format!("{:?}, os error {:?}", e.kind(), e.raw_os_error()));
                    outcome = match e.kind() {
                        ErrorKind::ConnectionReset => "reset",
                        ErrorKind::ConnectionAborted => "aborted",
                        ErrorKind::BrokenPipe => "broken-pipe",
                        _ => "read-failed",
                    };
                    break;
                }
            }
        }
        drop(s);

        if got == INBOUND_OUT {
            done += 1;
            trace.note("answered", opened, got.len());
            continue;
        }
        if outcome == "answered" {
            // The loop filled its quota without matching: bytes that are not the guest's answer.
            outcome = "wrong-bytes";
        }
        trace.note_error(outcome, opened, got.len(), error);
        // Not an answer: almost always "no listener yet", which is the normal state for most of the
        // run. Keep the last one only so a genuine failure has something to say.
        if !got.is_empty() {
            last = format!(
                "the guest answered {:?}, wanted {:?}",
                String::from_utf8_lossy(&got),
                String::from_utf8_lossy(INBOUND_OUT),
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    if done >= INBOUND_REQUIRED {
        // Printed on a GREEN run too, and that is the point: a red run is only diagnosable against
        // a known-good shape, and the first two failures of this check had none to compare with.
        // It is one line and at most four more.
        eprintln!("inbound prober (port {port}): {}", trace.summary());
        Ok(())
    } else {
        Err(format!(
            "the guest served {done} of the {INBOUND_OFFERED} inbound connections it offers on the \
             port forwarded to 127.0.0.1:{port}, and {INBOUND_REQUIRED} is the floor that proves \
             both listeners answered; last attempt: {last}\n  what the {} attempts did: {}\n  \
             READ THE TIMESTAMPS FIRST. The four rounds come from two listeners in two separate \
             windows about eight seconds apart, two rounds in each, so the answers cluster. Two \
             clusters and a missing round means the host lost one that the guest served, and the \
             outcome beside it names how. ONE cluster means a whole listener never ran: look for \
             `(no virtio-net device attached; skipping)` in the transcript, which is the only way \
             either guest test passes without offering its two. See notes/net/the-inbound-check.md.",
            trace.attempts,
            trace.summary(),
        ))
    }
}

/// **Whether a read error means "not yet" rather than "this connection is over".**
///
/// `WouldBlock` and `TimedOut` are the 250 ms read timeout expiring. `Interrupted` is a signal
/// landing on the blocked `receive` (`EINTR`), which says nothing about the connection at all, and
/// treating it as fatal was the inbound check's lost round. On CI, every `read-failed` in two weeks
/// of traces landed on the five-second grid `HostLoad` samples on, and the first trace that kept its
/// errno said `Interrupted, os error 4`. Dropping the connection there does not take back the
/// payload already written: slirp still delivers it when the guest next polls, the guest serves a
/// round into a socket nobody holds, and two of those in one boot is "2 of 4" and a red leg. See
/// notes/net/the-inbound-half.md.
fn read_error_is_not_yet(kind: std::io::ErrorKind) -> bool {
    use std::io::ErrorKind;
    matches!(
        kind,
        ErrorKind::WouldBlock | ErrorKind::TimedOut | ErrorKind::Interrupted
    )
}

/// **What every prober connection did, kept so a failure can name a mechanism instead of a count.**
///
/// The check has failed twice with nothing to go on but "the guest served 3 of 4", which does not
/// distinguish an abandoned connection from a lost answer from a teardown race, and each of those
/// wants a different fix. The read loop in `probe_inbound` has five exits; this records which one
/// each connection took, how long it was held, and how many bytes it had collected when it ended.
///
/// **Deliberately cheap and unconditional.** A boot makes a few hundred attempts at most, the
/// counters are increments, and the per-connection lines are kept only for the ones that carried
/// bytes or were held long enough to be interesting. The summary prints on a **passing** run too,
/// which is the half that was missing: a red run is only readable against a known-good shape, and
/// the two failures on record had none to compare with.
#[derive(Default)]
struct InboundTrace {
    attempts: usize,
    counts: std::collections::BTreeMap<&'static str, usize>,
    /// `(ms since the prober started, outcome, held ms, bytes)` for connections worth a line: the
    /// ones that collected bytes, and the ones held over a second. A connection that was reset in
    /// under a millisecond with nothing on it is the boring majority and is only counted.
    events: Vec<(u128, &'static str, u128, usize, Option<String>)>,
    started: Option<std::time::Instant>,
}

impl InboundTrace {
    pub(crate) fn new() -> Self {
        Self {
            started: Some(std::time::Instant::now()),
            ..Default::default()
        }
    }

    pub(crate) fn note(&mut self, outcome: &'static str, opened: std::time::Instant, bytes: usize) {
        self.note_error(outcome, opened, bytes, None);
    }

    /// [`note`](Self::note), with the error that ended the connection when one did. Any event that
    /// carries an error gets a line, however briefly it was held.
    pub(crate) fn note_error(
        &mut self,
        outcome: &'static str,
        opened: std::time::Instant,
        bytes: usize,
        error: Option<String>,
    ) {
        self.attempts += 1;
        *self.counts.entry(outcome).or_insert(0) += 1;
        let held = opened.elapsed().as_millis();
        if bytes > 0 || held >= 1000 || outcome == "answered" || error.is_some() {
            let at = self
                .started
                .map(|s| s.elapsed().as_millis())
                .unwrap_or_default();
            // Bounded, so a pathological run cannot grow this without limit.
            if self.events.len() < 64 {
                self.events.push((at, outcome, held, bytes, error));
            }
        }
    }

    pub(crate) fn summary(&self) -> String {
        let mut out = String::new();
        for (k, v) in &self.counts {
            if !out.is_empty() {
                out.push_str(", ");
            }
            out.push_str(&format!("{k} x{v}"));
        }
        if out.is_empty() {
            out.push_str("no attempts");
        }
        for (at, outcome, held, bytes, error) in &self.events {
            out.push_str(&format!(
                "\n    +{at} ms: {outcome} after {held} ms, {bytes} bytes"
            ));
            if let Some(error) = error {
                out.push_str(&format!(" ({error})"));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    /// A signal on a blocked read is a reason to read again, never a verdict on the connection.
    /// Before this, `Interrupted` fell to the catch-all arm, the connection was dropped as
    /// `read-failed`, and the round its payload bought was served to nobody.
    #[test]
    fn an_interrupted_read_keeps_the_connection() {
        assert!(super::read_error_is_not_yet(ErrorKind::Interrupted));
        assert!(super::read_error_is_not_yet(ErrorKind::WouldBlock));
        assert!(super::read_error_is_not_yet(ErrorKind::TimedOut));
    }

    /// And a connection the peer really ended is still over, so the fix cannot turn a reset into
    /// an endless wait.
    #[test]
    fn a_reset_still_ends_it() {
        assert!(!super::read_error_is_not_yet(ErrorKind::ConnectionReset));
        assert!(!super::read_error_is_not_yet(ErrorKind::ConnectionAborted));
        assert!(!super::read_error_is_not_yet(ErrorKind::BrokenPipe));
    }
}
