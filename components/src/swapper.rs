//! **The operator: replace a running component under a talking client** (milestone 23,
//! DECISIONS §41).
//!
//! This is the milestone's flagship, and it is an unprivileged process. The kernel does not know a
//! swap is happening, has no notion of a component, and gained no method for one. What it supplies
//! is what it already supplied: endpoints that name no peer (§12), a death message (§26), a reap
//! that needs no construction authority (§32), and revocation (§13, §16, and §41's device
//! take-back). Everything else is code here.
//!
//! # Four roles: two rungs of the latency ladder, one component that stops answering, and one that carries state
//!
//! - [`ROLE_DIRECT`](swap_protocol::ROLE_DIRECT): the default rung. The stable name a client holds is the
//!   endpoint object itself, and the swap changes who is parked in `RECV_CAP` on it. **No process
//!   sits in the data path**, so the steady state costs exactly what an unbrokered call costs.
//! - [`ROLE_QUEUED`](swap_protocol::ROLE_QUEUED): the opt-in rung. A `broker` stands between producer and
//!   backend so the producer never blocks on an absent consumer. One extra hop, priced by the
//!   `broker_rtt` benchmark, and chosen per channel rather than imposed on every IPC.
//! - [`ROLE_HUNG`](swap_protocol::ROLE_HUNG): the same system as the direct rung, against an incumbent
//!   that **stops answering without dying** (milestone 23's third residual). Step 2 of the four is
//!   unavailable, because draining needs the incumbent's cooperation and that is exactly what is
//!   missing; the interesting result is that the other three steps do not. See
//!   notes/hung-component.md, and read its two open decisions before extending this role.
//! - [`ROLE_HANDOFF`](swap_protocol::ROLE_HANDOFF): **state handoff** (DECISIONS §209 (state handoff is an opaque blob over a granted frame, and it is optional)). The component
//!   carries a tally across the swap on a handoff page, and the swap is tried twice: against a
//!   replacement that cannot absorb the state, which must not commit, and then against one that can.
//!   See notes/state-handoff.md.
//! - [`ROLE_UNWARNED`](swap_protocol::ROLE_UNWARNED): the queued system **with the dependent never
//!   warned**, which is what a supervisor is left with when the dependent the graph names will not
//!   answer. It measures what skipping the warning costs, and the answer is latency, not loss. See
//!   notes/non-cooperative-fallback.md.
//!
//! # The direct swap, step by step, and why the order is this
//!
//! ```text
//!   1 BUILT    lay the replacement out, endow it, retype its TCB -- but do NOT configure or start
//!              it. A thread that has never been started is in nobody's queue, so it cannot take a
//!              request the incumbent is still there to serve.
//!   2 DRAINED  CALL OP_QUIESCE on the service endpoint itself. The endpoint's sender queue is
//!              FIFO, so by the time this arrives the incumbent has answered every request queued
//!              ahead of it. It replies and stops receiving.
//!   3 REVOKED  PageFrame::REVOKE the device capability: gone from every holder but us.
//!   4 STARTED  map the registers into the replacement, CONFIGURE, START. The down window ends.
//!   5 REAPED   the incumbent is told to touch the device it no longer has; it faults, its death
//!              arrives on the supervision endpoint, and Rendezvous::REAP collects the corpse.
//! ```
//!
//! **The roadmap put "start the new server" first and the revoke second, and building it that way
//! does not work.** §13 revocation is by *physical page*: a revoke that ran after the replacement
//! had been endowed with the device would take the replacement's copy too, and since the kernel
//! mints a device capability once, at boot, nothing could ever hand one back. So the replacement is
//! built and endowed with everything *except* the device, and receives the registers on the far
//! side of the revoke. What had to move is the endowment, not the build, which is why the down
//! window is still four syscalls wide. Recorded rather than quietly reordered.
//!
//! Name: ratified 2026-07-30 (calef, DECISIONS §39, landed by milestone 46), replacing `swapd`.
//! Refused `swapd` (the `-d` claim).

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

// Two shared modules: the swap system's protocol and the supervision tree's loader. Each binary
// uses a different slice of both, so the unused halves are expected (§38).
use component_plan::Provisions;
use supervision_protocol::{ChildEndowment, Retention};
use swap_protocol::log_checks as lc;
use user_mode_runtime::{
    cap_delete, map_into, map_page_frame, recv, recv_fault, revoke_frame, send,
};

/// What the kernel grants us, and nothing else.
const ROOT_UT: u64 = 0; // the construction budget: what every process here is built out of
const REPORT: u64 = 1; // WRITE|GRANT, so each child gets its own narrowed view
const DEVICE: u64 = 2; // the UART's registers, WRITE|GRANT: ours to lend, and ours to take back

// Pages per process we build is NOT here any more. It is `swap_protocol::INSTANCE_PAGES`, declared by
// each contract, and it reaches this program through `component_plan::Plan::pages`. It is a **peak**,
// because this operator never destroys a region: all five splits are live at once and the budget has
// to cover them together.

/// The channels this operator owns, created once out of its own budget.
struct Wiring {
    /// The stable name. One endpoint object; every client capability to it is handed out before
    /// either component exists, and nothing ever creates a second one.
    svc: u64,
    /// Our children's deaths, kernel-stamped (DECISIONS §26).
    faultep: u64,
    /// Our own coordination channel. Separate from the report endpoint on purpose: the report
    /// endpoint belongs to the test, and an operator that read it would be stealing the record.
    note: u64,
    /// How we speak to a component that has quiesced, and so is no longer listening on `svc`.
    poke: u64,
    /// The witness page, mapped read/write into us and into every component.
    log_page_frame: u64,
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(role: u64, initrd_len: u64, _a2: u64) -> ! {
    // SAFETY: forwarded from user_mode_runtime::initrd::initrd_bytes's own contract.
    let archive = unsafe { user_mode_runtime::initrd::initrd_bytes(initrd_len) };
    let Ok(fs) = nifefs::Fs::parse(archive) else {
        bail(1)
    };

    let w = Wiring {
        svc: obj(abi::objtype::RENDEZVOUS, 5),
        faultep: obj(abi::objtype::RENDEZVOUS, 6),
        note: obj(abi::objtype::RENDEZVOUS, 7),
        poke: obj(abi::objtype::RENDEZVOUS, 8),
        log_page_frame: page_frame(9),
    };
    if !map_page_frame(w.log_page_frame, swap_protocol::LOG_VA, true, ROOT_UT) {
        bail(10)
    }
    for i in 0..swap_protocol::PAGE {
        // SAFETY: a page we just mapped read/write into our own address space.
        unsafe { core::ptr::write_volatile((swap_protocol::LOG_VA as *mut u8).add(i as usize), 0) };
    }

    match role {
        swap_protocol::ROLE_QUEUED => queued(&fs, &w, true),
        swap_protocol::ROLE_UNWARNED => queued(&fs, &w, false),
        swap_protocol::ROLE_HUNG => hung(&fs, &w),
        swap_protocol::ROLE_HANDOFF => handoff(&fs, &w),
        _ => direct(&fs, &w),
    }
}

// ===============================================================================================
// The default rung: the endpoint is the stable name and nothing stands in the data path.
// ===============================================================================================

fn direct(fs: &nifefs::Fs, w: &Wiring) -> ! {
    let v1 = image(fs, "rust_swappable", 2);
    let v2 = image(fs, "c_swappable", 3);
    let client_img = image(fs, "chatty", 4);

    // **What this operator will route, per child, by the name the component uses for it.** This is
    // the whole of what used to be four literal endowment arrays: no rights are spelled here, no
    // addresses, and no slot order, because every one of those is declared by the component and
    // computed by `component_plan`. What is left is the only thing an operator actually knows, which
    // is which of *its own* objects answers to which name.
    //
    // Two tables rather than one, and the reason is not that the manifests would take too much from
    // a wide one (they take what they declare either way). It is that the two bounds are
    // independent: the manifest bounds what a component may *ask* for, and a routing table bounds
    // what this operator *offers* it. A client that could be routed the device is a client one
    // manifest edit away from holding it.
    let to_component = Provisions {
        held: &[
            ("service", w.svc),
            ("report", REPORT),
            ("operator", w.note),
            ("control", w.poke),
            ("witness", w.log_page_frame),
            ("uart", DEVICE),
        ],
    };
    let to_client = Provisions {
        held: &[("service", w.svc), ("report", REPORT), ("operator", w.note)],
    };

    // ------------------------------------------------------------------------------------------
    // **The control that must fail, before anything exists.** The queue broker's declaration names
    // `requests` and `backend`, and this channel routes neither: the queued rung is the other role's
    // system. So a component this operator cannot provide for is refused as a *value*, with nothing
    // built, nothing mapped and nothing started, which is what makes a manifest a request rather
    // than an instruction. A run in which this *succeeds* is failed loudly, the same way an
    // instance's post-revoke probe surviving is.
    // ------------------------------------------------------------------------------------------

    match component_plan::plan(&swap_protocol::BROKER, &to_component) {
        Ok(_) => bail(60),
        Err(refusal) => send(REPORT, swap_protocol::RPT_REFUSED, refusal.code(), 0),
    };

    let Ok(component) = component_plan::plan(&swap_protocol::CONSOLE, &to_component) else {
        bail(61)
    };
    let Ok(client) = component_plan::plan(&swap_protocol::CLIENT, &to_client) else {
        bail(62)
    };

    // ------------------------------------------------------------------------------------------
    // The incumbent.
    // ------------------------------------------------------------------------------------------

    start_child(&v1, &component, w.faultep, [1, 0, 0], 11); // a device, log entries from 0

    // ------------------------------------------------------------------------------------------
    // Step 1: build the replacement, **before anyone is talking**. Endowed with everything except
    // the device, and not configured, so it cannot run and cannot race the incumbent for requests.
    //
    // Doing this first is the roadmap's own ordering, and there is a measurement behind keeping it:
    // building a process is a few hundred syscalls, and when this ran *after* the swap trigger the
    // client got through its entire conversation on RISC-V before the operator was ready. Laying
    // the replacement out ahead of time is what keeps the down window four syscalls wide instead of
    // "however long a build takes".
    // ------------------------------------------------------------------------------------------

    // The one endowment this operator still writes by hand, and it is the deferral rather than the
    // authority: `maps_without_devices` is the declaration minus what a revoke is about to take, and
    // `devices` below is the rest of it. `component_plan` sorts device mappings last so both halves
    // are slices of one plan and neither can be built by hand.
    let Ok(b_region) = supervision_protocol::memory_region_split(ROOT_UT, component.pages()) else {
        bail(20)
    };
    let Ok((b_child, b_aspace)) = supervision_protocol::build_child_space(
        ROOT_UT,
        b_region,
        &v2,
        &ChildEndowment {
            caps: component.caps(),
            maps: component.maps_without_devices(),
            blobs: &[],
            fault: Some(w.faultep),
            ..ChildEndowment::new(Retention::Nothing)
        },
    ) else {
        bail(21)
    };
    send(
        REPORT,
        swap_protocol::RPT_STEP,
        swap_protocol::step::BUILT,
        swap_protocol::V2,
    );

    // ------------------------------------------------------------------------------------------
    // The client that will talk across the swap, and the wait for its conversation to be well under
    // way. The incumbent tells us on its own channel after it has served SWAP_TRIGGER requests, so
    // the swap lands in the middle of a live conversation rather than at a moment we picked by
    // counting yields.
    // ------------------------------------------------------------------------------------------

    start_child(
        &client_img,
        &client,
        w.faultep,
        [swap_protocol::ROLE_CLIENT, 0, 0],
        14,
    );
    expect_note(w.note, swap_protocol::NOTE_SWAP_NOW, 17);

    // ------------------------------------------------------------------------------------------
    // **Dependency-aware orchestration's own question, asked and answered before step 2 acts.**
    // Milestone 23's residual: if some other live component is a client of the one about to be
    // swapped, swapping it means warning that client first. On this channel the answer is a real
    // empty set rather than a fixture: nothing this operator runs declares `console` in its own
    // `depends_on` (`CLIENT` is a pure consumer and never needs telling, per §41's sender-queue
    // argument recorded in `component_plan::Requirements::depends_on`'s doc comment). Reported so
    // the test can check the graph agrees with what this channel has always done: nothing extra.
    // ------------------------------------------------------------------------------------------

    let live = [component_plan::LiveInstance {
        id: 1,
        reqs: &swap_protocol::CONSOLE,
    }];
    let Ok(deps) = component_plan::dependents("console", &live) else {
        bail(64)
    };
    let order = deps.quiesce_order();
    send(
        REPORT,
        swap_protocol::RPT_DEPENDENTS,
        order.len() as u64,
        order.first().copied().unwrap_or(0),
    );

    // ------------------------------------------------------------------------------------------
    // Step 2: drain. The quiesce request travels on the endpoint being drained, so FIFO ordering
    // does the waiting for us.
    // ------------------------------------------------------------------------------------------

    let (verdict, served) = user_mode_runtime::call(w.svc, swap_protocol::OP_QUIESCE, 0);
    if verdict != swap_protocol::QUIESCED {
        bail(22)
    }
    send(
        REPORT,
        swap_protocol::RPT_STEP,
        swap_protocol::step::DRAINED,
        served,
    );

    // ------------------------------------------------------------------------------------------
    // Step 3: take the registers back. Every other holder loses the capability and the mapping; we
    // keep ours, which is what makes this a transfer rather than a demolition (DECISIONS §41).
    // ------------------------------------------------------------------------------------------

    if revoke_frame(DEVICE) != 0 {
        bail(23)
    }
    send(
        REPORT,
        swap_protocol::RPT_STEP,
        swap_protocol::step::REVOKED,
        0,
    );

    // ------------------------------------------------------------------------------------------
    // Step 4: endow the replacement with the registers and start it. It drains whatever parked on
    // the service endpoint's sender queue while nobody was receiving.
    // ------------------------------------------------------------------------------------------

    for &(va, slot, mode) in component.devices() {
        if map_into(b_aspace, va, slot, mode) != 0 {
            bail(24)
        }
    }
    if supervision_protocol::configure_child(b_child.tcb, b_aspace, v2.entry()).is_err() {
        bail(25)
    }
    if !supervision_protocol::start_child(b_child, 1, 0, 0) {
        bail(26)
    }
    cap_delete(b_region);
    send(
        REPORT,
        swap_protocol::RPT_STEP,
        swap_protocol::step::STARTED,
        swap_protocol::V2,
    );

    // ------------------------------------------------------------------------------------------
    // Step 5: the receipt for the revoke, and the teardown.
    //
    // The incumbent is quiesced and alive, holding a virtual address that used to be a UART. Tell
    // it to read one register. If step 3 was real it faults, and the kernel tells us where.
    // ------------------------------------------------------------------------------------------

    send(w.poke, swap_protocol::POKE_PROBE, 0, 0);
    let mut corpses = 0u64;
    let revoke_enforced = wait_for_fault(w.faultep, &mut corpses, swap_protocol::DEV_VA);

    // The conversation is still running: the client is somewhere in its sixties, being served by the
    // replacement. Wait for it to finish before reading the witness page, or the requests it has not
    // made yet would read as requests nobody served.
    expect_note(w.note, swap_protocol::NOTE_CLIENT_DONE, 28);
    reap_to(w.faultep, &mut corpses, 2); // the incumbent and the client

    // ------------------------------------------------------------------------------------------
    // The attacker, once the honest system has finished being interesting. It gets exactly the
    // client's capabilities and tries to become the server on the endpoint it is a client of.
    // ------------------------------------------------------------------------------------------

    // The attacker is wired from the client's own declaration and the client's own routing table, so
    // "exactly the honest client's capabilities" is now a property of the code rather than of two
    // arrays a reader has to compare.
    start_child(
        &client_img,
        &client,
        w.faultep,
        [swap_protocol::ROLE_USURPER, 0, 0],
        30,
    );
    expect_note(w.note, swap_protocol::NOTE_ATTACK_DONE, 33);
    reap_to(w.faultep, &mut corpses, 3); // and the attacker

    // Retire the replacement and collect it too, so the run ends with every region back in this
    // budget. Not tidiness: this is the same lifecycle machinery the swap itself is made of, run to
    // the end, and the test asserts on the memory coming home.
    retire(w, &mut corpses, 4, 34);

    // Our own witness: the log page, read in our address space, after every writer is gone.
    // Independent of the client's verdict, which was computed from replies in a different address
    // space and reaches the test on its own.
    send(
        REPORT,
        swap_protocol::RPT_LOG,
        verdict_from_log(0, revoke_enforced),
        changed_at(0),
    );
    user_mode_runtime::exit()
}

// ===============================================================================================
// State handoff (DECISIONS §209): the component carries state across the swap, and a swap whose
// state was not absorbed does not commit.
// ===============================================================================================

/// **The swap, for a component that declared a handoff page.** Five things differ from [`direct`],
/// and each is a sentence of §209:
///
/// - **No device.** The state is the point, and a device would put this system behind the same x86
///   gap the other three are behind (`swap_protocol::probe_device`'s x86 arm). So it runs on all
///   three architectures.
/// - **One frame routed to every instance under the handoff role**, installed at build time. The
///   operator never maps it itself and never reads it: the bytes are the component's.
/// - **The replacements' `control` is a different endpoint from the incumbent's.** Both are built
///   from one declaration, and the declaration's role name is the component's while the object is
///   this operator's, so a resume meant for the incumbent cannot reach a replacement.
/// - **The incumbent is retired only after the replacement says it absorbed the state.** Before that,
///   nothing has committed.
/// - **A replacement that refuses leaves nothing behind.** It never received a request, it exits, and
///   the reap that collects it returns its region: the new grant is revoked by the machinery of §16 (object revocation), with
///   no rollback mechanism of its own. The incumbent is told to resume, holding the state it never
///   lost.
fn handoff(fs: &nifefs::Fs, w: &Wiring) -> ! {
    let v1 = image(fs, "rust_swappable", 2);
    let v2 = image(fs, "c_swappable", 3);
    let client_img = image(fs, "chatty", 4);

    let state = page_frame(90);
    let replacement_control = obj(abi::objtype::RENDEZVOUS, 91);

    let to_incumbent = Provisions {
        held: &[
            ("service", w.svc),
            ("report", REPORT),
            ("operator", w.note),
            ("control", w.poke),
            ("witness", w.log_page_frame),
            (component_plan::HANDOFF_ROLE, state),
        ],
    };
    let to_replacement = Provisions {
        held: &[
            ("service", w.svc),
            ("report", REPORT),
            ("operator", w.note),
            ("control", replacement_control),
            ("witness", w.log_page_frame),
            (component_plan::HANDOFF_ROLE, state),
        ],
    };
    let to_client = Provisions {
        held: &[("service", w.svc), ("report", REPORT), ("operator", w.note)],
    };

    // ------------------------------------------------------------------------------------------
    // **The control that must fail, before anything exists.** A supervisor that routes no handoff
    // page cannot carry state, and wiring a stateful component anyway would be a kill-and-replace
    // that reads exactly like a successful swap while the tally went to zero. The client's routing
    // table is such a supervisor, so asking it for the stateful contract is a real refusal.
    // ------------------------------------------------------------------------------------------

    match component_plan::plan(&swap_protocol::TALLY, &to_client) {
        Ok(_) => bail(92),
        Err(refusal) => send(REPORT, swap_protocol::RPT_REFUSED, refusal.code(), 0),
    };

    let Ok(incumbent) = component_plan::plan(&swap_protocol::TALLY, &to_incumbent) else {
        bail(93)
    };
    let Ok(replacement) = component_plan::plan(&swap_protocol::TALLY, &to_replacement) else {
        bail(94)
    };
    let Ok(client) = component_plan::plan(&swap_protocol::CLIENT, &to_client) else {
        bail(95)
    };
    if incumbent.handoff() != Some(swap_protocol::STATE_VA) {
        bail(96)
    }

    start_child(
        &v1,
        &incumbent,
        w.faultep,
        [swap_protocol::START_FRESH, 0, swap_protocol::LAYOUT_1],
        11,
    );

    // Both replacements built before anyone is talking, for `direct`'s reason: the down window is
    // then a start and an absorb wide, not a build.
    let refuser = build_unstarted(&v2, &replacement, w.faultep, 20);
    let absorber = build_unstarted(&v2, &replacement, w.faultep, 22);
    send(
        REPORT,
        swap_protocol::RPT_STEP,
        swap_protocol::step::BUILT,
        swap_protocol::V2,
    );

    start_child(
        &client_img,
        &client,
        w.faultep,
        [swap_protocol::ROLE_CLIENT, 0, 0],
        14,
    );
    expect_note(w.note, swap_protocol::NOTE_SWAP_NOW, 17);

    // ------------------------------------------------------------------------------------------
    // Attempt one: a replacement that cannot read the incumbent's layout.
    // ------------------------------------------------------------------------------------------

    let mut corpses = 0u64;
    drain(w, 97);
    launch(
        refuser,
        &v2,
        [swap_protocol::START_ABSORB, 0, swap_protocol::LAYOUT_2],
        98,
    );
    let (kind, _, found) = recv(w.note);
    if kind != swap_protocol::NOTE_REFUSED {
        bail(100)
    }
    // The refuser's exit is the only death possible here: the incumbent is quiesced and alive, and
    // the client is parked on the service endpoint's sender queue with nobody receiving.
    let (event, _) = collect_corpse(w.faultep, &mut corpses);
    if event == abi::fault::EVENT_FAULT {
        bail(101)
    }
    send(
        REPORT,
        swap_protocol::RPT_STEP,
        swap_protocol::step::ROLLED_BACK,
        found,
    );
    send(w.poke, swap_protocol::POKE_RESUME, 0, 0);

    // ------------------------------------------------------------------------------------------
    // Attempt two: the incumbent asks again once it has served another SWAP_TRIGGER requests, and
    // this replacement understands the layout. Only its `NOTE_ABSORBED` commits the swap.
    // ------------------------------------------------------------------------------------------

    expect_note(w.note, swap_protocol::NOTE_SWAP_NOW, 102);
    drain(w, 103);
    launch(
        absorber,
        &v2,
        [swap_protocol::START_ABSORB, 0, swap_protocol::LAYOUT_1],
        104,
    );
    let (kind, _, carried) = recv(w.note);
    if kind != swap_protocol::NOTE_ABSORBED {
        bail(106)
    }
    send(
        REPORT,
        swap_protocol::RPT_STEP,
        swap_protocol::step::ABSORBED,
        carried,
    );
    send(w.poke, swap_protocol::POKE_QUIT, 0, 0);
    send(
        REPORT,
        swap_protocol::RPT_STEP,
        swap_protocol::step::STARTED,
        swap_protocol::V2,
    );

    expect_note(w.note, swap_protocol::NOTE_CLIENT_DONE, 107);
    reap_to(w.faultep, &mut corpses, 3); // the refuser, the incumbent, and the client

    // Retire the replacement on its own control endpoint, which is not `w.poke`.
    let (verdict, _) = user_mode_runtime::call(w.svc, swap_protocol::OP_QUIESCE, 0);
    if verdict != swap_protocol::QUIESCED {
        bail(108)
    }
    send(replacement_control, swap_protocol::POKE_QUIT, 0, 0);
    reap_to(w.faultep, &mut corpses, 4);

    send(
        REPORT,
        swap_protocol::RPT_LOG,
        verdict_from_log(0, false),
        changed_at(0),
    );
    user_mode_runtime::exit()
}

/// Quiesce whoever is serving the stable endpoint, and report the drain.
fn drain(w: &Wiring, stage: u64) {
    let (verdict, served) = user_mode_runtime::call(w.svc, swap_protocol::OP_QUIESCE, 0);
    if verdict != swap_protocol::QUIESCED {
        bail(stage)
    }
    send(
        REPORT,
        swap_protocol::RPT_STEP,
        swap_protocol::step::DRAINED,
        served,
    );
}

/// A child laid out and endowed but not configured, so it cannot run: `direct`'s step 1, for a
/// component with nothing to defer.
struct Unstarted {
    child: supervision_protocol::Child,
    aspace: u64,
    region: u64,
}

fn build_unstarted(
    elf: &elf::Elf,
    plan: &component_plan::Plan,
    faultep: u64,
    stage: u64,
) -> Unstarted {
    let Ok(region) = supervision_protocol::memory_region_split(ROOT_UT, plan.pages()) else {
        bail(stage)
    };
    let Ok((child, aspace)) = supervision_protocol::build_child_space(
        ROOT_UT,
        region,
        elf,
        &ChildEndowment {
            caps: plan.caps(),
            maps: plan.maps(),
            blobs: &[],
            fault: Some(faultep),
            ..ChildEndowment::new(Retention::Nothing)
        },
    ) else {
        bail(stage + 1)
    };
    Unstarted {
        child,
        aspace,
        region,
    }
}

fn launch(u: Unstarted, elf: &elf::Elf, args: [u64; 3], stage: u64) {
    if supervision_protocol::configure_child(u.child.tcb, u.aspace, elf.entry()).is_err() {
        bail(stage)
    }
    if !supervision_protocol::start_child(u.child, args[0], args[1], args[2]) {
        bail(stage + 1)
    }
    cap_delete(u.region);
}

// ===============================================================================================
// The opt-in rung: a queue broker between producer and backend, so the producer never blocks on an
// absent consumer.
// ===============================================================================================

/// `warn` is `false` only on [`ROLE_UNWARNED`](swap_protocol::ROLE_UNWARNED): the graph is still
/// asked and still names `broker`, and the operator then does not tell it, as if it had not
/// answered. Everything else is identical, so the difference in the producer's verdict is the price
/// of the warning and nothing else.
fn queued(fs: &nifefs::Fs, w: &Wiring, warn: bool) -> ! {
    let v1 = image(fs, "rust_swappable", 2);
    let v2 = image(fs, "c_swappable", 3);
    let client_img = image(fs, "chatty", 4);
    let broker_img = image(fs, "broker", 40);

    // `svc` is the *back* endpoint here: what the broker forwards to and what a backend receives
    // on. `front` is what the producer holds, and it is the stable name on this channel.
    let front = obj(abi::objtype::RENDEZVOUS, 41);
    let base = swap_protocol::BROKER_LOG_BASE;

    // **Three routing tables, and the interesting thing about them is `service`.** The producer's
    // declaration is `swap_protocol::CLIENT`, byte for byte the one the direct channel's client is
    // wired from, and it asks to use an endpoint it calls `service`. Here that name resolves to the
    // broker's front endpoint instead of to the component's. One declaration, two routings, and the
    // program cannot tell which it got: that is the indirection the manifest buys, and it is what
    // makes a component's *peer* substitutable and not only the component.
    let to_backend = Provisions {
        held: &[
            ("service", w.svc),
            ("report", REPORT),
            ("operator", w.note),
            ("control", w.poke),
            ("witness", w.log_page_frame),
        ],
    };
    let to_broker = Provisions {
        held: &[
            ("requests", front),
            ("backend", w.svc),
            ("report", REPORT),
            ("operator", w.note),
        ],
    };
    let to_producer = Provisions {
        held: &[("service", front), ("report", REPORT), ("operator", w.note)],
    };

    // The control that must fail on this channel, and it is the mirror of the direct one: no device
    // is routed here, so the console component's declaration cannot be satisfied and is refused
    // before anything is built. This is the same refusal from the other side, which is why both
    // roles report it: a mechanism that only worked for the one component it was written against
    // would not be a mechanism.
    match component_plan::plan(&swap_protocol::CONSOLE, &to_backend) {
        Ok(_) => bail(60),
        Err(refusal) => send(REPORT, swap_protocol::RPT_REFUSED, refusal.code(), 0),
    };

    // No device on this channel: the backend behind a broker is a plain service, and mixing the
    // device story into the queue story would make it unclear which mechanism carried which claim.
    let Ok(backend) = component_plan::plan(&swap_protocol::BACKEND, &to_backend) else {
        bail(61)
    };
    let Ok(broker) = component_plan::plan(&swap_protocol::BROKER, &to_broker) else {
        bail(62)
    };
    let Ok(producer) = component_plan::plan(&swap_protocol::CLIENT, &to_producer) else {
        bail(63)
    };

    start_child(&v1, &backend, w.faultep, [0, base, 0], 42);
    start_child(&broker_img, &broker, w.faultep, [0, 0, 0], 43);
    start_child(
        &client_img,
        &producer,
        w.faultep,
        [swap_protocol::ROLE_PRODUCER, 0, 0],
        44,
    );

    expect_note(w.note, swap_protocol::NOTE_SWAP_NOW, 45);

    // ------------------------------------------------------------------------------------------
    // **Dependency-aware orchestration, for real this time.** `BOP_DOWN` below used to be
    // unconditional: every system on this channel happens to have exactly one component
    // (`broker`) that forwards synchronously to the backend, so "always warn it" and "warn
    // whoever the graph names" have always produced the same four syscalls. What changed is which
    // one this operator actually asked. `broker`'s own manifest (`swap_protocol::BROKER`) declares
    // `depends_on: &["backend"]`, so a two-instance live registry naming this system's broker and
    // backend, checked against `component_plan::dependents`, is what decides whether `BOP_DOWN` is
    // sent at all -- not this function's own memory of what it built five lines up.
    // ------------------------------------------------------------------------------------------

    let live = [
        component_plan::LiveInstance {
            id: 1,
            reqs: &swap_protocol::BACKEND,
        },
        component_plan::LiveInstance {
            id: 2,
            reqs: &swap_protocol::BROKER,
        },
    ];
    let Ok(deps) = component_plan::dependents("backend", &live) else {
        bail(64)
    };
    let order = deps.quiesce_order();
    send(
        REPORT,
        swap_protocol::RPT_DEPENDENTS,
        order.len() as u64,
        order.first().copied().unwrap_or(0),
    );

    // Tell every dependent the graph named to take custody. From here until BOP_UP there is no
    // backend at all on this channel, and the producer keeps running: that window is the whole
    // reason this rung exists. This system's registry has exactly one entry (`broker`, id 2), so
    // the loop below sends `BOP_DOWN` once; a system with a second forwarding dependent would send
    // it to each one the graph named, in the order the graph returned them.
    //
    // **A `CALL`, so a dependent that does not answer hangs this operator too**, which is the
    // defect notes/non-cooperative-fallback.md starts from. `warn == false` is the measurement of
    // the way out: what the swap costs if the warning is simply never sent.
    for &id in order.iter().filter(|_| warn) {
        if id == 2 {
            let (r, _) = user_mode_runtime::call(front, swap_protocol::BOP_DOWN, 0);
            if r != 0 {
                bail(46)
            }
        }
    }

    // Quiesce the backend and let it die, exactly as on the direct channel, minus the device.
    let (verdict, served) = user_mode_runtime::call(w.svc, swap_protocol::OP_QUIESCE, 0);
    if verdict != swap_protocol::QUIESCED {
        bail(47)
    }
    send(
        REPORT,
        swap_protocol::RPT_STEP,
        swap_protocol::step::DRAINED,
        served,
    );
    send(w.poke, swap_protocol::POKE_QUIT, 0, 0);
    let mut corpses = 0u64;
    reap_to(w.faultep, &mut corpses, 1);

    // The replacement, in a different language, on the same back endpoint, wired from the same
    // declaration the outgoing one was.
    start_child(&v2, &backend, w.faultep, [0, base, 0], 48);
    send(
        REPORT,
        swap_protocol::RPT_STEP,
        swap_protocol::step::STARTED,
        swap_protocol::V2,
    );

    // Release the backlog, one dependent at a time, in the reverse of the order they were warned:
    // the graph's own resume order. The broker drains in arrival order before it answers, so this
    // call returning means every buffered item has reached the new backend.
    for &id in order.iter().rev().filter(|_| warn) {
        if id == 2 {
            let (r, _drained) = user_mode_runtime::call(front, swap_protocol::BOP_UP, 0);
            if r != 0 {
                bail(49)
            }
        }
    }

    expect_note(w.note, swap_protocol::NOTE_CLIENT_DONE, 50);
    reap_to(w.faultep, &mut corpses, 2); // and the producer

    // Shut the channel down so the run leaves nothing running and nothing spent: the broker exits,
    // then the backend quiesces and exits, and both corpses are collected.
    let _ = user_mode_runtime::call(front, swap_protocol::OP_QUIESCE, 0);
    expect_note(w.note, swap_protocol::NOTE_BROKER_DONE, 51);
    reap_to(w.faultep, &mut corpses, 3); // and the broker
    retire(w, &mut corpses, 4, 52); // and the replacement backend

    send(
        REPORT,
        swap_protocol::RPT_LOG,
        verdict_from_log(base, false),
        changed_at(base),
    );
    user_mode_runtime::exit()
}

// ===============================================================================================
// The third residual: a component that stops answering **without dying**.
//
// Every failure the rest of this program handles is a death. The incumbent quiesces because it was
// asked to, or it faults on a device it no longer has, and either way the kernel sends a
// five-word message to the supervision endpoint, `Rendezvous::REAP` collects the corpse, and the
// region comes home. A component that simply stops answering produces none of that: it holds its
// endpoint, holds its device, sits `Blocked`, and is indistinguishable from a healthy server waiting
// for work, because that is what a healthy server waiting for work is.
//
// This role runs the swap against one, and reports three things a supervisor learns the hard way.
// See notes/hung-component.md; the two questions this run does **not** answer (how a supervisor
// notices, and what it may do about a component that will not cooperate at all) are decisions rather
// than code, and that note states them.
// ===============================================================================================

fn hung(fs: &nifefs::Fs, w: &Wiring) -> ! {
    let v1 = image(fs, "rust_swappable", 2);
    let v2 = image(fs, "c_swappable", 3);
    let client_img = image(fs, "chatty", 4);

    // Identical to the direct channel's routing: same declarations, same objects, same rights.
    // Nothing about the wiring says this channel is different, which is the point. What differs is
    // one start argument, and a start argument carries no authority.
    let to_component = Provisions {
        held: &[
            ("service", w.svc),
            ("report", REPORT),
            ("operator", w.note),
            ("control", w.poke),
            ("witness", w.log_page_frame),
            ("uart", DEVICE),
        ],
    };
    let to_client = Provisions {
        held: &[("service", w.svc), ("report", REPORT), ("operator", w.note)],
    };
    let Ok(component) = component_plan::plan(&swap_protocol::CONSOLE, &to_component) else {
        bail(70)
    };
    let Ok(client) = component_plan::plan(&swap_protocol::CLIENT, &to_client) else {
        bail(71)
    };

    // The incumbent, told which one request it will never answer.
    start_child(
        &v1,
        &component,
        w.faultep,
        [1, 0, swap_protocol::WEDGE_SEQ],
        72,
    );

    // The replacement, built and endowed with everything except the device, and **not configured**,
    // exactly as on the direct channel and for the same measured reason: a build is a few hundred
    // syscalls, and doing it after the trigger loses the race against a conversation already in
    // flight. Here it matters more, not less: a hung component gives the operator no drain to hide
    // the build behind.
    let Ok(b_region) = supervision_protocol::memory_region_split(ROOT_UT, component.pages()) else {
        bail(75)
    };
    let Ok((b_child, b_aspace)) = supervision_protocol::build_child_space(
        ROOT_UT,
        b_region,
        &v2,
        &ChildEndowment {
            caps: component.caps(),
            maps: component.maps_without_devices(),
            blobs: &[],
            fault: Some(w.faultep),
            ..ChildEndowment::new(Retention::Nothing)
        },
    ) else {
        bail(76)
    };
    send(
        REPORT,
        swap_protocol::RPT_STEP,
        swap_protocol::step::BUILT,
        swap_protocol::V2,
    );

    start_child(
        &client_img,
        &client,
        w.faultep,
        [swap_protocol::ROLE_CLIENT, 0, 0],
        77,
    );
    expect_note(w.note, swap_protocol::NOTE_SWAP_NOW, 80);

    // ------------------------------------------------------------------------------------------
    // The hang. Served with `RECV_CAP` rather than `RECV`, because the incumbent announced it with a
    // `CALL`: taking the reply capability and never using it is what keeps that component parked,
    // and holding it is the only handle anything in this system has on a wedged process.
    // ------------------------------------------------------------------------------------------

    let (kind, release, served) = user_mode_runtime::recv_cap(w.note);
    if kind != swap_protocol::NOTE_WEDGED || release == abi::rendezvous::NO_CAP {
        bail(81)
    }
    send(REPORT, swap_protocol::RPT_WEDGED, swap_protocol::V1, served);

    // ------------------------------------------------------------------------------------------
    // **What the supervisor can see.** Two reports, and both of them are negative results.
    //
    // The domain, through `abi::rendezvous::SURVEY` (milestone 126), which is the only view of its own
    // children this operator has: every member `BLOCKED`, none `DEAD`. That is also what a healthy
    // idle system looks like from here, and no amount of looking again changes it.
    //
    // Then `Rendezvous::REAP` on every member, which is the supervisor's entire vocabulary over its
    // domain since DECISIONS §32: refused, every one, `StillAlive`. Asking about all of them rather
    // than about the one suspect is both easier (a survey returns tids and nothing that says which
    // is which) and the stronger claim.
    // ------------------------------------------------------------------------------------------

    let (members, states) = survey_domain(w.faultep);
    send(REPORT, swap_protocol::RPT_SURVEY, members, states);
    let (asked, refused) = ask_the_domain_to_be_collected(w.faultep);
    send(REPORT, swap_protocol::RPT_UNCOLLECTABLE, asked, refused);

    // ------------------------------------------------------------------------------------------
    // **Recovery, with no authority this operator did not already hold.** DECISIONS §32 records that
    // a supervisor which must restart a *hung* child "still needs the stronger right", meaning the
    // construction authority reaping was moved off. For restarting the **service**, that is not so,
    // and these four syscalls are the argument:
    //
    //   - The device comes back with `PageFrame::REVOKE`, which is `GRANT`-gated take-back (§41) and
    //     asks the current holder for nothing. It works on a live, wedged, wholly uncooperative
    //     holder exactly as it works on a quiesced one.
    //   - There is nothing to drain, and nothing to drain it *for*: a component that is not
    //     receiving has already achieved what `OP_QUIESCE` exists to achieve. The step that needs
    //     the incumbent's cooperation is the one step the hang makes unnecessary.
    //   - The replacement parks in `RECV_CAP` on the same endpoint and picks up whatever queued
    //     behind the silence, because the stable name is the endpoint object and the kernel's sender
    //     queue is the buffer (§41).
    //
    // What still needs the stronger right is **reclaiming the hung component's memory**, which is a
    // different thing from restarting its service and is not attempted here. notes/hung-component.md
    // has the argument and the reason it is a decision rather than a task.
    // ------------------------------------------------------------------------------------------

    if revoke_frame(DEVICE) != 0 {
        bail(82)
    }
    send(
        REPORT,
        swap_protocol::RPT_STEP,
        swap_protocol::step::REVOKED,
        0,
    );

    for &(va, slot, mode) in component.devices() {
        if map_into(b_aspace, va, slot, mode) != 0 {
            bail(83)
        }
    }
    if supervision_protocol::configure_child(b_child.tcb, b_aspace, v2.entry()).is_err() {
        bail(84)
    }
    if !supervision_protocol::start_child(b_child, 1, 0, 0) {
        bail(85)
    }
    cap_delete(b_region);
    send(
        REPORT,
        swap_protocol::RPT_STEP,
        swap_protocol::step::STARTED,
        swap_protocol::V2,
    );

    // ------------------------------------------------------------------------------------------
    // **The one thing only the hung component can do.** The service is back, and the caller whose
    // request the incumbent swallowed is still parked inside its `CALL`: restoring a service does
    // not recover a caller, and nothing above went anywhere near it.
    //
    // So the operator answers the incumbent's own `CALL`, which is the handle it has been holding
    // since the hang. The incumbent wakes, uses the reply capability it took from that caller, and
    // then reads the register it no longer owns. **In this run the wedge is deliberate and it
    // cooperates.** A real one does not, and then that caller waits for the life of the machine,
    // which is what makes this line the shape of the gap rather than its cure.
    // ------------------------------------------------------------------------------------------

    user_mode_runtime::reply(release, swap_protocol::NOTE_RELEASE, 0);

    let mut corpses = 0u64;
    let revoke_enforced = wait_for_fault(w.faultep, &mut corpses, swap_protocol::DEV_VA);
    expect_note(w.note, swap_protocol::NOTE_CLIENT_DONE, 86);
    reap_to(w.faultep, &mut corpses, 2); // the incumbent and the client
    retire(w, &mut corpses, 3, 87); // and the replacement

    send(
        REPORT,
        swap_protocol::RPT_LOG,
        verdict_from_log(0, revoke_enforced),
        changed_at(0),
    );
    user_mode_runtime::exit()
}

/// **Walk the supervision domain and count what state its members are in** (milestone 126's
/// `abi::rendezvous::SURVEY`). Returns `(members, states)`, the second packed by
/// `swap_protocol::survey_counts`.
///
/// A refusal is reported as `(u64::MAX, error)` rather than as an empty domain, for the reason
/// milestone 126 built the method with: a monitor that reports nothing because it could not look is
/// the worst failure it has available. This operator holds the endpoint it retyped itself, so it
/// holds `ENUMERATE` and cannot be refused; saying so in code is cheaper than assuming it.
fn survey_domain(faultep: u64) -> (u64, u64) {
    let mut counts = [0u64; 5];
    let mut members = 0u64;
    let mut cursor = abi::survey::DONE;
    loop {
        let (next, _tid, state) = user_mode_runtime::survey(faultep, cursor);
        if next < 0 {
            return (u64::MAX, (-next) as u64);
        }
        let next = next as u64;
        if next == abi::survey::DONE {
            break;
        }
        // A cursor that did not advance would spin here forever. The kernel does not do this; the
        // check is here so this loop terminates for every reader, which is `ps::collect`'s rule too.
        if next <= cursor {
            break;
        }
        if (state as usize) < counts.len() {
            counts[state as usize] += 1;
        }
        members += 1;
        cursor = next;
    }
    (
        members,
        swap_protocol::survey_counts(
            counts[abi::survey::READY as usize],
            counts[abi::survey::RUNNING as usize],
            counts[abi::survey::BLOCKED as usize],
            counts[abi::survey::DEAD as usize],
        ),
    )
}

/// **Ask the kernel to collect every member of the domain, and count the refusals.** Returns
/// `(asked, refused_still_alive)`.
///
/// This is not a reap loop that happens to fail. It is the assertion that a supervisor's whole
/// vocabulary over a live domain is empty: `Rendezvous::REAP` (DECISIONS §32) authorizes *collecting a
/// corpse* and refuses a thread that is still alive, deliberately, because killing is the stronger
/// act and lives elsewhere. Against a hung component, "collect the corpse" is the only verb a
/// supervisor has and there is no corpse.
///
/// It is side-effect free by construction: `reap_supervised` decides `StillAlive` before it looks up
/// a region, so a refused reap moves nothing and frees nothing.
fn ask_the_domain_to_be_collected(faultep: u64) -> (u64, u64) {
    // The raw syscall return, which is the **negative** discriminant: `abi::Error`'s variants are
    // already negative (`StillAlive = -9`) and `invoke` hands them back unchanged. Programs that
    // report an error upward negate it to get a positive code (`chatty`'s attack report does), and
    // comparing against a negated constant here was this lane's one wrong line: it made every
    // refusal look like a success and the test said 0 of 2.
    let still_alive = abi::Error::StillAlive as i64;
    let mut asked = 0u64;
    let mut refused = 0u64;
    let mut cursor = abi::survey::DONE;
    loop {
        let (next, tid, _state) = user_mode_runtime::survey(faultep, cursor);
        if next < 0 {
            break;
        }
        let next = next as u64;
        if next == abi::survey::DONE || next <= cursor {
            break;
        }
        asked += 1;
        if user_mode_runtime::reap(faultep, tid) == still_alive {
            refused += 1;
        }
        cursor = next;
    }
    (asked, refused)
}

// ===============================================================================================
// The pieces every role shares.
// ===============================================================================================

/// Split a region, build a child in it, start it, and drop both capabilities. Neither is the thing
/// itself: a TCB capability is not the thread (dropping it leaves the thread running), and since
/// DECISIONS §32 the region capability is not the reap either.
///
/// **It takes a plan rather than an endowment**, which is milestone 23's manifest lane in one
/// signature: the caller says which component and which supervision endpoint, and every capability,
/// every mapping and the region size come from the component's own declaration. `args` stays a
/// caller's business because it is configuration and not authority (which log entries this instance
/// writes, and which of `chatty`'s three roles it is playing).
fn start_child(
    elf: &elf::Elf,
    plan: &component_plan::Plan,
    faultep: u64,
    args: [u64; 3],
    stage: u64,
) {
    let Ok(region) = supervision_protocol::memory_region_split(ROOT_UT, plan.pages()) else {
        bail(stage)
    };
    let endow = ChildEndowment {
        caps: plan.caps(),
        maps: plan.maps(),
        blobs: &[],
        fault: Some(faultep),
        ..ChildEndowment::new(Retention::Nothing)
    };
    let Ok(child) = supervision_protocol::build_child(ROOT_UT, region, elf, &endow) else {
        bail(stage + 1)
    };
    if !supervision_protocol::start_child(child, args[0], args[1], args[2]) {
        bail(stage + 2)
    }
    cap_delete(region);
}

/// Wait for one death, report it, and collect the corpse through the supervision endpoint it
/// arrived on (DECISIONS §32). Returns `(event, fault address)`.
///
/// **Every child this operator starts is supervised and every corpse is collected**, so a run ends
/// with all five instance regions back in this budget. The one that is not tidiness is the one
/// below; the rest are here so the test can assert that a swap system reclaims itself.
fn collect_corpse(faultep: u64, collected: &mut u64) -> (u64, u64) {
    let (event, tid, _pc, addr, _) = recv_fault(faultep);
    send(REPORT, swap_protocol::RPT_DEATH, tid, event);
    // We hold no capability to that region: we deleted it the moment the child was started, and the
    // authority for this is the supervision relationship, not the memory.
    if user_mode_runtime::reap(faultep, tid) != 0 {
        bail(27)
    }
    *collected += 1;
    (event, addr)
}

/// Collect corpses until `target` of them have been collected in total.
///
/// **Counted, not ordered.** Deaths arrive in whatever order the children happen to die in, and a
/// program that waited for a *particular* child's death would hang the first time two of them
/// finished the other way round. Counting is enough because the only thing done with a corpse here
/// is reap it, and the one death whose identity matters is picked out by [`wait_for_fault`].
fn reap_to(faultep: u64, collected: &mut u64, target: u64) {
    while *collected < target {
        collect_corpse(faultep, collected);
    }
}

/// **Wait for the receipt.** Collect corpses until one is a *fault*, and say whether it faulted on
/// the page `expect_addr` sits in.
///
/// A loop rather than a single receive because the client may finish and exit at any moment, and a
/// clean exit arriving first must not be mistaken for the answer. Telling them apart is exactly
/// what DECISIONS §26's two event codes are for.
///
/// The page, not the byte: the fault address the kernel reports is the *register* the component was
/// reading (`DEV_VA + 0x18` for a PL011's flag register), and what the revoke took away is the page.
/// Comparing the whole address would have been comparing the register layout.
fn wait_for_fault(faultep: u64, collected: &mut u64, expect_addr: u64) -> bool {
    loop {
        let (event, addr) = collect_corpse(faultep, collected);
        if event == abi::fault::EVENT_FAULT {
            send(REPORT, swap_protocol::RPT_SITE, addr, expect_addr);
            send(
                REPORT,
                swap_protocol::RPT_STEP,
                swap_protocol::step::REAPED,
                0,
            );
            return addr & !(swap_protocol::PAGE - 1) == expect_addr;
        }
    }
}

/// Retire the last live instance on a channel: quiesce it, tell it to go, collect its corpse. The
/// swap's own machinery, run once more with nothing to replace.
fn retire(w: &Wiring, collected: &mut u64, target: u64, stage: u64) {
    let (verdict, _) = user_mode_runtime::call(w.svc, swap_protocol::OP_QUIESCE, 0);
    if verdict != swap_protocol::QUIESCED {
        bail(stage)
    }
    send(w.poke, swap_protocol::POKE_QUIT, 0, 0);
    reap_to(w.faultep, collected, target);
}

fn expect_note(note: u64, want: u64, stage: u64) {
    let (kind, _, _) = recv(note);
    if kind != want {
        bail(stage)
    }
}

fn image<'a>(fs: &nifefs::Fs<'a>, name: &str, stage: u64) -> elf::Elf<'a> {
    match fs.read(name).map(elf::Elf::parse) {
        Some(Ok(e)) => e,
        _ => bail(stage),
    }
}

fn obj(objtype: u64, stage: u64) -> u64 {
    match supervision_protocol::retype_obj_from(ROOT_UT, objtype) {
        Ok(s) => s,
        Err(()) => bail(stage),
    }
}

fn page_frame(stage: u64) -> u64 {
    match supervision_protocol::retype_page_frame_from(ROOT_UT) {
        Ok(s) => s,
        Err(()) => bail(stage),
    }
}

/// **The operator's verdict**, read out of the log page byte by byte.
fn verdict_from_log(base: u64, revoke_enforced: bool) -> u64 {
    let mut bits = lc::NO_GAP | lc::MONOTONE;
    let mut seen_v1 = false;
    let mut seen_v2 = false;
    let mut last = 0u64;
    for seq in 0..swap_protocol::REQUESTS {
        let v = swap_protocol::log_get(base + seq);
        if v == 0 {
            bits &= !lc::NO_GAP; // a request nobody served: lost in the down window
        }
        if v < last {
            bits &= !lc::MONOTONE; // the old instance answered after the new one: two owners
        }
        if v == swap_protocol::V1 {
            seen_v1 = true;
        }
        if v == swap_protocol::V2 {
            seen_v2 = true;
        }
        last = v;
    }
    if seen_v1 && seen_v2 {
        bits |= lc::BOTH_VERSIONS;
    }
    if revoke_enforced {
        bits |= lc::REVOKE_ENFORCED;
    }
    bits
}

/// The sequence number the version changed at, so the test can see the swap landed inside the
/// conversation rather than at one of its ends.
fn changed_at(base: u64) -> u64 {
    let mut last = 0u64;
    for seq in 0..swap_protocol::REQUESTS {
        let v = swap_protocol::log_get(base + seq);
        if last != 0 && v != last {
            return seq;
        }
        last = v;
    }
    0
}

/// Report which stage failed, then trap. A half-built system is not worth limping along, and the
/// stage code turns "nothing happened" into a legible failure.
fn bail(stage: u64) -> ! {
    send(REPORT, swap_protocol::RPT_FAILED, stage, 0);
    swap_protocol::fail()
}

user_mode_runtime::panic_handler!();
