//! **`user_mode_runtime`**: the tiny user-mode runtime shared by nife userspace programs (milestone
//! 19f.6). It ships on all three architectures, so the level it runs at is named the way the tree
//! names it everywhere a single ISA is not in view: `EL0` on aarch64, `U-mode` on riscv64, `ring 3`
//! on `x86_64`, and "user mode" when the sentence covers all three.
//!
//! One syscall wrapper (`invoke`) and the three things every program builds on it: `send`, `recv`,
//! and `exit`. That is the whole crate. It exists because milestones 19f.2-5 split the userspace
//! into distinct binaries (`least_authority_demo`, `console`, `input`, `shell`, plus `hello`), each
//! of which had
//! copied these functions verbatim. The extraction waited on purpose until the split was done: only
//! then was the shared surface known rather than guessed, which is the DECISIONS rule about not
//! building an abstraction before its requirements exist.
//!
//! Milestone 27 added one more thing every program *may* build on: [`heap`], the untyped-backed
//! `GlobalAlloc` that turns the budget a program was granted into `Vec` and `String`. It is a
//! module, not a default: a program that never allocates links no allocator.
//!
//! Milestone 139 added a third: [`mapped_window`], the raw volatile-access-into-a-mapped-page
//! seven drivers had each hand-rolled (a DMA page or a shared IPC frame, read and written by
//! `unsafe { core::ptr::read_volatile/write_volatile }` at every call site). Same shape as the
//! panic handler above: the invariant was one thing asserted N times by hand, and only the
//! declaration needed to be per-caller.
//!
//! And a fourth, in the same round: [`initrd`], the `INITRD_VA` slice seven `init`-shaped programs
//! each reconstructed by hand with `core::slice::from_raw_parts`. A different pattern from
//! `mapped_window` (a whole `'static` slice, not a bounds-checked per-offset accessor), but the
//! same §94 shape underneath: one invariant, copied verbatim into seven declarations.
//!
//! Round 7 read every remaining raw `invoke(...)` call site in `user/` (123 of them, the
//! milestone's own largest unmigrated cluster) and found the same shape at nearly all of them: a
//! method whose own `# Safety` obligation is [`invoke`]'s own ("the kernel validates the capability
//! and the method before acting"), asserted by hand at every call site with nothing
//! call-site-specific to check. Fourteen new thin wrappers below (`retype_page_frame` through
//! `send_cap`), plus [`is_granted`] (the shape of §94 (what may live in a userspace library) again:
//! five programs' identical probe) and the opt-in [`virtio`] module (device-specific, so scoped
//! like `mapped_window` rather than added here), cover all but one of them; see that one call
//! site's own comment (`window.rs`'s refusal probe) for why it stays raw.
//!
//! The `#[panic_handler]` is **still not an item here, and now the trap underneath it is**
//! (milestone 130). A panic handler is per-final-binary: exactly one may exist in a linked program,
//! so an item in this library would force it on every program that links the crate and collide with
//! any program that wants its own (as `hello` does). That has been recorded since 19f.6 and it is
//! still true. What went stale beside it was the sentence "each binary keeps its own one-line
//! handler; it is trivial": the handler grew to fifteen lines with two `unsafe` blocks and two
//! `// SAFETY:` comments, and by the time anyone counted, the trap instruction was inlined at
//! **forty-eight sites** in seven variants, one of which called [`exit`] instead of trapping and so
//! reported a clean death for a panicking program. The constraint was right and the inference from
//! it was not: a *handler* cannot live in a library, but the *trap* always could.
//!
//! So [`trap`] is here, and [`panic_handler!`] is a macro that expands to the handler in the
//! binary. The linking property survives, and the claim below about this being the one place in
//! userspace that names the two ABIs becomes true rather than aspirational. Device helpers (a UART
//! `putc`, echo logic) still stay in the drivers that own them: those are not runtime, they are the
//! program.
//!
//! # Examples
//!
//! **This crate is the one place in the tree where an example genuinely cannot run**, and the reason
//! is worth stating rather than hiding behind a fence marker. Every function here traps to the kernel
//! from EL0: `svc`/`ecall` on a machine with no nife kernel under it is a fault, not a syscall, and
//! `script/test`'s host pass excludes this crate and everything that depends on it (the exclusion set
//! is derived and checked by `script/lint`). So the examples below are `no_run`: they are type-checked
//! against the real signatures on an aarch64 host and are **not executed anywhere**. The things that
//! *can* be checked are the wire contracts layered over them, which is where those crates put their
//! examples (`byte_sink_protocol`, `filesystem_protocol`, `entropy_protocol`).
//!
//! A program's whole life, in the four calls that make up this crate. Note what is absent: there is
//! no `open`, no path, and no way to name anything that was not handed over.
//!
//! ```no_run
//! use user_mode_runtime::{exit, recv, send};
//!
//! /// A pipeline stage: read three words off the endpoint in slot 0, pass them to slot 1.
//! fn relay() -> ! {
//!     const IN: u64 = 0;
//!     const OUT: u64 = 1;
//!     loop {
//!         // `recv` blocks until a sender rendezvouses. The rendezvous IS the flow control: there
//!         // is no buffer to fill and no back-pressure to invent.
//!         let (w0, w1, w2) = recv(IN);
//!         if send(OUT, w0, w1, w2) < 0 {
//!             // A negative return is an `abi::Error`. `Gone` here means the reader exited, which
//!             // is this system's SIGPIPE, arriving as a return code rather than as a signal.
//!             exit();
//!         }
//!     }
//! }
//! ```
//!
//! A client of a service uses [`call`], which blocks until the reply lands. The reply arrives through
//! a one-shot capability the kernel mints, so the client never names the server and the server never
//! names the client:
//!
//! ```no_run
//! use user_mode_runtime::call;
//!
//! # fn ask() {
//! const SERVICE: u64 = 2;
//! let (r0, r1) = call(SERVICE, 0x0100_0000_0000_0008, 0);
//!
//! // Negative-as-u64 is enormous, which is how a wire contract tells "no capability in that slot"
//! // from an answer without a probe request. See `entropy_protocol::delivered`.
//! assert!((r0 as i64) >= 0 || abi::Error::from_ret(r0 as i64).is_some());
//! # let _ = r1;
//! # }
//! ```
//!
//! # Three ABIs, one surface
//!
//! The syscall instruction and the register file differ by architecture, and this is the one place
//! in userspace that names them. aarch64 uses `svc #0` with the syscall number in `x8` and arguments
//! in `x0..x5`; RISC-V uses `ecall` with the number in `a7` and arguments in `a0..a5`; `x86_64` uses
//! `syscall` with the number in `rax` and arguments in `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9`
//! ([DECISIONS §124](../../../design/decisions/124-x86-64-syscall-abi.md), ratified 2026-08-24).
//! All three return in the first argument register (`x0` / `a0` / `rdi`). The kernel reconciles them
//! in `TrapFrame` (DECISIONS §17); here we simply select the right asm at compile time. Every
//! function's signature, semantics, and the `abi` constants are identical across all three.
//!
//! **`x86_64` is the arm that costs more than a transliteration**, and there are exactly three places
//! it does. `syscall` clobbers `rcx` and `r11` unconditionally, so every site declares them; `rdtsc`
//! answers in two halves rather than one register (see [`now`]); and there is **no architected
//! counter frequency at all**, which is why [`cntfrq`] carries a `BUGS` section rather than a number
//! with a comment. Everything else is the same three instructions in a different spelling.
//!
//! Name: ratified 2026-09-13 (calef, milestone 285), replacing `user_rt`. Two halves, argued
//! separately. **`rt`** was an **abbreviation that needs a decoder**, the first of the three
//! failure modes AGENTS.md names, and this crate's own first line had always spelled it out; the
//! precedent is `cred_proto` to `credential_proto`, ratified 2026-08-23 for "spell out the
//! contraction fully" (milestone 265 has since taken that crate to `credential_protocol`). Nothing outside the tree owns the spelling (no specification, no wire
//! format, no command-line flag), so the acronym test applies at full force here in a way it did
//! not to `initrd`. **`user_`** expanded to **`user_mode_`** because in this tree `user` means *a
//! person* several hundred times over: milestone 49 is users and attribution, `identity_provisioner`
//! creates them, `login` authenticates them, and `schedule_store` calls itself the per-user
//! schedule store. `user_runtime` would have kept reading as "the runtime belonging to a user".
//! "User mode" is this tree's architecture-neutral phrase for the unprivileged level, where `EL0`
//! is aarch64's word for it and `U-mode` and `ring 3` are the other two architectures'. Refused
//! `user_runtime` (leaves the prefix ambiguous between a person and a privilege level) and
//! `el0_runtime` (a crate name correct on one architecture and wrong on the two others, which is
//! what AGENTS.md rule 5 exists to catch).
//!
//! Introduced 2026-07-25 as `user_rt`, when the shared runtime was lifted out of five binaries that
//! had copied it verbatim, and provisional from then until milestone 285. It was half-argued in a
//! way that kept it unargued: milestone 63 ratified `user_heap` over `uheap` in part on the ground
//! that "`user_rt` already establishes `user_` as the prefix", so the prefix was on the record
//! because this crate had established it, which is a circle rather than a reason. Milestone 285
//! closed the circle by ruling on the prefix itself rather than by citing either crate.

#![no_std]

pub mod child_stub;
pub mod heap;
pub mod initrd;
pub mod mapped_window;
pub mod virtio;

/// The raw five-register round trip through `SYS_INVOKE` (milestone 139 round 2). `cap`, `method`
/// and two more arguments go in `x0..x3`/`a0..a3` (the fifth, `x4`/`a4`, is spare and always zero on
/// input); the kernel's reply comes back in the same five registers, `x0..x4`/`a0..a4`. This is now
/// the one place the actual trap instruction and register file appear for a `SYS_INVOKE` call:
/// [`invoke`] and every multi-word method below ([`recv`], [`recv_cap`], [`recv_fault`], [`call`],
/// [`survey`], [`list`]) used to each hand-roll their own `asm!` block asserting the identical
/// invariant ("`svc`/`ecall` traps to the kernel, which validates before acting") at a register
/// layout that differed only in which of the five words the caller happened to read back. Six
/// functions, two architectures, twelve hand-written copies of one assertion: the §94 shape this
/// milestone names as the reduction worth making. Now there are two, one per architecture, and
/// every caller above is a safe wrapper that just picks which return words it wants.
///
/// One behavioural note for a reader diffing this against the asm the individual functions used to
/// carry: a few of them ([`recv`], [`recv_cap`], [`recv_fault`]) left `x2`/`a2` with no `in`
/// operand at all, so the kernel received whatever value happened to already be in that register
/// (harmless, since `RECV`/`RECV_CAP` read no input words). Routing them through this shared
/// primitive means they now pass an explicit `0` there instead, which is a strict tightening, not a
/// behaviour change: the kernel still ignores it.
///
/// # Safety
/// `svc`/`ecall` traps to the kernel. The kernel validates the capability and the method before
/// acting; that is its whole job. The caller is trusting the kernel, not the other way around.
#[cfg(target_arch = "aarch64")]
unsafe fn invoke5(cap: u64, method: u64, a0: u64, a1: u64, a2: u64) -> (u64, u64, u64, u64, u64) {
    let (mut w0, mut w1, mut w2, mut w3, mut w4): (u64, u64, u64, u64, u64);
    // SAFETY: see the function doc; `x8` selects SYS_INVOKE (DECISIONS §10), `x0..x4` carry the
    // five-word ABI in both directions. `asm!` is unsafe because the compiler cannot check that,
    // not because a caller can get it wrong.
    unsafe {
        core::arch::asm!(
            "svc #0",
            in("x8") abi::SYS_INVOKE,
            inlateout("x0") cap => w0,
            inlateout("x1") method => w1,
            inlateout("x2") a0 => w2,
            inlateout("x3") a1 => w3,
            inlateout("x4") a2 => w4,
            options(nostack),
        );
    }
    (w0, w1, w2, w3, w4)
}

/// The raw five-register round trip (RISC-V). See the aarch64 twin's doc for the contract this
/// collapses; only the trap instruction and register file differ: `ecall`, number in `a7`, the five
/// words in `a0..a4`.
///
/// # Safety
/// `ecall` traps to the kernel, which validates the capability and method before acting. Same
/// contract as the aarch64 twin: the caller trusts the kernel, not the other way around.
#[cfg(target_arch = "riscv64")]
unsafe fn invoke5(cap: u64, method: u64, a0: u64, a1: u64, a2: u64) -> (u64, u64, u64, u64, u64) {
    let (mut w0, mut w1, mut w2, mut w3, mut w4): (u64, u64, u64, u64, u64);
    // SAFETY: see the function doc; `a7` selects SYS_INVOKE (DECISIONS §10), `a0..a4` carry the
    // five-word ABI in both directions.
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") abi::SYS_INVOKE,
            inlateout("a0") cap => w0,
            inlateout("a1") method => w1,
            inlateout("a2") a0 => w2,
            inlateout("a3") a1 => w3,
            inlateout("a4") a2 => w4,
            options(nostack),
        );
    }
    (w0, w1, w2, w3, w4)
}

/// The raw five-register round trip (`x86_64`, milestone 161). See the aarch64 twin's doc for the
/// contract this collapses; only the trap instruction and register file differ. `syscall`, the
/// number in `rax`, the five words in `rdi`, `rsi`, `rdx`, `r10`, `r8` (DECISIONS §124).
///
/// **Two operands here have no counterpart on the other two architectures**, and both are the
/// instruction rather than a choice. `syscall` writes the return address into `rcx` and the
/// caller's `RFLAGS` into `r11`, unconditionally, so both are declared clobbered; a version of
/// this without them compiles and then corrupts whichever local the register allocator had put
/// there. And `r10` carries the fourth word instead of the C ABI's `rcx` for exactly the same
/// reason, which is why §124 records that substitution as forced rather than preferred.
///
/// `options(nostack)` still holds: `syscall` does not push, which is the whole reason the kernel's
/// entry path has to park `rsp` by hand (see `arch/x86_64/trap.s`).
///
/// # Safety
/// `syscall` traps to the kernel, which validates the capability and method before acting. Same
/// contract as the aarch64 twin: the caller trusts the kernel, not the other way around.
#[cfg(target_arch = "x86_64")]
unsafe fn invoke5(cap: u64, method: u64, a0: u64, a1: u64, a2: u64) -> (u64, u64, u64, u64, u64) {
    let (mut w0, mut w1, mut w2, mut w3, mut w4): (u64, u64, u64, u64, u64);
    // SAFETY: see the function doc; `rax` selects SYS_INVOKE (DECISIONS §10, §124), and the five
    // argument registers carry the five-word ABI in both directions.
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") abi::SYS_INVOKE,
            inlateout("rdi") cap => w0,
            inlateout("rsi") method => w1,
            inlateout("rdx") a0 => w2,
            inlateout("r10") a1 => w3,
            inlateout("r8") a2 => w4,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );
    }
    (w0, w1, w2, w3, w4)
}

/// Invoke a capability: the one syscall a userspace program makes. `cap` names a capability in the
/// process's capability table, `method` selects the operation, and `a0..a2` are its arguments; the return is
/// the kernel's `i64` result. Everything else in this crate is built on this.
///
/// # Safety
/// `svc`/`ecall` traps to the kernel. The kernel validates the capability and the method before
/// acting; that is its whole job. The caller is trusting the kernel, not the other way around.
pub unsafe fn invoke(cap: u64, method: u64, a0: u64, a1: u64, a2: u64) -> i64 {
    // SAFETY: forwarded from this function's own contract, which is `invoke5`'s contract exactly.
    unsafe { invoke5(cap, method, a0, a1, a2).0 as i64 }
}

/// `SEND` three words on the endpoint capability in `slot`. Blocks until a receiver takes them.
pub fn send(slot: u64, w0: u64, w1: u64, w2: u64) -> i64 {
    // SAFETY: `svc` traps to EL1, which validates the capability named by `slot`.
    unsafe { invoke(slot, abi::rendezvous::SEND, w0, w1, w2) }
}

/// **Collect the corpse of a child this supervision endpoint supervises** (DECISIONS §32).
/// `tid` is the thread id the kernel stamped on the death message [`recv_fault`] returned. `0` on
/// success; a negative [`abi::Error`] otherwise, and the three that matter are worth telling apart:
/// `StillAlive` (not dead yet, so wait or escalate to the owner's `MemoryRegion::DESTROY`),
/// `NotSupervised` (not a child of this endpoint, or already collected), and `NotPermitted` (the
/// corpse's region is not reclaimable yet).
///
/// The point of the method: this needs no capability to the child's memory, so a supervisor can be
/// a process that cannot build one. The reclaimed pages go back to the builder's budget.
pub fn reap(slot: u64, tid: u64) -> i64 {
    // SAFETY: `svc`/`ecall`; the kernel validates the capability and the supervision relationship.
    unsafe { invoke(slot, abi::rendezvous::REAP, tid, 0, 0) }
}

/// **Read one entry of the domain this supervision endpoint supervises** (milestone 126,
/// `rendezvous::SURVEY`). Returns `(next_cursor, tid, state)`: start at `cursor = 0`, feed each
/// `next_cursor` back, and stop when [`abi::survey::DONE`] comes back.
///
/// A negative first word is an [`abi::Error`], and the one that matters is `NotPermitted`: this
/// endpoint capability does not carry `READ`, so the holder may send here but not look. **That is
/// a refusal and not an empty domain**, and a caller must print it as one.
///
/// Three words out of one `invoke`, so it is written like [`recv`] rather than through the
/// single-value helper.
///
/// This is [`survey_record`] with [`abi::survey::record::STATE`], kept as its own function because
/// the run state is what every caller in the tree wants and none of them should have to name a
/// selector to ask for it.
pub fn survey(slot: u64, cursor: u64) -> (i64, u64, u64) {
    survey_record(slot, cursor, abi::survey::record::STATE)
}

/// **Read one entry of a supervised domain, asking for a specific record** (`rendezvous::SURVEY`,
/// calef's 2026-09-21 selector ruling). Returns `(next_cursor, tid, word)`, walked exactly as
/// [`survey`] is: start at `cursor = 0`, feed each `next_cursor` back, stop at
/// [`abi::survey::DONE`].
///
/// `record` picks which per-thread fact lands in the third word; see [`abi::survey::record`] for
/// the values and what each one means. The cursor and the tid do not depend on it, so a caller that
/// wants two facts about one domain walks it twice and joins the two walks on the tid.
///
/// A negative first word is an [`abi::Error`]. Two matter. `NotPermitted` is the endpoint
/// capability not carrying `ENUMERATE`, which is a refusal and **not** an empty domain, and a
/// caller must print it as one. `BadMethod` is a record this kernel does not answer, which is what
/// a program built against a later kernel gets, and it is likewise not an empty domain.
///
/// Name: provisional. calef names public items.
pub fn survey_record(slot: u64, cursor: u64, record: u64) -> (i64, u64, u64) {
    // SAFETY: forwarded from `invoke5`'s contract; SURVEY reads no more than the three words used.
    let (r0, w1, w2, ..) = unsafe { invoke5(slot, abi::rendezvous::SURVEY, cursor, record, 0) };
    (r0 as i64, w1, w2)
}

/// **Read one entry of what this address space has mapped** (milestone 126, `pmap`,
/// `address_space::LIST`, DECISIONS §114). Returns `(next_cursor, va, kind)`: start at `cursor = 0`, feed
/// each `next_cursor` back, and stop when [`abi::survey::DONE`] comes back. [`survey`]'s twin,
/// same three-word-out-of-one-`invoke` shape, one object type over.
///
/// A negative first word is an [`abi::Error`], and the one that matters is `NotPermitted`: this
/// capability does not carry `ENUMERATE`, so the holder may map into the space but not list it.
/// **That is a refusal and not an empty listing**, and a caller must print it as one.
pub fn list(slot: u64, cursor: u64) -> (i64, u64, u64) {
    // SAFETY: forwarded from `invoke5`'s contract; LIST reads no more than the three words used.
    let (r0, w1, w2, ..) = unsafe { invoke5(slot, abi::address_space::LIST, cursor, 0, 0) };
    (r0 as i64, w1, w2)
}

/// `RECV` three words on the endpoint capability in `slot`. Blocks until a sender arrives; returns
/// the three words the sender passed in `x0`, `x1`, `x2`.
pub fn recv(slot: u64) -> (u64, u64, u64) {
    // SAFETY: forwarded from `invoke5`'s contract; RECV reads no more than the three words used.
    let (w0, w1, w2, ..) = unsafe { invoke5(slot, abi::rendezvous::RECV, 0, 0, 0) };
    (w0, w1, w2)
}

/// `RECV` **all five words** on the endpoint capability in `slot`: `(w0, w1, w2, w3, w4)`.
///
/// The same `RECV` [`recv`] makes, read to its full width. `RECV` has returned five registers since
/// milestone 22 phase A (the kernel writes `w1..w4` directly; DECISIONS §26 implementation note 4),
/// because a fault notification is five words: `(event, tid, pc, addr, reserved)`. Ordinary
/// three-word IPC leaves the top two zero, which is why [`recv`] can keep ignoring them.
///
/// This exists for a **supervisor**, and it is the first thing in userspace to read `w3`: a
/// restart policy needs the event and the tid, but a *checker* needs the faulting address, which is
/// the only word that says where the dead thread actually pointed. No new syscall and no new method
/// (§26's whole surface claim): just the rest of a result that was already being returned.
pub fn recv_fault(slot: u64) -> (u64, u64, u64, u64, u64) {
    // SAFETY: forwarded from `invoke5`'s contract.
    unsafe { invoke5(slot, abi::rendezvous::RECV, 0, 0, 0) }
}

/// `RECV_CAP` on the endpoint capability in `slot`: receive a message that may carry a
/// capability. Blocks until one arrives; returns `(w0, cap_slot, w1)`, where `cap_slot` is where
/// the incoming capability landed in this thread's capability table, or [`abi::rendezvous::NO_CAP`] if the
/// message carried none. This is how a server receives a [`call`]: the delivered capability is
/// the one-shot Reply naming the caller (milestone 12, DECISIONS §12).
pub fn recv_cap(slot: u64) -> (u64, u64, u64) {
    // SAFETY: forwarded from `invoke5`'s contract; RECV_CAP reads no more than the three words used.
    let (w0, w1, w2, ..) = unsafe { invoke5(slot, abi::rendezvous::RECV_CAP, 0, 0, 0) };
    (w0, w1, w2)
}

/// `CALL` on the endpoint capability in `slot`: send two words and block until the server
/// replies through the one-shot Reply capability the kernel mints (milestone 12). Returns the
/// two reply words. The atomic send-and-wait that makes a request unmistakably answerable.
pub fn call(slot: u64, w0: u64, w1: u64) -> (u64, u64) {
    // SAFETY: forwarded from `invoke5`'s contract; CALL reads no more than the two words used.
    let (r0, r1, ..) = unsafe { invoke5(slot, abi::rendezvous::CALL, w0, w1, 0) };
    (r0, r1)
}

/// `REPLY` through the one-shot Reply capability in `slot`: deliver two words to the blocked
/// caller and wake it. The capability is consumed by the kernel on use (that is what makes it
/// one-shot), so the slot is free again when this returns.
pub fn reply(slot: u64, r0: u64, r1: u64) -> i64 {
    // SAFETY: `svc`/`ecall`; the kernel validates the Reply capability and consumes it.
    unsafe { invoke(slot, abi::reply::REPLY, r0, r1, 0) }
}

/// **Map the `PageFrame` capability in `frame_slot` at `va`**, drawing the page tables from the untyped
/// in `memory_region_slot`. `true` if the page is now there.
///
/// The verb a process that *holds* a page uses to put it in its own address space (milestone 108).
/// It replaces a page the kernel wired into the process at spawn, and the difference is not
/// cosmetic: a spawn-time mapping has no capability behind it, so nobody can narrow it, hand it on,
/// or take it back, while a frame the process mapped itself is recorded in the revocation database
/// and can be pulled out from under it by `PageFrame::REVOKE`. See notes/frames.md.
///
/// `writable` needs `WRITE` on the frame; a read-only mapping needs `READ`. A caller handed a
/// narrowed view that asks for more than it holds gets `false` and no mapping, which is the rights
/// ladder doing its job rather than an error to route around.
pub fn map_page_frame(frame_slot: u64, va: u64, writable: bool, memory_region_slot: u64) -> bool {
    // SAFETY: `svc`/`ecall`. The kernel validates the frame capability, the rights, the address and
    // the untyped before it touches a page table.
    unsafe {
        invoke(
            frame_slot,
            abi::page_frame::MAP,
            va,
            writable as u64,
            memory_region_slot,
        ) == 0
    }
}

/// Whether a capability is in `slot`, without touching whatever it names (milestone 139 round 7).
/// Invoke a method number no object type defines, so the call can only be refused, and read
/// *which* refusal came back: an empty slot answers `NoSuchSlot`, and a real object answers
/// `BadMethod`, a refusal from something that exists.
///
/// Lifted out of `date.rs`'s `clock_page` probe, which four more programs (`pgrep`, `pmap`, `ps`,
/// `watch`) had each copied verbatim, one of them naming the duplication out loud in its own doc
/// comment without anyone lifting it. The exact §94 shape the crate-level docs above describe.
pub fn is_granted(slot: u64) -> bool {
    /// A method number no object type defines, so the invocation can only ever be refused.
    const NO_SUCH_METHOD: u64 = 0xffff;
    // SAFETY: a syscall that cannot succeed; the kernel validates the slot before the method.
    let r = unsafe { invoke(slot, NO_SUCH_METHOD, 0, 0, 0) };
    r != abi::Error::NoSuchSlot as i64
}

/// `RETYPE` one page out of the untyped in `memory_region_slot` into a `PageFrame` capability the
/// caller now holds. Returns the slot the frame landed in, or a negative `abi::Error`
/// (`OutOfMemory` when the untyped is exhausted or the caller's table is full).
pub fn retype_page_frame(memory_region_slot: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates the untyped capability and its remaining budget.
    unsafe { invoke(memory_region_slot, abi::memory_region::RETYPE, 0, 0, 0) }
}

/// `RETYPE` a run of `pages` contiguous pages into one `PageFrame` capability, per §102 (a Frame
/// names a run of pages), `0` meaning one. Returns the slot, or a negative `abi::Error`: `OutOfMemory` when the run does not
/// fit, in which case the region's budget is untouched.
///
/// **The one call site in the tree that passes a count other than `0`**, and `script/lint` holds
/// it to that ("every RETYPE outside the run wrapper passes 0"), so a caller cannot ask for a run
/// by accident through a raw `invoke`.
///
/// Name: provisional (the lane for milestone 23 (a capability-routed component OS with live
/// replacement), 2026-09-26).
pub fn retype_page_frame_run(memory_region_slot: u64, pages: u64) -> i64 {
    // SAFETY: as `retype_page_frame`; the kernel checks the count against the remaining budget.
    unsafe { invoke(memory_region_slot, abi::memory_region::RETYPE, pages, 0, 0) } // run wrapper
}

/// `RETYPE_OBJ` one page out of the untyped in `memory_region_slot` into a kernel object of
/// `objtype` (see [`abi::objtype`]). Returns the slot holding a full-rights capability to the new
/// object, or a negative `abi::Error` (`BadMethod` for an unknown `objtype`, `OutOfMemory` when the
/// untyped or either table is exhausted).
pub fn retype_object(memory_region_slot: u64, objtype: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates the untyped, the objtype and the budget before it
    // touches a page.
    unsafe {
        invoke(
            memory_region_slot,
            abi::memory_region::RETYPE_OBJ,
            objtype,
            0,
            0,
        )
    }
}

/// `SPLIT` `pages` off the untyped's unspent budget in `memory_region_slot` into a new child
/// untyped. Returns the slot holding a full-rights capability to the child, or a negative
/// `abi::Error` (`NotPermitted` without `WRITE`, `OutOfMemory` when the budget or a table is
/// exhausted).
pub fn split_region(memory_region_slot: u64, pages: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates `WRITE` and the remaining budget before it splits
    // anything off.
    unsafe { invoke(memory_region_slot, abi::memory_region::SPLIT, pages, 0, 0) }
}

/// `DESTROY` the region in `memory_region_slot`: reclaim it and every object retyped from it
/// (object revocation, the region-owner's half). `0` on success; a negative `abi::Error`
/// (`NotPermitted` while a live thread still occupies it, or if it has been `SPLIT` into children,
/// or without `WRITE`).
pub fn destroy_region(memory_region_slot: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates `WRITE` and that nothing still occupies or splits
    // from the region before it reclaims anything.
    unsafe { invoke(memory_region_slot, abi::memory_region::DESTROY, 0, 0, 0) }
}

/// `MAP` (untyped): retype one page out of the untyped in `memory_region_slot` and map it,
/// writable, at `va` in the caller's own address space, in one step. `0` on success; a negative
/// `abi::Error` (`OutOfMemory` when the untyped is exhausted).
///
/// The one-step twin of [`retype_page_frame`] followed by [`map_page_frame`]: this never produces
/// a `PageFrame` capability the caller can delegate or revoke, it just spends the untyped's budget
/// directly on a mapped page.
pub fn map_region_page(memory_region_slot: u64, va: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates the untyped, the address, and maps a fresh page
    // from its own budget.
    unsafe { invoke(memory_region_slot, abi::memory_region::MAP, va, 0, 0) }
}

/// `REVOKE` the `PageFrame` in `frame_slot`: un-share it (or, on a device capability, take it back;
/// see `abi::page_frame::REVOKE`'s own doc for the asymmetry). `0` on success; a negative
/// `abi::Error` (`NotPermitted` without `GRANT`).
pub fn revoke_frame(frame_slot: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates `GRANT` before it unmaps and deletes every
    // capability to the page.
    unsafe { invoke(frame_slot, abi::page_frame::REVOKE, 0, 0, 0) }
}

/// `MAP_INTO`: map the frame in `frame_slot` into the address space named by `aspace_slot`, at
/// `va`, per `mode` (`abi::address_space::MAP_RO`/`MAP_RW`/`MAP_CODE`). `0` on success; a negative
/// `abi::Error`.
///
/// The same obligation [`map_page_frame`] already carries, one address space over: milestone 134's
/// census flagged this method as carrying "real" per-call risk because it can perturb an address
/// space out from under code that assumed a mapping was fixed, but that is exactly the risk
/// `map_page_frame` already accepted for the caller's *own* space, and the kernel discharges the
/// same checks here (the address-space capability's `WRITE`, the frame's rights against `mode`,
/// the address) before it touches a page table. A caller aliasing or racing what this call changes
/// is a correctness question for the caller's own code, not a Rust-safety obligation this wrapper
/// could check and the raw `invoke` site could not.
pub fn map_into(aspace_slot: u64, va: u64, frame_slot: u64, mode: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates the address-space capability, the frame's rights
    // against `mode`, and the address before it touches a page table.
    unsafe {
        invoke(
            aspace_slot,
            abi::address_space::MAP_INTO,
            va,
            frame_slot,
            mode,
        )
    }
}

/// `CAP_INSERT`: copy the capability in the caller's `cap_slot`, narrowed to `rights`, into the
/// child TCB's capability table. `target = 0` places it in the first free slot; `target = n` places
/// it in slot `n - 1` (see `abi::thread_control_block::CAP_INSERT`'s own doc for the supervision-slot
/// use of an explicit target). Returns the slot it landed in, or a negative `abi::Error`.
pub fn tcb_cap_insert(tcb_slot: u64, cap_slot: u64, rights: u64, target: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates `WRITE` on the TCB and `GRANT` on the inserted
    // capability before it copies anything.
    unsafe {
        invoke(
            tcb_slot,
            abi::thread_control_block::CAP_INSERT,
            cap_slot,
            rights,
            target,
        )
    }
}

/// `CONFIGURE`: bind the address space in `aspace_slot` to the (embryo) TCB in `tcb_slot`, and set
/// where EL0 execution begins (`entry`) and on what user stack (`user_sp`). `aspace_slot` is
/// consumed: it becomes the thread's and dies with it. `0` on success; a negative `abi::Error`.
pub fn tcb_configure(tcb_slot: u64, entry: u64, user_sp: u64, aspace_slot: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates `WRITE` on both capabilities before it binds them.
    unsafe {
        invoke(
            tcb_slot,
            abi::thread_control_block::CONFIGURE,
            entry,
            user_sp,
            aspace_slot,
        )
    }
}

/// `START`: make the TCB in `tcb_slot` runnable. `x0`, `x1`, `x2` seed the child's own `x0`/`x1`/
/// `x2` (`a0`/`a1`/`a2` on RISC-V) on its first instruction, the kernel-side spelling of "this
/// thread's input" (`abi::thread_control_block::START`'s own doc comment says `_, _, _`, which is
/// stale: `kernel/src/syscall.rs`'s `START` arm forwards all three to `sched::start_thread_control_block`
/// unconditionally). Refuses a half-built thread (no bound address space, or no entry). `0` on
/// success; a negative `abi::Error`.
pub fn tcb_start(tcb_slot: u64, x0: u64, x1: u64, x2: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates `WRITE` and that the TCB is whole before it joins
    // the run queue.
    unsafe { invoke(tcb_slot, abi::thread_control_block::START, x0, x1, x2) }
}

/// `WAIT` on the `Irq` capability in `irq_slot`: block until the interrupt fires. The kernel masks
/// it when it fires and hands it to us as a message; nothing device-specific happens in the kernel.
pub fn irq_wait(irq_slot: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates the Irq capability before it blocks the caller.
    unsafe { invoke(irq_slot, abi::irq::WAIT, 0, 0, 0) }
}

/// `ACK` the `Irq` capability in `irq_slot`: re-enable the interrupt at the GIC once the device has
/// been quieted. Until this is called, the interrupt stays masked and cannot storm.
pub fn irq_ack(irq_slot: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates the Irq capability before it re-enables anything.
    unsafe { invoke(irq_slot, abi::irq::ACK, 0, 0, 0) }
}

/// `SEND_CAP` on the endpoint capability in `slot`: delegate a (possibly narrowed) copy of the
/// capability in `cap_slot`, narrowed to `rights`, alongside the data word `w1`, and block until a
/// receiver takes them. `0` or a positive ack on success; a negative `abi::Error`.
pub fn send_cap(slot: u64, cap_slot: u64, rights: u64, w1: u64) -> i64 {
    // SAFETY: `svc`/`ecall`. The kernel validates the endpoint, `GRANT` on the delegated capability,
    // and narrows it to `rights` before it delegates anything.
    unsafe { invoke(slot, abi::rendezvous::SEND_CAP, cap_slot, rights, w1) }
}

/// Give up the CPU (`SYS_YIELD`). Returns when the scheduler runs this thread again; if another
/// thread is ready, control goes there and back, which is one context-switch round trip.
#[cfg(target_arch = "aarch64")]
pub fn yield_now() {
    // SAFETY: `svc`; SYS_YIELD gives up the CPU and returns with nothing to clean up.
    unsafe {
        core::arch::asm!("svc #0", in("x8") abi::SYS_YIELD, options(nostack, nomem));
    }
}

/// Give up the CPU (RISC-V). `ecall`, `SYS_YIELD` in `a7`.
#[cfg(target_arch = "riscv64")]
pub fn yield_now() {
    // SAFETY: `ecall`; SYS_YIELD gives up the CPU and returns with nothing to clean up.
    unsafe {
        core::arch::asm!("ecall", in("a7") abi::SYS_YIELD, options(nostack, nomem));
    }
}

/// Give up the CPU (`x86_64`). `syscall`, `SYS_YIELD` in `rax`. `rcx` and `r11` are clobbered by the
/// instruction itself, and `nomem` survives that: neither is memory.
#[cfg(target_arch = "x86_64")]
pub fn yield_now() {
    // SAFETY: `syscall`; SYS_YIELD gives up the CPU and returns with nothing to clean up.
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") abi::SYS_YIELD,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, nomem),
        );
    }
}

/// **Write one byte to an x86 I/O port** (milestone 299). Not a syscall: `out` is an instruction,
/// and a ring-3 program may execute it exactly when it holds a `PortRange` capability naming `port`,
/// which the kernel enforces through the TSS I/O permission bitmap (DECISIONS §121, reversed
/// 2026-09-15). A program that executes this against a port it does not hold takes a general
/// protection fault, which its supervisor sees as a fault message; there is no page table to catch
/// a wrong guess, so the caller must know it holds the port.
///
/// The one thing in userspace besides the syscall wrappers that names a machine instruction, and it
/// is here for the same reason `in`/`out` live under `kernel/src/arch/x86_64/port.rs`: an
/// instruction belongs with the code that knows the ISA, so a driver writes `outb(0x3F8, b)` rather
/// than its own `asm!`.
#[cfg(target_arch = "x86_64")]
pub fn outb(port: u16, val: u8) {
    // SAFETY: `out` has no memory effect and no flag effect; `nostack` holds. The caller promises it
    // holds the port capability, so the CPU permits the instruction; without it this faults, which
    // is the enforcement working rather than undefined behaviour.
    unsafe {
        core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags));
    }
}

/// **Read one byte from an x86 I/O port** (milestone 299). The `in` twin of [`outb`]; the same
/// capability rule and the same fault on a port the caller does not hold. A read is not automatically
/// harmless: several legacy devices (the 16550's receive register at COM1's base) have read side
/// effects, which is exactly why a driver, not the kernel, decides when to issue one.
#[cfg(target_arch = "x86_64")]
pub fn inb(port: u16) -> u8 {
    let val: u8;
    // SAFETY: as `outb`; the caller holds the port capability or this faults.
    unsafe {
        core::arch::asm!("in al, dx", out("al") val, in("dx") port, options(nostack, preserves_flags));
    }
    val
}

/// Drop the capability in `slot` from this thread's capability table (`SYS_CAP_DELETE`). Deleting an empty
/// slot is a no-op. A program that retypes many objects (a loader, a spawner) frees each slot as
/// soon as it is done with it, so its fixed capability table does not fill.
#[cfg(target_arch = "aarch64")]
pub fn cap_delete(slot: u64) {
    // SAFETY: `svc`; SYS_CAP_DELETE frees a slot in the caller's own capability table, nothing to clean up.
    unsafe {
        core::arch::asm!(
            "svc #0",
            in("x8") abi::SYS_CAP_DELETE,
            in("x0") slot,
            options(nostack, nomem),
        );
    }
}

/// Drop the capability in `slot` (RISC-V). `ecall`, `SYS_CAP_DELETE` in `a7`, slot in `a0`.
#[cfg(target_arch = "riscv64")]
pub fn cap_delete(slot: u64) {
    // SAFETY: `ecall`; SYS_CAP_DELETE frees a slot in the caller's own capability table, nothing to clean up.
    unsafe {
        core::arch::asm!(
            "ecall",
            in("a7") abi::SYS_CAP_DELETE,
            in("a0") slot,
            options(nostack, nomem),
        );
    }
}

/// Drop the capability in `slot` (`x86_64`). `syscall`, `SYS_CAP_DELETE` in `rax`, slot in `rdi`.
#[cfg(target_arch = "x86_64")]
pub fn cap_delete(slot: u64) {
    // SAFETY: `syscall`; SYS_CAP_DELETE frees a slot in the caller's own capability table, nothing to clean up.
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") abi::SYS_CAP_DELETE,
            in("rdi") slot,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, nomem),
        );
    }
}

/// The virtual counter, `CNTVCT_EL0`: a monotonic tick count for self-timing. Readable at EL0 only
/// because the kernel opened `CNTKCTL_EL1.EL0VCTEN` (see kernel `timer::init` and notes/abi.md); the
/// read is a plain register move, no syscall. Pair with [`cntfrq`] to turn tick deltas into seconds.
#[cfg(target_arch = "aarch64")]
pub fn now() -> u64 {
    let t: u64;
    // SAFETY: reading a system register the kernel made EL0-readable. No side effects.
    unsafe {
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) t, options(nomem, nostack));
    }
    t
}

/// The monotonic tick count (RISC-V): `rdtime`, which reads the `time` CSR. Readable from U-mode
/// only because the kernel sets `scounteren.TM` in its per-hart timer init, the same shape as
/// aarch64 needing `CNTKCTL_EL1.EL0VCTEN`. That claim was aspirational until 2026-07-30: the bit was
/// never set and this worked only because QEMU's OpenSBI leaves it permitted. Pair with [`cntfrq`]
/// to get seconds.
#[cfg(target_arch = "riscv64")]
pub fn now() -> u64 {
    let t: u64;
    // SAFETY: reading the time CSR the kernel made U-mode-readable. No side effects.
    unsafe {
        core::arch::asm!("rdtime {}", out(reg) t, options(nomem, nostack));
    }
    t
}

/// The counter frequency in Hz, `CNTFRQ_EL0`: how many [`now`] ticks make a second. Constant for the
/// life of the machine (QEMU reports 62.5 MHz under TCG, the host's counter frequency under HVF).
///
/// This is the one architecture where the rate is always knowable, so [`cntfrq_checked`] here is
/// always `Some` and this function cannot fail. The other two read a page the kernel fills, and both
/// can report "unknown"; see [`cntfrq_checked`].
#[cfg(target_arch = "aarch64")]
pub fn cntfrq() -> u64 {
    let f: u64;
    // SAFETY: reading a system register; EL0-readable once EL0VCTEN is set.
    unsafe {
        core::arch::asm!("mrs {}, cntfrq_el0", out(reg) f, options(nomem, nostack));
    }
    f
}

/// The counter frequency in Hz, or `None` if this process cannot know it (aarch64: never, the
/// machine states it in `CNTFRQ_EL0`).
#[cfg(target_arch = "aarch64")]
pub fn cntfrq_checked() -> Option<u64> {
    Some(cntfrq())
}

/// The counter frequency in Hz (RISC-V), read from the **timebase page** the kernel maps read-only
/// into every process at [`counter_frequency_protocol::PAGE_VA`].
///
/// RISC-V has **no** register that reports the timebase. The machine states it in the device tree,
/// at `/cpus/timebase-frequency`, and a process holds neither a pointer to the blob nor a capability
/// naming the memory it lives in, so the kernel is the only party that can ever know this number.
/// `kernel::arch::riscv64::timer::init_frequency` reads it on the boot hart and
/// `kernel::user::timebase_page_phys` publishes it; this function is the reader, a plain load, no
/// syscall, the same ambient shape [`now`] already has.
///
/// **It returned a hardcoded `10_000_000` until 2026-09-21**, QEMU `virt`'s rate, with a comment
/// proposing an aux-vector as the eventual fix. radon (the VisionFive 2) runs its `time` CSR at
/// **4 MHz**: every userspace duration measured on that board was 2.5x too large and nothing said
/// so. The aux-vector was never built and is not needed; `x86_64` had already solved the same
/// problem with a page, one directory away, and this now uses it. calef's ruling that forced it:
/// *"We should ensure that the program only returns accurate numbers versus leveraging hard coded
/// ones."*
///
/// # Panics
///
/// If the rate is unknown: see [`cntfrq_checked`], which is this without the panic.
#[cfg(target_arch = "riscv64")]
pub fn cntfrq() -> u64 {
    cntfrq_checked().expect(
        "the counter frequency is unknown: this process's timebase page is zeroed or unrecognized",
    )
}

/// The counter frequency in Hz (RISC-V), or `None` if this process cannot know it. See [`cntfrq`]
/// for where the number comes from and [`cntfrq_checked`]'s `x86_64` twin for why "unknown" is a
/// state worth representing rather than papering over.
#[cfg(target_arch = "riscv64")]
pub fn cntfrq_checked() -> Option<u64> {
    // SAFETY: every path that builds a riscv64 process maps a page (real, or zeroed when the rate
    // was never learned) read-only at `counter_frequency_protocol::PAGE_VA` before it runs:
    // `kernel::user::load`, `kernel::user::map_timebase_page`, and
    // `supervision_protocol::build_child_space`. See that crate's `BUGS` section for the one shape
    // that does not, which faults here on purpose.
    let page = unsafe {
        counter_frequency_protocol::TimebasePage::new(counter_frequency_protocol::PAGE_VA)
    };
    page.hz()
}

/// The monotonic tick count (`x86_64`, milestone 161): `rdtsc`, which reads the time-stamp counter.
/// Readable from ring 3 because `CR4.TSD` is clear, which is the reset state and which this kernel
/// does not change; that is the same shape as aarch64 needing `CNTKCTL_EL1.EL0VCTEN` and RISC-V
/// needing `scounteren.TM`, with the difference that here the permissive state is the default and
/// the kernel would have to act to *close* it.
///
/// **`rdtsc` returns the count split across two 32-bit registers** (`edx:eax`), a shape inherited
/// from the Pentium that introduced it, so this is a shift and an or rather than a move. Writing it
/// as a single `out(reg)` compiles and reads only the low half, which is a counter that wraps every
/// four seconds at 1 GHz and looks correct in any test short enough to run.
///
/// **No serialisation, deliberately.** `rdtsc` is not ordered against surrounding instructions, so
/// a sufficiently tight measurement wants `lfence` or `rdtscp` around it. Pair this with [`cntfrq`]
/// and the granularity that buys is nanoseconds; the reordering window is tens of cycles, and the
/// kernel's own calibration accepts the same trade for the same reason
/// (`kernel/src/arch/x86_64/timer.rs`).
///
/// # BUGS
///
/// **On `x86_64` the cycle counter is ambient, and nobody chose that.** The other two architectures
/// give userspace a *coarse* counter and keep the fine one shut: aarch64 opens `CNTVCT_EL0` and, as
/// of milestone 228 (the cycle counters are closed by assumption, and on two architectures the
/// assumption is a comment), writes `PMUSERENR_EL0 = 0` so `PMCCNTR_EL0` stays closed; riscv64 opens
/// `scounteren.TM` and clears `CY`. Here there is one register for both jobs. The TSC *is* the
/// coarse clock and the cycle counter, `CR4.TSD` is clear at reset, this kernel never writes it, and
/// so every ring-3 program on this architecture holds a sub-nanosecond timing instrument it was
/// never granted. That is a state inherited from the reset value, not a position anyone argued for,
/// and it is recorded here rather than in a tracker so the next reader of this function meets it.
///
/// **The `rdpmc` door beside it did close.** `CR4.PCE` is a different bit from `CR4.TSD` and gates a
/// different instruction, and since fixed counter 2 runs at the TSC rate it is a second path to a
/// cycle-rate reading. Milestone 228 established it clear in `arch::init`, per core, because nothing
/// in this tree reads a performance counter from ring 3 and so closing it cost nothing. So the gap
/// below is `rdtsc` specifically, not "x86 counters" in general.
///
/// **It is not closed because closing it today would take the clock away.** Setting `CR4.TSD` with
/// nothing to replace it breaks `Instant`, `thread::sleep`, the random seed, smoltcp's timestamps in
/// `std_net`, and the benchmark harness, all at once and on the same instruction. So this is a
/// limitation with a price rather than an oversight, and paying it needs a second time source first:
/// a coarse monotonic value published in a page, which is DECISIONS §43's move (reading the clock is
/// a page, which put the wall clock in a page rather than a register) one axis over. Nothing here
/// proposes building that.
///
/// **What it costs meanwhile**, stated so §19 (architectural parity is a tenet) reports a known gap
/// rather than a silent one: `x86_64` answers milestone 75 (who may read the cycle counter, and by
/// what authority) with "everyone, always", by inheritance, whatever that decision concludes for the
/// other two. Linux names this exact asymmetry from the other side; its arm64 per-task
/// `PMUSERENR_EL0` work says it opens the counter only on request to avoid "the information leaks
/// x86 has".
#[cfg(target_arch = "x86_64")]
pub fn now() -> u64 {
    let (lo, hi): (u32, u32);
    // SAFETY: reading a counter ring 3 is permitted to read. No side effects, no memory touched.
    unsafe {
        core::arch::asm!("rdtsc", out("eax") lo, out("edx") hi, options(nomem, nostack));
    }
    ((hi as u64) << 32) | (lo as u64)
}

/// The counter frequency in Hz (`x86_64`), read from the **timebase page** the kernel maps
/// read-only into every process at [`counter_frequency_protocol::PAGE_VA`] (milestone 161's `cntfrq`
/// follow-up).
///
/// aarch64 has `CNTFRQ_EL0`, which states the rate. RISC-V has none, but the device tree does,
/// and the process cannot read it. x86 has **no architected rate a ring-3 program can ask for
/// directly**: `CPUID` leaf `0x15` gives the TSC's ratio to a crystal clock on the parts that
/// implement it, but a ring-3 program cannot calibrate one for itself the way the kernel's
/// fallback does, against the 8254 PIT: the PIT is at I/O ports `0x40..0x43`, `IOPL` is 0 and the
/// TSS's I/O permission bitmap is empty, so `in`/`out` from a process is a general protection
/// fault. That is not an oversight to route around, it is
/// [DECISIONS §121](../../../design/decisions/121-port-io-capability.md), which closed port I/O
/// to userspace **permanently**: a program that could calibrate its own clock by touching the PIT
/// would be a program that had escaped the confinement this kernel exists to enforce.
///
/// So the kernel is the only party that can ever know this number, and it publishes rather than
/// gates: `arch::x86_64::timer::init_frequency` reads `CPUID` leaf `0x15` first and falls back to
/// PIT calibration only if the part does not report one (see that function's own docs; under this
/// project's QEMU invocation the leaf is unavailable and calibration is what every boot actually
/// uses), then every kernel-side function that builds a top-level process's address space writes
/// the result into a page and maps it, read-only, before the process ever runs: `kernel::user::load`
/// (the generic ELF loader every ordinary test fixture calls) and the handful of functions that
/// build one by hand for a narrower world (`spawn_init`, and the `spawn_<program>`-shaped test
/// harnesses: `timetable_tests::spawn_timetable`, `authority_tests`' `root_supervisor` spawn,
/// `c_seam_tests::spawn_confiner`, `login_service`, `live_swap_tests`' `swapper` spawn; see
/// `kernel::user::map_x86_timebase_page`, which all of them call). This function is the reader: a
/// plain load through an unsafe pointer, no syscall, the same "ambient, no capability" shape
/// [`now`] already has.
///
/// # Panics
///
/// If the rate is unknown, which is a zeroed or unrecognized page. **It used to return 1 GHz
/// instead**, and that fallback was deleted on 2026-09-21 under calef's ruling: *"We should ensure
/// that the program only returns accurate numbers versus leveraging hard coded ones."* A plausible
/// wrong rate is worse than no rate, because it produces a benchmark figure somebody quotes; a
/// process that dies on the read produces nothing to quote. [`cntfrq_checked`] is this without the
/// panic, for a caller that would rather cope than die.
///
/// **The argument that fallback rested on has expired.** It was that widening this one architecture
/// to `Option<u64>` would make every caller of the portable [`monotonic_nanos`] handle a case the
/// other two could not produce. Two of three architectures read a page now, so the case is no
/// longer peculiar, and the shape this tree already uses for exactly this question is the kernel's
/// own: `arch::timer::frequency` panics and `arch::timer::frequency_checked` returns an `Option`,
/// with the reason written beside them ("a plausible zero is worse than a panic naming this file").
/// This is that pair, one privilege level down, so `monotonic_nanos` keeps its `u64` and nothing
/// fabricates a rate.
///
/// **What the panic costs, honestly**: a userspace panic here traps (see [`trap`]) and the kernel
/// kills the process, with no message, because a program with no console cannot print one. So the
/// symptom a reader meets is a killed process and a fault line, not a diagnostic naming the page.
/// That is the same floor every `expect` in userspace has and it is not special to this one.
///
/// # BUGS
///
/// **The one case that used to reach the fallback is gone, not merely refused.** A process built by
/// `supervision_protocol::build_child_space` (the tree's one userspace ELF loader, used by
/// `root_supervisor`, `spawner`, `system_initializer`, and every role `hello` builds, `coremark` and
/// `timetable`'s own `least_authority_demo` included) used to map a freshly retyped, *zeroed*
/// placeholder and so read 1 GHz. It now gets a page that crate fills from its own rate, read
/// through [`cntfrq_checked`], so a child inherits its parent's measured number. The residue is
/// real and small: a parent that does not know the rate writes a zeroed page, and the whole subtree
/// below it refuses rather than guessing.
#[cfg(target_arch = "x86_64")]
pub fn cntfrq() -> u64 {
    cntfrq_checked().expect(
        "the counter frequency is unknown: this process's timebase page is zeroed or unrecognized",
    )
}

/// The counter frequency in Hz (`x86_64`), or `None` if this process cannot know it: a zeroed page
/// (the rate was never measured, or the parent that built this process did not know it) or one
/// whose magic does not match. See [`cntfrq`] for where the number comes from and why the
/// unchecked twin refuses rather than substituting a constant.
#[cfg(target_arch = "x86_64")]
pub fn cntfrq_checked() -> Option<u64> {
    // SAFETY: every kernel-side space-building function this crate's own docs list maps a page
    // (real, or zeroed when the rate was never learned) read-only at
    // `counter_frequency_protocol::PAGE_VA` into every x86_64 process before it ever runs.
    let page = unsafe {
        counter_frequency_protocol::TimebasePage::new(counter_frequency_protocol::PAGE_VA)
    };
    page.hz()
}

/// **Which CPU this thread is running on**, or `None` if this process has no page to read it from
/// or has somehow not been switched in yet.
///
/// A plain load, no syscall, and that is the point rather than an optimization. calef ruled on
/// 2026-09-21 that a thread observing *itself* gets a page and a thread observing *another* gets a
/// selector, because the consumer here is a memory allocator keeping a per-CPU cache: it asks once
/// per allocation, and a crossing at that rate is not a cost you can amortize away. (The ruling's
/// `design/decisions/` section is on another branch and not on `main` yet, so it is named here
/// rather than cited.) `crates/current_cpu_protocol` carries the layout, the ordering argument and
/// the reason this is not a register on any of the three targets.
///
/// **The answer can be stale before you use it**, because the kernel may migrate this thread the
/// instruction after the load, and nothing in this function or that crate prevents it. A per-CPU
/// cache built on this must be *correct* when the answer turns out to be the previous core's and
/// merely faster when it is not. Linux solves the same problem with `rseq`'s restartable
/// sequences; this tree does not have those and is not pretending to.
///
/// **The value is a CPU id, not an index into anything you have counted.** Size a per-CPU array by
/// [`current_cpu_protocol::CPU_ID_BOUND`] and nothing else: the VisionFive 2's online set is
/// `{1, 2, 3}`, so counting cores and indexing by the count reaches a core that is not there and
/// misses one that is.
///
/// # EXAMPLES
///
/// ```no_run
/// # use user_mode_runtime::current_cpu;
/// let mut per_cpu = [0u64; current_cpu_protocol::CPU_ID_BOUND];
/// if let Some(cpu) = current_cpu() {
///     per_cpu[cpu] += 1;
/// }
/// ```
///
/// # BUGS
///
/// - **A process whose address space was never bound to a TCB has no page here**, and this faults
///   on an unmapped read rather than answering `None`. Every space that runs a thread gets one
///   (`kernel::user::AddressSpace::new` for the kernel-built ones,
///   `kernel::sched::configure_thread_control_block` for the userspace-built ones), so the shape
///   that faults is a space with no thread in it, which has nothing to ask. Stated because the
///   failure is a fault rather than a value, which is the one thing a caller cannot handle.
pub fn current_cpu() -> Option<usize> {
    // SAFETY: the kernel maps a page it owns, read-only, at `current_cpu_protocol::PAGE_VA` into
    // every address space that a TCB binds, before that thread's first instruction runs. See this
    // function's own BUGS section for the one shape that has no page.
    let page = unsafe { current_cpu_protocol::CurrentCpuPage::new(current_cpu_protocol::PAGE_VA) };
    page.cpu()
}

/// **Monotonic nanoseconds since boot**, from [`now`] and [`cntfrq`].
///
/// Here rather than in each program because two of them need it and the naive form is wrong:
/// `ticks * 1_000_000_000` overflows a `u64` about five minutes into a boot at 62.5 MHz, so the
/// conversion splits into whole seconds and a remainder. `components/src/clock.rs` found that the hard
/// way; `date` and the shell's `time` then wanted the same five lines, which is CLAUDE.md rule 7 at
/// the smallest size it comes in.
///
/// **This needs no capability**, and that is worth knowing when reading anything that calls it: the
/// counter is ambient (the kernel opened it to EL0), so a *duration* is measurable by any process.
/// What needs a capability is the **wall clock**, which is this plus an offset only the clock page
/// carries. See notes/clock.md.
pub fn monotonic_nanos() -> u64 {
    /// Nanoseconds in a second. Spelled here rather than taken from `clock_protocol`, because this
    /// crate is the syscall runtime and depends on `abi` alone; a runtime that pulled in a wire
    /// contract to name a unit would be the wrong direction for the dependency.
    const NANOS_PER_SEC: u64 = 1_000_000_000;
    let freq = cntfrq();
    let ticks = now();
    let secs = ticks / freq;
    let rem = ticks % freq;
    secs * NANOS_PER_SEC + rem * NANOS_PER_SEC / freq
}

/// Terminate this process. The kernel reaps the thread and frees its whole address space. Never
/// returns; the trailing spin is only there to satisfy the `-> !` type if `svc` ever came back.
pub fn exit() -> ! {
    // SAFETY: the syscall never returns; the trailing spin only satisfies the `-> !` type.
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("svc #0", in("x8") abi::SYS_EXIT, in("x0") 0u64, options(nostack, nomem));
    }
    #[cfg(target_arch = "riscv64")]
    // SAFETY: `ecall` with SYS_EXIT traps to the kernel, which never returns to this thread. The options promise it touches neither memory nor the stack.
    unsafe {
        core::arch::asm!("ecall", in("a7") abi::SYS_EXIT, in("a0") 0u64, options(nostack, nomem));
    }
    #[cfg(target_arch = "x86_64")]
    // SAFETY: `syscall` with SYS_EXIT traps to the kernel, which never returns to this thread. The
    // options promise it touches neither memory nor the stack; `rcx` and `r11` are the
    // instruction's own clobbers and are declared as such even here, where nothing comes back, so
    // this reads the same as every other `syscall` site rather than being a special case.
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") abi::SYS_EXIT,
            in("rdi") 0u64,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack, nomem),
        );
    }
    loop {
        core::hint::spin_loop();
    }
}

/// **Die where the mistake was.** Raise a breakpoint the kernel turns into a fault, so the process
/// is killed rather than allowed to limp on.
///
/// This is the other way a program can end, and the difference from [`exit`] is not a spelling.
/// `exit` reports `EVENT_EXIT` to a supervisor and this reports `EVENT_FAULT`
/// (`kernel/src/sched.rs`, DECISIONS §26), so a supervised child that traps is legible as having
/// failed and one that exits is not. A panic must take this path or it lies about what happened.
///
/// The instruction differs per architecture and `x86_64`'s is not the obvious one: `brk #0` on
/// aarch64, `ebreak` on riscv64, **`ud2`** on `x86_64`. See that arm for why its breakpoint
/// instruction cannot be used from ring 3.
///
/// The trailing spin never runs. It is here for the same reason [`exit`]'s is, to satisfy `-> !`
/// if the trap ever came back, and it spins rather than calling `exit` on purpose: `exit` would
/// turn an impossible situation into a clean-looking death, which is precisely the confusion the
/// paragraph above exists to prevent.
///
/// A verb, which is right for a function here: `send`, `recv`, `reap` and `exit` are all verbs, and
/// the naming tenet's noun rule is about crates, programs and modules rather than about the things
/// they do.
///
/// # BUGS
///
/// **The fault line names this function and nothing else, so every trap in a program looks
/// identical.** The kernel prints the faulting `pc`, and the `pc` of a trap is always the
/// breakpoint instruction below: one address per program, shared by every caller. A program with
/// forty `must(...)` sites that dies at one of them reports the same line whichever it was. Found
/// the hard way by milestone 230, whose whole first day was spent working out *which* call in
/// `crates/system_initializer` had failed, from a fault line that could not say. The return address
/// is right there in `x30` (aarch64), `ra` (riscv64), or on the stack (`x86_64`) at the moment the
/// kernel takes the fault, and printing it beside `pc` would have answered the question in one
/// boot. That is a change to the three `arch/*/exceptions.rs` fault printers rather than to this
/// crate, so it is a milestone of its own; until it exists, the workaround that worked is to make
/// the failure a *data* abort at an address derived from the return address, since `far` is
/// printed and carries whatever you fault on.
pub fn trap() -> ! {
    #[cfg(target_arch = "aarch64")]
    // SAFETY: `brk` traps; the kernel turns a trap from userspace into a kill. The options promise
    // it touches neither memory nor the stack.
    unsafe {
        core::arch::asm!("brk #0", options(nostack, nomem));
    };
    #[cfg(target_arch = "riscv64")]
    // SAFETY: `ebreak` traps; the kernel turns a trap from userspace into a kill. The options
    // promise it touches neither memory nor the stack.
    unsafe {
        core::arch::asm!("ebreak", options(nostack, nomem));
    };
    #[cfg(target_arch = "x86_64")]
    // SAFETY: `ud2` faults; the kernel turns a fault from ring 3 into a kill. The options promise
    // it touches neither memory nor the stack.
    //
    // **`ud2` rather than `int3`, and this was measured rather than chosen** (milestone 161).
    // `int3` is the obvious transliteration of `brk`/`ebreak` and it does not work from ring 3
    // here: a *software* interrupt is refused unless the IDT gate's DPL admits the caller's
    // privilege level, and this kernel's gates are all DPL 0, so `int3` from a process raises
    // **#GP with error code 0x1a** (`(3 << 3) | 2`, the vector it was refused, tagged as an IDT
    // selector) instead of #BP. The process does die, so the first version of this looked like it
    // worked; what it reported was a general protection fault at address zero, which names neither
    // the instruction nor the reason and would have sent the next reader hunting a segmentation
    // bug.
    //
    // Opening vector 3 to ring 3 is the other fix and Linux takes it, because Linux has ptrace and
    // wants a debugger to be able to plant breakpoints. There is no debugger here, so that would
    // widen what a process may do to buy nothing. `ud2` is a **fault the CPU raises** on an opcode
    // permanently reserved to be invalid, so no gate DPL is involved, and "this must never execute"
    // is exactly what the instruction means.
    unsafe {
        core::arch::asm!("ud2", options(nostack, nomem));
    };
    loop {
        core::hint::spin_loop();
    }
}

/// **The panic handler every nife program wants**, as a macro so it stays per-final-binary.
///
/// Write `user_mode_runtime::panic_handler!();` once at the top level of a binary and it expands to a
/// `#[panic_handler]` that calls [`trap`].
///
/// # Why a macro and not a plain item in this crate
///
/// A `#[panic_handler]` is per-final-binary: exactly one may exist in a linked program, so a
/// library that defines one forces it on every binary that links the library and collides with any
/// binary wanting its own. That constraint is real and this crate's header has recorded it since
/// milestone 19f.6. A macro keeps it: nothing is defined until a binary asks, and a program with
/// its own handler simply does not invoke this.
///
/// What the header got wrong, and what milestone 130 is fixing, is the clause after it: "each
/// binary keeps its own one-line handler; it is trivial." It stopped being one line. By the time
/// anyone counted it was fifteen, with two `unsafe` blocks and two `// SAFETY:` comments, at
/// forty-eight sites across `user/`, `crates/` and `redoxfs_server/`, in **seven** variants. One of
/// them (`terminal_sink_caretaker`) called `exit` instead of trapping, which reports a clean death
/// for a panicking program; it was latent only because that program happens to be spawned
/// unsupervised.
///
/// So the decision not to put a *handler* in a library was right, and the inference that each
/// binary must therefore hand-roll the *trap* did not follow. The trap belongs here, in the crate
/// whose header calls itself the one place in userspace that names the two ABIs, and the macro is
/// what lets that be true without breaking the linking property.
///
/// # Examples
///
/// ```ignore
/// #![no_std]
/// #![no_main]
///
/// user_mode_runtime::panic_handler!();
///
/// #[unsafe(no_mangle)]
/// pub extern "C" fn _start() -> ! {
///     user_mode_runtime::exit()
/// }
/// ```
///
/// # BUGS
///
/// It takes no arguments and ignores the `PanicInfo`, because a program with no console cannot
/// print one. A program that *can* print (it holds a terminal endpoint) and wants the message on
/// the way down still writes its own handler; this macro is the default, not a mandate.
///
/// Named for the item it expands to, which is the one thing a reader needs it to say.
#[macro_export]
macro_rules! panic_handler {
    () => {
        #[panic_handler]
        fn panic(_: &::core::panic::PanicInfo) -> ! {
            $crate::trap()
        }
    };
}
