#![no_std]
// `Result<_, ()>` throughout the build path, for `supervision_protocol`'s reason: every failure here is
// a syscall that already returned its own error through the ABI, and a second, richer error would be
// inventing detail the kernel did not provide. See that crate's head comment.
#![allow(clippy::result_unit_err)]
//! **The interactive system, built once** (milestone 96).
//!
//! There is **one** first process, `components/src/progenitor.rs`, on all three architectures
//! (milestone 266), and **one** system it builds, which is this crate. What that program still
//! holds is the table of slot numbers its own kernel granted, which is a fact about a boot path and
//! nothing else; everything from parsing the archive to serving the shell's last `run` is here.
//!
//! **There used to be two.** `user::initrd()` loaded an archive entry called `init`, which was
//! `fixtures/src/hello.rs`'s `init_boot` role on aarch64 and a separate program on riscv64: an alias
//! standing over two implementations of one job. This crate ended the duplicated *code* at
//! milestone 96 and 266 ended the duplicated *program*.
//!
//! Before this crate the construction and the spawn service were written twice, about three hundred
//! near-identical lines, and the failure that caused is the reason milestone 96 exists: a fix that
//! lands in one copy and not the other is **a boot that reaches userspace and prints nothing at
//! all**, with no fault and no message. That shape cost three separate lanes an evening each.
//! `script/swish-check` boots both ISAs and types at the prompt, which is what makes it the gate
//! that proves this: it is the only thing in the tree that runs the real progenitor.
//!
//! # Examples
//!
//! **This one cannot run either, and the reason is structural.** [`boot`] returns `!` and every step
//! it takes is a syscall on a capability the kernel granted at spawn, so there is nothing to assert
//! and nowhere to assert it: `script/test`'s host pass excludes this crate (it depends on `user_mode_runtime`'s
//! EL0 `asm!`, an exclusion `script/lint` derives and checks), and the thing that actually proves this
//! code is `script/swish-check`, which boots both ISAs and types at the prompt. So the example below
//! is `no_run`: type-checked against the real signatures, and executed by that gate.
//!
//! A progenitor's whole source, near enough. Everything a board contributes is the **table of slot numbers
//! its own kernel granted**, which is a fact about that boot path and nothing else:
//!
//! ```no_run
//! use system_initializer::{BootEndowment, boot};
//!
//! /// riscv64's the progenitor. The slot numbers come from `kernel::user::riscv_shell_boot` and are the only
//! /// thing this file knows that the other board's the progenitor does not.
//! fn init(initrd_len: u64, fs_rights: u64) -> ! {
//!     let endowment = BootEndowment {
//!         untyped: 2,
//!         uart_dev: 3,
//!         uart_irq: 4,
//!         clock_page: 5,
//!         config_page: 6,
//!         fs_ep: 7,
//!         fs_page: 8,
//!         // Always these three slots, whether or not this boot has a disk (`grant_at`, not
//!         // first-free numbering, on the kernel side): a boot with no virtio-rng device leaves
//!         // them empty, and `boot`'s own probe is what tells it apart from a granted one. Fixed
//!         // past the filesystem pair's own max reach (slot 8), not slot 7, because the
//!         // inert-configuration page (slot 6) shifted that pair down by one.
//!         virtio_rng: 9,
//!         virtio_rng_irq: 10,
//!         virtio_rng_dma: 11,
//!         // The graphical terminal stack (milestone 177, option A): empty on a boot with no GPU
//!         // or no keyboard attached, the same "absence, not failure" shape as the virtio-rng trio.
//!         disp_term_ep: 12,
//!         disp_term_page: 13,
//!         kbd_ep: 14,
//!         // The network card (milestone 590 (the booted system starts its network stack)): empty
//!         // on a boot with no virtio-net device, which is every real board today. Numbered here
//!         // past this example's own graphical trio; the real table is
//!         // `components/src/progenitor.rs`'s.
//!         virtio_net: 15,
//!         virtio_net_irq: 16,
//!         virtio_net_dma: 17,
//!         // An entropy service the kernel built on the CPU's seed instruction (milestone 595
//!         // (provisional)): empty whenever the virtio-rng trio is granted, and on a CPU with no
//!         // such instruction, which is every riscv64 part.
//!         entropy_ep: 18,
//!         // Empty here. On aarch64 this holds the kernel's report endpoint and a test SGI, because
//!         // that boot path is shared with milestone 19d's test roles; the progenitor deletes them with the
//!         // device authority once the drivers exist, rather than keeping delegable authority for
//!         // nothing.
//!         for_test_roles: &[],
//!     };
//!
//!     // `fs_rights` of 0 means this boot attached no RedoxFS disk, in which case `fs_ep` and
//!     // `fs_page` hold nothing at all and the system comes up without a filesystem.
//!     //
//!     // The fourth argument is milestone 154's second directory grant: `None` here, as at every
//!     // real entry point today, because what the second subtree should *be* is a boot-time
//!     // policy decision, not something this call site decides for itself.
//!     boot(&endowment, initrd_len, fs_rights, None)
//! }
//! ```
//!
//! Reading that struct literal is meant to tell you the complete authority of the system about to
//! exist, which is why the fields are named for what they *are* rather than numbered. Note what is
//! not in it: no second budget. (It said "no network" too until milestone 590 (provisional) granted
//! the network card; see [`BootEndowment::virtio_net`].)
//!
//! # What the kernel hands over, and what this builds from it
//!
//! [`BootEndowment`] names the capabilities the kernel granted: a construction budget, the UART's registers
//! as a device capability, the UART receive interrupt, a read-only view of the wall clock, when
//! this boot attached a RedoxFS disk the file service and the page its clients share with it, and,
//! when a GPU and a keyboard are both attached (milestone 177, option A), `display_terminal`'s own
//! served endpoint and output page, plus the endpoint the keyboard driver already `CALL`s. From
//! those, and nothing else, [`boot`] builds the whole interactive system out of its own budget:
//!
//! 1. **output**: the **console** server, reading text from a shared page and writing it to the
//!    UART, when this boot has no graphical terminal stack. When it does, there is no console
//!    server at all: `display_terminal` was already built kernel-side, before this process existed,
//!    and `line_editor` prints through it directly instead. **A terminal with no keyboard of its
//!    own** (a PC's firmware framebuffer, milestone 198's rung 1b) is neither: the console server is
//!    built as usual and also writes every byte to that terminal, so the UART and the screen say
//!    the same thing.
//! 2. **keystrokes**: the **input** driver, waiting on the UART receive interrupt and forwarding
//!    bytes, in the same "no graphical stack" case. When there is one, `keyboard_driver` plays this role
//!    instead, also built kernel-side, `CALL`ing `line_editor`'s own endpoint directly rather than
//!    going through this process at all (option A's whole point: no compositor in this path).
//! 3. the **line discipline** (`line_editor`, milestone 28): editing, echo, history, between
//!    whichever pair of the above this boot has. The same server either way; only its output-side
//!    wire shape changes (`mode`, `components/src/line_editor.rs`'s own `MODE_CONSOLE`/`MODE_DISPLAY`).
//! 4. the **shell**: prints and reads lines through the terminal endpoint, runs commands, and since
//!    milestone 86 holds a `READ` view of the wall clock so `time <command>` can measure one;
//! 5. the **terminal's sink adapter** (`terminal_sink_caretaker`, milestone 50), when the archive
//!    carries one: it holds the terminal and serves the sink contract, so a declared second stream
//!    can be pointed at the screen without handing anyone the endpoint that also reads the keyboard;
//! 6. the **undertaker** (`job_undertaker`), which collects a finished job's corpse so its region
//!    comes back to the pool.
//!
//! They are wired together with endpoints and shared pages this code creates. The kernel wires none
//! of it. Then the progenitor stays alive as the spawn service (milestone 31): the shell resolves a `run` into
//! a grant expression and directs the progenitor to load the named program and endow it with exactly what the
//! command named. Nothing here names an architecture: the console and input drivers hold the one
//! device-specific fact (the UART register layout), and the kernel grants the right device.
//!
//! # What it gives away once the system is up (milestone 22, the interactive increment)
//!
//! It used to hold the kernel's whole construction budget for life, which made every process in the
//! system one bug in the progenitor away from being built wrong. It no longer does. Once the boot servers
//! above are built it carves two bounded budgets off that root and **deletes the root**:
//!
//! - [`INIT_OWN_PAGES`] for its own scratch page tables, which is all it spends on itself; and
//! - [`JOBS_BUDGET_PAGES`] for the jobs the prompt asks for, one reclaimable region per job.
//!
//! It also gives up the UART device capability and the UART interrupt as soon as the drivers that
//! need them are built, and everything in [`BootEndowment::for_test_roles`] with them. **It keeps
//! the file service** (milestone 31 (a capability shell) phase 3, 2026-08-17): this sentence said it gave that away as
//! soon as the shell held it, and DECISIONS §208 (installing a package is granting it) and milestone 507 (installing a package: mutate, compose, or widen) repeated it as the reason the
//! spawner cannot read an installed program. It can; see "The filesystem stays" in `boot`, and
//! DECISIONS §219 (how the shell names an installed program to the spawner) for what that changes. The proof is a negative control taken from inside the process and
//! printed at the prompt, exactly the shape `root_supervisor` uses: after the delete, `RETYPE` and
//! `RETYPE_OBJ` on that slot must answer `NoSuchSlot` (there is nothing there) rather than
//! `NotPermitted` (there is, and you may not).
//!
//! The job budget is **renewable**, which is what makes bounding it cheap. Every job is built in its
//! own region split off [`JOBS_BUDGET_PAGES`] and is born supervised: `job_undertaker`, a process
//! holding one endpoint capability and nothing else, collects each corpse through `Rendezvous::REAP`
//! (DECISIONS §32) and the region's pages come back here (§13: a reclaimed region returns to its
//! owner, which is whoever split it). Before that, a spawned job's memory was spent for the life of
//! the boot.
//!
//! # What it refuses to load (milestone 104)
//!
//! The kernel measures the one program *it* loads, which is this one. Everything else in the
//! archive is loaded here, and those bytes used to be unchecked, so the chain of trust stopped at
//! the progenitor's entry. It does not now. The build packs a table of digests into the archive
//! ([`measured_boot::PROGRAM_MEASUREMENTS`]), the kernel's trust root vouches for that table exactly
//! as it vouches for this program's own bytes, and [`boot`] looks every program up in it before
//! loading it.
//!
//! **One rule: the progenitor grants nothing of its own to what it cannot vouch for.** Restated
//! 2026-09-26 by DECISIONS §219 (how the shell names an installed program to the spawner), from
//! "runs nothing it cannot vouch for": unvouched bytes a caller sends may run with the caller's
//! grants and the clock and configuration pages, but only for a session holding §219's D2
//! capability (`Channels::run_unvouched`; the boot prompt holds it by DECISIONS §221). That is
//! `spawn_service`'s image path. Everything this section describes below is the archive's, where
//! the rule still means refusal: a digest that does not match is a refusal, and so is a
//! name the table does not mention, for the reason the kernel's empty trust root is refused: a
//! check that passes when there is nothing to check against is not a check.
//!
//! What a refusal *costs* is not a second policy. It falls out of what the program was for, which is
//! a question this crate already had to answer for an archive entry that is simply missing, so a
//! refused program is treated exactly as a missing one:
//!
//! - **console, input, `line_editor`, swish, `job_undertaker`.** The system is made of these, so not
//!   running one and not having a system are the same outcome. The progenitor prints which one it refused and
//!   traps, which is `kernel::trust::require`'s decision one link down.
//! - **`terminal_sink_caretaker`.** Already optional: a boot without an adapter comes up and a
//!   declared second stream finds an empty slot. A refused adapter costs that same feature.
//! - **The spawnable programs.** Left out of the table `spawn_service` indexes, so the prompt
//!   answers "could not spawn" for them and everything else still works. Halting a running machine
//!   because `wc` changed would turn a build defect into an unbootable system, and it buys nothing:
//!   the guarantee is that nothing unvouched-for runs, and not spawning it is that guarantee.
//!
//! Recording a mismatch and loading anyway was considered and refused. There is no audit log to put
//! a record in, and a measurement that changes nothing about what runs is theatre; a chain whose
//! second link is advisory is not a chain.
//!
//! # BUGS
//!
//! **Nothing a host test could run proves any of this, and milestone 244 measured how much that
//! costs rather than leaving it as a feeling.** This crate reaches `user_mode_runtime` through the dependency
//! graph, so it does not build for the host at all; `script/lint`, `script/coverage` and
//! `.cargo/mutants.toml` all exclude it, and a gate in `script/lint` derives that set from cargo
//! metadata so the four lists cannot drift apart again. `script/swish-check` is what proves this
//! code, and it is a real gate: it boots both ISAs and types at the prompt, and it is the only
//! thing in the tree that runs a real progenitor.
//!
//! What it cannot do is say which of a mutation run's mutants it would have caught. So the question
//! milestone 244 asked was the one behind that: how much of these 2,632 lines (the count on
//! 2026-09-03, before this paragraph was added) is logic with a right answer a host could check, and
//! is it enough to be worth lifting out. **It is not**, and the split is worth stating because the
//! shape of it is the interesting part. `cargo mutants --list` generates **196** here, five more
//! than the 191 milestone 238's report scored, because the file moved between the two runs; the
//! fractions below are of the 196 counted on the day:
//!
//! | | mutants | what it is |
//! |---|---|---|
//! | [`boot`] | 97 | the build sequence: 34 are *deleting a field* from a `ChildEndowment` literal, 24 are slot-counter arithmetic, 12 flip a rights mask's `\|` |
//! | `spawn_service` | 32 | the `RECV` loop, one syscall per step |
//! | `fill_entropy`, `build_caretaker`, `announce`, `reclaim`, `must`, `must_ok`, `memory_region_split` | 28 | syscalls with the loop around them |
//! | `hex_password`, `sentence` and its `push`, [`boot`]'s own second copy of that `push`, `opt_cap`, `archive_name`, `measured` | 33 | pure: bytes in, bytes out, no capability touched |
//! | top-level `const` arithmetic | 6 | not movable; they are what the rest is written against |
//!
//! **One of those 33 has since been lifted, and it was the one that mattered** (milestone 246). The
//! aggregate above is right and the per-function reading of it is not: a mutant in `hex_password` or
//! `sentence` produces a wrong string, and the mutant in `measured` produced *a system that runs code
//! nobody vouched for*. That one line is now `measured_boot::verdict`, tested on the host, and
//! falsified: `unvouched: true` changed to `false` turns a test red. The rest of the table stands as
//! measured, and the aggregate argument for not lifting the others stands with it.
//!
//! **A fifth of the mutants and a fiftieth of the lines**, and every pure one is a leaf helper of
//! the sequence beside it: `measured` filled a `Lookup` that only [`boot`] destructured, `opt_cap`
//! reads one word out of one `recv_cap`, `archive_name` is `Some(p.name())`. Lifting them
//! buys 33 reachable mutants and costs a crate of fragments, a wider public surface, and a reader
//! holding two files to follow one boot. `redoxfs_server` runs the split this would be modelled on
//! and runs it the other way up: there the sans-IO core is most of the package and the EL0 binary
//! wraps it. Here the sequence *is* the package.
//!
//! **And the largest single group is the one no host test could reach even after a lift.** Thirty-four
//! of [`boot`]'s mutants delete a field from a `ChildEndowment` struct expression: a child built
//! with no `caps`, or no `stack_pages`. That is not logic with a wrong answer, it is a *declaration*
//! of what a component may do, and the only things that can check a declaration are the boot itself
//! or a test comparing it against a separately written expectation. Milestone 244's block proposes
//! the latter as a lane of its own.
//!
//! **A refused `console` or `line_editor` stops in silence.** Those two are what carry the progenitor's
//! output, so a refusal of either has no route to a person: the progenitor traps and the operator sees the
//! kernel's fault line for the progenitor and nothing else, indistinguishable from any other early progenitor
//! failure. Everything refused after them is named on the console. There is no debug-print syscall
//! (the kernel-served `Console` object went away at milestone 8) and driving the UART from here
//! would be a second copy of the console driver, per ISA, inside the process the drivers exist to
//! keep small.
//!
//! **An absent required component still traps with no message**, unchanged from before this
//! existed. That is a build that did not pack it rather than bytes somebody swapped, and it has
//! never had one.
//!
//! **The measurement is of the archive, not of memory over time.** It is checked once, when the
//! program is loaded. Nothing re-measures a running process, and nothing measures the pages the progenitor
//! wrote into a child after `build_child` copied them.
//!
//! The return of pages is **LIFO** (§16, `crates/regions`): a job region that is not at the top of
//! the budget's watermark when it is reclaimed returns nothing, and its run is a hole until this
//! process dies, which it never does. Sequential commands at a prompt are exactly LIFO and recover
//! fully; two jobs alive at once (a pipeline stage that outlives its producer) permanently costs one
//! region. A long enough session of concurrent pipelines still ends at "could not spawn".
//!
//! The loader's scratch window is never unmapped, so the progenitor keeps a **writable mapping of every page
//! it ever laid down for a child**. Reaping a job undoes that (region reclaim revokes every mapping
//! of the pages first, §13), but the boot servers are never reclaimed, so the progenitor can still read and
//! write the console's, the line editor's, the input driver's, the shell's and the sink adapter's
//! memory. Giving the construction budget away does not reach that, and nothing in the ABI unmaps a
//! page.
//!
//! Printing the negative control costs one more of those: the shell's output frame stays mapped here
//! for life, because there is no unmap and `PageFrame::REVOKE` would take it from the shell too.
//!
//! **A boot with a NIC waits for DHCP before it has a prompt** (milestone 590 (provisional)).
//! `net_stack` reports its lease with a blocking send and serves nobody until that send is taken, so
//! the network block receives it before building anything else. On QEMU's user-mode network the
//! answer is immediate; on a network with a virtio NIC and no DHCP server the boot would sit there
//! with no console to say why. Nothing grants a NIC on real hardware today, so the case is
//! unreached rather than closed. Taking the lease later, when the first declaring child is spawned,
//! would unblock the boot and cost the report endpoint a permanent slot, and the table has one left
//! (23 of 24, `kernel::cap::CAPABILITY_TABLE_PEAK_MEASURED`); milestone 590's block records it.
//!
//! **And it keeps a writable view of the NIC's DMA page**, at `NET_DMA_PEEK_VA`, for the rng's
//! reason (reading the physical base the kernel wrote there) and with its cost: there is no unmap.
//! This process can therefore write `net_stack`'s rings for the life of the boot. It could already
//! write every child's memory (the paragraph above), so this is one more page of an exposure it
//! already has, not a new kind.
//!
//! **The progenitor's capability table is finite, and running out of it prints nothing at all.** Every
//! capability held across a `build_child` is one the child's address space, frames and TCB cannot
//! have, and `build_child` answering `Err(())` is a silent halt. Three of the four evenings this
//! file has cost were that: once when the kernel grew two grants, once when a boot component was
//! built one step too early, and once when a block that had never run before started running
//! (milestone 230, below). The order below is load-bearing and the comments say where.
//!
//! **The table was sixteen slots, then seventeen, and is twenty-four now**
//! (`kernel::cap::CAPABILITY_TABLE_SLOTS`, raised by milestone 230 on 2026-09-02). Milestone 31
//! phase 3 measured this process at nine capabilities at rest and fifteen at peak, one slot from
//! the seventeen-slot wall, and that number described the boot as it then was: the login stack
//! below did not exist yet. It does now, and with a virtio-rng attached (DECISIONS §120's
//! 2026-08-26 amendment) entropy comes up, `entropy_client` is `Some`, `have_login_stack` is true,
//! and the credentialer build runs. Measured at that peak, this process holds **twenty-one** slots
//! at once: twelve it never gives back (the root untyped, six endpoints, four frames, and the
//! shell's TCB waiting to be started), the login block's own
//! `prov`/`verify`/`cred_ready`/`prov_page`/`verify_page`/`cred_budget`, and the address space and
//! page `build_child` is laying the credentialer down through.
//!
//! **The login stack was written against twenty-eight slots and nobody knew.** Milestone 49's lane
//! had raised the constant to `28 // TEMP: generous bisection value` while chasing an unrelated
//! flake; a cleanup put it back to 17 on the evidence that the prose said 17 and `script/test` was
//! green at 17, which is true and blind, because no suite in `script/test` boots this program. The
//! boot that does is `script/swish-check`, and it ran in neither `script/test` nor CI, so `main`
//! trapped here for five days behind a green tree. That is milestone 230's whole subject and the
//! reason both of those now run this.
//!
//! **What that means for the next capability added here.** Twenty-four leaves three slots over a
//! measured twenty-one, deliberately rather than generously: every previous raise took the number
//! to exactly what the day's boot needed, and every time the next addition hit the wall in silence.
//! If a change here needs a twenty-second permanent capability, the honest candidates for buying
//! one back are still the readiness endpoint (it could be retyped after the caretaker's build
//! rather than before, if the caretaker learned to take it another way) and the file page (nothing
//! but a second frame per grant retires it, which is `notes/shared-page-audit.md`'s proposed lane).
//!
//! **That twenty-second capability arrived, and so did a twenty-third.** Milestone 111 (a shell
//! that can endow a child with entropy)'s entropy endpoint took the peak to twenty-two, and
//! milestone 590's network-stack endpoint took it to **twenty-three of twenty-four** on a boot with a NIC (`kernel::cap::
//! CAPABILITY_TABLE_PEAK_MEASURED` carries both). One slot is left, so the next permanent
//! capability here should buy one back through the two candidates above before it is added.
//!
//! Name: ratified 2026-08-04 (calef, milestone 96), and it is the ratification that raised
//! milestone 115. Refused `system_builder` (milestone 63 had already refused it, for a reason still
//! true: `builder.rs` called itself "a minimal init: the system builder" when this was recorded,
//! so two programs would claim one phrase; that program was retired on 2026-09-14 by milestone 295
//! and the quotation is kept because a refusal is an account of why a name lost, not a description
//! of the tree) and `system_bootloader` (it claims a position in the
//! boot sequence it does not occupy, and milestone 88 will need the real one). A lane proposed
//! `system_builder` anyway and the maintainer endorsed it, because that refusal lived in one table
//! cell inside one milestone block and neither of them found it. The type this crate exports as
//! `BootEndowment` was ratified the same day, replacing `Grants`.
//!
//! **Asked again on 2026-09-13, and refused.** The program this crate is the logic of became
//! `progenitor` (milestone 266), and the obvious next question was whether the crate should follow.
//! calef: *"init is the issue not initializer."* The refusal is right for a reason that keeps being
//! got wrong. `init` is a truncated **verb**, which is why it lost; `initializer` is an **agent
//! noun**, the thing that initialises, which is exactly what *name things with nouns* asks for.
//! Milestone 266's own house-style list cites `initializer` beside `builder`, `spawner`,
//! `supervisor` and `provisioner` as evidence *for* the convention, and then a section later argued
//! the opposite; that contradiction stood for five days. And the crate is not the process: it is
//! the initialisation, as distinct from the thing that runs it, and it descends nothing, so
//! `progenitor` would fit it worse than it fits the program.

use grant_plan::{Prog, spawnproto};
use line_editor::proto;
// The loader, and the tree's only one since milestone 96. Named here rather than qualified at every
// call, because the point of the crate is that there is one of these.
use supervision_protocol::{
    ChildEndowment, Retention, build_child, retype_obj_from as retype_obj,
    retype_page_frame_from as retype_page_frame, start_child,
};
use user_mode_runtime::{call, cap_delete, invoke, is_granted, recv, recv_cap, send};

/// **The capabilities the kernel granted the progenitor, by slot.** Data the boot entry states
/// rather than code this crate repeats, so a board that grants a different layout says so in one
/// place instead of in this crate's logic.
///
/// The order comes from `kernel::user::boot_progenitor`, which has been one function on all three
/// architectures since milestone 166, and that is why `components/src/progenitor.rs` states it
/// once with no `cfg`. Until then aarch64's loader was shared with milestone 19d's test roles and
/// granted a report endpoint and a test interrupt this system has no use for, which displaced
/// every slot after the first; see [`for_test_roles`](BootEndowment::for_test_roles).
pub struct BootEndowment {
    /// The construction budget, held `WRITE | GRANT`: everything this system is made of.
    pub untyped: u64,
    /// The UART's registers, a device capability to map into the console and input drivers.
    pub uart_dev: u64,
    /// The UART receive interrupt, an `Irq` capability to delegate into the input driver.
    pub uart_irq: u64,
    /// **The wall clock** (milestone 51's wiring): a `PageFrame` capability with `READ` and nothing
    /// else, granted ahead of the filesystem pair so its slot is the same on every boot, whether or
    /// not a disk was attached, and granted **unconditionally**: a boot with no clock service hands
    /// us a zeroed page, which reads as `clock_protocol::state::UNKNOWN` and is the honest answer for a
    /// machine that does not know the time. The progenitor hands it on only to a child whose manifest declares
    /// a clock, and hands on `READ`, so nothing spawned from this prompt can set the time
    /// (DECISIONS §43).
    ///
    /// **The shell holds a narrowed view of this same frame** (milestone 86), mapped at
    /// `SH_CLOCK_VA`, which is what `time <command>` measures with. There is deliberately no
    /// second field for it: the shell's clock is not a separate kernel grant but this one handed on,
    /// so a field would ask each board to state the same slot number twice with nothing checking
    /// that the two agree. `READ` and no `GRANT` there too, so the shell can read the wall clock and
    /// can hand one to nothing it spawns.
    pub clock_page: u64,
    /// **The inert-configuration page** (milestone 47's environment-variable fork, DECISIONS §111):
    /// a `PageFrame` capability with `READ` and nothing else, granted ahead of the filesystem pair
    /// for [`clock_page`](BootEndowment::clock_page)'s own reason (so its slot is the same whether
    /// or not a disk was attached), and granted **unconditionally** for the same reason too: this
    /// boot's fixed defaults (`TZ=UTC`, `LANG=C`, `TERM=dumb`) are assembled once, before the progenitor
    /// exists, so every boot hands the progenitor a real, validated page rather than making the slot's
    /// presence depend on some other component having started (`kernel::user::boot_config_page`,
    /// the same shape as `boot_clock_page`, minus the service: nothing here runs, so there is no
    /// readiness handshake to wait on, only a page to assemble and write once).
    ///
    /// The progenitor hands it on only to a child whose manifest declares [`grant_plan::Manifest::config`],
    /// and hands on `READ`, so nothing spawned from this prompt can change what a shell hands its
    /// own children.
    pub config_page: u64,
    /// **The filesystem, when this boot has one** (milestone 50). The kernel wires the block server
    /// and the FS server before it starts us and grants the service endpoint plus the page its
    /// clients share with it. The rights that endpoint carries arrive in `fs_rights`, and **0 means
    /// this boot attached no RedoxFS disk**, in which case these two slots hold nothing at all.
    pub fs_ep: u64,
    /// The page the file service's clients share with it; see [`fs_ep`](BootEndowment::fs_ep).
    pub fs_page: u64,
    /// **A virtio-rng device, when this boot has one** (DECISIONS §120's 2026-08-26 amendment:
    /// "grant the QEMU-only virtio-rng stopgap"). The confined transport; `WRITE | GRANT`, so this
    /// process can delegate it to an entropy service it builds. **Absent** on real hardware
    /// (milestone 55's actual target has none) or a run with `NIFE_RNG` unset, in which case this
    /// slot holds nothing at all, the same "0/empty means absent" shape [`fs_ep`](BootEndowment::fs_ep)
    /// already carries; [`boot`] probes for it (`invoke`'s own `NoSuchSlot` on an ungranted slot)
    /// rather than being told, because there is no fourth `START` argument word left to tell it
    /// with (`role`, `initrd_len`, `fs_rights` already spend all three).
    pub virtio_rng: u64,
    /// The device's completion interrupt; see [`virtio_rng`](BootEndowment::virtio_rng). `READ |
    /// GRANT`, routed by the kernel but left unenabled on riscv64 (`kernel::user::VirtioBootGrant`'s
    /// own doc: the board's hart-lottery hazard means only the kernel, which knows the true boot
    /// hart, may enable it, and it already did before granting this).
    pub virtio_rng_irq: u64,
    /// **The DMA page the kernel wrote `direct_memory_access_phys` into**, at its own last eight bytes; see
    /// [`virtio_rng`](BootEndowment::virtio_rng). `READ | WRITE | GRANT`: this process maps it to
    /// read that value back out (entropy needs its own DMA region's physical base as a plain
    /// value; no capability exposes one), then delegates the same frame to the entropy service it
    /// builds, unread by anything else in between.
    pub virtio_rng_dma: u64,
    /// **`display_terminal`'s own served endpoint, when this boot has a graphical terminal stack**
    /// (milestone 177, option A). The kernel builds the virtio-gpu driver and `display_terminal`
    /// itself, before this process exists (`kernel::user::boot_graphical_terminal`, mirroring
    /// [`fs_ep`](BootEndowment::fs_ep)'s own shape: a virtio-gpu device alone needs eleven
    /// capability-table slots, which does not fit this process's own budget). `WRITE | GRANT`, so
    /// this process can hand it to `line_editor` as the endpoint it prints through in place of the
    /// console. **Absent** (holds nothing) on a boot with no GPU, no keyboard, or no
    /// `gpu_driver`/`display_terminal`/`keyboard_driver` program in the archive, the same "0/empty means absent"
    /// shape [`fs_ep`](BootEndowment::fs_ep) already carries; [`boot`] probes for it the same way.
    ///
    /// **Granted without [`kbd_ep`](BootEndowment::kbd_ep), it is a screen beside the serial
    /// console** (the shell on the firmware screen, milestone 198's rung 1b): the kernel put
    /// `display_terminal` on the firmware's framebuffer through `framebuffer_driver`, and the UART
    /// is still the console and the keystroke source. [`boot`] then hands this to the console
    /// server, which writes every byte to both, rather than to `line_editor`.
    pub disp_term_ep: u64,
    /// The physical page shared with `display_terminal`, written before an `OP_WRITE` on
    /// [`disp_term_ep`](BootEndowment::disp_term_ep); see that field's own doc for when this is
    /// absent. `READ | WRITE | GRANT`.
    pub disp_term_page: u64,
    /// **The endpoint the keyboard driver already holds `WRITE` (`CALL`) on**, when this boot has
    /// one (milestone 177, option A). The kernel spawns the keyboard driver kernel-side too, wired
    /// to this endpoint at its own spawn time, for a reason distinct from the GPU's: its target is
    /// `line_editor`'s own served endpoint, which does not exist until this process builds it, and
    /// a driver this process spawns can only be wired to capabilities it already holds. Granting
    /// `READ | WRITE | GRANT` here lets this process delegate `READ` to `line_editor` (as its own
    /// terminal endpoint, in place of a self-created one) and `WRITE` to `swish`, exactly the two
    /// views the plain-console boot already carves out of a self-created endpoint of the same
    /// shape. Absent in the same sense as
    /// [`disp_term_ep`](BootEndowment::disp_term_ep), and additionally on a boot whose terminal is
    /// a screen beside the serial console (see that field): **this slot is what tells a graphical
    /// boot from that one.**
    pub kbd_ep: u64,
    /// **A virtio-net device, when this boot has one** (milestone 590 (provisional), the booted
    /// system starts its network stack; promoted from the proposal
    /// `the-booted-system-has-no-network`). The confined transport, `WRITE | GRANT`, the
    /// [`virtio_rng`](BootEndowment::virtio_rng) trio's shape exactly: this process delegates it
    /// into the `net_stack` it builds and keeps nothing. **Absent** on every real board today and
    /// on any QEMU run with `NIFE_NET` unset, and [`boot`] probes for it the way it probes for the
    /// rng.
    pub virtio_net: u64,
    /// The network card's interrupt, `READ | GRANT`; see [`virtio_net`](BootEndowment::virtio_net)
    /// and, for why it arrives unenabled on riscv64, [`virtio_rng_irq`](BootEndowment::virtio_rng_irq).
    pub virtio_net_irq: u64,
    /// The network card's DMA page, with its own physical base written at its last eight bytes;
    /// [`virtio_rng_dma`](BootEndowment::virtio_rng_dma)'s shape and reason. `READ | WRITE | GRANT`.
    pub virtio_net_dma: u64,
    /// **An entropy service the kernel already built on the CPU's own seed instruction**
    /// (`RDSEED` on `x86_64`, `RNDRRS` on aarch64), when this boot has no virtio-rng (milestone 595
    /// (provisional), the `x86_64` half; promoted from the proposal
    /// `the-x86-64-progenitor-serves-entropy-from-rdseed`). The request endpoint, `WRITE | GRANT`,
    /// which is what [`boot`] would otherwise have made itself from the
    /// [`virtio_rng`](BootEndowment::virtio_rng) trio, already proven: the kernel waited for the
    /// service's readiness report and grants nothing on a refusal
    /// (`kernel::user::boot_instruction_entropy`, which also says why the kernel builds this one).
    ///
    /// **Never both.** The kernel grants this only when it granted no virtio-rng, so a boot has
    /// one entropy source or none. **Absent** on every riscv64 boot (neither instruction exists
    /// there), on a CPU without the instruction, and on any QEMU run with `NIFE_RNG` set; [`boot`]
    /// probes it the way it probes [`fs_ep`](BootEndowment::fs_ep).
    ///
    /// Name: provisional, lane `milestone/595-x86-std`, 2026-09-26. `fs_ep`'s shape, one service over.
    pub entropy_ep: u64,
    /// **Capabilities the kernel granted that the interactive system never uses**, deleted with the
    /// device authority once the drivers exist.
    ///
    /// Empty on riscv64. On aarch64 it is the kernel's report endpoint and the milestone-19d.2b test
    /// SGI, both of which exist because that boot path is shared with the test roles: nothing
    /// receives on the report here, and no interactive component waits on that interrupt. A progenitor
    /// that kept them would be keeping delegable authority for no reason, which is the same kind of
    /// thing the construction budget is.
    pub for_test_roles: &'static [u64],
}

/// **A second, disjoint directory capability for the shell** (milestone 154's "wiring a second
/// grant into the real boot", design/roadmap/154-multi-directory-namespace.md), passed to [`boot`]
/// alongside [`BootEndowment`] rather than folded into it: this is not a kernel grant like every
/// field above, it is something [`boot`] itself constructs (a second `fs_subtree_caretaker`,
/// narrowing the same file service [`BootEndowment::fs_ep`] already names) out of capabilities the
/// kernel already granted.
///
/// **`None` at every real entry point today.** The mechanism here is real and reachable through
/// the one function the real progenitor calls (`components/src/progenitor.rs`), not a synthetic
/// kernel-side test harness. What it does
/// not decide is *what* the second subtree should be: [DECISIONS
/// §126](../../../design/decisions/126-two-directory-cwd.md) named that a boot-time policy
/// question reserved for calef, so no shipped boot enables it. A second, separate gap: nothing
/// yet tells the shell process *which* label and cspace slot this landed at (the `START` ABI's
/// three words are already spoken for by the role, the argument, and the clock slot), so a shell
/// built with one of these today would hold a capability its own `Nav` has no way to learn about.
/// Provisional shape.
#[derive(Clone, Copy)]
pub struct SecondDirGrant {
    /// One component under the image root, the same shape a `DirGrant`'s `name` already takes.
    pub name: &'static str,
    /// The `filesystem_protocol::dir` rights the caretaker asks for on its descent.
    pub rights: u64,
}

/// Where the kernel maps the initrd archive, read-only. Must match `kernel::user::INITRD_VA`.
const INITRD_VA: u64 = 0x2000_0000;

/// Stack pages every child the progenitor builds gets, mapped down from `supervision_protocol::CHILD_STACK_VA`.
///
/// **Twelve since DECISIONS §67**, and every step of that number was measured rather than chosen.
/// Four overflowed at the first `ls > out.txt`; eight held until `2>` put a **second** `FileOut` on
/// `run_pipeline`'s frame, each carrying a 256-byte staging buffer by value, and the scripted wiring
/// faulted twenty-four bytes below its lowest page. Four extra rather than one, because every
/// previous instance bought exactly enough and the next change found the wall again. The cost is
/// 48 KiB of address space per child, which is nothing next to a page table.
///
/// `kernel::user::pipeline_service`'s `SHELL_EXTRA_STACK` must stay level with this: a test wiring
/// with less headroom than the boot wiring finds faults the boot does not have
/// (notes/pipes/the-boot.md).
/// The kernel cannot depend on this crate (it would drag `user_mode_runtime`'s EL0 syscall stubs in), so that
/// one is still a number in two places, and this is the one it follows.
pub const CHILD_STACK_PAGES: u64 = 12;

/// Where a child that declares a clock maps it, read-only. Must match `components/src/date.rs`'s
/// `CLOCK_VA` and `kernel/src/user/clock_service.rs`.
const CHILD_CLOCK_VA: u64 = 0x00c0_0000;

/// Where a child that declares the inert-configuration page maps it, read-only (DECISIONS §111).
/// Must match `components/src/printenv.rs`'s `CONFIG_VA`. A different address from `std_service.rs`'s
/// `CONFIG_PAGE_STD`: that std program is spawned by a different wiring entirely (the `-Zbuild-std`
/// farm's own harness), in its own address space, so there is no collision to avoid, only two
/// numbers that happen not to need to agree.
const CHILD_CONFIG_VA: u64 = 0x00e0_0000;

/// Where a supervised (interruptible) child maps its shared job frame (DECISIONS §24). Below the
/// ELF load address (`0x40_0000`) and the stack; must match `interrupt_heeder.rs` and
/// `interrupt_ignorer.rs`'s `JOB_PAGE_FRAME_VA`.
const CHILD_JOB_PAGE_FRAME_VA: u64 = 0x0030_0000;

/// Pages of untyped split off our own budget and handed the shell (milestone 31), so the shell can
/// in turn endow the programs it spawns (`run --mem N`) out of a budget that is genuinely *its own*.
/// The shell shrinks this by N pages per grant; the pages a spawned child pins are not reclaimed in
/// phase 1, so this is a session budget, not a renewable one. Must match swish.rs's
/// `SH_BUDGET_PAGES`.
const SH_BUDGET_PAGES: u64 = 128;

/// **What the progenitor keeps for itself after the boot servers are up** (milestone 22, the interactive
/// increment). It pays for one thing: the page tables reaching the loader's scratch window, which
/// are the progenitor's own mappings and must never come out of a child's region (tearing that region down
/// would free the progenitor's tables under a window it never unmaps). One L3 covers 512 scratch pages and a
/// job maps at most a couple of dozen, so this is thousands of commands' worth.
pub const INIT_OWN_PAGES: u64 = 128;

/// **One job's region**: everything a spawned program is made of, so a single reclaim frees all of
/// it. The biggest program the prompt can spawn is `date` at seven pages, plus
/// [`CHILD_STACK_PAGES`], a TCB, an address-space root, the intermediate tables for the four windows
/// a child touches, and the §13 mapping records. Forty is that with room to spare, and it is spent
/// per *live* job rather than per job ever run.
const JOB_REGION_PAGES: u64 = 40;

/// **One directory-granted job's region**: the program *and* the `fs_subtree_caretaker` that carries
/// its grant, plus the two endpoints between them, all out of one carve.
///
/// One region rather than two is DECISIONS §92 read through §40's mechanism. A caretaker's serve loop
/// never returns, so it never dies of its own accord; built in a region of its own, that region never
/// comes home and §16's LIFO rule then pins the region above it too, and six `rm`s would end the
/// prompt. Built out of the region it serves, it is inside the client's subtree, and the one reclaim
/// `job_undertaker` already performs ends both. **The two endpoints are retyped from this region too,
/// and that is load-bearing rather than tidy**: `sched::reap_region_objects` sweeps a region's
/// endpoints before it looks at its threads, and that sweep is what wakes a caretaker parked in
/// `RECV` so it can be collected. An endpoint carved from the progenitor's own budget would leave it blocked on
/// something the teardown never touches, and a blocked thread never reaches the `schedule()` that
/// spends §16's kill.
///
/// **Ninety-six rather than eighty**, and the extra is not slack for its own sake: the caretaker is
/// a second address space with its own tables and its own stack, and the failure mode of getting it
/// wrong is `build_child` answering `Err(())` mid-boot-command, which reads at the prompt as "could
/// not spawn" with no way to tell a small region from an empty pool.
const DIR_JOB_REGION_PAGES: u64 = 96;

/// The stack a `fs_subtree_caretaker` gets, beyond the one page `build_child` maps for it.
///
/// Four rather than [`CHILD_STACK_PAGES`]'s twelve, and it is a measurement of the program rather
/// than a guess: it has no allocator, no recursion, and one frame that matters, a
/// `[Option<u64>; 16]` handle table plus a `grant::MAX_NAME` name buffer. Twelve pages per caretaker
/// would be 32 KiB of a region this crate is already sizing carefully.
const CARETAKER_STACK_PAGES: u64 = 4;

/// Where a `fs_subtree_caretaker` and the program it serves both map the FS contract's shared page.
/// Must match `components/src/fs_subtree_caretaker.rs`'s `PAGE_VA` and `components/src/rm.rs`'s.
///
/// One address for both because they are two ends of one contract and neither is the other's parent:
/// a request travels caretaker-to-server and program-to-caretaker through the same frame, so a
/// second VA would only be a second name for the same page.
const FS_CLIENT_PAGE_VA: u64 = 0x0060_0000;

/// **One shell-boot second-directory caretaker's region** (milestone 154's "wiring a second
/// grant into the real boot"). Sized for one caretaker alone, the way [`CARETAKER_STACK_PAGES`]
/// already is: unlike [`DIR_JOB_REGION_PAGES`], nothing else is built out of this region, because
/// the confined program here is the shell itself, already built directly out of `ut` rather than
/// out of a region of its own. Conservative rather than tight (the same headroom
/// [`JOB_REGION_PAGES`] carries for one child), since this is spent once per boot rather than
/// once per command.
const SECOND_DIR_CARETAKER_PAGES: u64 = JOB_REGION_PAGES;

/// **The job pool.** Six live jobs at once, which is far more than a prompt has ever needed and is
/// deliberately small: the whole claim of this increment is that a *bounded* budget is enough once
/// the regions come back, so a budget nobody could exhaust would prove nothing. `script/swish-check`
/// runs thirteen jobs through it, so widening this silently retires that gate.
///
/// **Plus one `std` program's region** (milestone 595 (provisional)), so a `std` job has room of its
/// own rather than needing every native job before it reclaimed first: it is nearly ten native
/// regions' worth ([`grant_plan::STD_REGION_PAGES`]), and a pool of 240 would fit it only when
/// empty. The ratchet above still holds at 624 pages: `script/swish-check` runs more than twenty
/// jobs, which is well past what the pool could hold without the regions coming back.
pub const JOBS_BUDGET_PAGES: u64 = JOB_REGION_PAGES * 6 + grant_plan::STD_REGION_PAGES;

/// Where the progenitor maps the shell's output frame in **its own** address space, to print the one line it
/// ever prints (the dropped-authority negative control). Well clear of the progenitor's segments, its stack,
/// and the loader's scratch window at `0x1000_0000`.
const INIT_OUT_VA: u64 = 0x0f00_0000;

/// Where the progenitor briefly maps the virtio-rng DMA page, in **its own** address space, to read
/// [`VIRTIO_DMA_PHYS_OFFSET`] back out before handing the same frame on to entropy. Distinct from
/// [`INIT_OUT_VA`] and never unmapped (this file's own BUGS: there is no unmap in the ABI), the
/// same permanent-scratch cost that address already carries.
const RNG_DMA_PEEK_VA: u64 = 0x0f10_0000;

/// The DMA region's own physical base, written inside the page itself at its last eight bytes
/// (`kernel::user::VIRTIO_DMA_PHYS_OFFSET`; the two constants must agree, and the kernel-side
/// one carries the reasoning for exactly this offset). Named separately here because reading it
/// happens in this crate and writing it happens in the kernel; there is no crate the two could
/// share it through (rule 7's own carve-out: this is a kernel/progenitor boot convention, one program's
/// bytes handed to another it spawned, not a contract between two peer user programs).
const VIRTIO_DMA_PHYS_OFFSET: u64 = 4096 - 8;

/// Where entropy maps its own DMA page. Must match `components/src/entropy.rs`'s `DMA_VA`.
const RNG_DMA_VA: u64 = 0x0000_0000_0090_0000;

/// `entropy.rs`'s own spawn-argument convention (`components/src/entropy.rs`'s `MODE_VIRTIO`): the
/// kernel's (and now this crate's) shared understanding with the one program it spawns, not a wire
/// contract between two user programs, so rule 7 does not apply the way it does to `RNG_DMA_VA`.
const RNG_MODE_VIRTIO: u64 = 0;

// -------------------------------------------------------------------------------------------
// The network stack (milestone 590 (provisional), the booted system starts its network stack).
// -------------------------------------------------------------------------------------------

/// Where this process briefly maps the network card's DMA page to read [`VIRTIO_DMA_PHYS_OFFSET`]
/// back out, [`RNG_DMA_PEEK_VA`]'s twin and with its cost: never unmapped, because there is no
/// unmap in the ABI, so this process keeps a writable view of the NIC's rings for the life of the
/// boot. It already keeps one of every page it laid down for a child (this module's BUGS), so the
/// exposure is one more page of a kind it already has, not a new kind.
const NET_DMA_PEEK_VA: u64 = 0x0f30_0000;

/// Where `net_stack` maps its DMA page. Must match `components/src/net_transport.rs`'s `DMA_VA`, the
/// same kernel-and-spawner convention [`RNG_DMA_VA`] is.
const NET_DMA_VA: u64 = 0x0000_0000_0090_0000;

/// `net_stack`'s heap budget, in pages: matches
/// `kernel::user::virtio_service::NET_SERVER_BUDGET_PAGES`,
/// which carries the measurement (the heap is capped at 96 pages by `net_stack`'s own `HEAP_MAX`,
/// and the rest is page tables for the heap and for clients' shared frames). Stated on this side for
/// [`CRED_BUDGET_PAGES`]'s reason: a spawner states the budget it hands, and there is no crate a
/// kernel constant and a user-mode spawner could share it through that would not be one number
/// with two readers who both need the comment.
const NET_STACK_BUDGET_PAGES: u64 = 128;

/// `net_stack`'s stack, in pages: the kernel harness maps eight extra pages under the one every
/// process gets (`kernel::user::virtio_service::NET_SERVER_STACK_PAGES`, "smoltcp builds packets
/// on the stack"), and nine is those together, since `build_child` counts the whole stack.
const NET_STACK_STACK_PAGES: u64 = 9;

/// `net_stack`'s own spawn-argument convention: entry role 0 is the server
/// (`components/src/net_stack.rs`'s
/// `_start`). [`RNG_MODE_VIRTIO`]'s reasoning.
const NET_STACK_ROLE_SERVER: u64 = 0;

/// `line_editor.rs`'s own spawn-argument convention (`components/src/line_editor.rs`'s `MODE_CONSOLE`):
/// [`RNG_MODE_VIRTIO`]'s own reasoning, one program over. The pre-milestone-177 wiring: prints
/// through the console's bespoke two-endpoint protocol.
const LINE_EDITOR_MODE_CONSOLE: u64 = 0;
/// `line_editor.rs`'s own `MODE_DISPLAY`: milestone 177's wiring, prints through
/// `display_terminal`'s `OP_WRITE`/one-`CALL` contract instead.
const LINE_EDITOR_MODE_DISPLAY: u64 = 1;

// The VAs each program hardcodes; they must match console.rs / input.rs / line_editor.rs / swish.rs.
const CON_SHARED_VA: u64 = 0x0060_0000; // console reads text here; line_editor writes it
/// Where the console maps the page `display_terminal` reads an `OP_WRITE`'s bytes from, when a
/// screen was wired beside the UART. Must match `components/src/console.rs`'s `SCREEN_OUT_VA`.
const CON_SCREEN_OUT_VA: u64 = 0x0068_0000;
/// `console.rs`'s own `MODE_SCREEN`: [`LINE_EDITOR_MODE_CONSOLE`]'s reasoning, one program over.
/// `0`, what every other boot passes, is the UART alone.
const CONSOLE_MODE_SCREEN: u64 = 1;
/// A child's capability grants and page mappings, the two slices a `ChildEndowment` takes as `caps`
/// and `maps`. Named so the `x86_64`-vs-others split of the console and input endowments (a port
/// capability held rather than a page mapped, milestone 299) is a one-line `let` per branch without
/// clippy's type-complexity lint firing on the bare tuple. `'a` ties the slices to the `let`'s block.
type EndowmentSlices<'a> = (&'a [(u64, u64)], &'a [(u64, u64, u64)]);

// The two UART mappings are aarch64/riscv64 only: x86 holds COM1 as a port range, which has no page
// to map (milestone 299), so on that architecture these VAs name nothing and the console/input
// drivers hold the port capability instead.
#[cfg_attr(target_arch = "x86_64", allow(dead_code))]
const CON_UART_VA: u64 = 0x0070_0000; // console's UART mapping
const TERM_OUT_VA: u64 = 0x0080_0000; // line_editor reads the shell's text/prompts here
const TERM_IN_VA: u64 = 0x0090_0000; // line_editor delivers completed lines here
#[cfg_attr(target_arch = "x86_64", allow(dead_code))]
const IN_UART_VA: u64 = 0x00a0_0000; // input driver's UART mapping
const SH_OUT_VA: u64 = 0x00c0_0000; // the shell's view of the TERM_OUT frame (swish.rs OUT_VA)
const LINE_VA: u64 = 0x00b0_0000; // the shell's view of the TERM_IN frame
const SH_FS_VA: u64 = 0x0060_0000; // the shell's half of the FS contract (swish.rs FS_VA)

/// Where the **shell** maps its own read-only clock (milestone 86). Must match swish.rs's
/// `SH_CLOCK_VA`. A different address from [`CHILD_CLOCK_VA`], because that is where a *child* maps
/// its clock and the shell already maps the terminal's output frame there; two address spaces may
/// agree on an address, one may not.
const SH_CLOCK_VA: u64 = 0x00d0_0000;

// -------------------------------------------------------------------------------------------
// The login stack of milestone 49 (users and attribution): credentialer, identity_provisioner,
// login, login_audit_receiver.
// -------------------------------------------------------------------------------------------

/// Where `credentialer` maps its own provision page. Must match `components/src/credentialer.rs`'s own
/// `PROV_VA`.
const CRED_SVC_PROV_VA: u64 = 0x0000_0000_00e0_0000;
/// Where `credentialer` maps its own verify page. Must match the same file's `VERIFY_VA`.
const CRED_SVC_VERIFY_VA: u64 = 0x0000_0000_00e1_0000;
/// Where `login` maps its relay of the verify page (the same physical frame as
/// [`CRED_SVC_VERIFY_VA`], mapped into a different address space). Must match `components/src/login.rs`'s
/// own `CRED_VA`.
const LOGIN_CRED_VA: u64 = 0x0000_0000_00e3_0000;
/// Where `identity_provisioner` maps the identity/secret this boot stages for it. Must match
/// `components/src/identity_provisioner.rs`'s own `REQ_VA`.
const IDP_REQ_VA: u64 = 0x0000_0000_00e4_0000;
/// Where `identity_provisioner` maps `credentialer`'s provision page (the same physical frame as
/// [`CRED_SVC_PROV_VA`]). Must match the same file's own `PROV_VA`.
const IDP_PROV_VA: u64 = 0x0000_0000_00e0_0000;
/// Where `identity_provisioner` maps the file service's shared page. Must match the same file's own
/// `FS_VA`.
const IDP_FS_VA: u64 = 0x0000_0000_00e5_0000;
/// Where this process briefly maps the page it stages `identity_provisioner`'s request into, in its
/// own address space, before delegating the same physical frame on. Distinct from every VA above
/// (those are addresses inside a *child's* address space); in the same scratch family as
/// [`RNG_DMA_PEEK_VA`] and [`INIT_OUT_VA`].
const PROVISION_SCRATCH_VA: u64 = 0x0f20_0000;

/// `credentialer`'s own construction budget, in pages: matches
/// `kernel::user::credential_service::CRED_BUDGET_PAGES` (6 MiB, sized from `credentialer::Cost::DEFAULT`'s
/// own scratch requirement, not guessed).
const CRED_BUDGET_PAGES: u64 = 1536;
/// Extra stack pages `credentialer` needs, beyond the one page `build_child` maps: matches
/// `kernel::user::credential_service::CRED_STACK_PAGES`, itself a measurement (Argon2id's inner
/// loop overflows one page) rather than a guess.
const CRED_STACK_PAGES: u64 = 16;
/// `login`'s own construction budget under a real boot, in pages. Sized for a handful of real
/// sessions across the boot's whole life (`OWN_UT_PAGES` 128 + `CHANNEL_UT_PAGES` 32, both
/// `components/src/login.rs`'s own one-time costs, plus `CARETAKER_REGION_PAGES` 64 + `CLIENT_BUDGET_PAGES`
/// 64 per session that never logs out), not the much larger figure
/// `kernel::user::login_tests::CONSTRUCTION_PAGES` carries for a whole guest-test suite's worth of
/// logins against one shared instance.
const LOGIN_CONSTRUCTION_PAGES: u64 = 768;
/// Extra stack pages `login` needs, beyond the one page `build_child` maps: matches
/// `kernel::user::login_service::LOGIN_STACK_PAGES`, "sized against `credentialer.rs`'s own lesson"
/// (that file's own comment).
const LOGIN_STACK_PAGES: u64 = 16;

/// `credentialer.rs`'s own readiness sentinel, duplicated here the same way its `PROV_VA`/`VERIFY_VA`
/// already are: a binary crate cannot be imported, so every wiring site that needs to recognise this
/// word states it again (`kernel::user::credential_service`'s own `RPT_READY` is the same
/// duplication one level over). Must match `components/src/credentialer.rs`'s own `RPT_READY`.
const CRED_RPT_READY: u64 = 0x_c2ed_0000_0000_0001;
/// `identity_provisioner.rs`'s own success report code, duplicated for the same reason
/// (`kernel::user::identity_provisioner_service`'s own `RPT_OK` is the identical duplication).
/// Must match `components/src/identity_provisioner.rs`'s own `RPT_OK`.
const IDP_RPT_OK: u64 = 1;

/// **The demo identity this boot provisions**, once per boot, with a freshly generated password
/// (see `have_login_stack`'s own block for the reasoning): a role name rather than a specific
/// family member's, deliberately, because the whole point of a boot-generated credential is that
/// nothing here has to decide *whose* account this is (`design/roadmap/49-users-and-attribution.md`'s
/// own BUGS names this as the reason the generated shape was recommended over a baked-in one).
const DEMO_IDENTITY: &[u8] = b"operator";
/// How many raw bytes of entropy the generated password draws. 12 bytes (96 bits) hex-encoded to
/// [`PASSWORD_HEX_LEN`] printable characters: comfortably past what an online guesser could work
/// through before the boot that generated it is long gone, and short enough to type at a keyboard.
const PASSWORD_BYTES: usize = 12;
/// [`PASSWORD_BYTES`] hex-encoded, two characters per byte.
const PASSWORD_HEX_LEN: usize = PASSWORD_BYTES * 2;

/// Draw `out.len()` bytes from the entropy service through `request`
/// ([`entropy_protocol::MAX_BYTES`] at a time), the identical loop `components/src/credentialer.rs`'s own
/// `fill` performs as a *client* of that same service (this process is, briefly, one too: it draws
/// the generated password's own raw bytes before `credentialer` ever exists). `false` when the
/// service could not supply them, which the caller treats as fatal to generating a password at all
/// -- the same "never invent a weak one" posture `credentialer.rs`'s own `fill` documents for a
/// salt.
fn fill_entropy(request: u64, out: &mut [u8]) -> bool {
    let mut done = 0;
    while done < out.len() {
        let want = (out.len() - done).min(entropy_protocol::MAX_BYTES as usize);
        // SAFETY: `invoke` traps to the kernel, which validates the capability and the method
        // before acting.
        let (r0, r1) = call(
            request,
            entropy_protocol::req(entropy_protocol::GET, want as u64),
            0,
        );
        let Some(n) = entropy_protocol::delivered(r0) else {
            return false;
        };
        if n < want {
            return false;
        }
        done += entropy_protocol::take(n, r1, &mut out[done..]);
    }
    true
}

/// Hex-encode `bytes` into `out` (`out.len() == 2 * bytes.len()`), lowercase: the shape a cloud
/// image's generated first-boot password already takes, printable and typeable at a keyboard
/// without a decoder.
fn hex_password(bytes: &[u8], out: &mut [u8]) {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    for (i, &b) in bytes.iter().enumerate() {
        out[i * 2] = DIGITS[(b >> 4) as usize];
        out[i * 2 + 1] = DIGITS[(b & 0xf) as usize];
    }
}

/// **Build the interactive system and become its spawn service.** Never returns: the last thing it
/// does is park in `RECV` on the shell's spawn channel for the life of the boot.
///
/// `initrd_len` is the archive length the kernel passed at entry; `fs_rights` is the `filesystem_protocol::dir`
/// rights the file-service endpoint carries, and 0 means this boot attached no disk. `second_dir`
/// is milestone 154's addition: `Some` hands the shell a second, disjoint directory capability
/// (see [`SecondDirGrant`] for what this does and does not decide); every real entry point passes
/// `None` today.
pub fn boot(
    g: &BootEndowment,
    initrd_len: u64,
    fs_rights: u64,
    second_dir: Option<SecondDirGrant>,
) -> ! {
    // The archive the kernel mapped read-only; its length arrived at entry.
    // SAFETY: the kernel mapped `initrd_len` bytes of reserved RAM, read-only, at INITRD_VA. It is
    // reserved memory that outlives every process, so the borrow is honest for the whole boot.
    let archive =
        unsafe { core::slice::from_raw_parts(INITRD_VA as *const u8, initrd_len as usize) };
    let Ok(fs) = nifefs::Fs::parse(archive) else {
        fail()
    };

    // **The table the progenitor measures what it loads against** (milestone 104). The kernel vouched for this
    // entry before it started us, exactly as it vouched for our own bytes
    // (`kernel::trust::require_program_measurements`), so the table is worth what this process is
    // worth and the chain extends by induction rather than by widening the kernel.
    //
    // Bytes that are not UTF-8 become the **empty** table rather than a fault, and an empty table
    // vouches for nothing: every lookup below answers `Unmeasured`, the console is refused, and the
    // boot stops. That is the same direction to be wrong in as the kernel's empty trust root, and it
    // is the failure mode a measured boot must never get backwards.
    let table = fs
        .read(measured_boot::PROGRAM_MEASUREMENTS)
        .and_then(|b| core::str::from_utf8(b).ok())
        .unwrap_or("");

    // **The image's package catalogue** (milestone 198 (a package manager) rung 3a's installer):
    // what `package install` checks a package's digest against. Measured like a program, because
    // it is an archive entry above the table, and read as the **empty** catalogue when the table
    // refuses it or the archive has none, which vouches for no package: the same direction to be
    // wrong in as the table's own rule above.
    let catalogue = fs
        .read(package_archive::CATALOGUE)
        .filter(|bytes| {
            measured_boot::verify_in_manifest(table, package_archive::CATALOGUE, bytes).is_ok()
        })
        .and_then(|b| core::str::from_utf8(b).ok())
        .unwrap_or("");

    let con_elf = measured(&fs, table, "console");
    let in_elf = measured(&fs, table, "input");
    let td_elf = measured(&fs, table, "line_editor");
    let sh_elf = measured(&fs, table, "swish");
    // **The terminal's sink adapter** (milestone 50's last remainder). Optional on purpose: an
    // initrd built without it still boots, and a program that declares a second stream then finds
    // an empty slot and says what it has to say in-band. A missing component should cost a feature,
    // not a prompt. An adapter the table refuses costs exactly the same feature, which is the whole
    // policy in one line: the progenitor treats what it cannot vouch for as what is not there.
    let sink_elf = measured(&fs, table, "terminal_sink_caretaker").elf;
    // **The entropy service** (DECISIONS §120's 2026-08-26 amendment), optional in exactly the
    // adapter's own sense: a boot with no `entropy` program in its initrd, or a table that refuses
    // it, simply never builds the service, and [`g.virtio_rng`](BootEndowment::virtio_rng)'s own
    // probe finds nothing to build it *from* either way on most boots (real hardware, or a run
    // with `NIFE_RNG` unset). Neither case is a broken boot, so neither belongs with the required
    // three below.
    let ent_elf = measured(&fs, table, "entropy").elf;
    // **The network stack** (milestone 590 (provisional)), optional in the entropy service's sense
    // and for its reasons: a boot with no NIC, or an archive whose table refuses `net_stack`, comes
    // up without a network rather than without a prompt.
    let net_elf = measured(&fs, table, "net_stack").elf;
    // **Milestone 49's login stack** (`credentialer`, `identity_provisioner`, `login`,
    // `login_audit_receiver`): optional in exactly the entropy service's own sense, and gated on
    // it too -- there is no salt, no password and no credential store without real entropy, so a
    // boot with no entropy service has no login path either, the same way it has no login path
    // with no filesystem (`have_login_stack`, below, checks both). Read here with the rest of this
    // pass, not built yet: building happens later, after the shell's own construction has
    // returned this table to its resting count (see that block's own comment for why the timing
    // matters).
    let cred_elf = measured(&fs, table, "credentialer").elf;
    let idp_elf = measured(&fs, table, "identity_provisioner").elf;
    let login_elf = measured(&fs, table, "login").elf;
    let audit_elf = measured(&fs, table, "login_audit_receiver").elf;
    // The undertaker (milestone 22, the interactive increment). Read here with the rest, because
    // the archive is only readable while we hold it and every failure below is one `fail`. Required
    // rather than optional, unlike the adapter above: without it a bounded job pool fills and the
    // prompt stops spawning, which is a broken system and not a missing feature.
    let reaper_elf = measured(&fs, table, "job_undertaker");
    // **The subtree caretaker** (milestone 31 phase 3), which is not a boot component and not a
    // spawnable program: it is what a *directory grant* is made of, one per invocation, so it is
    // read here with the rest and built by the spawn service far below.
    //
    // Optional in exactly `terminal_sink_caretaker`'s sense, and the missing-component rule decides
    // it rather than a second policy: without one, a directory grant cannot be delivered and the
    // prompt says so, which costs `rm` and nothing else. A refusal by the measurement table costs
    // the same, because the progenitor treats what it cannot vouch for as what is not there.
    let care_elf = measured(&fs, table, "fs_subtree_caretaker").elf;
    // **And the same bytes again, unparsed, because `login` needs them too** (milestone 233).
    //
    // That program builds one caretaker per authenticated session, so it needs an image to build
    // from. It used to read the archive itself, which worked only under the kernel's own test
    // harness: `build_child` can map a page only if this process holds a `PageFrame` capability for
    // it, and the archive is reserved RAM the frame allocator does not own and no capability names.
    // So the real boot started `login` with no archive at all and it died at `_start` on every
    // boot. It is handed a copy of exactly the one program it needs instead, as a blob (see the
    // login block far below, and `login_protocol::CARETAKER_ELF_VA` for the whole account).
    //
    // Empty when this process would not vouch for the bytes, which is the same answer `care_elf`
    // above already gives the spawn service: `login` then comes up and denies every login rather
    // than refusing to start. Held across the shadowing of `fs` further down, which is why it is
    // read here with the rest rather than at the point of use.
    let care_blob: &[u8] = if care_elf.is_some() {
        fs.read("fs_subtree_caretaker").unwrap_or(&[])
    } else {
        &[]
    };

    // **The programs the shell can spawn** (milestone 31), measured and parsed here rather than
    // after the giveaway: the announcement further down is the only thing the progenitor ever says, so the
    // verdicts have to exist before it. One `Option<elf::Elf>` is five words, so moving the whole
    // table up the frame costs a few hundred bytes and buys a person being told at boot instead of
    // at the prompt. (It said "seven of them" while `PROG_COUNT` was nine and then ten; a count
    // written beside a constant that moves is a count that goes stale, so this one is not written.)
    //
    // The refusals are collected in the same pass, because a second pass would hash every program a
    // second time. Only entries the archive **has** count as refusals: `rm` is deliberately not
    // loadable here, and a program this initrd never packed was never spawnable, so neither is news.
    let mut progs: [Option<elf::Elf>; grant_plan::PROG_COUNT] = core::array::from_fn(|_| None);
    let mut refused = [""; grant_plan::PROG_COUNT];
    let mut refused_n = 0usize;
    for (id, slot) in progs.iter_mut().enumerate() {
        let Some(name) = Prog::from_id(id as u64).and_then(archive_name) else {
            continue;
        };
        let found = measured(&fs, table, name);
        if found.unvouched {
            refused[refused_n] = name;
            refused_n += 1;
        }
        *slot = found.elf;
    }

    let ut = g.untyped;

    // **Freed here rather than after the console, line discipline and input are all built**
    // (milestone 47, DECISIONS §111's config-page lane): `for_test_roles` is dead weight on this
    // boot from the instant `g` exists (it names capabilities milestone 19d's test roles use and
    // the interactive system never touches), and this capability table has only sixteen slots, a
    // margin this file's own comment two screens down already documents as having broken once
    // (milestone 50's two extra kernel grants left "no slot left" for the shell's `build_child`).
    // Adding a fourth permanently-held kernel grant (the config page, alongside the clock page and
    // the filesystem pair) reproduced exactly that failure during the console/line-discipline/input
    // build, silently, because nothing between here and the old deletion point ever reads a
    // `for_test_roles` capability's contents: only its *slot* matters, and freeing the slot two
    // component-builds earlier costs nothing this boot was using.
    for &c in g.for_test_roles {
        cap_delete(c);
    }

    // **Graphical, when this boot has a GPU and a keyboard both attached** (milestone 177, option
    // A). Probed the same way the virtio-rng trio is (`user_mode_runtime::is_granted`, since there
    // is no fourth `START` argument word left to be told with instead): `disp_term_ep` and `kbd_ep`
    // are granted together or not at all (`kernel::user::boot_graphical_terminal`'s own contract),
    // so checking one stands for both.
    //
    // **Computed and acted on here, at the very top, not where it is first needed** (found by
    // hitting the wall the same way the paragraph below describes: bisection, not reasoning).
    // `disp_term_ep`/`disp_term_page`/`kbd_ep` are kernel grants, alive from spawn, so on a
    // graphical boot they inflate the resting baseline for the **whole** function exactly the way
    // the virtio-rng trio already does (this paragraph's own next one) -- and this function's
    // baseline was already tight enough that the virtio-rng trio's own three slots forced entropy
    // to move all the way to the top once. Three more permanent slots on top of that pushed the
    // *entropy* build itself past the wall (`must(build_child(...))` for `ent_program`, which
    // retypes an `AddressSpace` and a `THREAD_CONTROL_BLOCK` from this same table, first-free,
    // with only `for_test_roles`'s two freed slots to draw from). `uart_dev`/`uart_irq` are dead
    // weight on a graphical boot from this line on (no console, no input driver ever reaches
    // them), so freeing them here, before entropy spends anything, is what buys those two slots
    // back before the peak that needed them.
    //
    // **A terminal with no keystroke source of its own is a screen beside the serial console**, not
    // a graphical boot (the shell on the firmware screen, milestone 198's rung 1b). The kernel
    // grants `disp_term_ep`/`disp_term_page` alone, with `kbd_ep` empty, when it put a terminal on
    // the firmware's framebuffer (`kernel::user::boot_screen_terminal`): the UART stays the
    // console and the keystroke source exactly as on a plain boot, and the console server hands
    // every byte it writes to that terminal as well. So `kbd_ep` is what tells the two apart.
    let has_graphical = is_granted(g.disp_term_ep) && is_granted(g.kbd_ep);
    let has_screen = !has_graphical && is_granted(g.disp_term_ep);
    // **Read before anything below retypes**, and the position is the mechanism: every object this
    // function makes lands in the first free slot, so once one has, "slot 16 holds something" no
    // longer means the kernel put it there. See [`BootEndowment::entropy_ep`].
    let kernel_built_entropy = is_granted(g.entropy_ep);
    if has_graphical {
        cap_delete(g.uart_dev);
        cap_delete(g.uart_irq);
    }

    // **The entropy service** (DECISIONS §120's 2026-08-26 amendment), when the kernel granted a
    // virtio-rng device and the archive carries a program for it. Built **here, before anything
    // else**, and that positioning is load-bearing rather than a style choice.
    //
    // An earlier version of this built entropy after the console and line discipline, in the gap
    // their own three capabilities free ("The console's three capabilities go back now"). That
    // reads as the more generous gap, but it counts the wrong thing: the virtio-rng trio is
    // granted by the *kernel*, at spawn, so it inflates the resting baseline for the **whole**
    // function, not just the block that builds it. The earliest peak this function ever reaches
    // (retyping `request`/`reply`/`term_ep`/`con_shared`/`term_out`/`term_in`, all six before
    // console is even built) is already tight against the sixteen-slot wall on its own account
    // (`crates/system_initializer`'s own BUGS: "one slot from the wall" describes the *shell's*
    // peak, and the terminal-plumbing peak just named is close behind it); adding three more
    // permanent slots to a baseline that peak already stresses pushed it over, and the boot
    // faulted building `term_in`, in total silence, with no route to a person (this file's own
    // BUGS on why a refusal this early has none). Found by bisection, not reasoning: an isolated
    // test granted the progenitor one single harmless extra capability, at an arbitrary slot, with no code
    // anywhere naming or using it, and the identical silent fault reproduced.
    //
    // Building here instead avoids the collision rather than widening anything: this is the
    // *only* point in the function where the virtio-rng trio's three slots and the terminal
    // plumbing's six are never both live at once. Entropy is built and its three slots are
    // released before `request` is ever retyped, so every peak downstream of this block is
    // exactly what it was before this amendment existed.
    //
    // **Absence is not a failure, twice over**: no device (real hardware, or `NIFE_RNG` unset) and
    // no `entropy` program in the archive both leave the system exactly as it was before this
    // amendment, the same "a missing component costs a feature, not a boot" posture the sink
    // adapter and the subtree caretaker already take.
    //
    // **`entropy_ready`, not an inline `announce`.** This process has no terminal yet at this
    // point in the boot (that is the whole reason the peak accounting above works out), so the
    // outcome is carried forward as data and said later, at the existing, already-mapped print
    // site right before `term_ep`/`term_out` are dropped (see that site's own comment).
    //
    // # BUGS (historical: resolved below, this update)
    //
    // **Used to prove only the device chain.** This block builds the service and confirms it drew
    // real bytes from the real device; `entropy_client`, below, is what changed: `request`'s
    // the progenitor-side copy is now kept (not `cap_delete`d) exactly when a client will need it, so
    // `credentialer` can be handed a working view of it further down.
    let mut entropy_ready = false;
    // **`request`'s the progenitor-side copy, kept for the life of the boot** (milestone 49's boot-wiring
    // update, widened by milestone 111). `None` on every path that `cap_delete`s it (device
    // absent, mapping failed, the handshake did not answer `READY`); `Some(request)` exactly once
    // entropy is proven up.
    //
    // **It used to be dropped once `credentialer` held its own copy**, with the login stack as its
    // only consumer. Milestone 111 gave it a second one that outlives the boot: a child whose
    // manifest declares [`grant_plan::Manifest::entropy`] is endowed a `WRITE` view of this same
    // endpoint at spawn, so the progenitor is the only process that can hand a program at the prompt a
    // random source, exactly as it is the only one that can hand it a clock. That costs one
    // permanent capability slot in a table milestone 230 measured at 21 of 24 at peak; milestone
    // 231's `capability slots: N of M at peak` line is what says whether that is still true, and it
    // is printed by every boot.
    let mut entropy_client: Option<u64> = None;
    if let Some(ent_program) = ent_elf.as_ref() {
        // `invoke`'s own `NoSuchSlot` (-1) on an ungranted slot is the probe: there is no fourth
        // `START` argument word left to be told with instead (see
        // [`BootEndowment::virtio_rng`]'s own doc). `virtio_rng`/`virtio_rng_irq`/`virtio_rng_dma`
        // are granted as one triple or not at all (`kernel::user::boot_virtio_rng_device`), so a
        // negative answer here means all three slots are empty, not just this one.
        // SAFETY: `invoke` traps to the kernel, which validates the capability and the method
        // before acting; a probe against an empty slot is exactly what that contract is built to
        // answer safely.
        let magic = unsafe { invoke(g.virtio_rng, abi::virtio::READ_REG, 0, 0, 0) };
        if magic >= 0 {
            // SAFETY: as above.
            if unsafe {
                invoke(
                    g.virtio_rng_dma,
                    abi::page_frame::MAP,
                    RNG_DMA_PEEK_VA,
                    1,
                    ut,
                )
            } == 0
            {
                // SAFETY: just mapped read/write, one page, ours alone until entropy is built and
                // holds its own copy of the same frame; `VIRTIO_DMA_PHYS_OFFSET` is inside it and
                // outside entropy's own ring-and-buffer layout (that constant's own doc).
                let direct_memory_access_phys = unsafe {
                    core::ptr::read_unaligned(
                        (RNG_DMA_PEEK_VA as *const u8)
                            .add(VIRTIO_DMA_PHYS_OFFSET as usize)
                            .cast::<u64>(),
                    )
                };
                let request = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let ready = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let entropy = must(build_child(
                    ut,
                    ut,
                    ent_program,
                    &ChildEndowment {
                        caps: &[
                            (request, abi::rights::READ),
                            (g.virtio_rng_irq, abi::rights::READ),
                            (g.virtio_rng, abi::rights::WRITE),
                            (ready, abi::rights::WRITE),
                        ],
                        maps: &[(RNG_DMA_VA, g.virtio_rng_dma, abi::address_space::MAP_RW)],
                        stack_pages: CHILD_STACK_PAGES,
                        ..ChildEndowment::new(Retention::Nothing)
                    },
                ));
                must_ok(start_child(
                    entropy,
                    RNG_MODE_VIRTIO,
                    direct_memory_access_phys,
                    0,
                ));
                cap_delete(g.virtio_rng_irq);
                cap_delete(g.virtio_rng);
                cap_delete(g.virtio_rng_dma);
                // Block for the service's own proof of life: it fetches a first bufferful from the
                // real device before answering, so this means "a client that asks will be
                // answered", not merely "the handshake completed" (`components/src/entropy.rs`'s own
                // doc).
                let (verdict, _, _) = recv(ready);
                cap_delete(ready);
                entropy_ready = verdict == entropy_protocol::READY;
                if entropy_ready {
                    // Kept for the login stack below (`credentialer`'s own client view) and, since
                    // milestone 111, for the spawn service after it; see `entropy_client`'s own
                    // doc.
                    entropy_client = Some(request);
                } else {
                    // No login stack will be built without a working entropy service (this
                    // block's own comment on `have_login_stack`), so `request` has no further use
                    // on this path.
                    cap_delete(request);
                }
            } else {
                cap_delete(g.virtio_rng_irq);
                cap_delete(g.virtio_rng);
                cap_delete(g.virtio_rng_dma);
            }
        }
    }
    // **Or the service the kernel built on the CPU's seed instruction** (milestone 595
    // (provisional)), which arrives already proven: the kernel waited for its readiness report and
    // grants nothing on a refusal ([`BootEndowment::entropy_ep`]). One slot rather than the trio's
    // three, granted only when the trio was not, so this block's peak argument above still holds.
    // Nothing is built here, so nothing is spent here either.
    let mut entropy_from_instruction = false;
    if entropy_client.is_none() && kernel_built_entropy {
        entropy_ready = true;
        entropy_from_instruction = true;
        entropy_client = Some(g.entropy_ep);
    }

    // **The network stack** (milestone 590 (provisional), the booted system starts its network
    // stack), when the kernel granted a virtio-net device (slots 13-15) and the archive carries
    // `net_stack`. Built here, straight after entropy and before any of the terminal plumbing, for
    // the entropy block's own reason: the NIC trio is a kernel grant, alive from spawn, so it
    // inflates this function's baseline until the moment it is deleted, and the terminal plumbing
    // is where the early peak is. After this block the trio is gone on every path, built or not,
    // and what is left is one endpoint.
    //
    // **What it is given is what the kernel harness gives it, and no more** (`kernel::user::
    // virtio_service::wire_net_server`, which every network test uses): the confined transport,
    // the interrupt, the DMA page, a heap budget, a report endpoint for its DHCP lease, and `READ`
    // on the endpoint its clients `CALL`. Its inbound authority is `NO_LISTEN_GRANT`: nothing at
    // this prompt may listen, because nothing here has asked to and a listen grant is a policy
    // somebody has to choose (milestone 107 (the socket contract learns to accept)). The proposal this block answers named that as an
    // open question; "none" is the answer for every program the prompt can spawn today.
    //
    // **It blocks the boot until DHCP answers**, which is this block's one real cost, recorded in
    // milestone 590's BUGS: on QEMU's user-mode network the answer is immediate, and on a network
    // with a NIC and no DHCP server the prompt would never appear. The lease is what
    // `net_stack`'s first message says, and it cannot serve a client before it has sent it.
    let mut network: Option<(u64, u64)> = None;
    if is_granted(g.virtio_net) {
        if let Some(net_program) = net_elf.as_ref() {
            network = Some(build_net_stack(ut, net_program, g));
        }
        cap_delete(g.virtio_net_irq);
        cap_delete(g.virtio_net);
        cap_delete(g.virtio_net_dma);
    }

    // **The two components that have to exist before the progenitor can say anything.** The console writes
    // the UART and the line discipline is its only client, so a refusal of either has no route to a
    // person and this is the one case that stops in silence (see this module's BUGS). Everything
    // else is checked below, after they are running.
    let (Some(con_elf), Some(td_elf)) = (con_elf.elf, td_elf.elf) else {
        fail()
    };

    // `has_graphical` and the early `uart_dev`/`uart_irq` free both already happened, at the top
    // of this function, before the entropy block: see that comment for why the timing is
    // load-bearing rather than tidiness.

    // The endpoints and shared pages we own and hand out, each retyped with full rights so we can
    // delegate narrowed views. `term_ep` is the terminal contract's one endpoint: the discipline
    // serves it; the input driver (or `keyboard_driver`) and the shell only hold WRITE on it, and neither can
    // tell what is on the other side (notes/terminal-contract.md). In the graphical case it is not
    // ours to create: `keyboard_driver` already holds `WRITE` on `g.kbd_ep`, fixed at its own kernel-side spawn
    // time, so that is the endpoint `line_editor` has to serve instead of a fresh one.
    let term_out = must(retype_page_frame(ut)); // shell -> line_editor text and prompts
    let term_in = must(retype_page_frame(ut)); // line_editor -> shell completed lines

    let term_ep = if has_graphical {
        g.kbd_ep
    } else {
        must(retype_obj(ut, abi::objtype::RENDEZVOUS))
    };

    if has_graphical {
        // No console, no input driver, no separate request/reply pair or shared page: `keyboard_driver` and
        // `display_terminal` are already running (built kernel-side, before this process existed),
        // and `line_editor` is their only client. It prints through `display_terminal`'s own
        // `OP_WRITE`/one-`CALL` contract (`g.disp_term_ep`/`g.disp_term_page`, `LINE_EDITOR_MODE_DISPLAY`)
        // instead of the console's bespoke two-endpoint one.
        let r = build_child(
            ut,
            ut,
            &td_elf,
            &ChildEndowment {
                caps: &[
                    (term_ep, abi::rights::READ),
                    (g.disp_term_ep, abi::rights::WRITE),
                ],
                maps: &[
                    (CON_SHARED_VA, g.disp_term_page, abi::address_space::MAP_RW),
                    (TERM_OUT_VA, term_out, abi::address_space::MAP_RO),
                    (TERM_IN_VA, term_in, abi::address_space::MAP_RW),
                ],
                stack_pages: CHILD_STACK_PAGES,
                ..ChildEndowment::new(Retention::Nothing)
            },
        );
        let line_editor = must(r);
        must_ok(start_child(line_editor, LINE_EDITOR_MODE_DISPLAY, 0, 0));

        // `g.disp_term_ep`/`g.disp_term_page` are ours no further: `line_editor` holds its own
        // narrowed copies, granting was a copy rather than a move (the same reason `con_shared`
        // is freed on the console side below), and nothing else in this boot ever needs to reach
        // `display_terminal` directly. `term_ep` (== `g.kbd_ep` in this branch) is not freed here:
        // it is freed with the console branch's own copy, further down, by code that does not
        // know or care which branch produced it.
        cap_delete(g.disp_term_ep);
        cap_delete(g.disp_term_page);
    } else {
        let request = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
        let reply = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
        let con_shared = must(retype_page_frame(ut)); // line_editor -> console text

        // 1. Console server: reads text from the shared page, writes it to the UART.
        //
        // **On aarch64/riscv64 the UART is a page**, so the console holds it as a mapped
        // `DeviceFrame` (`CON_UART_VA`). **On x86 COM1 is a port range**, which has nothing to map
        // (DECISIONS §121, reversed 2026-09-15): the console holds it as a `PortRange` capability
        // delegated into its table instead, and the kernel's TSS I/O bitmap is what lets its `out`
        // reach the port. So `g.uart_dev` moves from a `maps` entry to a `caps` entry on x86, and
        // the UART mapping is dropped. It lands in the child's next free slot (2, or 3 behind a
        // screen); the console never invokes it by slot (it executes `out` directly), it only has
        // to hold it.
        //
        // **And a screen beside the UART, when the kernel wired one** (`has_screen`, above): the
        // terminal's endpoint lands in the console's slot 2, ahead of x86's port range (which the
        // console holds but never names by slot), and the page that terminal reads is mapped at
        // `CON_SCREEN_OUT_VA`. Arrays with a count rather than one slice literal per case, because
        // the screen is a runtime fact and the port range a compile-time one, and four literals
        // would be four places to get the slot order wrong.
        let mut con_caps = [(0, 0); 4];
        let mut con_maps = [(0, 0, 0); 3];
        let (mut ncaps, mut nmaps) = (0, 0);
        for cap in [(request, abi::rights::READ), (reply, abi::rights::WRITE)] {
            con_caps[ncaps] = cap;
            ncaps += 1;
        }
        con_maps[nmaps] = (CON_SHARED_VA, con_shared, abi::address_space::MAP_RO);
        nmaps += 1;
        if has_screen {
            con_caps[ncaps] = (g.disp_term_ep, abi::rights::WRITE);
            ncaps += 1;
            con_maps[nmaps] = (
                CON_SCREEN_OUT_VA,
                g.disp_term_page,
                abi::address_space::MAP_RW,
            );
            nmaps += 1;
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            // mode ignored for a DeviceFrame
            con_maps[nmaps] = (CON_UART_VA, g.uart_dev, abi::address_space::MAP_RO);
            nmaps += 1;
        }
        #[cfg(target_arch = "x86_64")]
        {
            con_caps[ncaps] = (g.uart_dev, abi::rights::WRITE); // COM1's port range, held not mapped
            ncaps += 1;
        }
        let con = must(build_child(
            ut,
            ut,
            &con_elf,
            &ChildEndowment {
                caps: &con_caps[..ncaps],
                maps: &con_maps[..nmaps],
                stack_pages: CHILD_STACK_PAGES,
                ..ChildEndowment::new(Retention::Nothing)
            },
        ));
        must_ok(start_child(
            con,
            if has_screen { CONSOLE_MODE_SCREEN } else { 0 },
            0,
            0,
        ));
        // The console holds its own narrowed copies of the terminal's endpoint and page now, and
        // nothing else in this boot prints to that terminal directly: everyone prints through the
        // console. Freed here rather than with `request`/`reply` below for the sixteen-slot reason
        // this function's every early free gives.
        if has_screen {
            cap_delete(g.disp_term_ep);
            cap_delete(g.disp_term_page);
        }

        // 2. The line discipline: serves the terminal endpoint, prints through the console. It is
        // the console's only client; everyone else prints through it. `LINE_EDITOR_MODE_CONSOLE`
        // (0) is `start_child`'s own default, so this is unchanged from before
        // milestone 177 gave `line_editor` a second mode.
        let line_editor = must(build_child(
            ut,
            ut,
            &td_elf,
            &ChildEndowment {
                caps: &[
                    (term_ep, abi::rights::READ),
                    (request, abi::rights::WRITE),
                    (reply, abi::rights::READ),
                ],
                maps: &[
                    (CON_SHARED_VA, con_shared, abi::address_space::MAP_RW), // it fills what the console reads
                    (TERM_OUT_VA, term_out, abi::address_space::MAP_RO),
                    (TERM_IN_VA, term_in, abi::address_space::MAP_RW),
                ],
                stack_pages: CHILD_STACK_PAGES,
                ..ChildEndowment::new(Retention::Nothing)
            },
        ));
        must_ok(start_child(line_editor, LINE_EDITOR_MODE_CONSOLE, 0, 0));

        // `request`/`reply`/`con_shared` are ours no further: the console and `line_editor` both
        // hold their own narrowed copies (the console's own doc, one screen down, gives the
        // sixteen-slot reason this cannot wait). The graphical branch above never retypes these at
        // all, so there is nothing of theirs to free here.
        for c in [request, reply, con_shared] {
            cap_delete(c);
        }
    }

    // **Refuse the system if a required component is not the one that was measured** (milestone
    // 104), here, because this is the earliest point at which the progenitor can be read by a person and the
    // latest at which nothing unmeasured has been built. The console and the line discipline above
    // are running; nothing else is.
    //
    // Halting is not a second policy. The policy is that the progenitor grants nothing of its own to
    // what it cannot vouch for (§219), which for an archive program means it does not run it, and
    // for a component the whole system is made of, not running it and not having a system are the
    // same outcome. What it costs is decided by what the program was for, which is a question the progenitor
    // already had to answer for an archive entry that is simply missing.
    let unvouched: [&str; 3] = [
        if in_elf.unvouched { "input" } else { "" },
        if sh_elf.unvouched { "swish" } else { "" },
        if reaper_elf.unvouched {
            "job_undertaker"
        } else {
            ""
        },
    ];
    if unvouched.iter().any(|n| !n.is_empty()) {
        // The shell's output page, in our own space, so the refusal can be read. The giveaway below
        // maps it for the same reason and this path never reaches the giveaway; a refusal nobody
        // can see is most of what this milestone was written to fix.
        // SAFETY: `invoke` traps to the kernel, which validates the capability and the method before
        // acting (user_mode_runtime's contract).
        if unsafe { invoke(term_out, abi::page_frame::MAP, INIT_OUT_VA, 1, ut) } == 0 {
            let mut buf = [0u8; SENTENCE];
            announce(
                term_ep,
                sentence(
                    &mut buf,
                    b"progenitor: cannot vouch for",
                    &unvouched,
                    b"; halting rather than building an unmeasured system\n",
                ),
            );
        }
        fail()
    }
    // Past the refusal, so an absent component still traps the way it always has: a build that did
    // not pack the console's client is a broken build, not an attack, and it has never had a
    // message.
    let (Some(in_elf), Some(sh_elf), Some(reaper_elf)) = (in_elf.elf, sh_elf.elf, reaper_elf.elf)
    else {
        fail()
    };

    // 3. Input driver: waits on the UART receive interrupt, forwards raw bytes to the terminal.
    // **Skipped entirely in the graphical case**: `keyboard_driver` already plays this role, spawned
    // kernel-side and wired directly to `term_ep` before this process existed. `g.uart_dev`/
    // `g.uart_irq` are still granted either way (their slot numbers do not move; see
    // `BootEndowment`'s own doc), simply unused, and freed below regardless of mode.
    if !has_graphical {
        // **On aarch64/riscv64 input is interrupt-driven**: it holds the receive interrupt
        // (`g.uart_irq`) and the UART page (`IN_UART_VA`). **On x86 it polls COM1's port range**, the
        // same capability the console holds, because the receive line is not yet routed to a
        // userspace waiter on this architecture (milestone 299; see `components/src/input.rs`). So on
        // x86 it gets the `PortRange` capability delegated into its table (slot 1) and neither the
        // interrupt nor the UART mapping. It never invokes the port cap by slot; holding it is what
        // the TSS bitmap enforcement reads.
        #[cfg(not(target_arch = "x86_64"))]
        let (in_caps, in_maps): EndowmentSlices = (
            &[
                (term_ep, abi::rights::WRITE),
                (g.uart_irq, abi::rights::READ),
            ],
            &[(IN_UART_VA, g.uart_dev, abi::address_space::MAP_RO)],
        );
        #[cfg(target_arch = "x86_64")]
        let (in_caps, in_maps): EndowmentSlices = (
            &[
                (term_ep, abi::rights::WRITE),
                (g.uart_dev, abi::rights::WRITE), // COM1's port range, polled not waited on
            ],
            &[],
        );
        let input = must(build_child(
            ut,
            ut,
            &in_elf,
            &ChildEndowment {
                caps: in_caps,
                maps: in_maps,
                stack_pages: CHILD_STACK_PAGES,
                ..ChildEndowment::new(Retention::Nothing)
            },
        ));
        must_ok(start_child(input, 0, 0, 0));
    }

    // **The console's capabilities go back now, before the shell is built**, and that is not
    // tidiness: this capability table has sixteen slots, and milestone 50 added two more kernel grants (the
    // file service and its page). With them held, the shell's `build_child` had no slot left to
    // retype an address space into and failed silently, which presented as a boot that brought up
    // the console and then printed nothing. Nothing below needs these: line_editor is the console's
    // only client (or `display_terminal`'s, in the graphical case) and it already holds its
    // narrowed copies.
    //
    // **The device and the interrupt go with them** (milestone 22, the interactive increment). Both
    // drivers that need them exist and hold their own narrowed copies, and nothing below builds
    // another driver, so a progenitor that kept them would be keeping the authority to hand the UART to
    // anything it later builds. Dropping them here is the same act as dropping the construction
    // budget further down, one boot stage earlier. `unused` is whatever else this board's kernel
    // granted that the interactive system never had a use for.
    //
    // **Already freed in the graphical case**, earlier and for a stronger reason than tidiness
    // (see the comment where `has_graphical` frees them, above): this table had no slot to spare
    // for `term_out`/`term_in` otherwise. Freeing an empty slot here would be harmless, but
    // skipping it is what makes the earlier comment's claim ("free the instant they are dead
    // weight") true rather than aspirational.
    if !has_graphical {
        cap_delete(g.uart_dev);
        cap_delete(g.uart_irq);
    }
    // `for_test_roles` no longer needs freeing here: it is freed at the top of this function now,
    // two component-builds earlier, for the reason recorded there.

    // **The spawn channel is retyped here, not with the rest**, and the reason is the same sixteen
    // slots: holding two more endpoints through the three builds above is what pushed this capability table
    // over. They are the shell's and the service's, so this is also where they belong.
    let spawn_ep = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
    let result_ep = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
    // **The supervision endpoint every job is born holding** (milestone 22, the interactive
    // increment; DECISIONS §26's spawn-slot convention). We keep it for its `GRANT`, which is all we
    // need it for: to place a `READ` view of it in each job's reserved fault slot. We never receive
    // on it. `job_undertaker` does, and collecting is the only thing that endpoint authorizes.
    let deaths = must(retype_obj(ut, abi::objtype::RENDEZVOUS));

    // 4. The shell: prints and reads lines through the terminal, holds the spawn channel, and holds
    // its own untyped budget (slot 3) so `run --mem N` grants from memory that is genuinely the
    // shell's. WRITE lets it SPLIT the budget; GRANT lets it delegate the split to the progenitor. We carve
    // that budget from our own untyped and hand it over the same way we hand any capability.
    //
    // Slot 4 is the filesystem when this boot has one, which is the whole of what `>` and `<` need
    // (milestone 50, notes/pipes.md): the shell resolves a redirection against it and writes the
    // file itself. Narrowed to WRITE, which on an endpoint is the right to CALL, and without GRANT,
    // so the shell can hand it to nobody.
    //
    // **And a read-only clock last** (milestone 86), which is what `time <command>` measures with.
    // It is [`BootEndowment::clock_page`], the same frame this progenitor was granted and hands to a child
    // whose manifest declares a clock; the shell is simply another holder of a narrowed view. `READ`
    // and no `GRANT`, deliberately: the shell can read the wall clock and can hand one to nothing it
    // spawns, so which processes can read the time is still decided by the manifests this crate
    // reads (DECISIONS §43) rather than by anything typed at a prompt.
    //
    // Last in the list, so a boot with no filesystem takes exactly the path it took before this
    // existed. Its slot therefore moves (4 without a disk, 5 with one), which is why the shell is
    // *told* the number in `x2`/`a2` instead of assuming one; see swish.rs's `CLOCK_SLOT`.
    let sh_budget = must(memory_region_split(ut, SH_BUDGET_PAGES));
    let with_fs = fs_rights != 0;

    // **The second grant, when this boot was configured with one** (milestone 154's "wiring a
    // second grant into the real boot"). Built here, before the shell, for the same reason
    // `build_caretaker` is always called before the process that will hold its endpoint: the
    // caretaker must exist and answer ready before anything could `CALL` it.
    //
    // Meaningless without a first directory ([`SecondDirGrant`]'s own doc), so this is `None`
    // whenever the boot has no filesystem at all, and it degrades to "no second grant" rather
    // than a boot failure whenever the caretaker cannot be built, the same posture this file
    // already takes toward a missing `sink_elf` or `care_elf`: a component the archive did not
    // pack or the table would not vouch for costs a feature, not a boot.
    //
    // # BUGS
    //
    // **Unverified against a real boot.** `second_dir` is `None` at every shipped entry point
    // (DECISIONS §126: what the subtree should be is calef's call), so this branch has never run
    // under `script/swish-check`, which is the only thing in the tree that runs a real progenitor.
    // `build_caretaker` retypes two more objects into *this process's* capability table right
    // where the comment two screens up already documents this table as tight ("the shell's
    // `build_child` had no slot left ... and failed silently"). The failure mode if this pushes
    // the progenitor over sixteen slots is exactly that one: a boot that reaches userspace and prints
    // nothing. Whoever first passes `Some` here should watch for it and run `script/swish-check`
    // before trusting this path.
    let second_dir_ep: Option<u64> = second_dir.filter(|_| with_fs).and_then(|sd| {
        assert!(
            filesystem_protocol::grant::fits(sd.name.as_bytes()),
            "a granted directory's name rides in two argument words; this one does not fit",
        );
        let region = memory_region_split(ut, SECOND_DIR_CARETAKER_PAGES).ok()?;
        let care = care_elf.as_ref()?;
        let (lo, hi) = filesystem_protocol::grant::pack_name(sd.name.as_bytes());
        let spec = filesystem_protocol::grant::spec(sd.name.len(), sd.rights);
        build_caretaker(
            ut,
            region,
            care,
            Fs {
                ep: g.fs_ep,
                page: g.fs_page,
            },
            (lo, hi, spec),
        )
    });

    // Slot 4 is the filesystem when this boot has one, which is the whole of what `>` and `<` need
    // (milestone 50, notes/pipes.md): the shell resolves a redirection against it and writes the
    // file itself. Narrowed to WRITE, which on an endpoint is the right to CALL, and without GRANT,
    // so the shell can hand it to nobody.
    //
    // **Slot 5, when this boot also has a second grant**, is [`second_dir_ep`]: the narrowed
    // caretaker endpoint built above. It shares [`SH_FS_VA`] rather than needing a map of its own,
    // for `narrow_dir`'s own reason one level narrower: caretaker and client both map the *same*
    // physical frame the FS server shares with everything downstream of it, and the shell is one
    // thread of control with at most one `CALL` in flight, so it is never mid-request on both
    // endpoints at once.
    //
    // **And a read-only clock last, always** (milestone 86), which is what `time <command>`
    // measures with. It is [`BootEndowment::clock_page`], the same frame this progenitor was granted and
    // hands to a child whose manifest declares a clock; the shell is simply another holder of a
    // narrowed view. `READ` and no `GRANT`, deliberately: the shell can read the wall clock and
    // can hand one to nothing it spawns, so which processes can read the time is still decided by
    // the manifests this crate reads (DECISIONS §43) rather than by anything typed at a prompt.
    //
    // Clock last in the list whatever else is granted, which is why the shell is *told* its slot
    // in `x2`/`a2` instead of assuming one; see swish.rs's `CLOCK_SLOT`. That was already true
    // before this milestone (4 without a disk, 5 with one); it now also moves to 6 when a second
    // grant lands, and the mechanism that keeps the shell honest about the number is unchanged.
    let mut sh_caps = [(0u64, 0u64); 7];
    let mut sh_maps = [(0u64, 0u64, 0u64); 4];
    let mut n_caps = 0usize;
    let mut n_maps = 0usize;
    sh_caps[n_caps] = (term_ep, abi::rights::WRITE);
    n_caps += 1;
    sh_caps[n_caps] = (spawn_ep, abi::rights::WRITE);
    n_caps += 1;
    sh_caps[n_caps] = (result_ep, abi::rights::READ);
    n_caps += 1;
    sh_caps[n_caps] = (sh_budget, abi::rights::WRITE | abi::rights::GRANT);
    n_caps += 1;
    sh_maps[n_maps] = (SH_OUT_VA, term_out, abi::address_space::MAP_RW); // text and prompts
    n_maps += 1;
    sh_maps[n_maps] = (LINE_VA, term_in, abi::address_space::MAP_RO); // completed lines
    n_maps += 1;
    if with_fs {
        sh_caps[n_caps] = (g.fs_ep, abi::rights::WRITE);
        n_caps += 1;
        sh_maps[n_maps] = (SH_FS_VA, g.fs_page, abi::address_space::MAP_RW);
        n_maps += 1;
    }
    if let Some(ep) = second_dir_ep {
        sh_caps[n_caps] = (ep, abi::rights::WRITE);
        n_caps += 1;
    }
    sh_caps[n_caps] = (g.clock_page, abi::rights::READ);
    n_caps += 1;
    sh_maps[n_maps] = (SH_CLOCK_VA, g.clock_page, abi::address_space::MAP_RO);
    n_maps += 1;
    // Which slot the clock landed in, for the shell's `x2`: the count of what went before it, the
    // same arithmetic `build_child` does when it fills the capability table from zero.
    let sh_clock_slot: u64 = n_caps as u64 - 1;
    // **Built but not started**, because the drop below has to happen while the shell's output page
    // is still ours alone to write: the negative control is printed through it, and a running shell
    // would be printing its banner into the same page.
    let shell = must(build_child(
        ut,
        ut,
        &sh_elf,
        &ChildEndowment {
            caps: &sh_caps[..n_caps],
            maps: &sh_maps[..n_maps],
            stack_pages: CHILD_STACK_PAGES,
            ..ChildEndowment::new(Retention::Nothing)
        },
    ));
    cap_delete(sh_budget); // our copy; the shell holds its own now
    // The caretaker's endpoint was only ever the means of wiring: the shell holds its own copy and
    // the caretaker holds the other end, the same disposal `spawn_service`'s dynamic directory
    // grants already give their own narrowed endpoint below.
    if let Some(ep) = second_dir_ep {
        cap_delete(ep);
    }

    // Free every boot cap the spawn service does not need, so the progenitor's 16-slot capability table has room to
    // build a supervised child (which holds a job untyped and a job frame while the loader retypes
    // an address space, frames, and a TCB). The drivers and the shell hold the narrowed copies that matter.
    //
    // **Only the input frame, and the two that stay have a reason each.** `term_ep` is still ours to
    // delegate: the sink adapter below is handed `WRITE` on it, and the drop announcement is a
    // `CALL` on it. `term_out` is where that announcement stages its bytes. Both go back the moment
    // their last use is done, and after that this process holds no way to reach the terminal at all.
    cap_delete(term_in);
    // **The filesystem stays** (milestone 31 phase 3, 2026-08-17). It used to go here, with a
    // comment saying "the day `rm` is reachable from the prompt, the progenitor keeps the endpoint instead,
    // because building a `fs_subtree_caretaker` is its job and not the shell's". This is that day.
    //
    // Two slots held for the life of the boot, and it is worth being precise about what they buy and
    // what they cost. They buy the only delivery mechanism a directory grant has: the caretaker must
    // hold the file service to attenuate it, the shell's copy carries no `GRANT`, and a program
    // spawned without the capability its command line named would be the worst failure this model
    // has. They cost two of the progenitor's sixteen capability table slots, permanently, which takes the spawn service's
    // resting endowment from seven capabilities to nine and its peak from thirteen to fifteen. That
    // peak is the number to watch: it is a directory-granted spawn, and it is one slot from the wall.
    // See `spawn_dir_grant`, which counts it.
    let fs = with_fs.then_some(Fs {
        ep: g.fs_ep,
        page: g.fs_page,
    });

    // 5. **The terminal's sink adapter** (milestone 50's last remainder, notes/sink-protocol.md,
    // DECISIONS §67). It holds the terminal `WRITE` and serves the sink contract on an endpoint of
    // its own, so a child can be handed "the terminal" as a place to put bytes **without** being
    // handed the terminal endpoint, which also carries `OP_READLINE` and would be the keyboard.
    //
    // **After the shell and before the giveaway, and both halves of that are load-bearing.**
    //
    // After the shell, because of this capability table's sixteen slots: building the adapter earlier put the progenitor
    // one slot over while the loader was retyping the shell's address space, and the symptom was
    // the one this system has already seen, a boot that reaches userspace and then prints nothing at
    // all. That constraint is about the shell's build, not about being the last thing built, and
    // milestone 22 is what made the difference visible: the adapter is now the fifth of six boot
    // components rather than the last of five, and the capability table has room either way.
    //
    // Before the giveaway, because this is a **system** component and the root untyped is what the
    // system is built from. Everything below hands that budget away and proves it is gone, so an
    // adapter built after it would have to come out of [`INIT_OWN_PAGES`], the scratch budget sized
    // for page tables and nothing else. Spending a whole program out of that pool would be invisible
    // here and would surface as some later child failing to map a scratch page.
    let term_sink = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
    if let Some(elf) = sink_elf.as_ref() {
        let adapter = must(build_child(
            ut,
            ut,
            elf,
            &ChildEndowment {
                caps: &[
                    (term_sink, abi::rights::READ),
                    (term_ep, abi::rights::WRITE),
                ],
                stack_pages: CHILD_STACK_PAGES,
                ..ChildEndowment::new(Retention::Nothing)
            },
        ));
        // Started here even though the shell deliberately is not: the adapter owns no page and
        // prints only what a client sends it, and it has no clients until the spawn service below
        // hands one its endpoint. It cannot write into the page the announcement below stages in.
        must_ok(start_child(adapter, 0, 0, 0));
    }

    // **Milestone 49's login stack**: `credentialer`, `identity_provisioner`, `login`,
    // `login_audit_receiver`. Built here, after the shell (this table's own tightest peak, "one
    // slot from the wall") has already returned its own transient capabilities, and before the
    // giveaway below spends `ut` down to nothing: everything this block needs (`ut` itself,
    // `term_ep`, the file service pair, a live client view of entropy) is still ours to spend here
    // and nowhere later, the identical reasoning the entropy block's own comment gives for building
    // *that* where it does.
    //
    // Optional, in the sink adapter's own sense: a boot with no filesystem, no working entropy
    // service, or any of these four programs missing or unvouched for simply has no login path,
    // and this whole block is skipped rather than faulted on. `login_ready`/`login_password` are
    // carried forward as data (the same shape `entropy_ready` already uses) because this process
    // has no terminal mapped yet at this point in the boot; the actual announcement happens later,
    // at the existing print site, right before `term_ep` is dropped.
    let have_login_stack = with_fs
        && entropy_client.is_some()
        && cred_elf.is_some()
        && idp_elf.is_some()
        && login_elf.is_some()
        && audit_elf.is_some();
    let mut login_ready = false;
    let mut login_password = [0u8; PASSWORD_HEX_LEN];
    // **The run-unvouched endpoint** (DECISIONS §219 gate D2), retyped the first time something is
    // placed on it: inside the block below when `login` is built, and after it otherwise. Never
    // before `login`'s build, because this process's table peaks inside this block and this
    // endpoint is held for the life of the boot; see [`Channels::run_unvouched`].
    let mut run_unvouched: Option<u64> = None;
    if !have_login_stack {
        // **Not given back any more** (milestone 111). A boot with a working entropy service and
        // no login stack (no filesystem, or one of the four programs missing) still reaches a
        // prompt, and `uuid` at that prompt needs this endpoint; before, the slot was released
        // here because nothing downstream had a use for it.
    } else {
        let request = entropy_client.expect("have_login_stack checked entropy_client.is_some()");

        // **The generated password, drawn before anything else in this block**: `credentialer`'s
        // own posture ("it never invents a salt") one level up. No unpredictable bits, no
        // password, and this whole block is abandoned rather than provisioning one with something
        // weaker (DECISIONS §42).
        let mut password_bytes = [0u8; PASSWORD_BYTES];
        // `request` is not released on either arm any more (milestone 111): the spawn service holds
        // it for the life of the boot, so a service that could not fill a password is still a
        // service a program at the prompt may `CALL` and be refused by honestly.
        if !fill_entropy(request, &mut password_bytes) {
        } else {
            hex_password(&password_bytes, &mut login_password);

            // 1. **`credentialer`** (milestone 56/65, the secrets service): built the same way
            // entropy was, permanent for the life of the boot, holding the client view of entropy
            // this block just drew a password from.
            let prov = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
            let verify = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
            let cred_ready = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
            let prov_page = must(retype_page_frame(ut));
            let verify_page = must(retype_page_frame(ut));
            let cred_budget = must(memory_region_split(ut, CRED_BUDGET_PAGES));
            let cred_program = cred_elf.as_ref().expect("have_login_stack checked this");
            let cred_child = must(build_child(
                ut,
                ut,
                cred_program,
                &ChildEndowment {
                    caps: &[
                        (prov, abi::rights::READ),
                        (verify, abi::rights::READ),
                        (request, abi::rights::WRITE),
                        (cred_budget, abi::rights::WRITE),
                        (cred_ready, abi::rights::WRITE),
                    ],
                    maps: &[
                        (CRED_SVC_PROV_VA, prov_page, abi::address_space::MAP_RW),
                        (CRED_SVC_VERIFY_VA, verify_page, abi::address_space::MAP_RW),
                    ],
                    stack_pages: CRED_STACK_PAGES,
                    ..ChildEndowment::new(Retention::Nothing)
                },
            ));
            must_ok(start_child(cred_child, 0, 0, 0));
            // `request` is deliberately **not** deleted here, unlike `cred_budget` below.
            // `credentialer` holds its own copy, and so does this process, because the spawn
            // service endows the same endpoint to a child declaring `Manifest::entropy`
            // (milestone 111). See `entropy_client`'s own doc for the slot it costs.
            cap_delete(cred_budget);

            // **`cred_ready` is not read yet, and that ordering is load-bearing rather than an
            // oversight.** `credentialer.rs`'s own `_start` sends its one readiness message
            // (`RPT_READY`) only *after* `provision()` returns, which is only after this process's
            // own `SEAL` arrives (`credentialer.rs`'s own "Two phases" doc: phase one is `RECV` on
            // the provision endpoint, forever, until sealed). A `recv(cred_ready)` here, before
            // provisioning has even been attempted, is not "wait for the service to come up" the
            // way the entropy block's own `recv(ready)` is -- it is "wait for a message that
            // cannot exist until this same function seals the store a few lines further down", a
            // deadlock this process would never wake from. Found by running `script/swish-check`
            // and watching it hang rather than fault: no `[PANIC]`, nothing kept building, because
            // this process was genuinely blocked rather than trapped.
            //
            // 2. **Provision the generated credential and the demo identity's home subtree, as
            // one act, through `identity_provisioner`** (milestone 155): stage the plaintext
            // into a page this process maps briefly, hand it to the tool along with `prov`
            // (before its seal) and this boot's own `g.fs_ep`.
            let req_frame = must(retype_page_frame(ut));
            // SAFETY: `invoke` traps to the kernel, which validates the capability and the
            // method before acting.
            if unsafe { invoke(req_frame, abi::page_frame::MAP, PROVISION_SCRATCH_VA, 1, ut) } != 0
            {
                cap_delete(req_frame);
            } else {
                // SAFETY: just mapped read/write, one page, ours alone until it is delegated
                // on below.
                let req_slice = unsafe {
                    core::slice::from_raw_parts_mut(
                        PROVISION_SCRATCH_VA as *mut u8,
                        credential_protocol::PAGE,
                    )
                };
                req_slice.fill(0);
                let placed = credential_protocol::place(
                    req_slice,
                    DEMO_IDENTITY,
                    &login_password,
                    credential_protocol::provision::PUT,
                );
                placed.expect(
                    "DEMO_IDENTITY and login_password are both well within credential_protocol's \
                     bounds by construction; a None here is a logic bug in this file, not a \
                     runtime condition",
                );

                let idp_report = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let idp_program = idp_elf.as_ref().expect("have_login_stack checked this");
                let idp_child = must(build_child(
                    ut,
                    ut,
                    idp_program,
                    &ChildEndowment {
                        caps: &[
                            (prov, abi::rights::WRITE),
                            (g.fs_ep, abi::rights::WRITE),
                            (idp_report, abi::rights::WRITE),
                        ],
                        maps: &[
                            (IDP_REQ_VA, req_frame, abi::address_space::MAP_RW),
                            (IDP_PROV_VA, prov_page, abi::address_space::MAP_RW),
                            (IDP_FS_VA, g.fs_page, abi::address_space::MAP_RW),
                        ],
                        stack_pages: CHILD_STACK_PAGES,
                        ..ChildEndowment::new(Retention::Nothing)
                    },
                ));
                must_ok(start_child(
                    idp_child,
                    DEMO_IDENTITY.len() as u64,
                    login_password.len() as u64,
                    0,
                ));
                cap_delete(req_frame);

                let (idp_code, _, _) = recv(idp_report);
                cap_delete(idp_report);
                login_ready = idp_code == IDP_RPT_OK;

                // Wipe the staged plaintext regardless of outcome: this process's own copy of
                // the generated password should not sit in memory longer than provisioning
                // needs it.
                // SAFETY: still the same page this process mapped above, and the only writer
                // between it being staged and this wipe.
                let req_slice = unsafe {
                    core::slice::from_raw_parts_mut(
                        PROVISION_SCRATCH_VA as *mut u8,
                        credential_protocol::PAGE,
                    )
                };
                credential_protocol::wipe(req_slice);
            }
            // Ours no further either way: `identity_provisioner` holds its own copy when it was
            // built at all, and this process never maps this page again.
            cap_delete(prov_page);

            // **Seal the store regardless of whether provisioning succeeded.** `credentialer`
            // must leave phase one before its `VERIFY` endpoint will ever answer anything
            // (`credentialer.rs`'s own "two phases" doc), and a login against an empty or
            // partially-provisioned store should deny cleanly rather than the whole boot
            // hanging with a service that can never be asked a real question.
            call(
                prov,
                credential_protocol::req(credential_protocol::provision::SEAL, 0, 0),
                0,
            );
            cap_delete(prov);

            // **Now** `cred_ready` answers: the seal just sent is exactly what makes
            // `provision()` return and `credentialer.rs`'s own `_start` reach its one
            // `send(READY, RPT_READY, ...)`. A dead-on-arrival service (its own
            // `E_ENTROPY`/`E_SCRATCH` startup failure, which happens before phase one even
            // begins) still answers here too, with its own `0xDEAD_...` word instead, so this
            // `recv` is never left permanently unanswered by that path either.
            let (cred_rv, _, _) = recv(cred_ready);
            cap_delete(cred_ready);
            if cred_rv != CRED_RPT_READY {
                cap_delete(verify);
                cap_delete(verify_page);
                login_ready = false;
            } else {
                // 3. **`login`** (milestone 49): built the same way, holding narrowed views of
                // what this process already has (the file service pair, a fresh construction
                // budget) plus a client view of `credentialer`'s own verify endpoint just built,
                // and (this update) a `WRITE | GRANT` view of the terminal so it can hand the
                // single-session terminal to its first successful caller
                // (`components/src/login.rs`'s "The terminal: single-session, deny cleanly").
                //
                // `login_audit_receiver` is built *first* and started before `login` ever runs,
                // so the receiver for `login`'s blocking `AUDIT` send exists before there is any
                // way to reach it (`components/src/login_audit_receiver.rs`'s own doc on why this
                // ordering matters).
                let audit = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let audit_program = audit_elf.as_ref().expect("have_login_stack checked this");
                let audit_child = must(build_child(
                    ut,
                    ut,
                    audit_program,
                    &ChildEndowment {
                        caps: &[(audit, abi::rights::READ)],
                        stack_pages: CHILD_STACK_PAGES,
                        ..ChildEndowment::new(Retention::Nothing)
                    },
                ));
                must_ok(start_child(audit_child, 0, 0, 0));

                let login_request = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let login_result = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
                let login_ut = must(memory_region_split(ut, LOGIN_CONSTRUCTION_PAGES));
                let login_program = login_elf.as_ref().expect("have_login_stack checked this");
                let login_child = must(build_child(
                    ut,
                    ut,
                    login_program,
                    &ChildEndowment {
                        caps: &[
                            (login_request, abi::rights::READ),
                            (login_result, abi::rights::WRITE | abi::rights::GRANT),
                            (verify, abi::rights::WRITE),
                            (g.fs_ep, abi::rights::WRITE | abi::rights::GRANT),
                            // **`WRITE` only, not `READ | WRITE`.** This process itself holds
                            // only `WRITE | GRANT` on the real file service's shared page (the
                            // kernel's own grant to the progenitor), and `WRITE` is all a writable mapping
                            // ever checks (`kernel::syscall::page_frame_map`'s own comment: "a
                            // read/write mapping needs WRITE on the frame"). See
                            // `components/src/login.rs`'s own comment on its matching delegation to its
                            // clients for the fuller account. Found here first: `must(build_child(...))`
                            // for `login` refused this exact `SEND_CAP` when it asked for `READ`
                            // too, in total silence, the same way every other capacity mismatch in
                            // this function has.
                            (g.fs_page, abi::rights::WRITE | abi::rights::GRANT),
                            (login_ut, abi::rights::WRITE | abi::rights::GRANT),
                            (audit, abi::rights::WRITE),
                            (term_ep, abi::rights::WRITE | abi::rights::GRANT),
                        ],
                        maps: &[(LOGIN_CRED_VA, verify_page, abi::address_space::MAP_RW)],
                        // **What milestone 233 added, and it costs this process no slots at all.**
                        // `login` needs one program's bytes and a table to check them against; it
                        // used to read the whole archive, which this process cannot delegate
                        // (reserved RAM, no capability names it) and so never did. `blobs` copies
                        // through `supervision_protocol`'s `fill_and_map`, which holds one frame
                        // capability at a time and deletes it, so this adds nothing to the
                        // 21-of-24 peak `kernel::cap::CAPABILITY_TABLE_PEAK_MEASURED` records:
                        // `build_child` already reaches that same transient peak copying `login`'s
                        // own segments.
                        blobs: &[
                            (login_protocol::CARETAKER_ELF_VA, care_blob),
                            (login_protocol::PROGRAM_MEASUREMENTS_VA, table.as_bytes()),
                        ],
                        stack_pages: LOGIN_STACK_PAGES,
                        ..ChildEndowment::new(Retention::Nothing)
                    },
                ));
                // **`login` hands the run-unvouched capability to each session it builds**
                // (DECISIONS §219 gate D2; calef's consequence: a user whose session was not given
                // it cannot run unvouched bytes). Placed after the build rather than in `caps`
                // above, because the build is this table's peak (`kernel::cap::
                // CAPABILITY_TABLE_PEAK_MEASURED`) and the endpoint is held from here on: retyped
                // now, it costs a slot the loader's transient address space and page have just
                // given back. `WRITE | GRANT`, because `login` must delegate it; each session gets
                // `WRITE` alone (`components/src/login.rs`).
                let d2 = *run_unvouched
                    .get_or_insert_with(|| must(retype_obj(ut, abi::objtype::RENDEZVOUS)));
                must_ok(place_at(
                    login_child.tcb,
                    d2,
                    abi::rights::WRITE | abi::rights::GRANT,
                    spawnproto::RUN_UNVOUCHED_SLOT,
                ));
                // The two blob lengths, in `x0` and `x1`, which is the rest of that contract: a
                // mapping with no length is a slice this process cannot bound.
                must_ok(start_child(
                    login_child,
                    care_blob.len() as u64,
                    table.len() as u64,
                    0,
                ));
                // login holds its own copies now; ours were only ever the means of wiring.
                cap_delete(login_request);
                cap_delete(login_result);
                cap_delete(verify);
                cap_delete(verify_page);
                cap_delete(login_ut);
                cap_delete(audit);
            }
        }
    }

    // **Give the construction budget away** (milestone 22, the interactive increment). Two bounded
    // carves and then the root itself: after this line the progenitor can spend at most `INIT_OWN_PAGES` on
    // itself and `JOBS_BUDGET_PAGES` on the prompt's jobs, and it can no longer reach the rest of the
    // memory the kernel handed it or delegate the root to anything it builds.
    //
    // Two budgets rather than one, and the split is load-bearing rather than tidy: the job pool's
    // watermark must move for **jobs only**, or the LIFO return-of-pages (§16) never fires. A scratch
    // page table carved out of the same region between a job's split and its reap would sit above
    // that job's run, so the reclaim would find it is not the top and give back nothing.
    // **The boot prompt holds the run-unvouched capability** (DECISIONS §219 gate D2), and this is
    // the one line in this file that decides it. calef ruled it 2026-09-26 (DECISIONS §221 (the
    // boot prompt is the owner's console), ruling 1): the boot prompt is the owner's root console,
    // and whoever holds it is the owner, as with single-user or recovery mode elsewhere. Deleting
    // this call makes the prompt refuse every unvouched image, as it did before D2 existed.
    // `WRITE` alone, so the shell can present it and can hand it to nothing it spawns.
    let run_unvouched =
        run_unvouched.unwrap_or_else(|| must(retype_obj(ut, abi::objtype::RENDEZVOUS)));
    must_ok(place_at(
        shell.tcb,
        run_unvouched,
        abi::rights::WRITE,
        spawnproto::RUN_UNVOUCHED_SLOT,
    ));

    let own_ut = must(memory_region_split(ut, INIT_OWN_PAGES));
    let jobs_ut = must(memory_region_split(ut, JOBS_BUDGET_PAGES));
    // The shell's output page, in our own space, so we can say what just happened. This mapping is
    // permanent (there is no unmap, and `PageFrame::REVOKE` would take the page from the shell too); see
    // this module's BUGS.
    // SAFETY: `invoke` traps to the kernel, which validates the capability and the method before
    // acting (user_mode_runtime's contract).
    if unsafe { invoke(term_out, abi::page_frame::MAP, INIT_OUT_VA, 1, ut) } != 0 {
        fail()
    }
    cap_delete(ut);

    // And prove it from the inside, on the two primitives that build things, before anything else
    // runs. `NoSuchSlot` (-1) rather than `NotPermitted` (-3) is the whole claim: the capability is
    // *gone*, not narrowed, so there is nothing there to name. This is `root_supervisor`'s proof at
    // the interactive prompt, and `script/swish-check` reads the sentence.
    // SAFETY: as above: the kernel validates the capability and the method.
    let frame = unsafe { invoke(ut, abi::memory_region::RETYPE, 0, 0, 0) };
    // SAFETY: as above: the kernel validates the capability and the method.
    let object = unsafe {
        invoke(
            ut,
            abi::memory_region::RETYPE_OBJ,
            abi::objtype::THREAD_CONTROL_BLOCK,
            0,
            0,
        )
    };
    announce(
        term_ep,
        if frame == -1 && object == -1 {
            b"progenitor: construction budget dropped; retype answers NoSuchSlot\n"
        } else {
            b"progenitor: construction budget NOT dropped; it can still build\n"
        },
    );
    // **And what the measurement decided** (milestone 104), on the same terms as the line above: a
    // claim about what the progenitor refuses is worth what the check behind it is worth, and only the progenitor can
    // run that check. The affirmative line is the load-bearing one. A measured boot's natural bug is
    // for the check to evaporate when the build step does not run, and a boot that says nothing
    // looks exactly like a boot that measured everything, so `script/swish-check` reads this
    // sentence and a boot that stopped measuring fails the gate instead of passing quietly.
    let mut buf = [0u8; SENTENCE];
    announce(
        term_ep,
        if refused_n == 0 {
            b"progenitor: every program measured against the archive table\n"
        } else {
            sentence(
                &mut buf,
                b"progenitor: measurement refused",
                &refused[..refused_n],
                b"; they cannot be spawned\n",
            )
        },
    );
    // **The entropy service's own outcome** (DECISIONS §120's 2026-08-26 amendment), said here
    // rather than where it was decided: `entropy_ready` was set long before this process had a
    // terminal at all (the whole reason the build happens first, at the top of this function; see
    // that block's own comment), and [`INIT_OUT_VA`] is not mapped into the progenitor's own space until the
    // line above this one first used it, so nothing earlier in this function could `announce`
    // regardless. Silent when there was nothing to build (no device, or no `entropy` program in
    // the archive), the same posture the sink adapter and the subtree caretaker already take for a
    // missing component.
    if entropy_ready {
        announce(
            term_ep,
            if entropy_from_instruction {
                &b"progenitor: entropy service up; the kernel built it on the CPU's seed instruction\n"[..]
            } else {
                &b"progenitor: entropy service up; drew real bytes from a virtio-rng device\n"[..]
            },
        );
    }
    // **The network stack's outcome** (milestone 590 (provisional)), said here for
    // `entropy_ready`'s
    // reason. The address is the lease `net_stack` reported, so the line is evidence rather than
    // an intention: DHCP completed through the confined NIC before the prompt existed.
    if let Some((_, lease)) = network {
        let mut line = [0u8; 80];
        let head = b"progenitor: network stack up; DHCP leased ";
        line[..head.len()].copy_from_slice(head);
        let mut n = head.len();
        n += render_ipv4(lease as u32, &mut line[n..]);
        line[n] = b'\n';
        announce(term_ep, &line[..n + 1]);
    }
    // **The generated login credential** (milestone 49's boot-wiring update), said here for
    // [`entropy_ready`]'s own reason: `login_ready` was decided long before this process had a
    // terminal. Printed exactly once, before the prompt, the shape a cloud image's generated
    // first-boot password already takes: this process's own copy was wiped the moment
    // `identity_provisioner` used it (see that block's own comment), so this line is the only
    // place the password exists once it scrolls past. Silent when there is no login stack at all
    // (no entropy, no filesystem, or one of the four programs missing/unvouched), the same posture
    // every other optional boot component already takes.
    //
    // **It used to say `init: login ready` and that was the false half of milestone 233.** The
    // measurement behind this line is `identity_provisioner` answering `IDP_RPT_OK`: a credential
    // store exists and it holds this identity and this secret. That is worth reporting and is kept
    // exactly as it was. What it is not is a statement that the `login` process is alive, and for
    // an unknown number of weeks it was not: this process started it with no way to reach the
    // archive it parsed at `_start`, so it faulted before serving anything while this line went on
    // announcing a service. A reader has no way to tell a sentence's evidence from its wording, so
    // the wording now names the evidence.
    //
    // **Nothing here can honestly claim more, and the claim moved rather than being dropped.**
    // Knowing `login` survived means `login` saying so, which is a message, an endpoint and a wait
    // this process does not have, and a boot that blocks on a child's readiness is a boot that
    // hangs when the child is the thing that is broken. The survival claim lives in
    // `script/swish-check` instead, which fails the boot if the kernel reported killing any user
    // thread; see `swish_check_leg`'s own `no user thread was killed` check.
    if login_ready {
        fn push(buf: &mut [u8; SENTENCE], n: &mut usize, src: &[u8]) {
            for &b in src {
                if *n < SENTENCE {
                    buf[*n] = b;
                    *n += 1;
                }
            }
        }
        let mut buf = [0u8; SENTENCE];
        let mut n = 0usize;
        push(
            &mut buf,
            &mut n,
            b"progenitor: login credentials provisioned -- identity '",
        );
        push(&mut buf, &mut n, DEMO_IDENTITY);
        push(&mut buf, &mut n, b"' password '");
        push(&mut buf, &mut n, &login_password);
        push(&mut buf, &mut n, b"' (shown once; use it now)\n");
        announce(term_ep, &buf[..n]);
    }
    cap_delete(term_ep);
    cap_delete(term_out);

    // 6. The undertaker, out of what is left of our own budget. Two capabilities and nothing else:
    // `READ` on the supervision endpoint, so it can free a job's memory and can never spend it, and
    // `WRITE` on the result endpoint, so it can say the one word a job the kernel killed cannot say
    // for itself (milestone 235, `grant_plan::spawnproto::JOB_FAULTED`). `WRITE` without `GRANT`:
    // the right to speak on that channel, not to hand it to anything.
    //
    // Slot order is the contract with the program, which names them `DEATHS` (0) and `REPORT` (1).
    let reaper = must(build_child(
        own_ut,
        own_ut,
        &reaper_elf,
        &ChildEndowment {
            caps: &[(deaths, abi::rights::READ), (result_ep, abi::rights::WRITE)],
            stack_pages: CHILD_STACK_PAGES,
            ..ChildEndowment::new(Retention::Nothing)
        },
    ));
    must_ok(start_child(reaper, 0, 0, 0));

    // Role 0 (the prompt), and `arg1` is the rights its directory capability carries. A shell told 0
    // holds no directory and says so at every verb that would need one. `arg2` is the clock slot
    // (milestone 86), which moves with whether this boot had a disk, so it is told rather than
    // assumed.
    must_ok(start_child(shell, 0, fs_rights, sh_clock_slot));

    spawn_service(
        Channels {
            spawn_ep,
            result_ep,
            deaths,
            own_ut,
            jobs_ut,
            clock_page: g.clock_page,
            config_page: g.config_page,
            // The terminal's sink, if this initrd carried an adapter to serve it. This is what a
            // declared second stream gets by default (DECISIONS §67): the shell names a file with
            // `2>` and otherwise the bytes go straight to the screen, through a process that can do
            // nothing else with them.
            term_sink: sink_elf.is_some().then_some(term_sink),
            fs,
            // **The client view of the entropy service, if this boot has one** (milestone 111).
            // The same endpoint the login stack drew its password from, kept rather than dropped,
            // so a program a *person* types can be endowed randomness the same way `date` is
            // endowed a clock. `None` here is not a boot failure: it is a `uuid` that says on its
            // second stream that it holds no entropy capability, which is the true sentence.
            entropy: entropy_client,
            // **The stack's client endpoint, if this boot built one** (milestone 590
            // (provisional)), `entropy`'s shape one service over.
            network: network.map(|(stack, _)| stack),
            catalogue,
            run_unvouched,
        },
        &progs,
        care_elf,
    )
}

/// The archive entry a spawnable program is loaded from.
///
/// It answered `None` for `rm` until 2026-08-17, because `rm` is endowed a **directory** capability
/// and the progenitor had deleted the file service during the boot, so there was nothing to attenuate. Keeping
/// the slot empty was the honest answer to that: spawning `rm` with nothing to remove from would be
/// the worst failure this model has, a program told to destroy something, holding nothing, saying
/// nothing. Milestone 31 phase 3 removed the cause rather than the symptom, so the exception is gone
/// and every spawnable program is loaded the same way.
fn archive_name(p: Prog) -> Option<&'static str> {
    Some(p.name())
}

/// **Place `cap`, narrowed to `rights`, at `slot` of an unstarted child's table** (`CAP_INSERT` with
/// an explicit target, which the ABI numbers from one). `false` if the slot was taken or the right
/// could not be granted.
fn place_at(tcb: u64, cap: u64, rights: u64, slot: u64) -> bool {
    user_mode_runtime::tcb_cap_insert(tcb, cap, rights, slot + 1) >= 0
}

// The slot the run-unvouched capability is placed in sits just under the kernel's reserved fault
// slot; `grant_plan` states the number without depending on `abi`, so the relation is held here.
const _: () = assert!(spawnproto::RUN_UNVOUCHED_SLOT == abi::fault::FAULT_EP_SLOT - 1);

/// Turn a `recv_cap` slot into `Some(slot)`, or `None` if the message carried no capability.
fn opt_cap(slot: u64) -> Option<u64> {
    if slot == abi::rendezvous::NO_CAP {
        None
    } else {
        Some(slot)
    }
}

/// Everything the spawn service holds for its whole life, so the loop's signature says what the progenitor's
/// remaining authority *is*: two channels, one supervision endpoint it only ever delegates from, and
/// two bounded budgets. The root construction budget is deliberately not in here; it is gone.
struct Channels {
    /// READ: the shell's `run` requests arrive here.
    spawn_ep: u64,
    /// WRITE: a child's answer channel, and our own spawn-failed sentinel.
    result_ep: u64,
    /// GRANT: placed `READ` in every job's reserved fault slot, so `job_undertaker` collects it.
    deaths: u64,
    /// Our own scratch budget: page tables for the loader's scratch window, and nothing else.
    own_ut: u64,
    /// The job pool. One region per job, split off here and returned here when the job is reaped.
    jobs_ut: u64,
    /// READ on the wall clock, endowed to a child whose manifest declares one (DECISIONS §43).
    clock_page: u64,
    /// READ on the inert-configuration page, endowed to a child whose manifest declares
    /// [`grant_plan::Manifest::config`] (DECISIONS §111). [`clock_page`](Channels::clock_page)'s
    /// twin in every respect: unconditional, read-only, and handed on only where the manifest asks.
    config_page: u64,
    /// WRITE-delegable: the endpoint the terminal's sink adapter serves, which is where a declared
    /// second stream goes when the command line named no file for it (DECISIONS §67). `None` when
    /// this initrd carried no adapter, and then a declaring child simply gets no second stream.
    /// This is authority to *print*, and nothing else: the adapter holds the terminal, we do not.
    term_sink: Option<u64>,
    /// **The file service and the page its clients share with it** (milestone 31 phase 3), or `None`
    /// on a boot with no disk. `WRITE | GRANT` on the endpoint: this is the directory capability a
    /// `fs_subtree_caretaker` attenuates, and the only reason the progenitor keeps it past the boot.
    ///
    /// It used to be dropped once the shell held its narrowed copy, with a comment saying why it
    /// would have to come back. This is that day. The shell's copy carries no `GRANT`, so the shell
    /// holds nothing it could hand a caretaker; the progenitor is the only process here that can build one,
    /// which is what made `rm` a refusal at the prompt for six weeks.
    fs: Option<Fs>,
    /// **A client view of the entropy service** (milestone 111), endowed to a child whose manifest
    /// declares [`grant_plan::Manifest::entropy`]. `None` on a boot that built no entropy service
    /// (no virtio-rng device, no `entropy` program in the initrd, or a service that did not answer
    /// `READY`), and then a declaring child is spawned with an empty [`grant_plan::ENTROPY_SLOT`]
    /// and says so on its second stream rather than drawing predictable bytes.
    ///
    /// [`deaths`](Channels::deaths)'s shape rather than [`clock_page`](Channels::clock_page)'s: an
    /// endpoint the progenitor holds and places a narrowed copy of, not a frame it maps. `WRITE` is what a
    /// child gets, which on an endpoint is the right to `CALL` and nothing more; the progenitor keeps the
    /// full-rights capability it retyped, and never receives on it.
    ///
    /// **The shell does not hold one.** The shell needs no randomness of its own (nothing it does
    /// as a builtin is unpredictable), and endowing it one would be an authority nothing uses,
    /// which is the same call `config_page` and `deaths` already make and the opposite of
    /// `clock_page`, which the shell holds because `time` measures with it.
    entropy: Option<u64>,
    /// **The network stack's client endpoint** (milestone 590 (provisional)), endowed to a child
    /// whose manifest declares [`grant_plan::Manifest::network`]. [`entropy`](Channels::entropy)'s
    /// shape exactly: the progenitor keeps the full-rights capability it retyped and never receives
    /// on it, and places `WRITE` for a declaring child. `None` on a boot with no NIC or no
    /// `net_stack`, and then a declaring child holds an empty [`grant_plan::NETWORK_SLOT`] and says
    /// so. The shell holds none, for `entropy`'s reason: nothing it does as a builtin reaches the
    /// network.
    network: Option<u64>,
    /// **The image's package catalogue**, measured (milestone 198 rung 3a's installer): one
    /// `<stem> <digest>` line per package the image vouches for (`package_archive::CATALOGUE`).
    /// Empty when the archive carried none or the table refused it, and then every
    /// `package install` is refused as not catalogued.
    catalogue: &'static str,
    /// **READ on the run-unvouched endpoint** (DECISIONS §219 gate D2): the only receive right on
    /// it. The boot shell and `login` hold `WRITE` at `spawnproto::RUN_UNVOUCHED_SLOT`, and `login`
    /// passes `WRITE` to each session it builds. A request that sets `spawnproto::Wiring::
    /// run_unvouched` is followed by one `SEND` on it, which this process takes with a `RECV`;
    /// arriving is the proof, because nothing but a holder can send here.
    ///
    /// One slot for the life of the boot, and the table had one left (23 of 24). It is retyped
    /// after `login`'s build, the peak, so it is not held across it; `script/swish-check`'s
    /// `capability slots:` line is what says whether that held.
    run_unvouched: u64,
}

/// The file service, as the progenitor holds it for the life of the boot.
///
/// A struct rather than two `Option<u64>` fields because the two are one fact: a boot either has a
/// filesystem or it does not, and `Some(ep)` with `None` page is not a state that can exist.
#[derive(Clone, Copy)]
struct Fs {
    /// The service endpoint, `WRITE | GRANT`. The directory capability, rooted at the image root.
    ep: u64,
    /// The frame every client of that service maps to trade bytes with it.
    page: u64,
}

/// The spawn service loop: serve the shell's `run` requests forever. The progenitor is the ELF loader the
/// shell directs; it inserts only what the shell endows, so a spawned program can reach nothing the
/// command line did not name.
///
/// Two shapes (`grant_plan::spawnproto`). A **normal** job: the shell sends the request and, if
/// `--mem` rode along, one delegated untyped; we split a region off the job pool, build the child in
/// it, endow it the result endpoint (and the budget) plus its supervision endpoint, and start it. A
/// **supervised** (interruptible) job: the shell leads the delegation with a job untyped and a shared
/// job frame; we build the whole child *from that untyped* (so the shell's region owns it and can
/// `DESTROY` it to tear it down, DECISIONS §24), map the job frame in, endow nothing else, start it,
/// and send `SPAWN_OK` once as the shell's go-ahead. The `progs` array is indexed by [`Prog::id`], so
/// it is [`grant_plan::PROG_COUNT`] long: a variant added to `grant_plan` without a slot here would
/// be an out-of-bounds read in the progenitor.
///
/// **Only the normal shape is supervised**, and that is not an oversight. An interruptible job's
/// region belongs to the shell, which tears it down itself on the second `^C` (§24's
/// forcible tier) and after a clean finish; endowing it a supervision endpoint here would put a
/// second party in the teardown path for memory that is not ours, racing the shell's `DESTROY`.
fn spawn_service(
    c: Channels,
    progs: &[Option<elf::Elf>; grant_plan::PROG_COUNT],
    care_elf: Option<elf::Elf>,
) -> ! {
    let Channels {
        spawn_ep,
        result_ep,
        deaths,
        own_ut,
        jobs_ut,
        clock_page,
        config_page,
        term_sink,
        fs,
        entropy,
        network,
        catalogue,
        run_unvouched,
    } = c;
    // Whether the file page is mapped here yet, for the activation set (first image request).
    let mut fs_mapped = false;
    loop {
        let (w0, w1, w2) = recv(spawn_ep);
        // **An edit to the activation set, not a spawn** (milestone 198 rung 3a's installer). Asked
        // first, because under this bit no other bit of word 2 means anything.
        if let Some(verb) = spawnproto::activation(w1, w2) {
            let (status, live) = activate(
                verb,
                w0,
                &Activating {
                    spawn_ep,
                    own_ut,
                    jobs_ut,
                    fs,
                    catalogue,
                    network,
                },
                &mut fs_mapped,
            );
            let (r0, r1, r2) = spawnproto::activation_reply(status, live);
            send(result_ep, r0, r1, r2);
            continue;
        }
        let arg = spawnproto::arg(w1);
        let mem_pages = spawnproto::mem_pages(w2);
        let wiring = spawnproto::wiring(w2);
        let interruptible = wiring.interruptible;
        // **Under `IMAGE_BIT` word 0 is a length, not a program** (DECISIONS §219 option D), so no
        // row of `progs` is consulted: the program is whatever the bytes are.
        let prog = if wiring.image {
            None
        } else {
            Prog::from_id(spawnproto::prog_id(w0))
        };

        // **The directory grant's two data messages, before any capability** (milestone 31 phase 3).
        // They are read here rather than inside the branch that uses them because the shell has
        // already sent them: a request that announced them and a progenitor that did not drain them would
        // leave the endpoint holding words the *next* command would read as its own.
        let grant = wiring.dir.then(|| (recv(spawn_ep), recv(spawn_ep)));

        // **The image's frames, after the data and before every other capability** (§219 D). The
        // child's region is split *first*, then the staging region, so the staging region is the
        // top of the job pool when it is destroyed below and its pages go back (a region returns
        // its pages to its parent only when it is the most recent carve, `memory_regions`'
        // `return_to_parent`). A plain line is the only shape the shell sends; anything else is
        // drained and refused.
        let image_region = if wiring.image && !interruptible && !wiring.dir {
            memory_region_split(jobs_ut, JOB_REGION_PAGES).ok()
        } else {
            None
        };
        let staging = if wiring.image {
            receive_image(
                spawn_ep,
                spawnproto::image_len(w0),
                own_ut,
                jobs_ut,
                image_region.is_some(),
            )
        } else {
            None
        };

        // Receive the delegated caps in protocol order: the interrupt pair first (job untyped, job
        // frame), then the sink, then the source, then the diagnostics, then the screen-narrowed
        // tail's completion endpoint (DECISIONS §106), then any --mem untyped. No promise, no
        // receive, so both sides stay in lockstep.
        let (job_ut, job_fr) = if interruptible {
            (opt_cap(recv_cap(spawn_ep).1), opt_cap(recv_cap(spawn_ep).1))
        } else {
            (None, None)
        };
        let sink = if wiring.sink {
            opt_cap(recv_cap(spawn_ep).1)
        } else {
            None
        };
        let source = if wiring.source {
            opt_cap(recv_cap(spawn_ep).1)
        } else {
            None
        };
        let diagnostics = if wiring.diagnostics {
            opt_cap(recv_cap(spawn_ep).1)
        } else {
            None
        };
        // **The narrowed tail's completion endpoint** (DECISIONS §106), in the same delegation
        // order as everything else: a fresh capability the shell minted and kept a copy of, so
        // the progenitor installs it as this child's fault target and the shell can `RECV` its exit instead
        // of draining bytes it will no longer see.
        let screen = if wiring.screen {
            opt_cap(recv_cap(spawn_ep).1)
        } else {
            None
        };
        let budget = if mem_pages > 0 {
            opt_cap(recv_cap(spawn_ep).1)
        } else {
            None
        };
        // **The run-unvouched capability, presented** (DECISIONS §219 gate D2): the last message
        // of any request that claimed it, taken whether or not this one turns out to need it, so
        // the caller's `SEND` is never left waiting. It arrived on the endpoint only a holder can
        // send on, which is all it proves and all it has to. See `spawnproto::RUN_UNVOUCHED_BIT`.
        let presented = wiring.run_unvouched && {
            recv(run_unvouched);
            true
        };

        // **Vouched or not** (§219 D), and **what the bytes declare** (milestone 597, provisional:
        // a program carries its manifest in an ELF note). A hit is endowed with the manifest its
        // own note carries, or `grant_plan::NO_NOTE_MANIFEST` if it carries none: the digest that
        // vouched covers the note, so installing the package installed its manifest. A miss is
        // built only for a caller that presented the run-unvouched capability, and then with
        // `grant_plan::UNVOUCHED_MANIFEST` whatever its note asks (§219: an unvouched note grants
        // nothing). Any other miss is refused with its own word, and bytes whose manifest forbids
        // this request (`grant_plan::image_manifest`, `image_request_fits`) with another.
        let mut unvouched = false;
        let mut refused_by_manifest = false;
        let mut image_manifest = None;
        let image_elf = staging.and_then(|_| {
            let bytes = staged_image(spawnproto::image_len(w0));
            let vouch = vouched(bytes, fs, own_ut, &mut fs_mapped);
            unvouched = !vouch;
            let elf = if vouch || presented {
                elf::Elf::parse(bytes).ok()
            } else {
                None
            };
            let elf = elf?;
            match endowed_image(&elf, vouch, arg, mem_pages) {
                Some(m) => {
                    image_manifest = Some(m);
                    Some(elf)
                }
                None => {
                    refused_by_manifest = true;
                    None
                }
            }
        });
        // **The one manifest everything below reads**, so an image is endowed from what
        // `endowed_image` decided and never from a `Prog` row: `prog` stays `None` for it, and no
        // row's authority can reach it by a path this line does not name.
        let manifest = if wiring.image {
            image_manifest
        } else {
            prog.map(|p| p.manifest())
        };
        let failure = if unvouched && !presented {
            spawnproto::SPAWN_UNVOUCHED
        } else if refused_by_manifest {
            spawnproto::SPAWN_REFUSED_BY_MANIFEST
        } else {
            spawnproto::SPAWN_FAILED
        };
        let elf = if wiring.image {
            image_elf.as_ref()
        } else {
            prog.and_then(|p| progs[p.id() as usize].as_ref())
        };
        // Read from the program's own declaration, not from the request: a clock is not something
        // the command line can designate, so there is no bit on the wire for it (`Manifest::clock`).
        let wants_clock = manifest.is_some_and(|m| m.clock);
        // Same reasoning, one authority over (milestone 126): a **process domain** is not something
        // a person designates either. There is no /proc to name and no pid space to scan, so what a
        // program may see is decided here, by which supervision endpoint the progenitor puts in its capability table.
        let wants_domain = manifest.is_some_and(|m| m.domain);
        // `clock`'s twin again: the inert-configuration page is the progenitor's to endow, not something a
        // command line can designate, so there is no bit on the wire for it either
        // (`Manifest::config`).
        let wants_config = manifest.is_some_and(|m| m.config);
        // `clock`'s family a fourth time, and the first member of it that is an endpoint rather
        // than a page (milestone 111). Randomness is not something a command line designates, so
        // there is no bit on the wire for it either; the program's own declaration is what decides,
        // which is what keeps a program's dependence on unpredictable bytes visible in what it
        // holds. A boot with no entropy service answers `None` here and a declaring child is born
        // with an empty slot, which is the state its second stream exists to report.
        let wants_entropy = manifest.is_some_and(|m| m.entropy);
        // And a fifth (milestone 590 (provisional)): a network is not something a command line
        // designates either.
        let wants_network = manifest.is_some_and(|m| m.network);

        if interruptible {
            // Build the whole child from the shell's job untyped, mapping the shared job frame; no
            // capabilities in its capability table (it reports through the frame and exits). SPAWN_OK is the
            // go-ahead the shell waits for before it starts watching the frame.
            let built = match (elf, job_ut, job_fr) {
                (Some(e), Some(job), Some(fr)) => build_child(
                    own_ut,
                    job,
                    e,
                    &ChildEndowment {
                        maps: &[(CHILD_JOB_PAGE_FRAME_VA, fr, abi::address_space::MAP_RW)],
                        stack_pages: CHILD_STACK_PAGES,
                        ..ChildEndowment::new(Retention::Nothing)
                    },
                )
                .ok(),
                _ => None,
            };
            match built {
                Some(child) => {
                    let ok = start_child(child, 0, arg, 0);
                    send(
                        result_ep,
                        if ok {
                            spawnproto::SPAWN_OK
                        } else {
                            spawnproto::SPAWN_FAILED
                        },
                        0,
                        0,
                    );
                }
                None => {
                    send(result_ep, spawnproto::SPAWN_FAILED, 0, 0);
                }
            }
        } else {
            // **A region of its own, split first now** (milestone 31 phase 3). It used to be split
            // just before the build; a directory grant needs it earlier, because the caretaker is
            // built out of the **same** region as the program it serves. That is DECISIONS §92's
            // decision and §40's mechanism: a child's resources come from its supervisor's region,
            // so one reclaim ends both and the caretaker cannot outlive the grant it carries.
            //
            // **A `std` program's region is bigger, because it is also the heap** (milestone 595
            // (provisional)); see [`grant_plan::STD_REGION_PAGES`] for why one region rather than
            // two. It covers a caretaker as well, so a directory grant does not change its size.
            // An image request never takes this arm: `grant_plan::image_can_carry` refuses a `std`
            // manifest for an image, whose region is sized before the frames (and so the note)
            // arrive (milestone 595's block, Follow-on).
            let std_layout = manifest.is_some_and(|m| m.runtime == grant_plan::Runtime::Std);
            let region = if wiring.image {
                // Split before the frames were taken; see `image_region` above.
                image_region
            } else {
                memory_region_split(
                    jobs_ut,
                    if std_layout {
                        grant_plan::STD_REGION_PAGES
                    } else if wiring.dir {
                        DIR_JOB_REGION_PAGES
                    } else {
                        JOB_REGION_PAGES
                    },
                )
                .ok()
            };

            // **The caretaker, built before the program it serves**, because the program's slot 0 is
            // the endpoint this returns. `None` means either that this is not a directory grant or
            // that the delivery failed, and `dir_failed` below is what keeps those apart: a grant
            // that could not be delivered must **not** spawn the program anyway.
            let narrowed = if wiring.dir {
                match (region, fs, care_elf.as_ref(), grant) {
                    (Some(r), Some(fs), Some(care), Some((care_words, _))) => {
                        build_caretaker(own_ut, r, care, fs, care_words)
                    }
                    _ => None,
                }
            } else {
                None
            };
            // A directory grant that produced no capability. The program is not built: it was told
            // to act on something and holds nothing, which is the one outcome this model must never
            // trade away.
            let dir_failed = wiring.dir && narrowed.is_none();

            // **Slot 0 is the output**, and milestone 50 is the whole of what changed here: it is
            // the shared result endpoint unless the shell delegated a sink, in which case the sink
            // goes there instead and the child never learns that anything is different. `>` and the
            // left of a `|` are this line.
            //
            // **Except behind a directory grant**, where the narrowed endpoint takes slot 0 and the
            // output moves to slot 1. That is not a second convention invented here: it is the
            // contract `components/src/rm.rs` already documents and the kernel's `start_granted_dir`
            // already wires, so one program means one thing in a guest test and at the real prompt.
            // The grant goes first because it is the authority the command line named, and the
            // output is what every program gets whether it named anything or not.
            //
            // Slot 1 is the input source when there is one, and otherwise the `--mem` untyped, which
            // is safe only because no manifest declares both today. `grant_plan` is where that stops
            // being true, and the order here is the contract; see notes/pipes.md's BUGS.
            //
            // **Except when the shell narrowed this tail to the screen** (DECISIONS §106): the same
            // default-routing shape as the clock and the diagnostic stream below, applied to slot 0.
            // The shell asked for this by delegating a `screen` completion endpoint rather than by
            // declaring anything the child can read back, so the child's own slot 0 is exactly as
            // opaque to it as a redirected one always was.
            let default_screen = term_sink.filter(|_| screen.is_some());
            let out = (
                sink.or(default_screen).unwrap_or(result_ep),
                abi::rights::WRITE,
            );
            // Six rather than five (one more than the tightest bound any manifest shipped today
            // reaches): `dir` takes two, and `clock`, `config`, `source` and `budget` are each one
            // more, so a hypothetical program declaring all of directory, clock, config, source and
            // budget at once would need six. No shipped manifest does (no program declares both
            // `clock` and `config`, or a directory grant alongside either), but the array is sized
            // for what the *fields* allow rather than for what the table happens to contain today,
            // which is `clock_map`'s own ordered-slot debt (notes/pipes.md's `BUGS`) one field over.
            let mut caps = [out; 6];
            let mut n = 1usize;
            if let Some(dir_ep) = narrowed {
                caps[0] = (dir_ep, abi::rights::WRITE);
                caps[1] = out;
                n = 2;
            }
            // **The clock, which nothing on the command line asked for** (milestone 51's wiring).
            // It comes from the manifest rather than from the request, because a person does not
            // designate a clock: `date` declares that it reads one, and the progenitor is the only process
            // here holding a page it could hand over. Before the source and the budget, so `date`'s
            // clock is slot 1, which is unambiguous only because no manifest declares a clock *and*
            // an input. That is the same ordered-slot debt notes/pipes.md's BUGS already records.
            if wants_clock {
                caps[n] = (clock_page, abi::rights::READ);
                n += 1;
            }
            // **The inert-configuration page, `clock`'s twin** (DECISIONS §111). Same reasoning,
            // same ordering rule: before the source and the budget, so `printenv`'s config page is
            // slot 1 exactly as `date`'s clock is, and for the identical reason (no manifest
            // declares `config` alongside an input either, today).
            if wants_config {
                caps[n] = (config_page, abi::rights::READ);
                n += 1;
            }
            if let Some(src) = source {
                // READ only. A pipe's reader must not be able to write back up its own input, which
                // would make a pipeline a two-way channel nobody asked for.
                caps[n] = (src, abi::rights::READ);
                n += 1;
            }
            if let Some(b) = budget {
                // Narrowed to WRITE: the child may spend it, not lend it.
                caps[n] = (b, abi::rights::WRITE);
                n += 1;
            }
            // **The declared second stream, at the slot the manifest names** (DECISIONS §67). Read
            // from the program's own declaration for the same reason the clock is: the shell knows
            // there is one (it minted the endpoint) and only the program knows where it goes. The
            // slot is high and explicit rather than next-in-line, because how many low slots this
            // child gets depends on what else the line granted, and a stream the program probes for
            // by number cannot move under it.
            let diag_slot = manifest.and_then(|m| m.output.diagnostics_slot());
            // **Where the second stream goes when the line did not say.** The shell delegates an
            // endpoint only for a `2>`, because that is the case it has to back a file for. With no
            // operator on the line the destination is the **terminal's own sink**, which is the progenitor's
            // to endow exactly as the clock is: the shell holds nothing it could hand over, and a
            // person does not designate a screen.
            //
            // That is also what keeps a redirected `date`'s complaint off the redirection. The
            // shell drains the output into the file and never sees these bytes at all.
            let default_diag = term_sink.filter(|_| diag_slot.is_some());
            // Either half missing means no second stream reaches the child, and it then says what it
            // has to say in-band, which is what every program did before §67.
            //
            // **Two named slots now, and they are placed the same way for the same reason.** The
            // second is a view over the domain this child is *about to be born into*: `deaths` is
            // the endpoint every job the progenitor spawns for this shell is supervised by, so a viewer handed
            // it sees exactly this shell's jobs, including itself, and nothing else on the machine.
            // The progenitor, the shell, the terminal and the filesystem server are all outside it, which is
            // the confinement claim and is checked by `kernel::user::survey_tests`.
            //
            // **The right is `ENUMERATE`, and it used to be `READ`** (fixed 2026-08-17). `READ` on a
            // supervision endpoint is also what `RECV` and `abi::rendezvous::REAP` take, so the old
            // grant would have let a viewer take a death message out from under `job_undertaker` or
            // collect a corpse, and only the viewer's own source code said it did not. A domain names
            // its members and does not act on them (calef, 2026-08-17); `capability::Rights::ENUMERATE`
            // is what makes that a property of the grant. notes/process-view.md carries the argument.
            let mut placed_buf = [(0u64, 0u64, 0u64); 4];
            let mut placed_n = 0usize;
            if let (Some(ep), Some(slot)) = (diagnostics.or(default_diag), diag_slot) {
                placed_buf[placed_n] = (slot, ep, abi::rights::WRITE);
                placed_n += 1;
            }
            if wants_domain {
                // `ENUMERATE` alone, deliberately. Granting `READ` here would hand a viewer
                // `RECV` and `REAP` on the supervision endpoint as well, which is authority to
                // collect a child rather than to name one. See `Rights::ENUMERATE`.
                placed_buf[placed_n] = (grant_plan::DOMAIN_SLOT, deaths, abi::rights::ENUMERATE);
                placed_n += 1;
            }
            // **The third named slot** (milestone 111), placed for the same reason the other two
            // are: how many low slots a child gets depends on what else the line granted it, and
            // `uuid` reads a fixed number.
            //
            // `WRITE` alone, and the narrowing is the grant rather than a formality. On a
            // rendezvous `WRITE` is the right to `CALL`, so a declaring child may ask the service
            // for bytes; `READ` would additionally let it `RECV`, which is to take another
            // client's request out from under the service, and `GRANT` would let it hand a random
            // source to anything it spawned. Neither is given, so the only thing this capability
            // authorizes is the one thing the manifest declared.
            if let (true, Some(ep)) = (wants_entropy, entropy) {
                placed_buf[placed_n] = (grant_plan::ENTROPY_SLOT, ep, abi::rights::WRITE);
                placed_n += 1;
            }
            // **The fourth named slot** (milestone 590 (provisional)), `entropy`'s narrowing for
            // `entropy`'s reasons: `WRITE` is the right to `CALL` the socket contract, and neither
            // `READ` (taking another client's request off the stack's endpoint) nor `GRANT`
            // (handing the network to a grandchild) rides with it. `unreachable_network_witness`
            // at the prompt is the check that this line runs only for a declaring child.
            if let (true, Some(ep)) = (wants_network, network) {
                placed_buf[placed_n] = (grant_plan::NETWORK_SLOT, ep, abi::rights::WRITE);
                placed_n += 1;
            }
            let placed: &[(u64, u64, u64)] = &placed_buf[..placed_n];
            let clock_map = [(CHILD_CLOCK_VA, clock_page, abi::address_space::MAP_RO)];
            let config_map = [(CHILD_CONFIG_VA, config_page, abi::address_space::MAP_RO)];
            // Both pages, which only `grant_plan::UNVOUCHED_MANIFEST` declares (§219 gate D2).
            let both_map = [clock_map[0], config_map[0]];
            // **The FS contract's shared page, for a program behind a directory grant.** The same
            // frame the caretaker maps and the same frame the FS server maps: one page for all three
            // parties, sound because every request on both hops is a blocking `CALL`, so the client
            // is parked inside its own call for the whole time the caretaker is using it.
            let dir_map = [(
                FS_CLIENT_PAGE_VA,
                fs.map_or(0, |f| f.page),
                abi::address_space::MAP_RW,
            )];
            // The region's own comment lives at the split above, which milestone 31 phase 3 moved
            // earlier so a caretaker could be built out of it. Everything the child is made of comes
            // out of that carve, and a single reclaim frees all of it; the clock frame and the FS
            // page are ours and are only *mapped* into the child, so they are untouched when the
            // region goes.
            // **Two extra mappings at most, today.** A program that declared a directory grant AND a
            // clock AND the config page would need three, and this chain only ever offers two; no
            // manifest reaches that combination (the directory program, `rm`, declares neither
            // clock nor config, and only `grant_plan::UNVOUCHED_MANIFEST` declares both clock and
            // config, with no directory), so the gap is unreached rather than closed. The same ordered-slot debt `wants_clock`'s own
            // comment above already names for `caps`, one structure over; see notes/pipes.md's
            // `BUGS`.
            let maps: &[(u64, u64, u64)] = if narrowed.is_some() {
                &dir_map
            } else if wants_clock && wants_config {
                &both_map
            } else if wants_clock {
                &clock_map
            } else if wants_config {
                &config_map
            } else {
                &[]
            };
            // **The std layout** (milestone 595 (provisional)): the same authorities, placed where
            // nife's `std` reads them instead of in order. Computed here, beside the native arrays it
            // replaces, so both shapes read from the one set of decisions above (which output, which
            // directory, whether the manifest declared a clock).
            let std_parts = match (std_layout, region) {
                (true, Some(r)) => Some(StdLayout::new(
                    r,
                    out,
                    narrowed.map(|ep| (ep, fs.map_or(0, |f| f.page))),
                    wants_clock.then_some(clock_page),
                    wants_config.then_some(config_page),
                    entropy.filter(|_| wants_entropy),
                )),
                _ => None,
            };
            let built = match (elf.filter(|_| !dir_failed), region) {
                (Some(e), Some(r)) if std_layout => std_parts.as_ref().and_then(|l| {
                    build_child(
                        own_ut,
                        r,
                        e,
                        &ChildEndowment {
                            caps: &l.caps,
                            placed: l.placed(),
                            maps: l.maps(),
                            // Born supervised exactly as a native job is, so `job_undertaker`
                            // collects it and the region comes back.
                            fault: Some(screen.unwrap_or(deaths)),
                            stack_pages: std_runtime_protocol::STACK_PAGES,
                            ..ChildEndowment::new(Retention::Nothing)
                        },
                    )
                    .ok()
                }),
                (Some(e), Some(r)) => {
                    // Born supervised: `deaths` goes in the reserved fault slot, where `START` reads
                    // it and clears it, so the job cannot forge messages on its own death channel.
                    // The declared second stream rides the same named-slot mechanism at the low slot
                    // its manifest picked; the two cannot collide, because the fault slot is last.
                    build_child(
                        own_ut,
                        r,
                        e,
                        &ChildEndowment {
                            caps: &caps[..n],
                            placed,
                            maps,
                            // **A screen-narrowed child is supervised by the shell's own fresh
                            // endpoint instead of `deaths`** (DECISIONS §106), so the shell can
                            // `RECV` its exit directly rather than racing the progenitor's reaper for the
                            // same message. Its memory still comes from `region` (unchanged, still
                            // this job pool), and REAP still returns it to this pool regardless of
                            // who holds the supervision endpoint (DECISIONS §26: the reclaimed
                            // region returns to its *builder*, not its supervisor). The trade is
                            // that this child is outside `deaths`'s domain for its short life, so
                            // it will not appear in a concurrent `ps`/`pgrep`; see this function's
                            // BUGS.
                            fault: Some(screen.unwrap_or(deaths)),
                            stack_pages: CHILD_STACK_PAGES,
                            ..ChildEndowment::new(Retention::Nothing)
                        },
                    )
                    .ok()
                }
                _ => None,
            };
            // **The staged copy is spent once the child is built**: `build_child` copied every
            // segment into the child's own region. Destroying the staging region revokes its
            // mapping here and returns its pages to the job pool (it is the pool's top carve).
            if let Some(st) = staging {
                supervision_protocol::memory_region_destroy(st);
                cap_delete(st);
            }
            let ok = match built {
                Some(child) => {
                    // **A program behind a directory grant is started with the grant's own three
                    // words** rather than with an integer, which is `rm`'s shape: a spec carrying
                    // the options and two words of name (`filesystem_protocol::grant`). The progenitor forwards what the
                    // shell packed and reads none of it; see `spawnproto::GRANT_WORDS`.
                    //
                    // **A `std` program is started with nothing in its argument registers**: nife's
                    // `_start` ignores all three, and `rm`'s grant words would mean nothing to it.
                    // How a `std` program is told what to do is DECISIONS §170 (how a foreign program
                    // is told what to do), which is open.
                    let (a0, a1, a2) = match grant {
                        _ if std_layout => (0, 0, 0),
                        Some((_, child)) => child,
                        None => (0, arg, 0),
                    };
                    start_child(child, a0, a1, a2)
                }
                None => false,
            };
            // The narrowed endpoint was only ever the means of wiring: the child holds its own copy
            // and the caretaker holds the other end. Dropped whether or not the build worked, so a
            // failed spawn does not cost this capability table a slot for the rest of the boot.
            if let Some(dir_ep) = narrowed {
                cap_delete(dir_ep);
            }
            // Our capability to the job's region goes back now. It was only ever the means of
            // building: since §32 the reap is a method on the supervision endpoint, so nothing in
            // this system holds a capability to a *live* job's memory. A build or a start that
            // failed leaves nothing running in the region, so we reclaim it here rather than wait
            // for a death that will never come.
            if let Some(r) = region {
                if !ok {
                    reclaim(r);
                }
                cap_delete(r);
            }
            // **A redirected child owes the shell no answer**, because its answer is going
            // somewhere else, so the shell has nothing to read and no way to find out that the
            // spawn failed. One ack closes that hole. An unredirected child is unchanged: the
            // child's own message is the shell's single read, and a failure is the sentinel.
            //
            // **A screen-narrowed child owes the same ack, for the same reason** (DECISIONS §106):
            // its output is not going to `result_ep` either, and its completion signal is the fault
            // endpoint above, not this one, so a build failure has to reach the shell here or not
            // at all.
            if wiring.sink || wiring.screen {
                send(
                    result_ep,
                    if ok { spawnproto::SPAWN_OK } else { failure },
                    0,
                    0,
                );
            } else if !ok {
                send(result_ep, failure, 0, 0);
            }
            // **A child that was never built cannot end its own second stream**, and the shell
            // drains that stream to `OP_EOF` before it reads anything else, so nothing would ever
            // come back. The progenitor closes it on the child's behalf. It is the same hole `SPAWN_OK`
            // closed for the output side, one stream over.
            if !ok && let Some(ep) = diagnostics {
                send(ep, byte_sink_protocol::eof(), 0, 0);
            }
        }

        // Drop our copies of every delegated cap: the child holds what it needs (the job frame is
        // mapped, the budget and the streams inserted), and the shell holds the originals it kept
        // (the job untyped for teardown, the pipe it minted). This keeps the progenitor's 16-slot capability table from
        // filling across a long session.
        for s in [job_ut, job_fr, sink, source, diagnostics, screen, budget]
            .into_iter()
            .flatten()
        {
            cap_delete(s);
        }
    }
}

// -------------------------------------------------------------------------------------------
// The thin shapes over the ABI. The loader itself is `supervision_protocol`'s, which is the tree's
// only one since milestone 96.
// -------------------------------------------------------------------------------------------

/// **A `std` child's endowment, placed where nife's `std` reads it** (milestone 595 (provisional)).
///
/// The progenitor's native spawn fills a child's capability table from slot 0 in a documented order,
/// output first. nife's `std` fixes a slot per authority instead (`crates/std_runtime_protocol`), and
/// the two disagree about slot 0 itself: a native child's output is a `std` child's heap budget. So
/// a `std` program built the native way takes its stdout endpoint for an allocator and dies on its
/// first `println!`. This is the other shape, built from the same decisions [`spawn_service`] has
/// already made (which output, which directory, which of the manifest's pages).
///
/// - slot 0: the job's own region, narrowed to `WRITE`, as the heap's budget. See
///   [`grant_plan::STD_REGION_PAGES`] for why it is the same region the child is built from.
/// - slot 1: the output, exactly the capability a native child would get at slot 0.
/// - slot 4 and a page at `FS_PAGE`: the caretaker's narrowed endpoint, if the line granted a
///   directory. The same frame the caretaker and the file server map, at the `std` address.
/// - slot 5 and a read-only page at `CLOCK_PAGE`, slot 7 and one at `CONFIG_PAGE`: the manifest's
///   clock and configuration pages.
/// - slot 6: the entropy service, `WRITE`, if the manifest declared it and this boot built one.
///
/// Slots 2 and 3, the network, stay empty: `grant_plan`'s
/// `a_std_program_declares_only_what_the_std_layout_can_hold` keeps any `std` manifest from asking.
///
/// Name: provisional.
///
/// # BUGS
///
/// - **The child holds `WRITE` on the region it is built in**, because that region is its heap
///   (`grant_plan::STD_REGION_PAGES` says why one region). `WRITE` on a region is also `SPLIT`,
///   and a region that has been split cannot be destroyed until its children are, so a program
///   that splits its own heap pins its job region: `job_undertaker` never reclaims it, and the
///   pool is one region smaller until reboot. nife's `std` never splits (its allocator only
///   `MAP`s), so this takes a program written to do it. Closing it needs a right that allows `MAP`
///   and not `SPLIT`, which is the syscall surface and an architect's call.
/// - **The directory half is built and never exercised at the prompt.** No `std` manifest declares
///   a directory yet, because which word on a line becomes a `std` program's directory is the
///   designation half of DECISIONS §170 (how a foreign program is told what to do). The kernel
///   harness proves the same slot and page from its side (`fs_service::start_std_full`).
/// - **The network half is not wired.** The progenitor would have to mint slot 3's socket-frame
///   budget as well as place slot 2, and nothing needs it yet.
struct StdLayout {
    caps: [(u64, u64); 2],
    placed: [(u64, u64, u64); 4],
    placed_n: usize,
    maps: [(u64, u64, u64); 3],
    maps_n: usize,
}

// `caps` lands in order from slot 0, so the two in-order slots must be 0 and 1. Checked here rather
// than assumed, because the contract crate could renumber them and this would build a child whose
// heap and output were swapped.
const _: () = assert!(
    std_runtime_protocol::MEMORY_REGION_SLOT == 0 && std_runtime_protocol::STDOUT_SLOT == 1
);

impl StdLayout {
    fn new(
        region: u64,
        out: (u64, u64),
        dir: Option<(u64, u64)>,
        clock: Option<u64>,
        config: Option<u64>,
        entropy: Option<u64>,
    ) -> Self {
        use std_runtime_protocol as rt;
        let mut l = StdLayout {
            caps: [(region, abi::rights::WRITE), out],
            placed: [(0, 0, 0); 4],
            placed_n: 0,
            maps: [(0, 0, 0); 3],
            maps_n: 0,
        };
        let place = |l: &mut StdLayout, slot: u64, cap: u64, rights: u64| {
            l.placed[l.placed_n] = (slot, cap, rights);
            l.placed_n += 1;
        };
        let map = |l: &mut StdLayout, va: u64, cap: u64, mode: u64| {
            l.maps[l.maps_n] = (va, cap, mode);
            l.maps_n += 1;
        };
        if let Some((ep, page)) = dir {
            place(&mut l, rt::FS_DIR_SLOT, ep, abi::rights::WRITE);
            map(&mut l, rt::FS_PAGE, page, abi::address_space::MAP_RW);
        }
        if let Some(page) = clock {
            place(&mut l, rt::CLOCK_SLOT, page, abi::rights::READ);
            map(&mut l, rt::CLOCK_PAGE, page, abi::address_space::MAP_RO);
        }
        if let Some(ep) = entropy {
            place(&mut l, rt::ENTROPY_SLOT, ep, abi::rights::WRITE);
        }
        if let Some(page) = config {
            place(&mut l, rt::CONFIG_SLOT, page, abi::rights::READ);
            map(&mut l, rt::CONFIG_PAGE, page, abi::address_space::MAP_RO);
        }
        l
    }

    fn placed(&self) -> &[(u64, u64, u64)] {
        &self.placed[..self.placed_n]
    }

    fn maps(&self) -> &[(u64, u64, u64)] {
        &self.maps[..self.maps_n]
    }
}

/// **Build a `fs_subtree_caretaker` for one directory grant and hand back the narrowed endpoint**
/// (milestone 31 phase 3, DECISIONS §92).
///
/// This is the whole of what "the command line is a grant expression" needed and did not have. The
/// FS service's unit of authority is a *directory* (§27) and `rm rmtree/rm-keep` says less than that,
/// so the narrowing is a **caretaker**: a process that holds the file service, descends once into the
/// granted directory asking for exactly the granted rights, and serves the same contract on an
/// endpoint of its own. The program then holds that endpoint and **nothing that names the FS
/// server**, so "it cannot reach a sibling directory" is a property of its capability table rather than of a
/// branch it is trusted to take.
///
/// `region` is the client's region, and everything here comes out of it; see
/// [`DIR_JOB_REGION_PAGES`] for why that is the lifetime rule rather than a convenience.
/// `care_words` are the caretaker's three `START` words exactly as the shell packed them.
///
/// # The handshake is what makes this safe to call from the progenitor
///
/// The progenitor has no second thread: it is this loop, and a `RECV` that never completes is a machine that
/// never takes another command. So the readiness endpoint is not an optimization, it is the thing
/// that bounds this call, and `fs_subtree_caretaker` answers `DESCENT_REFUSED` rather than trapping
/// precisely so that `rm nosuchdir/x` costs a refusal instead of the prompt.
///
/// # BUGS
///
/// **A caretaker that dies before it answers still parks the progenitor.** The handshake covers the refusal a
/// person can cause by typing a name that is not there; it does not cover an image whose caretaker
/// faults on its own stack, because a corpse sends nothing. Nothing in the ABI offers a receive with
/// a deadline, and giving the progenitor one would mean a second thread inside the process this system's whole
/// design keeps small. The exposure is a build defect rather than an input, and it is the same one
/// `kernel::user::fs_service::wait_for_caretaker` has carried since milestone 47.
fn build_caretaker(
    own_ut: u64,
    region: u64,
    care: &elf::Elf,
    fs: Fs,
    care_words: (u64, u64, u64),
) -> Option<u64> {
    let narrow_ep = retype_obj(region, abi::objtype::RENDEZVOUS).ok()?;
    let ready = retype_obj(region, abi::objtype::RENDEZVOUS).ok()?;
    // Its whole authority, and reading these three lines is reading it: the file service to
    // attenuate, the endpoint it will serve, and one place to say it is ready. No untyped, no clock,
    // no terminal, and nothing that could name another process.
    let built = build_child(
        own_ut,
        region,
        care,
        &ChildEndowment {
            caps: &[
                (fs.ep, abi::rights::WRITE),
                (narrow_ep, abi::rights::READ),
                (ready, abi::rights::WRITE),
            ],
            maps: &[(FS_CLIENT_PAGE_VA, fs.page, abi::address_space::MAP_RW)],
            stack_pages: CARETAKER_STACK_PAGES,
            ..ChildEndowment::new(Retention::Nothing)
        },
    );
    // **Deliberately unsupervised** (`fault: None`). A caretaker built into the client's region is
    // already collected by the client's reap, and giving it a fault slot on `deaths` would put a
    // second death message on that endpoint for a thread whose region the first reap had already
    // taken away: `job_undertaker` would then be asked to collect a tid the scheduler no longer
    // knows, which it reads as the kernel contradicting itself and traps on.
    let child = built.ok()?;
    let started = start_child(child, care_words.0, care_words.1, care_words.2);
    if !started {
        cap_delete(ready);
        cap_delete(narrow_ep);
        return None;
    }
    // The one bounded wait. `READY` means the descent succeeded and everything the client can reach
    // it will reach through the handle that one request minted.
    let (verdict, _, _) = recv(ready);
    cap_delete(ready);
    if verdict == filesystem_protocol::fixture::READY {
        Some(narrow_ep)
    } else {
        // A refused descent. The caretaker has already exited; the endpoint goes back, and the
        // caller answers the prompt without building the program.
        cap_delete(narrow_ep);
        None
    }
}

/// **Give a failed job's region back, retrying while something in it can still run.**
///
/// A `DESTROY` of a region holding a live thread is refused with §16's kill armed, and one
/// preemption later the retry succeeds; that is `sched::reclaim_region`'s documented contract and it
/// is why the shell's `^C` escalation is a loop. It reaches this path when a directory grant's
/// caretaker was built and the program behind it was not: the caretaker is parked in `RECV`, the
/// endpoint sweep wakes it, and a single attempt would leave [`DIR_JOB_REGION_PAGES`] spoken for
/// until the machine stops. That is the *out of memory* path, which is exactly where a leak hurts
/// most.
///
/// Bounded and silent on failure, which is not the same call `job_undertaker` makes and the
/// difference is who is running: that program's whole job is collecting, so giving up there means a
/// leak nobody will see and trapping is the loud answer. This is the spawn service, and a trap here
/// takes the prompt down over one command's memory. The pool is bounded and renewable, so a region
/// this could not reclaim costs later commands and does not end them.
fn reclaim(region: u64) {
    for _ in 0..RECLAIM_ATTEMPTS {
        if supervision_protocol::memory_region_destroy(region) {
            return;
        }
        user_mode_runtime::yield_now();
    }
}

/// How many times [`reclaim`] retries. Small, because the only resident it ever waits on is a
/// caretaker that has already been woken and doomed, and one preemption is enough.
///
/// # BUGS
///
/// **One preemption is enough and sixty-four attempts do not reliably buy one**, which is a
/// different claim from the one above and the measurement is not this crate's. `login_test_client`
/// carried this exact loop, over the same refusal, and it was measured on 2026-08-27 (see that
/// program's `destroy_with_retry` and notes/load-sensitive-assertions.md): a `yield_now` that finds
/// work on this core returns in about 130 us, so sixty-four of them can elapse in 8 ms, where the
/// preemption being waited for is a whole tick period (10 ms) away. Whether the count covers the
/// tick is a scheduling outcome the host decides, and at roughly 2x oversubscription it stopped
/// covering it in about one run in three. That client now waits on the property with a clock; this
/// loop was left as it is, deliberately, because the two failures are not the same size: a test
/// client that gives up early fails a run, and this gives up quietly and strands
/// [`DIR_JOB_REGION_PAGES`] until the machine stops. **The failure has never been observed here**,
/// and it would be invisible if it happened, which is the argument for a lane rather than for a
/// number: this wants the same clock-bounded wait and a way to notice when it expires.
const RECLAIM_ATTEMPTS: usize = 64;

/// Carve `pages` off `ut` into a new child untyped we can delegate (milestone 31). The SPLIT grants
/// full rights on the child, including GRANT, so a memory budget can be handed on. The error code
/// `supervision_protocol` returns is for the dropped-authority proof, which this crate takes from the
/// raw `invoke` instead, so it is dropped here.
/// **Build `net_stack` from the NIC trio and wait for its DHCP lease** (milestone 590
/// (provisional)). Returns `(stack, lease)`: the full-rights endpoint its clients `CALL`, which
/// this process keeps, and the IPv4 address `net_stack` reported, big-endian in the low 32 bits.
///
/// The endowment is `kernel::user::virtio_service::wire_net_server`'s, slot for slot, because that
/// is the one every network test proves: report `WRITE` (0), interrupt `READ` (1), transport
/// `WRITE` (2), heap budget `WRITE` (3), client endpoint `READ` (4), the DMA page at
/// [`NET_DMA_VA`], and `socket_protocol::NO_LISTEN_GRANT` as its third argument. The caller
/// deletes the trio; this deletes everything else it made except the endpoint it returns.
///
/// Failures trap, as the entropy build's do: every step here is a retype or a build out of this
/// process's own budget, and one failing means the boot's arithmetic is wrong, which is a defect to
/// see rather than a feature to go without.
fn build_net_stack(ut: u64, program: &elf::Elf, g: &BootEndowment) -> (u64, u64) {
    // SAFETY: `invoke` traps to the kernel, which validates the capability and the method.
    let mapped = unsafe {
        invoke(
            g.virtio_net_dma,
            abi::page_frame::MAP,
            NET_DMA_PEEK_VA,
            1,
            ut,
        )
    } == 0;
    must_ok(mapped);
    // SAFETY: just mapped read/write, one page; the offset is inside it and outside
    // `net_transport`'s rings and buffers (`kernel::user::VIRTIO_DMA_PHYS_OFFSET`'s own doc).
    let direct_memory_access_phys = unsafe {
        core::ptr::read_unaligned(
            (NET_DMA_PEEK_VA as *const u8)
                .add(VIRTIO_DMA_PHYS_OFFSET as usize)
                .cast::<u64>(),
        )
    };
    let stack = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
    let report = must(retype_obj(ut, abi::objtype::RENDEZVOUS));
    let budget = must(memory_region_split(ut, NET_STACK_BUDGET_PAGES));
    let child = must(build_child(
        ut,
        ut,
        program,
        &ChildEndowment {
            caps: &[
                (report, abi::rights::WRITE),
                (g.virtio_net_irq, abi::rights::READ),
                (g.virtio_net, abi::rights::WRITE),
                (budget, abi::rights::WRITE),
                (stack, abi::rights::READ),
            ],
            maps: &[(NET_DMA_VA, g.virtio_net_dma, abi::address_space::MAP_RW)],
            stack_pages: NET_STACK_STACK_PAGES,
            ..ChildEndowment::new(Retention::Nothing)
        },
    ));
    must_ok(start_child(
        child,
        NET_STACK_ROLE_SERVER,
        direct_memory_access_phys,
        socket_protocol::NO_LISTEN_GRANT,
    ));
    cap_delete(budget);
    // `net_stack`'s first and only message on this endpoint: the lease, sent with a blocking
    // `send`, so it enters its serve loop only once this receive has taken it.
    let (lease, _, _) = recv(report);
    cap_delete(report);
    (stack, lease)
}

/// Write `addr` (big-endian octets in the low 32 bits) as dotted decimal at the front of `out`;
/// the length. `out` must hold fifteen bytes.
fn render_ipv4(addr: u32, out: &mut [u8]) -> usize {
    let mut n = 0;
    for (i, octet) in addr.to_be_bytes().into_iter().enumerate() {
        if i > 0 {
            out[n] = b'.';
            n += 1;
        }
        let mut started = false;
        for div in [100u8, 10, 1] {
            let d = octet / div % 10;
            if d != 0 || started || div == 1 {
                out[n] = b'0' + d;
                n += 1;
                started = true;
            }
        }
    }
    n
}

// -------------------------------------------------------------------------------------------
// An image request: the executable's bytes as frames the caller owns (DECISIONS §219 option D,
// ruled 2026-09-26; milestone 198 (a package manager) rung 3a).
// -------------------------------------------------------------------------------------------

/// Where the progenitor maps the file service's shared page to read the activation set. Mapped
/// once, on the first image request, with page tables from its own budget. Clear of every other
/// window this process maps (`INIT_OUT_VA` and the three peek pages below it, and the loader's
/// scratch window, which starts at `0x1000_0000` and only grows).
const ACTIVATION_FS_VA: u64 = 0x0f40_0000;

/// **Where the progenitor keeps its own copy of an image**, [`spawnproto::IMAGE_MAX_PAGES`] pages
/// at most. A fixed window rather than more of the never-reused scratch, because every page mapped
/// here comes from a staging region *this process* destroys once the child is built, and a destroy
/// revokes the mapping (DECISIONS §13 (frame revocation)), so the next request finds the window empty again. The page
/// tables behind it come from `own_ut` and are reused.
const IMAGE_STAGING_VA: u64 = 0x0f80_0000;

/// **Take an image request's frames off the spawn endpoint and copy them into pages of our own.**
/// Returns the staging region holding the copy, or `None` if it could not be staged.
///
/// Every one of `pages` frames is received whatever happens, because the caller has already
/// committed to sending them and the grants it sends next must land in the `RECV_CAP`s that expect
/// them. A frame that arrives after staging failed is deleted unread.
///
/// # Why a copy, and why one slot
///
/// The caller keeps its own mapping of these frames, so hashing them in place would let it change
/// the bytes between the hash and the build (§219: "the progenitor hashes its own copy"). So each
/// frame is mapped read-only through the loader's never-reused scratch window, its capability is
/// deleted **before** the next thing is taken, and a page retyped from the staging region receives
/// the copy. At most one capability of the caller's is in this table at any moment, and the
/// staging page's own capability is deleted as soon as it is mapped: an image of any size costs one
/// transient slot plus the staging region's.
///
/// `stage` is false when the request cannot be built anyway (no region for the child, an
/// interruptible or directory-granted image), and then this only drains.
fn receive_image(spawn_ep: u64, len: u64, own_ut: u64, jobs_ut: u64, stage: bool) -> Option<u64> {
    let pages = spawnproto::image_pages(len);
    let staging = if stage && pages > 0 && pages <= spawnproto::IMAGE_MAX_PAGES {
        memory_region_split(jobs_ut, pages).ok()
    } else {
        None
    };
    let mut ok = staging.is_some();
    // The caller's frames each get a fresh scratch page, never reused: their mappings here are
    // revoked only when the *caller* reclaims them, which this process does not control.
    let peek = if ok {
        supervision_protocol::scratch_pages(pages)
    } else {
        0
    };
    for i in 0..pages {
        let Some(frame) = opt_cap(recv_cap(spawn_ep).1) else {
            ok = false;
            continue;
        };
        let (Some(st), true) = (staging, ok) else {
            cap_delete(frame);
            continue;
        };
        let theirs = peek + i * spawnproto::IMAGE_PAGE;
        // SAFETY: `invoke` is the syscall; the kernel checks `READ` on the frame and the region.
        ok = unsafe { invoke(frame, abi::page_frame::MAP, theirs, 0, own_ut) } == 0;
        // The mapping outlives the capability, and the slot is what is scarce.
        cap_delete(frame);
        if !ok {
            continue;
        }
        let ours = IMAGE_STAGING_VA + i * spawnproto::IMAGE_PAGE;
        ok = match retype_page_frame(st) {
            Ok(page) => {
                // SAFETY: as above; the page is ours, fresh, and mapped read/write.
                let mapped = unsafe { invoke(page, abi::page_frame::MAP, ours, 1, own_ut) } == 0;
                cap_delete(page);
                mapped
            }
            Err(()) => false,
        };
        if ok {
            let n = (len - i * spawnproto::IMAGE_PAGE).min(spawnproto::IMAGE_PAGE) as usize;
            // SAFETY: both pages were mapped just above, `theirs` read-only and `ours` read/write,
            // one page each, and `n` is at most a page. They cannot overlap: different windows.
            unsafe {
                core::ptr::copy_nonoverlapping(theirs as *const u8, ours as *mut u8, n);
            }
        }
    }
    match (staging, ok) {
        (Some(st), true) => Some(st),
        (Some(st), false) => {
            supervision_protocol::memory_region_destroy(st);
            cap_delete(st);
            None
        }
        _ => None,
    }
}

/// The staged copy [`receive_image`] made, as bytes.
fn staged_image(len: u64) -> &'static [u8] {
    // SAFETY: `receive_image` mapped and filled `image_pages(len)` pages from `IMAGE_STAGING_VA`,
    // and the caller destroys the staging region (revoking the mapping) only after its last use of
    // this slice, within the same iteration of the spawn loop.
    unsafe { core::slice::from_raw_parts(IMAGE_STAGING_VA as *const u8, len as usize) }
}

/// **Is this image vouched for?** Its SHA-256, looked up in the live generation of the activation
/// set on the file service (`activation_set`). `false` on a miss, and on every way of failing to
/// read the table: no disk, no `activation` directory, a `current` that does not parse, a
/// generation that is malformed or larger than a page. A table that cannot be read vouches for
/// nothing, which is `measured_boot`'s rule one table over.
///
/// The file page is the one the shell and every caretaker share with the server. That is sound for
/// the reason `build_caretaker` gives: the shell is parked in its `RECV` on the result endpoint for
/// the whole of a spawn, so nothing else is mid-request on it.
fn vouched(bytes: &[u8], fs: Option<Fs>, own_ut: u64, fs_mapped: &mut bool) -> bool {
    let Some(files) = FsCalls::map(fs, own_ut, fs_mapped) else {
        return false;
    };
    let digest = measured_boot::sha256(bytes);
    let d = files.named(
        fs_op::OPENDIR,
        fs_op::ROOT,
        activation_set::DIRECTORY,
        dir::READ,
    );
    if d < 0 {
        return false;
    }
    let mut table = [0u8; PAGE_BYTES];
    let hit = match files.live_generation(d as u64, &mut table) {
        Ok((_, n)) => core::str::from_utf8(&table[..n]).is_ok_and(|table| {
            matches!(activation_set::lookup_digest(table, &digest), Ok(Some(_)))
        }),
        Err(()) => false,
    };
    files.close(d);
    hit
}

/// **The manifest a file's bytes are endowed with, or `None` to refuse them** (milestone 597,
/// provisional). Their note is read with `elf::Elf::note` and decoded by `manifest_note`; a note
/// that is there and unreadable refuses the bytes, since running them as if they carried none would
/// endow a program with a manifest it did not declare. `grant_plan::image_manifest` then applies
/// the vouch (the shell's `caps` calls the same function to preview it), and the request's own
/// words must fit what results.
fn endowed_image(
    elf: &elf::Elf<'_>,
    vouched: bool,
    arg: u64,
    mem_pages: u64,
) -> Option<grant_plan::Manifest> {
    let declared = match elf.note(manifest_note::OWNER, manifest_note::MANIFEST) {
        Ok(None) => None,
        Ok(Some(d)) => Some(manifest_note::decode(d).ok()?),
        Err(_) => return None,
    };
    let m = grant_plan::image_manifest(declared, vouched).ok()?;
    grant_plan::image_request_fits(&m, arg, mem_pages).then_some(m)
}

use filesystem_protocol::{dir, fs as fs_op};

/// One page, the unit the file service trades bytes in, and the most a generation may hold.
const PAGE_BYTES: usize = spawnproto::IMAGE_PAGE as usize;

/// **The progenitor's calls on the file service**, through the page it maps at
/// [`ACTIVATION_FS_VA`]. Shared by [`vouched`], which reads the activation set, and [`activate`],
/// which writes it (milestone 198 rung 3a).
///
/// The page is the one the shell and every caretaker share with the server. That is sound for the
/// reason `build_caretaker` gives: the shell is parked in its `RECV` on the result endpoint for the
/// whole of a spawn or an activation, so nothing else is mid-request on it.
struct FsCalls {
    ep: u64,
}

impl FsCalls {
    /// Map the file page here, once, on first use. `None` on a boot with no disk, or if the map
    /// was refused.
    fn map(fs: Option<Fs>, own_ut: u64, mapped: &mut bool) -> Option<Self> {
        let fs = fs?;
        if !*mapped {
            // SAFETY: the syscall; read/write because names and data are written into this page.
            *mapped =
                unsafe { invoke(fs.page, abi::page_frame::MAP, ACTIVATION_FS_VA, 1, own_ut) } == 0;
        }
        mapped.then_some(Self { ep: fs.ep })
    }

    fn page() -> *mut u8 {
        ACTIVATION_FS_VA as *mut u8
    }

    /// A name-taking verb: the name goes at the page's start, its length in the request.
    fn named(&self, verb: u64, handle: u64, name: &str, w1: u64) -> i64 {
        let len = name.len().min(PAGE_BYTES);
        // SAFETY: the page is mapped read/write (`map`) and `len` is at most a page.
        unsafe { core::ptr::copy_nonoverlapping(name.as_ptr(), Self::page(), len) };
        call(self.ep, fs_op::req(verb, handle, len as u64), w1).0 as i64
    }

    fn close(&self, handle: i64) {
        call(self.ep, fs_op::req(fs_op::CLOSE, handle as u64, 0), 0);
    }

    /// Read a file from offset 0 into `out`, up to `out.len()` bytes (at most a page). `None` on an
    /// error.
    fn read(&self, handle: u64, out: &mut [u8]) -> Option<usize> {
        let want = out.len().min(PAGE_BYTES);
        let n = call(self.ep, fs_op::req(fs_op::READ, handle, want as u64), 0).0 as i64;
        if n < 0 {
            return None;
        }
        let n = (n as usize).min(want);
        // SAFETY: the server wrote `n` bytes (at most `want`, at most a page) at the page's start.
        unsafe { core::ptr::copy_nonoverlapping(Self::page(), out.as_mut_ptr(), n) };
        Some(n)
    }

    /// Replace a file's contents with `bytes`, a page at a time. `false` at the first short or
    /// refused write.
    fn replace(&self, handle: u64, bytes: &[u8]) -> bool {
        if call(self.ep, fs_op::req(fs_op::TRUNCATE, handle, 0), 0).0 as i64 != 0 {
            return false;
        }
        for (i, chunk) in bytes.chunks(PAGE_BYTES).enumerate() {
            // SAFETY: the page is mapped read/write and a chunk is at most a page.
            unsafe { core::ptr::copy_nonoverlapping(chunk.as_ptr(), Self::page(), chunk.len()) };
            let at = (i * PAGE_BYTES) as u64;
            let n = call(
                self.ep,
                fs_op::req(fs_op::WRITE, handle, chunk.len() as u64),
                at,
            )
            .0 as i64;
            if n != chunk.len() as i64 {
                return false;
            }
        }
        true
    }

    /// Open `name` in `parent` for writing, creating it if it is not there.
    fn open_or_create(&self, parent: u64, name: &str) -> i64 {
        let h = self.named(fs_op::OPEN, parent, name, 0);
        if h >= 0 {
            h
        } else {
            self.named(fs_op::CREATE, parent, name, 0)
        }
    }

    /// Descend into `name` in `parent` with every right, making it if it is not there.
    fn directory(&self, parent: u64, name: &str) -> i64 {
        let h = self.named(fs_op::OPENDIR, parent, name, dir::ALL);
        if h >= 0 {
            h
        } else {
            self.named(fs_op::MKDIR, parent, name, dir::ALL)
        }
    }

    /// Whether generation `number` has a file in the activation directory.
    fn generation_exists(&self, act: u64, number: u32) -> bool {
        let mut name = [0u8; 10];
        let h = self.named(
            fs_op::OPEN,
            act,
            activation_set::generation_name(number, &mut name),
            0,
        );
        if h >= 0 {
            self.close(h);
        }
        h >= 0
    }

    /// **The live generation**: its number and its table, read into `out`. `(0, 0)` when there is
    /// no `current` at all, which is a disk nothing was ever installed on and vouches for nothing.
    /// `Err` when there is a `current` and it, or the generation it names, cannot be read or is
    /// larger than a page: a table that cannot be read vouches for nothing, and must not be
    /// written over either.
    fn live_generation(&self, act: u64, out: &mut [u8; PAGE_BYTES]) -> Result<(u32, usize), ()> {
        let c = self.named(fs_op::OPEN, act, activation_set::CURRENT, 0);
        if c < 0 {
            return Ok((0, 0));
        }
        let mut line = [0u8; 16];
        let got = self.read(c as u64, &mut line);
        self.close(c);
        let number = got
            .and_then(|n| core::str::from_utf8(&line[..n]).ok())
            .and_then(activation_set::parse_current)
            .ok_or(())?;
        let mut name = [0u8; 10];
        let t = self.named(
            fs_op::OPEN,
            act,
            activation_set::generation_name(number, &mut name),
            0,
        );
        if t < 0 {
            return Err(());
        }
        let n = self.read(t as u64, &mut out[..]);
        self.close(t);
        // A page and not a byte more: a generation that fills one may have been cut short, and a
        // table that ends mid-line vouches for nothing. `activation_set`'s BUGS: forty entries fit.
        match n {
            Some(n) if n < PAGE_BYTES => Ok((number, n)),
            _ => Err(()),
        }
    }

    /// **Write generation `number` whole, then make it live, then make that durable.** The order
    /// is the whole of the crash story: a generation file is never rewritten, `current` names only
    /// a generation that was written completely, and the flip itself is a `RENAME` of a staged
    /// `current` onto the real one, so a reader meets the old line or the new one and never half
    /// of each. `table` is `None` for a rollback, whose generation already exists.
    fn commit(&self, act: u64, number: u32, table: Option<&[u8]>) -> bool {
        if let Some(table) = table {
            let mut name = [0u8; 10];
            let g = self.named(
                fs_op::CREATE,
                act,
                activation_set::generation_name(number, &mut name),
                0,
            );
            if g < 0 {
                return false;
            }
            let written = self.replace(g as u64, table);
            self.close(g);
            if !written {
                return false;
            }
        }
        let mut line = [0u8; 16];
        let Ok(n) = activation_set::format_current(number, &mut line) else {
            return false;
        };
        let staged = self.open_or_create(act, activation_set::CURRENT_STAGED);
        if staged < 0 {
            return false;
        }
        let written = self.replace(staged as u64, &line[..n]);
        self.close(staged);
        if !written {
            return false;
        }
        // RENAME: both names in the page, source first; the destination's handle and length ride
        // in the second word.
        let src = activation_set::CURRENT_STAGED.as_bytes();
        let dst = activation_set::CURRENT.as_bytes();
        // SAFETY: the page is mapped read/write and both names together are a few bytes.
        unsafe {
            core::ptr::copy_nonoverlapping(src.as_ptr(), Self::page(), src.len());
            core::ptr::copy_nonoverlapping(dst.as_ptr(), Self::page().add(src.len()), dst.len());
        }
        let renamed = call(
            self.ep,
            fs_op::req(fs_op::RENAME, act, src.len() as u64),
            fs_op::rename_dst(act, dst.len() as u64),
        )
        .0 as i64;
        if renamed != 0 {
            return false;
        }
        // Durable before the answer, because the answer is what a person acts on: "installed"
        // followed by a power cut that forgets it would be the prompt saying something untrue.
        // A device that cannot flush answers `EOPNOTSUPP`, and then the generation is written and
        // live and only as durable as the device makes it; that is recorded, not refused.
        call(self.ep, fs_op::req(fs_op::SYNC, fs_op::ROOT, 0), 0);
        true
    }
}

/// What [`activate`] needs from the spawn service, named so the call site says it.
struct Activating {
    spawn_ep: u64,
    own_ut: u64,
    jobs_ut: u64,
    fs: Option<Fs>,
    catalogue: &'static str,
    /// The network stack's endpoint, for [`spawnproto::Activation::Fetch`]. `None` on a boot with
    /// no stack, and then a fetch is refused as [`spawnproto::ActivationStatus::NoNetwork`].
    network: Option<u64>,
}

/// **Serve one activation request** (milestone 198 (a package manager) rung 3a's installer,
/// DECISIONS §208 (installing a package is granting it, and the activation set is versioned)).
/// Returns the reply's status and the generation live afterwards.
///
/// - **Install**: the package's frames are staged exactly as an image's are ([`receive_image`]), so
///   what is checked is this process's copy. `package_archive::installable` decides on the bytes:
///   the image's catalogue must vouch for the whole file, and the member named after the package
///   is the program. Its bytes go to `packages/<stem>/<program>`, a place per package version that
///   is never rewritten with other bytes, and a new generation records the program's digest.
/// - **Remove**: a new generation without the program. Its bytes stay where they are, so a rollback
///   can bring it back; nothing collects them yet.
/// - **Rollback**: `current` names the generation one below the live one. Nothing else is written.
/// - **Fetch**: the name's stem is looked up in the image's catalogue, the package is fetched over
///   the network stack this process built at boot ([`fetch`]), and what arrived is installed as
///   **Install** installs a file's bytes, with one more check: it must be the package asked for
///   (`package_archive::installable_as`).
/// - **Vouch** (DECISIONS §221 (the boot prompt is the owner's console)): the executable's frames
///   are staged as an install's are, and a new generation records this process's hash of its own
///   copy under the name that follows, marked `activation_set::OWNER`. Nothing is placed: the
///   digest vouches for the bytes wherever they are, and a rollback undoes it.
///
/// Each of the five ends in [`FsCalls::commit`], so the only thing that ever changes what runs is
/// one rename of `current`. **Who may do this** is whoever holds the spawn endpoint, which is the
/// boot prompt alone, and DECISIONS §221 ruled that whoever holds that prompt is the machine's
/// owner, who may also write `activation/` directly. `spawnproto`'s BUGS say what changes the day
/// another session holds a spawn endpoint.
fn activate(
    verb: Option<spawnproto::Activation>,
    w0: u64,
    a: &Activating,
    fs_mapped: &mut bool,
) -> (spawnproto::ActivationStatus, u32) {
    use spawnproto::{Activation, ActivationStatus as S};
    // The request's own trailing messages come off the endpoint first, whatever happens next, so a
    // refusal never leaves words behind that the next request would read as its own.
    let staging = match verb {
        Some(Activation::Install | Activation::Vouch) => {
            receive_image(a.spawn_ep, w0, a.own_ut, a.jobs_ut, true).map(|st| (st, w0))
        }
        _ => None,
    };
    let mut named = [0u8; filesystem_protocol::grant::MAX_NAME];
    // A vouch's name follows its frames, which `receive_image` has just taken.
    let named_len = if matches!(
        verb,
        Some(Activation::Remove | Activation::Fetch | Activation::Vouch)
    ) {
        let (lo, hi, len) = recv(a.spawn_ep);
        filesystem_protocol::grant::unpack_name(lo, hi, len as usize, &mut named)
    } else {
        0
    };
    let named = &named[..named_len];
    // What a fetch holds until the end of this request: the page it traded bytes with the stack
    // through, and the staged package. Destroyed in the reverse of the order they were split, below.
    let mut socket_region = None;
    let mut fetched = None;

    let outcome = (|| {
        let Some(verb) = verb else {
            return (S::Unknown, 0);
        };
        let Some(files) = FsCalls::map(a.fs, a.own_ut, fs_mapped) else {
            return (S::StoreFailed, 0);
        };
        let act = files.directory(fs_op::ROOT, activation_set::DIRECTORY);
        if act < 0 {
            return (S::StoreFailed, 0);
        }
        let act_h = act as u64;
        let mut old = [0u8; PAGE_BYTES];
        let answer = match files.live_generation(act_h, &mut old) {
            Err(()) => (S::StoreFailed, 0),
            // A live table that is not text is a table nothing can edit, the same answer as one
            // that cannot be read.
            Ok((live, n)) => match core::str::from_utf8(&old[..n]) {
                // The fetch comes after the live generation is read, so a refusal still names
                // what is in force, and before anything is written.
                Ok(table) if verb == Activation::Fetch => {
                    match fetch(a, named, &mut socket_region, &mut fetched) {
                        Ok(stem) => edit(
                            &files,
                            act_h,
                            Activation::Install,
                            live,
                            table,
                            fetched,
                            Some(stem),
                            a.catalogue,
                            named,
                        ),
                        Err(status) => (status, live),
                    }
                }
                Ok(table) => edit(
                    &files,
                    act_h,
                    verb,
                    live,
                    table,
                    staging,
                    None,
                    a.catalogue,
                    named,
                ),
                Err(_) => (S::StoreFailed, live),
            },
        };
        files.close(act);
        answer
    })();

    // The staged package first, then the socket page, because a region gives its pages back to
    // the job pool only when it is the most recent carve (`memory_regions`' `return_to_parent`).
    // Destroying the socket page's region also revokes it out of `net_stack`'s address space.
    for region in [
        staging.map(|(st, _)| st),
        fetched.map(|(st, _)| st),
        socket_region,
    ]
    .into_iter()
    .flatten()
    {
        supervision_protocol::memory_region_destroy(region);
        cap_delete(region);
    }
    outcome
}

/// The part of [`activate`] that decides and writes, with the live generation already read.
#[allow(clippy::too_many_arguments)]
fn edit(
    files: &FsCalls,
    act: u64,
    verb: spawnproto::Activation,
    live: u32,
    table: &str,
    staging: Option<(u64, u64)>,
    wanted: Option<&str>,
    catalogue: &str,
    named: &[u8],
) -> (spawnproto::ActivationStatus, u32) {
    use spawnproto::{Activation, ActivationStatus as S};
    // The next generation's number: the first one above the live generation with no file. After a
    // rollback the numbers above the live one are taken, and a generation is never rewritten.
    let next = || {
        let mut m = live + 1;
        while files.generation_exists(act, m) && m < u32::MAX {
            m += 1;
        }
        m
    };
    let mut new = [0u8; PAGE_BYTES];
    match verb {
        // Never reaches here: [`activate`] fetches, then edits as an `Install` of what arrived.
        Activation::Fetch => (S::Unknown, live),
        Activation::Rollback => {
            if live <= 1 {
                return (S::NoEarlier, live);
            }
            let back = live - 1;
            if !files.generation_exists(act, back) || !files.commit(act, back, None) {
                return (S::StoreFailed, live);
            }
            (S::Done, back)
        }
        Activation::Remove => {
            let Ok(program) = core::str::from_utf8(named) else {
                return (S::NotInstalled, live);
            };
            let n = match activation_set::without_entry(table, program, &mut new) {
                Ok(n) => n,
                Err(activation_set::Error::NotInstalled) => return (S::NotInstalled, live),
                Err(_) => return (S::StoreFailed, live),
            };
            let m = next();
            if !files.commit(act, m, Some(&new[..n])) {
                return (S::StoreFailed, live);
            }
            (S::Done, m)
        }
        // **The owner's vouch** (DECISIONS §221 ruling 1; §195 clause 3). The digest is this
        // process's hash of its own copy, the one a spawn of the same file computes, so the entry
        // is found by the next run of those bytes and by nothing else.
        Activation::Vouch => {
            let Some((_, len)) = staging else {
                return (S::Unknown, live);
            };
            let bytes = staged_image(len);
            let Ok(program) = core::str::from_utf8(named) else {
                return (S::NotExecutable, live);
            };
            if elf::Elf::parse(bytes).is_err() {
                return (S::NotExecutable, live);
            }
            let entry = activation_set::Entry {
                program,
                package: activation_set::OWNER,
                digest: measured_boot::sha256(bytes),
            };
            let n = match activation_set::with_entry(table, &entry, &mut new) {
                Ok(n) => n,
                Err(activation_set::Error::BadName) => return (S::NotExecutable, live),
                Err(_) => return (S::StoreFailed, live),
            };
            let m = next();
            if !files.commit(act, m, Some(&new[..n])) {
                return (S::StoreFailed, live);
            }
            (S::Done, m)
        }
        Activation::Install => {
            let Some((_, len)) = staging else {
                return (S::Unknown, live);
            };
            let bytes = staged_image(len);
            let decided = match wanted {
                // Fetched by name: it must also be the package asked for.
                Some(stem) => package_archive::installable_as(catalogue, stem, bytes),
                None => package_archive::installable(catalogue, bytes),
            };
            let got = match decided {
                Ok(got) => got,
                Err(package_archive::Refusal::NoProgram) => return (S::NoProgram, live),
                Err(_) => return (S::NotCatalogued, live),
            };
            let mut stem = [0u8; package_archive::STEM_LEN];
            let Ok(package) = package_archive::Package::parse(bytes) else {
                return (S::NotCatalogued, live);
            };
            let stem = package.stem(&mut stem);
            // The program's bytes, where a person can run them (DECISIONS §219 option D hashes
            // whatever they run, so where they live is a convenience, not a trust decision).
            let packages = files.directory(fs_op::ROOT, activation_set::PACKAGES);
            if packages < 0 {
                return (S::StoreFailed, live);
            }
            // `packages/<name>/<version>/<program>`: one component per field, because a prompt
            // component is at most sixteen bytes and a stem is longer. The architecture is this
            // machine's, and the stem the generation records still names it.
            let name = files.directory(packages as u64, package.name());
            let version = if name >= 0 {
                files.directory(name as u64, package.version())
            } else {
                -1
            };
            let placed = version >= 0 && {
                let f = files.open_or_create(version as u64, got.program);
                let ok = f >= 0 && files.replace(f as u64, got.bytes);
                if f >= 0 {
                    files.close(f);
                }
                ok
            };
            for h in [version, name, packages] {
                if h >= 0 {
                    files.close(h);
                }
            }
            if !placed {
                return (S::StoreFailed, live);
            }
            let entry = activation_set::Entry {
                program: got.program,
                package: stem,
                digest: got.digest,
            };
            let Ok(n) = activation_set::with_entry(table, &entry, &mut new) else {
                return (S::StoreFailed, live);
            };
            let m = next();
            if !files.commit(act, m, Some(&new[..n])) {
                return (S::StoreFailed, live);
            }
            (S::Done, m)
        }
    }
}

/// Where the progenitor maps the page it trades bytes with `net_stack` through during a fetch.
/// Clear of [`ACTIVATION_FS_VA`] (one page) and of [`IMAGE_STAGING_VA`], where the body goes.
/// Every page mapped here comes from a region [`activate`] destroys at the end of the request, so
/// the window is empty again for the next fetch.
const FETCH_SOCKET_VA: u64 = 0x0f50_0000;

/// **The socket number the progenitor fetches on**: the last one, because every program at the
/// prompt that dials out uses 0 (`network_echo_client`). The contract's socket numbers are shared by
/// every client of one stack (milestone 590 (the booted system starts its network stack)'s BUGS),
/// so this is a convention, not a partition; notes/packages.md's BUGS says what that leaves open.
const FETCH_SID: u64 = socket_protocol::MAX_SOCKETS as u64 - 1;

/// This machine's architecture as a package stem spells it.
const ARCHITECTURE: &str = if cfg!(target_arch = "aarch64") {
    "aarch64"
} else if cfg!(target_arch = "riscv64") {
    "riscv64"
} else {
    "x86_64"
};

/// **Fetch the package `name` over the network**, for [`spawnproto::Activation::Fetch`]
/// (milestone 198 (a package manager) rung 3a's fetch). Returns the stem the image's catalogue
/// names it by, with the package staged at [`IMAGE_STAGING_VA`] and its region and length in
/// `staging`, exactly where [`receive_image`] leaves a file's bytes; [`edit`] then installs it.
///
/// In order, and the order is the argument:
///
/// 1. **The catalogue first.** A name the image vouches for no package by is refused before a
///    connection is opened, so a person cannot make this process fetch anything the image would
///    not install, and the answer to a typo costs no network.
/// 2. **One page shared with the stack**, retyped from a region of its own and handed over with
///    `OP_ATTACH_PAGE_FRAME` on [`FETCH_SID`]. The region is destroyed when the request ends, which
///    revokes the page out of the stack too: nothing about a fetch outlives it.
/// 3. **`GET /<stem>.nifepkg`** from the package source (`socket_protocol::fixture`), read through
///    `http_response`, which holds only the head and refuses what it cannot read exactly. The body
///    lands in pages split once the declared length is known and found to fit
///    [`spawnproto::IMAGE_MAX_PAGES`].
///
/// **Nothing here decides whether the bytes may be installed.** A body that arrived whole is only
/// bytes; [`edit`] checks them against the catalogue as it checks a file a person pointed at, which
/// is why plain HTTP is enough on this rung (DECISIONS §195 (a reviewed recipe vouches for a
/// package)). What this adds to the progenitor is a parser of network input *before* that check:
/// `http_response`'s head reader, a fixed 2 KiB buffer, host-tested. notes/packages.md weighs it
/// against a fetching program.
fn fetch(
    a: &Activating,
    name: &[u8],
    socket_region: &mut Option<u64>,
    staging: &mut Option<(u64, u64)>,
) -> Result<&'static str, spawnproto::ActivationStatus> {
    use socket_protocol::fixture::{PACKAGE_PEER_HOST, PACKAGE_PEER_IP, PACKAGE_PEER_PORT};
    use socket_protocol::*;
    use spawnproto::ActivationStatus as S;

    let stem = core::str::from_utf8(name)
        .ok()
        .and_then(|name| package_archive::catalogued_stem(a.catalogue, name, ARCHITECTURE))
        .ok_or(S::NoSuchPackage)?;
    let stack = a.network.ok_or(S::NoNetwork)?;

    let region = memory_region_split(a.jobs_ut, 1).map_err(|()| S::FetchFailed)?;
    *socket_region = Some(region);
    let page = retype_page_frame(region).map_err(|()| S::FetchFailed)?;
    // SAFETY: `invoke` is the syscall; the page is ours and fresh, the window is clear (see
    // FETCH_SOCKET_VA), and the page tables come from our own budget.
    let mapped = unsafe { invoke(page, abi::page_frame::MAP, FETCH_SOCKET_VA, 1, a.own_ut) } == 0;
    let attached = mapped
        && user_mode_runtime::send_cap(
            stack,
            page,
            abi::rights::READ | abi::rights::WRITE,
            req(OP_ATTACH_PAGE_FRAME, FETCH_SID),
        ) >= 0;
    // The mapping and the stack's copy outlive this capability, and the slot is what is scarce.
    cap_delete(page);
    if !attached {
        return Err(S::FetchFailed);
    }
    let window = |off: u64| (FETCH_SOCKET_VA + off) as *mut u8;

    if call(stack, req(OP_OPEN_TCP, FETCH_SID), 0).0 != REP_OK {
        return Err(S::FetchFailed);
    }
    let got = (|| {
        // SAFETY: (and for every access through `window` below) the page is mapped read/write at
        // FETCH_SOCKET_VA for the whole of this request, and the stack writes it only while this
        // process is blocked in a `CALL` to it.
        unsafe {
            core::ptr::copy_nonoverlapping(PACKAGE_PEER_IP.as_ptr(), window(OFF_DST_IP), 4);
            core::ptr::copy_nonoverlapping(
                PACKAGE_PEER_PORT.to_le_bytes().as_ptr(),
                window(OFF_DST_PORT),
                2,
            );
        }
        if call(stack, req(OP_CONNECT, FETCH_SID), 0).0 != CONNECT_ESTABLISHED {
            return Err(S::FetchFailed);
        }
        let mut path = [0u8; 1 + package_archive::STEM_LEN + 8];
        let mut at = 0;
        for part in ["/", stem, ".nifepkg"] {
            path[at..at + part.len()].copy_from_slice(part.as_bytes());
            at += part.len();
        }
        let path = core::str::from_utf8(&path[..at]).map_err(|_| S::FetchFailed)?;
        let mut request = [0u8; 160];
        let n = http_response::get_request(PACKAGE_PEER_HOST, path, &mut request)
            .ok_or(S::FetchFailed)?;
        // SAFETY: as above; `n` is at most 160, well inside the payload area.
        unsafe { core::ptr::copy_nonoverlapping(request.as_ptr(), window(OFF_PAYLOAD), n) };
        if call(stack, req(OP_SEND, FETCH_SID), n as u64).0 != n as u64 {
            return Err(S::FetchFailed);
        }
        receive_body(a, stack, staging)
    })();
    // Closed before the answer is judged, so a failed fetch still gives the socket back.
    let _ = call(stack, req(OP_CLOSE, FETCH_SID), 0);
    got.map(|()| stem)
}

/// The response half of [`fetch`]: read until `http_response` says the body is whole, splitting the
/// staging region once the head has declared a length that fits, and copying each read's body into
/// it. `Ok` only for a complete `200` whose body is non-empty and fits an image.
fn receive_body(
    a: &Activating,
    stack: u64,
    staging: &mut Option<(u64, u64)>,
) -> Result<(), spawnproto::ActivationStatus> {
    use socket_protocol::*;
    use spawnproto::ActivationStatus as S;
    let max = spawnproto::IMAGE_MAX_PAGES * spawnproto::IMAGE_PAGE;
    let mut response = http_response::Response::new();
    let mut filled = 0u64;
    while !response.is_complete() {
        let (n, _) = call(stack, req(OP_RECV, FETCH_SID), 0);
        if n == 0 || n > DATA_MAX as u64 {
            // The peer went away, or the stack failed, before the body was whole.
            return Err(S::FetchFailed);
        }
        // SAFETY: the stack just wrote `n` bytes at OFF_PAYLOAD of the page mapped at
        // FETCH_SOCKET_VA, and it does not write it again until this process calls it.
        let read = unsafe {
            core::slice::from_raw_parts((FETCH_SOCKET_VA + OFF_PAYLOAD) as *const u8, n as usize)
        };
        let body = response.feed(read).map_err(|_| S::FetchFailed)?;
        if response.status().is_some_and(|s| s != 200) {
            return Err(S::FetchFailed);
        }
        if staging.is_none()
            && let Some(len) = response.content_length()
        {
            if len == 0 || len > max {
                return Err(S::FetchFailed);
            }
            *staging = Some((stage_pages(a, len)?, len));
        }
        if !body.is_empty() {
            // `feed` hands back no more body than the head declared, and the staging region was
            // sized from that declaration, so this copy stays inside it.
            // SAFETY: `stage_pages` mapped `image_pages(len)` pages read/write from
            // IMAGE_STAGING_VA, and `filled + body.len()` is at most `len`.
            unsafe {
                core::ptr::copy_nonoverlapping(
                    body.as_ptr(),
                    (IMAGE_STAGING_VA + filled) as *mut u8,
                    body.len(),
                );
            }
            filled += body.len() as u64;
        }
    }
    Ok(())
}

/// Split a staging region for `len` bytes and map every page of it at [`IMAGE_STAGING_VA`], as
/// [`receive_image`] does for a caller's frames. The region, or `FetchFailed` with nothing held.
fn stage_pages(a: &Activating, len: u64) -> Result<u64, spawnproto::ActivationStatus> {
    let pages = spawnproto::image_pages(len);
    let st = memory_region_split(a.jobs_ut, pages)
        .map_err(|()| spawnproto::ActivationStatus::FetchFailed)?;
    for i in 0..pages {
        let mapped = match retype_page_frame(st) {
            Ok(page) => {
                // SAFETY: as in `receive_image`: the page is ours, fresh, and mapped read/write.
                let ok = unsafe {
                    invoke(
                        page,
                        abi::page_frame::MAP,
                        IMAGE_STAGING_VA + i * spawnproto::IMAGE_PAGE,
                        1,
                        a.own_ut,
                    )
                } == 0;
                cap_delete(page);
                ok
            }
            Err(()) => false,
        };
        if !mapped {
            supervision_protocol::memory_region_destroy(st);
            cap_delete(st);
            return Err(spawnproto::ActivationStatus::FetchFailed);
        }
    }
    Ok(st)
}

fn memory_region_split(ut: u64, pages: u64) -> Result<u64, ()> {
    supervision_protocol::memory_region_split(ut, pages).map_err(|_| ())
}

/// **Say one sentence at the terminal**, through the line discipline, the way the shell does: stage
/// the bytes in the shell's output page (mapped here at [`INIT_OUT_VA`]) and `CALL` `OP_WRITE`.
///
/// The only thing this process ever prints, and it is called before the shell is started so nothing
/// else is writing that page. It exists for the negative control: a claim about what the progenitor can no
/// longer do is worth only as much as the check behind it, and only the holder can run that check.
fn announce(term_ep: u64, text: &[u8]) {
    let out = INIT_OUT_VA as *mut u8;
    for (i, &b) in text.iter().enumerate() {
        // SAFETY: the shell's output frame is mapped read/write here, and one line is far under a
        // page.
        unsafe { core::ptr::write_volatile(out.add(i), b) };
    }
    call(term_ep, proto::req(proto::OP_WRITE, text.len() as u64), 0);
}

// -------------------------------------------------------------------------------------------
// The measurement (milestone 104): the progenitor measures what the progenitor loads.
// -------------------------------------------------------------------------------------------

/// Read a program out of the archive and measure it against the table, or refuse it.
///
/// **The decision itself is [`measured_boot::verdict`], not this function** (milestone 246). It used
/// to be eighteen lines here, and this crate cannot be reached by a host test, so the branch that
/// sets `unvouched` on a substituted program was proved by nothing: changing its `true` to `false`
/// started an unvouched binary and turned nothing in the tree red. Moving the three-way answer into
/// `measured_boot` put it beside `verify_in_manifest`, which is where the rest of that rule already
/// lives, and left this line: an archive read and a call. `script/swish-check` still proves that the
/// boot *makes* the call, on both ISAs, with the table the kernel measured, and nothing about that
/// changed.
fn measured<'a>(fs: &nifefs::Fs<'a>, table: &str, name: &str) -> measured_boot::Verdict<'a> {
    measured_boot::verdict(table, name, fs.read(name))
}

/// The buffer one printed sentence is composed in. A page would fit, but the sentences are one line
/// each and a fixed frame-local buffer is what keeps [`sentence`] free of an allocator.
const SENTENCE: usize = 192;

/// Compose `prefix name name ... suffix` into `buf`, skipping empty names.
///
/// A hand-rolled `write!` because there is no allocator and `core::fmt` would drag its machinery
/// into a program whose whole job is to be small. It **truncates** rather than failing: a diagnostic
/// cut short still names the first thing that went wrong, and the alternative is a refusal that says
/// nothing at all.
fn sentence<'b>(
    buf: &'b mut [u8; SENTENCE],
    prefix: &[u8],
    names: &[&str],
    suffix: &[u8],
) -> &'b [u8] {
    fn push(buf: &mut [u8; SENTENCE], n: &mut usize, src: &[u8]) {
        for &b in src {
            if *n < SENTENCE {
                buf[*n] = b;
                *n += 1;
            }
        }
    }
    let mut n = 0usize;
    push(buf, &mut n, prefix);
    for name in names {
        if name.is_empty() {
            continue;
        }
        push(buf, &mut n, b" ");
        push(buf, &mut n, name.as_bytes());
    }
    push(buf, &mut n, suffix);
    &buf[..n]
}

/// Unwrap a `Result<_, ()>` or fault: a half-built system is not worth limping along. Generic since
/// DECISIONS §142, because `build_child` now returns a `Child` (a tcb plus what this builder keeps
/// over it) where it used to return a bare slot.
fn must<T>(r: Result<T, ()>) -> T {
    match r {
        Ok(v) => v,
        Err(()) => fail(),
    }
}

/// Fault unless the syscall succeeded.
fn must_ok(ok: bool) {
    if !ok {
        fail();
    }
}

/// Trap. The kernel prints the pc and kills the process, which is the legible way for a builder to
/// say that the system it was asked to build cannot exist.
fn fail() -> ! {
    supervision_protocol::fail()
}
