//! **Nine roles of milestone 19d and 19e: a userspace parent that builds children out of ELFs it
//! parsed.** Which one runs is chosen by the word the kernel puts in `x0` at `_start`, the way a
//! real kernel hands a new process its argc.
//!
//! Six of them are the parent. `INIT` builds a plain child; `INIT_DEV` builds one and hands it a
//! device's registers; `INIT_IRQ` builds one and delegates it an interrupt; `INIT_CONSOLE` builds
//! the real console server and then plays its client; `INIT_LEAST_AUTHORITY_DEMO` and
//! `INIT_COREMARK` build programs they did not write and start them with an argument. Three are
//! the children those roles build: `CHILD` reports a word, `DEV_CHILD` reads a device's identity
//! registers, `IRQ_CHILD` blocks on an interrupt.
//!
//! **The name is wrong and the rename is calef's** (see the limitation below). What is left here
//! is not a greeting and has not been since 19d.
//!
//! # The principle this file is the exception to
//!
//! **A program does one thing, and a role is an exception that has to say why.** The tree enacted
//! that rule five times without ever writing it down: the least-authority demo left in 19f.2, the
//! console in 19f.3, the input driver in 19f.4, the shell in 19f.5, and `init_boot` in 266. Five
//! departures, each recorded in its own file as a local fix, and no principle anywhere; so the
//! sixth arm anybody wanted to add had nothing to read that said not to. Milestone 291 deleted
//! seven more roles, split fourteen into programs, and states the rule here, where a reader meets
//! the last binary in this tree still dispatching on `x0`.
//!
//! **Why these nine are the exception, for now.** Six of them build a child out of an ELF, and
//! three of them *are* that child: the parent finds its child by looking *itself* up in the
//! archive ([`ROLES_ENTRY`]) and re-entering its own image at a different role. Splitting the
//! parents from the children means each parent naming its child's archive entry instead, which
//! changes what `kernel::user::spawn_hello` has to know: today it always re-enters this binary,
//! and six roles becoming six programs means it would have to name an entry per role instead.
//! That is a boot-path change rather than a fixtures change, and it is the one piece 291 did not
//! take. See
//! `design/roadmap/291-one-program-one-job.md`.
//!
//! # Bugs
//!
//! **The name does not describe the contents, and this block used to claim otherwise.** Until
//! milestone 291 it read *"The limitation that used to be recorded here is closed (milestone
//! 266)"*, on the grounds that 266 had moved the boot role out. 266 moved **one** role out and
//! thirty-one remained, so the claim was false on the day it was written and stayed false for
//! three weeks. What is here now is nine roles of userspace process construction, which `hello`
//! describes no better than it described thirty-one. calef deferred the new name until there was
//! something settled to name (2026-09-14: *"If there is anything left then we can consider a name
//! for what remains"*); this is what is left.
//!
//! **A caller that asks for a role this binary does not have gets a trap**, which is deliberate
//! (the `_` arm used to run the self-checker and report success), but it is a trap rather than a
//! message: the kernel counts a fault and the spawner waiting on a report waits until its
//! watchdog. Nothing here can do better, because a program entered at a role it does not have may
//! not hold a capability to report on.
//!
//! Name: unrecorded, and overdue. Nobody wrote down why `hello` is called `hello` and nobody
//! needed to while it was a first program: it is the universal name for one, and this was the
//! first program this kernel ever loaded, on 2026-07-14. It has not been a first program since
//! milestone 19d and stopped being a catalogue at 291; the rename is calef's and is deferred until
//! there is something settled to name. See the `# Bugs` section above.

#![no_std]
// Program entry points, not the crates/ library surface milestone 68's ratchet tracks
// (DECISIONS §107): each `[[bin]]` is its own crate root with one `_start`, and 58 of them
// documenting an OS-facing ABI entry point is not what the lint is for.
#![allow(missing_docs)]
#![no_main]

/// The endowment a child is born holding, for the one loader this tree has (milestone 96). The
/// interactive boot's own use of it is in `crates/system_initializer`; what is left here is milestone
/// 19d's test roles, which build a child out of one budget and hand it two or three capabilities.
use supervision_protocol::{Child, ChildEndowment, Retention};
use user_mode_runtime::{exit, irq_wait, map_page_frame, recv, send};

/// Roles, as passed in `x0` by the kernel.
// Roles 0, 2, 7, 9 to 19 and 42 were the milestone 7-19 capability demonstrations and the granted
// cycle-counter reader. Every one of them is its own program in `fixtures/src/` since milestone
// 291, and the whole table is in `design/roadmap/291-one-program-one-job.md`. Roles 1 and 3 to 6
// had left over 19f, 27 at 266, and 8, 13 and 30 to 40 at 291, which found them already duplicated
// in `components/src/block_driver.rs`.
//
// **The numbers are not reused and the gaps are not tidied.** A role number is the word the kernel
// puts in `x0`, so it is a value the kernel's test wiring and this file agree on, and
// `design/roadmap/301-one-grant-order-for-the-progenitor.md` records six `spawn_hello`
// tests that name them. Renumbering would be an edit to a wire value bought with nothing.
const INIT: u64 = 20;
const CHILD: u64 = 21;
const DEV_CHILD: u64 = 22;
const IRQ_CHILD: u64 = 26;
// 23-25 and 28-29 are init roles, declared below with their functions. 27 is the progenitor's,
// which `kernel::user::PROGENITOR_ROLE` holds and this binary never sees.

#[unsafe(no_mangle)]
// `_arg2`: the third `START` word. None of the nine reads it any more. The one role that
// did was `init_boot`, which carried the filesystem rights the kernel granted the boot process;
// that role is `components/src/progenitor.rs` now (milestone 266), and the parameter stays in the
// signature because the kernel passes three words to every program it enters.
//
// The second word is `initrd_len`, and it was called `dma_phys` until milestone 291, when it was
// wrong for every role that read it. To a virtio driver the second `START` word is a DMA region's
// physical address; to an init role it is the initrd archive's length. This binary carried both
// kinds, so one parameter had two meanings and was named after the one the init roles never used.
// The seven virtio roles are `block_driver`'s now, so only the archive length is left.
pub extern "C" fn _start(role: u64, initrd_len: u64, _arg2: u64) -> ! {
    match role {
        INIT => init(initrd_len),
        INIT_DEV => init_dev(initrd_len),
        INIT_CONSOLE => init_console(initrd_len),
        INIT_IRQ => init_irq(initrd_len),
        IRQ_CHILD => irq_child(),
        INIT_LEAST_AUTHORITY_DEMO => init_least_authority_demo(initrd_len),
        INIT_COREMARK => init_coremark(initrd_len),
        CHILD => child(),
        DEV_CHILD => dev_child(),
        // **A role this binary does not have is a fault, not a default.** The arm here was
        // `_ => self_check_client()` until milestone 291, which meant a caller that asked for a
        // role this program had never heard of got a program that checked its own image, made one
        // syscall and exited cleanly. Every spawner in the tree would read that as success.
        // `block_driver` and `builder` both trap on an unknown role; this now matches them.
        _ => user_mode_runtime::trap(),
    }
}

/// The bytes of the program named `name` in the initrd (milestone 19f). The initrd is a nifefs
/// archive the kernel maps read-only at [`user_mode_runtime::initrd::INITRD_VA`]; init indexes it by name
/// rather than treating the whole blob as a single ELF. `initrd_len` (the archive length) arrives
/// in `x1` at entry. Returns `None` if the archive will not parse or holds no such program.
///
/// Through 19f.1 every program was a role of *this* binary, so callers looked up the one entry the
/// kernel had loaded and re-entered it at a different role; 19f.2 added distinct entries a caller
/// can name directly (`"least_authority_demo"` and so on).
fn program(initrd_len: u64, name: &str) -> Option<&'static [u8]> {
    // SAFETY: forwarded from user_mode_runtime::initrd::initrd_bytes's own contract.
    let archive = unsafe { user_mode_runtime::initrd::initrd_bytes(initrd_len) };
    nifefs::Fs::parse(archive).ok()?.read(name)
}

/// **The archive entry holding this binary**, which is now the same name on every machine.
///
/// Several roles build a child out of *this* program's own ELF and re-enter it at a different
/// role ([`CHILD`], [`DEV_CHILD`], [`IRQ_CHILD`]). To do that they have to find hello in the
/// archive. The kernel side of the same fact is `kernel::user::HELLO_ENTRY`; the two must agree.
///
/// **Three `cfg` arms stood here until milestone 266**, because aarch64 packed hello as `init`
/// (there hello also carried the boot role) while the other two boards packed it as `hello`. That
/// asymmetry cost a real bug before it was understood: a hardcoded `"init"` was right on aarch64
/// and silently wrong on RISC-V, so this program happily built a child out of `builder`'s ELF and
/// started it at a role `builder` does not have; the child reached for an initrd mapping it did not
/// own, faulted, was killed, and the test waiting on its report blocked until the watchdog fired.
/// Nothing said "wrong program"; it just never answered. One progenitor retired the alias, and the
/// three arms with it.
const ROLES_ENTRY: &str = "hello";

/// The init role that builds a device-driver child (milestone 19d.2); matches kernel test wiring.
const INIT_DEV: u64 = 23;
/// The init role that brings up the real console server and prints through it (milestone 19d.2b).
const INIT_CONSOLE: u64 = 24;
/// The init role that builds an interrupt-driven child, to prove IRQ delegation (milestone 19d.2b).
const INIT_IRQ: u64 = 25;
/// The init role that builds a `least_authority_demo`, passes it an argument via START, and reports its answer
/// (milestone 19e: the first workload that needs START to carry data, not just a role).
const INIT_LEAST_AUTHORITY_DEMO: u64 = 28;
/// The argument init hands its `least_authority_demo` in the [`INIT_LEAST_AUTHORITY_DEMO`] role; the `least_authority_demo` returns its square.
const WORKER_INPUT: u64 = 7;
/// The init role that builds the CoreMark compute workload and reports the CRC it computed
/// (milestone 19e: the first *real* workload, not a toy).
const INIT_COREMARK: u64 = 29;
/// The word a milestone-19d child reports through the endpoint init granted it.
const CHILD_WORD: u64 = 0xC0FFEE;

/// **The init role, milestone 19d.** The role in which this binary is the parent: it parses an ELF
/// and starts a child, with the loader in userspace rather than in the kernel. It is **not** the
/// first process; since milestone 266 that is `progenitor`, and this role's name has not followed
/// it (the constants below are unratified, and a rename of them is calef's). init holds a
/// building untyped (slot 0) and a report endpoint (slot 1, `WRITE|GRANT`); the initrd is mapped
/// read-only at [`user_mode_runtime::initrd::INITRD_VA`], and its length arrives in `x1`.
///
/// It parses that ELF (the `elf` crate, linked into userspace) and loads it as a **child**: a
/// second instance of this same program, entered at role [`CHILD`], built entirely by init out
/// of its own budget through the granular verbs (retype an address space, copy each segment into
/// retyped frames and map them in, retype a TCB, endow it, configure, start). The child reports
/// a word home; receiving it proves init parsed a real ELF and built a running process, with the
/// kernel never touching the child's bytes. See kernel/src/user.rs `spawn_hello`.
fn init(initrd_len: u64) -> ! {
    init_build(initrd_len, false)
}

/// **init delegates an interrupt to a driver it builds, milestone 19d.2b.** The third and last
/// delegatable device authority (after endpoints and device MMIO): an *interrupt capability*. init
/// holds one for a test interrupt (slot 3, the kernel routed it); it builds a child and hands it
/// that Irq cap, then starts the child. The child blocks in the interrupt's `WAIT` until the
/// interrupt fires, then reports. Receiving the report proves init can build an interrupt-driven
/// driver -- the mechanism the input and virtio drivers need for their completions.
fn init_irq(initrd_len: u64) -> ! {
    const MEMORY_REGION: u64 = 0;
    const REPORT: u64 = 1;
    const TEST_IRQ: u64 = 3; // the Irq cap the kernel granted this program (spawn_hello)

    let Some(init_bytes) = program(initrd_len, ROLES_ENTRY) else {
        fail_report(REPORT)
    };
    let Ok(elf) = elf::Elf::parse(init_bytes) else {
        fail_report(REPORT)
    };

    // The child gets the report endpoint (slot 0) and the interrupt (slot 1).
    let caps: &[(u64, u64)] = &[
        (REPORT, abi::rights::WRITE),
        (TEST_IRQ, abi::rights::READ), // WAIT/ACK the interrupt
    ];
    let Ok(child) = build_child(MEMORY_REGION, &elf, caps, &[]) else {
        fail_report(REPORT)
    };
    check(start_child(child, IRQ_CHILD, 0, 0));
    exit();
}

/// **init builds a `least_authority_demo` and hands it an argument, milestone 19e.** The first workload that needs
/// `START` to carry *data*, not just a role: every child before this took only its role in `x0`,
/// but a `least_authority_demo` computes on an input, and that input has to reach it. init builds a `least_authority_demo`
/// child endowed with the report endpoint (slot 0) and starts it with [`WORKER_INPUT`] in `x1`
/// (the second `START` argument, new in 19e). The `least_authority_demo` squares it and reports home. Receiving
/// `WORKER_INPUT * WORKER_INPUT` proves the argument crossed the `START` boundary intact: the
/// mechanism the interactive `run <n>` command and, later, real spawned services stand on.
fn init_least_authority_demo(initrd_len: u64) -> ! {
    const MEMORY_REGION: u64 = 0;
    const REPORT: u64 = 1;

    // The least_authority_demo is its own binary now (19f.2), loaded from the archive by name, not a role of this
    // one. init parses it exactly as it parses any program it did not write.
    let Some(demo_bytes) = program(initrd_len, "least_authority_demo") else {
        fail_report(REPORT)
    };
    let Ok(elf) = elf::Elf::parse(demo_bytes) else {
        fail_report(REPORT)
    };

    // The least_authority_demo's whole authority: the report endpoint as its slot 0, so its one SEND lands where
    // the test (or, in the boot system, the shell) is waiting.
    let caps: &[(u64, u64)] = &[(REPORT, abi::rights::WRITE)];
    let Ok(child) = build_child(MEMORY_REGION, &elf, caps, &[]) else {
        fail_report(REPORT)
    };
    // x0 is unused (a standalone binary needs no role selector); the input is in x1 (the multi-arg
    // START that 19e added).
    check(start_child(child, 0, WORKER_INPUT, 0));
    exit();
}

/// **init builds the CoreMark compute workload, milestone 19e: the first real workload.** Same shape
/// as `init_least_authority_demo`, but the child is the `"coremark"` binary and it computes something substantial
/// (a CoreMark-derived run) rather than a toy square. init grants it the report endpoint (slot 0)
/// and starts it; the workload runs a fixed iteration count and SENDs the run's CRC home. Receiving
/// `coremark::PINNED_CRC_64` proves a real compute program ran correctly against the native ABI.
fn init_coremark(initrd_len: u64) -> ! {
    const MEMORY_REGION: u64 = 0;
    const REPORT: u64 = 1;

    let Some(bytes) = program(initrd_len, "coremark") else {
        fail_report(REPORT)
    };
    let Ok(elf) = elf::Elf::parse(bytes) else {
        fail_report(REPORT)
    };
    let caps: &[(u64, u64)] = &[(REPORT, abi::rights::WRITE)];
    let Ok(child) = build_child(MEMORY_REGION, &elf, caps, &[]) else {
        fail_report(REPORT)
    };
    check(start_child(child, 0, 0, 0)); // no args: the workload's iteration count is fixed
    exit();
}

/// **An interrupt-driven child, milestone 19d.2b.** Holds a report endpoint (slot 0) and an
/// interrupt capability (slot 1), both handed to it by init. It waits for the interrupt as a
/// message, then reports the agreed word. Blocking here forever if the interrupt never arrives is
/// the negative case: the test would hang, so a passing test is the interrupt being delivered
/// through the capability init delegated.
fn irq_child() -> ! {
    const REPORT: u64 = 0;
    const IRQ: u64 = 1;
    const IRQ_WORD: u64 = 0x1590; // "IRQ 0" ish; any fixed value the test asserts

    let _ = irq_wait(IRQ); // blocks until the interrupt the kernel routed for this cap fires
    send(REPORT, IRQ_WORD, 0, 0);
    exit();
}

/// **init brings up the real console server, milestone 19d.2b.** The step past 19d.2a's ID-read
/// probe: init builds the *actual* print server (its own `"console"` binary since 19f.3) as a child
/// and drives it. The server needs four things, and init provides all of them out of its own budget
/// and the
/// capabilities it holds: a request endpoint (the server RECVs a length on it), a reply endpoint
/// (it ACKs), a shared page (the client writes text, the server reads it), and the UART's
/// registers (device-typed, from 19d.2a). init then plays the client: it writes a line into the
/// shared page, sends the length, the server prints it to the real UART and acks, and init reports
/// the acked length home. The report proves the whole userspace-built console works: a driver init
/// constructed, wired to a channel init created, driving hardware init delegated.
fn init_console(initrd_len: u64) -> ! {
    const MEMORY_REGION: u64 = 0;
    const REPORT: u64 = 1;
    const UART_DEV: u64 = 2;
    const SHARED_VA: u64 = address_space_map::pair_page(0x0060_0000); // must match the console server's SHARED_VA
    const CHILD_UART_VA: u64 = address_space_map::pair_page(0x0070_0000); // must match the console server's UART_VA

    // The console server is its own binary now (19f.3): init loads "console" by name and builds it,
    // rather than entering hello at a console role.
    let Some(con_bytes) = program(initrd_len, "console") else {
        send(REPORT, 0, 0, 0);
        exit();
    };
    let Ok(elf) = elf::Elf::parse(con_bytes) else {
        send(REPORT, 0, 0, 0);
        exit();
    };

    // The channel to the server, and a shared page to hand it the text.
    let Ok(request) = retype_obj(MEMORY_REGION, abi::objtype::RENDEZVOUS) else {
        fail_report(REPORT)
    };
    let Ok(reply) = retype_obj(MEMORY_REGION, abi::objtype::RENDEZVOUS) else {
        fail_report(REPORT)
    };
    let Ok(shared) = retype_page_frame(MEMORY_REGION) else {
        fail_report(REPORT)
    };

    // Map the shared page read/write in init's own space, so init (the client) can write into it.
    if !map_page_frame(shared, SHARED_VA, true, MEMORY_REGION) {
        fail_report(REPORT);
    }

    // Build the server: slot 0 = request (READ, it receives), slot 1 = reply (WRITE, it acks);
    // the shared page read-only and the UART device-typed, at the VAs the server expects.
    let caps: &[(u64, u64)] = &[(request, abi::rights::READ), (reply, abi::rights::WRITE)];
    let maps: &[(u64, u64, u64)] = &[
        (SHARED_VA, shared, abi::address_space::MAP_RO),
        (CHILD_UART_VA, UART_DEV, abi::address_space::MAP_RO),
    ];
    let Ok(child) = build_child(MEMORY_REGION, &elf, caps, maps) else {
        fail_report(REPORT)
    };
    check(start_child(child, 0, 0, 0)); // no role selector: console is its own binary

    // Now init is the client. Write a line into the shared page, ask the server to print it.
    let msg = b"nife: the console server was built and started by userspace init.
";
    // SAFETY: init mapped the shared page read/write at SHARED_VA above.
    unsafe {
        let dst = core::slice::from_raw_parts_mut(SHARED_VA as *mut u8, msg.len());
        dst.copy_from_slice(msg);
    }
    check(send(request, msg.len() as u64, 0, 0) == 0); // the server prints, then acks on reply
    let (acked, _, _) = recv(reply);
    send(REPORT, acked, 0, 0); // report the length the server acknowledged
    exit();
}

/// Report a build failure (word 0) and exit. A `-> !` helper so the `else` arms above read cleanly.
fn fail_report(report: u64) -> ! {
    send(report, 0, 0, 0);
    exit();
}

/// A variant of init that builds the **device driver** child (19d.2): same loader, but it hands
/// the child the UART device capability it holds (slot 2). Entered at role [`INIT_DEV`].
fn init_dev(initrd_len: u64) -> ! {
    init_build(initrd_len, true)
}

fn init_build(initrd_len: u64, device: bool) -> ! {
    const MEMORY_REGION: u64 = 0;
    const REPORT: u64 = 1;
    const UART_DEV: u64 = 2; // the UART device cap the kernel granted this program (spawn_hello)
    const CHILD_UART_VA: u64 = address_space_map::pair_page(0x0070_0000);

    let Some(init_bytes) = program(initrd_len, ROLES_ENTRY) else {
        send(REPORT, 0, 0, 0);
        exit();
    };
    let Ok(elf) = elf::Elf::parse(init_bytes) else {
        send(REPORT, 0, 0, 0);
        exit();
    };

    // The child's authority: its report endpoint at slot 0 (WRITE). A driver also gets the UART.
    let caps: &[(u64, u64)] = &[(REPORT, abi::rights::WRITE)];
    let no_maps: &[(u64, u64, u64)] = &[];
    let dev_maps: &[(u64, u64, u64)] = &[(CHILD_UART_VA, UART_DEV, abi::address_space::MAP_RO)];
    let maps = if device { dev_maps } else { no_maps };

    match build_child(MEMORY_REGION, &elf, caps, maps) {
        Ok(child) => {
            let role = if device { DEV_CHILD } else { CHILD };
            check(start_child(child, role, 0, 0));
        }
        Err(_) => {
            send(REPORT, 0, 0, 0);
        }
    }
    exit();
}

/// **A device driver child, milestone 19d.2.** Built by init exactly like [`child`], but init
/// also mapped a device's MMIO (the PL011 UART) into this child's address space at `UART_VA`
/// before starting it. This child reads the PL011's PrimeCell identification registers, whose
/// values are the fixed `0xB105F00D` ("BIOS FOOD") every real PL011 returns, and reports them.
/// Reading that constant proves the mapping is a real, device-typed view of the actual UART, not
/// normal memory and not the wrong page: init delegated device authority and the driver used it.
fn dev_child() -> ! {
    const REPORT: u64 = 0; // init inserted the report cap as slot 0
    const UART_VA: u64 = address_space_map::pair_page(0x0070_0000); // where init mapped the UART registers

    // The four PrimeCell ID bytes live at 0xFF0, 0xFF4, 0xFF8, 0xFFC and read 0x0D,0xF0,0x05,0xB1.
    // SAFETY: init mapped the UART, device-typed, at UART_VA before starting us; these are
    // read-only ID registers, so reading them has no side effect.
    let id = unsafe {
        let base = UART_VA as *const u32;
        let b0 = base.byte_add(0xFF0).read_volatile() & 0xFF;
        let b1 = base.byte_add(0xFF4).read_volatile() & 0xFF;
        let b2 = base.byte_add(0xFF8).read_volatile() & 0xFF;
        let b3 = base.byte_add(0xFFC).read_volatile() & 0xFF;
        (b3 << 24) | (b2 << 16) | (b1 << 8) | b0
    };
    send(REPORT, id as u64, 0, 0);
    exit();
}

/// **A milestone-19d child.** Built by init from an ELF init parsed, entered here with role
/// [`CHILD`] in `x0`. Its whole authority is one capability init granted it in slot 0: a report
/// endpoint. It SENDs the agreed word and exits, which is the observable proof that init's
/// userspace load produced a running thread.
fn child() -> ! {
    const REPORT: u64 = 0; // init inserted the report cap as the child's first slot
    send(REPORT, CHILD_WORD, 0, 0);
    exit();
}

/// Build a child process from `elf`, out of `untyped`. `caps` are inserted into the child's capability table
/// at slots 0, 1, ... in order (each `(init_slot, rights)`: the capability init holds in
/// `init_slot`, narrowed to `rights`). `maps` are extra pages mapped into the child before it starts
/// (each `(child_va, init_slot, mode)`: init's `PageFrame` or `DeviceFrame` cap, mapped at `child_va` with
/// a `MAP_*` mode), which is how init hands a driver its registers and a shared buffer (19d.2).
/// Returns the [`Child`], ready to start.
///
/// **The loader itself is `supervision_protocol`'s, and is the tree's only one** (milestone 96). It
/// used to be written out here, once more in `system_initializer`, and once more in that crate, with
/// a fault slot in each; a change that landed in one of the three was a change the other two did not
/// get. What is left here is the call shape hello's remaining roles want: one budget for both halves
/// of the build, no blobs, no supervision, and the stack every child in this system gets.
fn build_child(
    untyped: u64,
    elf: &elf::Elf,
    caps: &[(u64, u64)],
    maps: &[(u64, u64, u64)],
) -> Result<Child, ()> {
    supervision_protocol::build_child(
        untyped,
        untyped,
        elf,
        &ChildEndowment {
            caps,
            maps,
            stack_pages: system_initializer::CHILD_STACK_PAGES,
            // **`Retention::Nothing`, and this file is where DECISIONS §142's audit landed.** All
            // five of this program's roles kept their child's TCB capability past `START` before
            // §142 was written, the only five sites in the tree that did, and not one of them
            // recorded a reason. There was none: every one of them calls `exit()` a few lines
            // later, so the capability was not retained on purpose, it was dropped by the process
            // dying rather than by anybody deciding. Written down, the answer is the same one the
            // other twenty-five sites give.
            ..ChildEndowment::new(Retention::Nothing)
        },
    )
}

/// Retype a kernel object (endpoint | address space | tcb) out of `untyped`; returns its cap slot.
fn retype_obj(untyped: u64, objtype: u64) -> Result<u64, ()> {
    supervision_protocol::retype_obj_from(untyped, objtype)
}

/// Retype a page of `untyped` into a `PageFrame` capability; returns its cap slot.
fn retype_page_frame(untyped: u64) -> Result<u64, ()> {
    supervision_protocol::retype_page_frame_from(untyped)
}

/// Start a configured child, handing it `arg0`, `arg1`, `arg2` as its first three registers, and
/// dispose of its TCB capability as [`build_child`] declared. True if the kernel started it.
fn start_child(child: Child, arg0: u64, arg1: u64, arg2: u64) -> bool {
    supervision_protocol::start_child(child, arg0, arg1, arg2)
}

/// The only way this program can say "no": a `brk`, which the kernel treats as a fault and kills
/// us for. A failed check must be indistinguishable from a broken program, because it is one.
fn check(ok: bool) {
    if !ok {
        fail();
    }
}

/// Trap, killing this program where the mistake was. Kept as a local name because the call sites
/// above read as "this build step failed", not as "execute a breakpoint"; the instruction itself
/// is `user_mode_runtime`'s since milestone 130. The comment this replaces called it "the one arch-specific
/// line in the program", which was true of `hello` and false of the tree: there were forty-eight.
fn fail() -> ! {
    user_mode_runtime::trap()
}

user_mode_runtime::panic_handler!();
