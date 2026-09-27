---
status: PARTIAL
raised: 2026-08-18
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 139. Drive the unsafe count down, and cinch the ratchet behind it

Minted 2026-08-18 by calef, immediately after folding the unsafe census into
milestone 134: *"Can we also create a milestone to drive down the unsafe metrics and cinch up the
ratchet?"*

Milestone 134's instrument (the census and the ceiling relation in `script/lint`)
has existed and been live all along; this milestone spent it for its first real reduction below.

<!-- prose-budget: exception. 13,846 words (wc -w, this marker included) against a 3,000-word
     cap. Ratified by calef on 2026-09-25 (UTC), who ruled that this block is not split. Reason:
     large and rarely referenced; the maintainer counted 2 files linking it that day. Roadmap blocks
     as a class stay under the cap, per §212 (a prose budget), which refused a class exemption.
     Marker syntax is PROVISIONAL until the prose-budget gate exists. -->

## What was built

The `MappedWindow` cluster, the milestone's first real reduction. Seven userspace programs
(`entropy`, `kbd`, `net_transport`, `multicast_dns_responder`, `socket_test_client`, `smb_server`, `ntp`)
each hand-rolled the same `r8`/`w8`/`r16`/`w16`/`r32` volatile-access functions over a DMA page or
a shared IPC frame, one hand-written `// SAFETY:` comment per function asserting the same
invariant ("this offset is inside the page the kernel mapped here") by hand at every call site --
the exact §94 (what) shape this milestone's own text names as the best available reduction.
`user_rt::mapped_window::MappedWindow` (new) holds that invariant once, at construction, and turns
every access into a bounds-checked call: a wrong offset used to be a silent out-of-bounds volatile
access and is now a panic naming the access, which is a real soundness improvement the hand-written
copies never had, not just a relocation.

Measured precisely from the diff (a before/after tree census is contaminated by unrelated
concurrent growth -- five days between this milestone's baseline and this reduction added 30
unrelated `unsafe` blocks elsewhere in the tree at roughly the tree's own rate): 32 `unsafe {`
blocks removed across the seven programs, 11 added (9 window constructions -- one per program
except `smb_server`, which needs two: one for its boot-wired FS channel sized to
`fs::TRANSFER_MAX`, one for its runtime-mapped socket frame -- plus the 2 generic `read`/`write`
methods inside `MappedWindow` itself). Net -21. `smb_server.rs` alone is flat (11 blocks before
and after), still a real reduction by this milestone's own criterion 2 (raw pointer arithmetic
replaced by a typed, bounds-checked abstraction) even though it does not move that file's own
count.

Density: 93.4 (799 blocks over 85,476 lines) immediately before this reduction, 90.8 (778 over
85,526) after -- essentially the density the milestone's own baseline recorded on 2026-08-18 (93.0),
despite five days of unrelated tree growth in between, which is what makes density rather than raw
count the number worth trusting. Full measurement history in `notes/unsafe-obligations.md`'s table.

The ratchet, cinched: `<!--count-at-most:unsafe-density-outside-arch-->` lowered from 100 to 97
in the same commit (`notes/unsafe-obligations.md`, `notes/counted-claims.md`,
`notes/register-of-measures.md`), 7 points of headroom above the 90.8 this reduction reached -- the
same absolute headroom the original 100-vs-93 ceiling carried, not a zero-headroom ceiling that
would fail the next legitimate unsafe block anywhere outside `arch/`. The reasoning for why this
measurement gets headroom (unlike `unsafe-thread-safety-claims`' zero-headroom ceiling) is recorded
in `notes/unsafe-obligations.md` beside the marker.

## Round 2 (2026-08-24): `user_rt`, `ipc`, and the broader `read_volatile`/`write_volatile` sweep

The three concrete next steps round 1 named, taken in the order it suggested.

`crates/user_rt`'s `SYS_INVOKE` round trip collapsed, a second real §94-shaped reduction. Six
methods (`recv`, `recv_cap`, `recv_fault`, `call`, `survey`, `list`), each duplicated once per
architecture, had each hand-rolled its own `asm!` block asserting the identical invariant
("`svc`/`ecall` traps to the kernel, which validates before acting") at a register layout that
differed only in which of the five return words the caller happened to read back: twelve
hand-written copies of one assertion. `invoke5` (new, private to the crate, one per architecture)
holds the trap once; every caller above it, including `invoke` itself, is now a safe wrapper with no
`asm!` of its own -- the exact "collapse N hand-written assertions of one invariant into one" shape
this block's own text names as the best available reduction. One honest behavioural note recorded in
`notes/unsafe-obligations.md` and in the code: three of the collapsed functions (`recv`, `recv_cap`,
`recv_fault`) used to leave one input register unset for the kernel to read as whatever value
happened to be there (harmless, since those methods read no input words); routing them through the
shared primitive means they now pass an explicit `0`, a strict tightening rather than a behaviour
change. Measured from the diff: 14 `unsafe {` blocks removed, 9 added, net -5, entirely inside
`crates/user_rt/src/lib.rs`.

`crates/inter_process_communication` read in full: no reduction found, and that is the milestone's own predicted outcome
for at least one target. Production code carries exactly three `unsafe` blocks, one each inside
`send`, `recv` and `remove_sender`, and each already asserts a genuinely different fact (which of
two queues, which node, under what caller contract) rather than the same fact copied three times --
there is no §94 shape to collapse here. The other 41 sites the crate's `unsafe {` count includes
(baseline "44 blocks") are doc examples, `#[cfg(kani)]` proof harnesses and `#[cfg(test)]` unit
tests, each deliberately exercising a distinct state-machine transition; collapsing those would
either be impossible (a test suite's whole value is that each call is a different scenario) or would
be exactly the "hides it behind a macro" anti-pattern this block refuses. The crate's own doc
comments already do the milestone's other job for comments rather than code: the shared proof
obligations are stated once, at the module and section level ("stated once here rather than
re-derived at each of the eleven/twenty-odd sites"), with each call site's own comment adding only
what is particular to it. Nothing to migrate; reported honestly rather than forcing a relocation to
move a number.

The broader `read_volatile`/`write_volatile` sweep round 1's own BUGS section asked for, run for
real this time. Grepping directly for `read_volatile`/`write_volatile` across `user/src/` (rather
than by the `r8`/`w8`/`r16` naming convention round 1 searched by name) surfaced roughly thirty
files; most turned out not to be the pattern (device register blocks with their own poll loops,
framebuffer/graphics code, or programs whose entire point is deliberately invalid or one-off memory
access -- see "What is still open" below for the accounting). One cluster was a clean, large match:
eight programs (`rm`, `fs_file_caretaker`, `sink`, `fs_subtree_caretaker`, `fs_nameset_caretaker`,
`login_test_client`, `fs_test_client`, `swish`) each hand-rolled a `put_page`/`get_page`-shaped
byte-copy loop over the page shared with the FS server, every one asserting "this VA is a mapped
page of this size" by hand in near-identical wording (`fs_nameset_caretaker` carries a second,
read-only window for its name set; `fs_test_client` carries five such helpers over one window sized
to `fs::TRANSFER_MAX` rather than one page). Migrated onto the existing
`user_rt::mapped_window::MappedWindow` (round 1's type, reused rather than duplicated, per this
block's own instruction). 21 `unsafe {` blocks removed, 10 added, net -11 across the nine files.
`fs_subtree_caretaker.rs` alone is flat (1 block before and after: one hand-rolled function traded
for one window construction), the same "still real by criterion 2" case `smb_server.rs` was in round
1: raw pointer arithmetic replaced by a typed, bounds-checked abstraction, even though the file's own
count does not move.

Combined round 2: 35 `unsafe {` blocks removed, 19 added, net -16. Measured from the diff against
this round's own base commit (`a269403e`), the same discipline round 1 used and for the same reason:
a tree-wide census is contaminated by unrelated concurrent growth. This round's paired measurement
happens to be uncontaminated regardless -- nothing else landed on this branch between the base commit
and this reduction, so the tree-wide census confirms the diff exactly: 792 blocks outside `arch/` at
the base commit, 776 after, precisely -16. Density (the ceiling's actual unit) moved only 90 to 89
truncated, because this reduction also removed lines (duplicated `asm!` blocks and SAFETY comments
along with the blocks themselves), so the denominator moved with the numerator for the first time
this ceiling has had to account for.

The ratchet, cinched again: `<!--count-at-most:unsafe-density-outside-arch-->` lowered from 97 to
96 in the same commit (`notes/unsafe-obligations.md`, `notes/counted-claims.md`,
`notes/register-of-measures.md`), keeping the same 7-point headroom the 100-vs-93 and 97-vs-90
ceilings both carried, now above the 89 this round reached.

## Round 3 (2026-08-24): `swish`'s remaining windows, `disk_surveyor`'s flag, `net_stack`'s cluster,
and two investigations that ended in a recommendation rather than a migration

Round 2's own handoff list named five items precisely enough that this round did not have to
re-derive a net; it took the three it named as clear migrations and investigated the two it named as
open questions.

`swish.rs`'s remaining two windows. `OUT_VA`/`LINE_VA` (`stage`/`read_line`, the shell's terminal
pages) and the job frame `spawn_interruptible`/`watch` signal through (`jf_load`/`jf_store`,
parametrized by a runtime `va`, "actually a *better* `MappedWindow` fit than the FS cluster, since
`new` already takes a runtime base" in round 2's own words). The terminal pair is flat by block count
(2 removed, 2 added at the two `const` window declarations) and still a real reduction by criterion 2
-- the "typed abstraction replaces raw pointer arithmetic" case `smb_server.rs` and
`fs_subtree_caretaker.rs` were in rounds 1 and 2. The job frame collapses for real: `jf_load`/
`jf_store` were two functions with their own `// SAFETY:` comments, called eight times combined
across `spawn_interruptible` and `watch`; one `MappedWindow`, constructed once right after the frame
is mapped, replaced both. 4 `unsafe {` blocks removed, 3 added, net -1, in `components/src/swish.rs`
alone.

`disk_surveyor.rs`'s `ROSTER_VA`. A single shared `u64` flag at a fixed VA the program maps
itself at runtime (`Frame::MAP`, not a boot-time wiring), read once in `ROLE_HOLDER`, read again
after the kernel deliberately revokes the mapping (the module's own negative control: the second
read must fault), and written once in `ROLE_PROBE` (refused by the kernel; the mapping is read-only).
The two deliberate-fault sites are the one honest exception recorded where a reader meets it:
`MappedWindow`'s bounds check cannot catch either fault, because offset 0 is inside the declared
window both times, so the real hardware fault happens inside `read`/`write` at exactly the access the
hand-written version made, and the test's behaviour is unchanged by the migration. 3 `unsafe {`
blocks removed, 2 added, net -1, in `components/src/disk_surveyor.rs` alone.

`net_stack.rs`'s `a_r8`/`a_r16`/`a_w16`/`a_w8` cluster, the exact naming variant
`user_rt::mapped_window`'s own doc comment already named as a shape round 1's search should have
caught and did not. Genuinely harder than the FS cluster, as round 2 flagged: the VA is not a fixed
constant but `socket_va(sid) = 0x00A0_0000 + sid * 0x1000`, a different page per open socket, and
every caller computed an absolute VA (`sk.va + OFF_X`) rather than holding a `(window, offset)` pair.
Migrating cleanly meant restructuring the socket-lifecycle state itself, not just swapping the four
functions' bodies for wrapper calls that still took a raw absolute VA (which would not have moved the
invariant at all, per this milestone's own instruction): `Sock.va: u64` (0 meaning "no frame") became
`Sock.window: Option<MappedWindow>` (`None` meaning the same thing), and the parallel `frame_va:
[u64; MAX_SOCKETS]` array became `frame_window: [Option<MappedWindow>; MAX_SOCKETS]`, constructed
once in `OP_ATTACH_FRAME` right after the kernel maps the frame -- the one place in the whole socket
lifecycle that needs to assert the invariant, instead of every one of the four functions' bodies.
Every downstream call site (`read_dst`, `udp_sendto`, `sock_recv`, `tcp_connect`, `tcp_accept`,
`udp_bind`, `tcp_send`) now takes or holds a `MappedWindow` rather than a raw VA, so the
restructuring reaches the caller side. One further site collapsed for the same reason though it was
never named `a_w8`: `sock_recv`'s payload-write loop had its own hand-rolled `write_volatile`,
identical in shape, folded into the same window. 5 `unsafe {` blocks removed (the four functions'
bodies plus the hand-rolled loop), 1 added (the window construction in `OP_ATTACH_FRAME`), net -4,
in `components/src/net_stack.rs` alone. `script/test`'s aarch64 and riscv64 net suites (DHCP, UDP, TCP
connect/accept/listen, the mDNS responder) passed clean, which is the load-bearing evidence here: the
restructuring touches per-socket lifecycle state, exactly the kind of change where a mistake shows up
as a flaky network test rather than a compile error.

Combined round 3: 12 `unsafe {` blocks removed, 6 added, net -6. Measured from the diff against
this round's own base commit (`f731894d`), uncontaminated: nothing else landed on this branch between
the base commit and this reduction, so the tree-wide census confirms it exactly: 776 blocks outside
`arch/` at the base commit (89 per 10,000, matching round 2's own final reading), 770 after, exactly
-6. Density moved 89 to 88 (truncated); the line count moved by only 15, mostly the comments
explaining the new windows, so the denominator barely moved this round, unlike round 2's
`asm!`-collapse.

The ratchet, cinched a third time: `<!--count-at-most:unsafe-density-outside-arch-->` lowered
from 96 to 95 in the same commit (`notes/unsafe-obligations.md`, `notes/counted-claims.md`,
`notes/register-of-measures.md`), keeping the same 7-point headroom every ceiling in this milestone
has carried, now above the 88 this round reached.

Investigation: device-register blocks (`console.rs`, `input.rs`, `driver.rs`, `clock.rs`,
`jh7110_trng.rs`) -- genuinely per-driver distinct, and `MappedWindow` is the wrong fit; a stronger
idiom already lives in this tree and is unused by these five files. Read in full. `driver.rs` is
already a different idiom (a raw `*const u8` with `.add(offset)`, not a named accessor pair) and
needs nothing. `clock.rs`'s two RTC drivers (`pl031_unix_nanos`, `goldfish_unix_nanos`) are each a
single one-shot function with its own `// SAFETY:` comment, called once; there is no duplication
inside the file to collapse. `console.rs`, `input.rs` and `jh7110_trng.rs` each already collapse
their own register access to one or two functions (`uart_put`; `rd`/`wr`; `r32`/`w32`) with one
`// SAFETY:` comment apiece, reused at every call site -- the §94 shape is already applied *within*
each file. What is not collapsed is *across* files: the wording of each file's comment rhymes
("this VA is our device mapping, handed to us at spawn, for the whole lifetime of this process"),
which looks like the same invariant copied, but reading closely it is not: each asserts a genuinely
different fact (a different VA, a different device, a different offset table), the same shape
`crates/inter_process_communication`'s three call sites turned out to have in round 2 ("which of two queues, which node,
under what caller contract... there is no §94 shape to collapse here"). A shared `RegisterBlock`
type wrapping these would relocate the assertion from "a local `rd`/`wr` function" to "a shared
type's constructor," not collapse it, which is this milestone's own named anti-pattern.

But the investigation did not end at "nothing to do." The tree already has the right idiom for
this shape, in the kernel, for the very same two devices. `kernel/src/drivers/pl011.rs` and
`kernel/src/drivers/ns16550.rs` drive the identical PL011/NS16550 hardware these five userspace
programs also drive, using `tock-registers`' `register_structs!`/`register_bitfields!` macros: one
`unsafe` block for the whole driver (the base-pointer construction), and every register offset
checked at compile time rather than asserted by hand, which is a stronger property than
`MappedWindow`'s runtime bounds check gives (`kernel/src/drivers/pl011.rs`'s own comment: "an
off-by-four here is a build error rather than a mystery at runtime"). This is exactly the "what does
this tree already do in the analogous case" answer AGENTS.md's fork-readiness section asks for, and
it settles the "MappedWindow or a new RegisterBlock type" question this round's brief posed: neither.
`tock-registers` is already a tree dependency (§46's "thin architectural primitive" category), and
migrating `console.rs`/`input.rs`/`jh7110_trng.rs` onto it would be a real reduction with a stronger
property than either alternative -- but it is new scope this round did not take, for two reasons
worth naming rather than a guess at effort: adding `tock-registers` to the `user` crate's dependency
graph is a dependency decision (rule 6, DECISIONS §46) this round did not have standing to make
unilaterally, and the three files it would touch gate boot output and keyboard input, which is
exactly the kind of blast radius this round's brief said to weigh before attempting a risky
restructuring. Left as a named follow-on: migrate `console.rs`, `input.rs` and `jh7110_trng.rs` onto
`tock_registers::register_structs!`, matching `kernel/src/drivers/pl011.rs`'s and
`kernel/src/drivers/ns16550.rs`'s own shape; `clock.rs` and `driver.rs` need nothing, per above.

Correction, 2026-08-24 (round 5's lane, found by reading the file rather than trusting the
paragraph above): `kernel/src/drivers/ns16550.rs` does not use `tock_registers`. The two
paragraphs above claim it does ("drive the identical PL011/NS16550 hardware... using
`tock-registers`' `register_structs!`/`register_bitfields!` macros"), twice, and both are wrong.
`ns16550.rs`'s own module doc says plainly: "This one uses plain volatile access rather than
`tock_registers` register blocks: the register indices are what the 16550 defines, and the stride
between them is a runtime value no static layout macro can express." The JH7110's real UART spaces
its registers four bytes apart (`reg-shift = <2>`); QEMU's emulated NS16550 spaces them one byte
apart, carried as runtime data (`Shape`, since the VisionFive 2 prep) rather than a compile-time
layout, which is exactly what `register_structs!` cannot express. Only `pl011.rs` uses
`tock_registers` today. This does not overturn round 3's conclusion (the PL011 follow-on named
below is still real and still worth taking); it changes which half of `console.rs` and `input.rs`
that follow-on actually applies to, which round 5 (below) worked out per-file rather than assuming
"the identical hardware" meant identical treatment.

Decided, 2026-08-24: calef, in conversation: *"Take the dependency for user, launch the
lane."* `tock-registers` is now a dependency of the `user` crate (`user/Cargo.toml`), pinned to
`"0.10"`, matching `kernel/Cargo.toml`'s own pin so the two crates using this library never skew.
See round 5 below for what actually migrated, including the runtime-stride finding above that
narrowed it.

Investigation: framebuffer/graphics code (`display.rs`, `painter.rs`, `window.rs`, `compositor.rs`,
`display_terminal.rs`) -- one decisive structural finding, and the performance question narrowed but
not settled. Read in full. The part of this pipeline that would actually run at real per-frame
volume carries no per-pixel unsafe at all, migrated or not. `compositor.rs`'s `paint`/`serve_frame`
call `compositor::composite(screen(), &srcs[..n], n, damage)`, the crate's host-tested pure logic,
over ordinary safe `&mut [u32]`/`&[u32]` slices obtained by exactly one `unsafe` call per frame
(`screen()`, `source(i)`), not one per pixel: the compositor's hot loop already is the "assert the
invariant once, then use safe indexing" idiom `MappedWindow` generalizes, just spelled as a slice
rather than as that type. This is the decisive part of round 2's "is a bounds check per pixel write a
real performance question" framing, because the answer is: not here, since there is no per-pixel
check (bounds or otherwise) on this path today.

What remains is genuinely per-pixel `unsafe`: `painter.rs`'s and `window.rs`'s `px_write`/`px_read`
(a client painting and then digesting its own surface, `graphics_protocol::PIXELS` = 8,192 accesses per
run for `painter.rs`, up to 2,048 for the largest window in `compositor::SCENE`), `display.rs`'s
`surface_pixel` (the driver's own post-flush digest, also 8,192 accesses per run, sharing its
`dma_read`/`dma_write` pair with a few dozen one-off virtqueue-field writes that are not the
performance question), and `display_terminal.rs`'s `paint`, which is the one file among the five
where the question is live rather than one-shot: it repaints a damaged rectangle on every terminal
update, driven by keystrokes rather than a fixed frame rate, so its volume is bounded by typing speed
and paste size rather than by a boot-time test, unlike the other four. None of these is the
"thousands of writes per frame, sixty times a second" sustained path round 2's handoff named; that
path turned out not to exist in this file set. But "bounded and one-shot" or "bounded by typing
speed" is a structural characterization, not a measurement: this round did not obtain an actual
instruction-count or cycle number for a `MappedWindow`-checked pixel write versus the current raw
one, because no such micro-benchmark exists yet and building one was out of this round's reasonable
scope alongside the three migrations above. Left as a narrowed follow-on rather than a migration
on a guess: the question a next lane needs to answer is no longer "does a bounds check survive 60fps"
(it does not need to, because nothing here runs at 60fps) but "does a bounds check cost enough at
2,048-8,192 accesses per one-shot run, or per keystroke-driven repaint, to matter" -- a much smaller
question, worth `script/icount` or `script/bench` rather than reasoning about it further.

## Round 4 (2026-08-24): measuring `MappedWindow`'s bounds-check cost, then migrating the
framebuffer/graphics cluster round 3 narrowed to

Round 3's own conclusion set this round's exact scope: `painter.rs`'s and `window.rs`'s
`px_write`/`px_read`, `display.rs`'s `surface_pixel`, and `display_terminal.rs`'s `paint`, all
bounded (2,048-8,192 accesses per one-shot test run, or a keystroke-driven repaint far smaller than
that), none of them the sustained 60fps path the milestone's original framing worried about. The
open question was no longer structural, it was a number: does `MappedWindow`'s bounds check cost
enough at these volumes to matter, "worth `script/icount` or `script/bench` rather than reasoning
about it further" in round 3's own words. This round got that number, then migrated on it.

The measurement. A temporary comparison loop over a page-sized buffer, one raw `write_volatile`
against one loop performing `MappedWindow::check`'s own arithmetic first
(`off.checked_add(size).is_some_and(|end| end <= len)`), at three volumes: 56 (one glyph cell,
`bitmap_font::GLYPH_W * GLYPH_H`, `display_terminal.rs`'s smallest keystroke-driven repaint), 2,048
(`window.rs`'s largest `compositor::SCENE` surface) and 8,192 (`graphics_protocol::PIXELS`, the
one-shot digest volume `painter.rs`, `display.rs` and `window.rs`'s own largest surface all share).
Run via `script/bench` (icount, deterministic instruction counts, the right instrument for a
path-length question rather than a magnitude one; notes/benchmarks.md's own table names icount for
exactly this job), both ISAs:

| | aarch64 (ticks/access) | riscv64 (ticks/access) |
|---|---|---|
| raw `write_volatile` | 8 | ~1.4 |
| `MappedWindow`-checked | 12 | ~2.0 |
| delta | 4 | ~0.6 |

Flat across all three volumes on both ISAs (the check's cost does not depend on how many accesses
follow it, only on the arithmetic itself), so the total overhead scales linearly and stays small in
absolute terms even at the largest bounded volume: 8,192 accesses costs ~29,000 extra aarch64 ticks,
under 30 `ipc_rtt` round trips (1,017 ticks each) -- inside a test that already pays several real
round trips (at least one `CALL` to a driver or the compositor) and, for `display.rs`, a real device
DMA completion at ~200 us wall clock (`fs_read`'s own reading of what a completion-interrupt wait
costs). Negligible on both ISAs, settling round 3's open question: the bounds check is not worth
avoiding at any volume this cluster actually sees.

The comparison itself was not kept in the tree. It answered a one-time design question rather than
measuring an ongoing primitive worth regression-gating forever (the same reasoning `bench.rs`'s
`map_new` shootdown probe uses for staying a `bench-probe:` line rather than a baseline row), and
keeping it would have cost this milestone's own ceiling two more `unsafe {` blocks (one per
comparison loop) for a question that is now closed -- working directly against the number this round
exists to lower. The numbers above are the reproducible record instead.

The migration, on that number. All four sites round 3 named, migrated onto `MappedWindow`:

- `painter.rs`'s `px_write`/`px_read`: a `const WINDOW` at `SURFACE_VA`, sized to
  `gfx::SURFACE_BYTES`, the same shape round 1's cluster used. 2 `unsafe {` blocks removed, 1
  added, net -1.
- `window.rs`'s `px_write`/`px_read`: not a `const`, unlike `painter.rs`'s twin. The compositor's
  `spawn_client` maps a different frame count per client (`SCENE[i].frames()`,
  `kernel/src/user/compositor_service.rs`), not knowable at this program's compile time, so the
  window is constructed once in `_start`, sized to `bytes` (this client's own `w * h * 4`, already
  computed and bound-checked against `compositor::MAX_SURFACE_BYTES` before any pixel is painted --
  milestone 43, notes/shared-page-audit.md finding 4), and threaded through both call sites
  (`px_write`, and `px_read` via a closure into `compositor::surface_checksum`). The genuinely
  per-client, runtime-sized case `net_stack.rs`'s socket cluster was in round 3. 2 removed, 1
  added, net -1.
- `display.rs`'s `dma_write`/`dma_read`: round 3 scoped this to `surface_pixel` alone, but
  `surface_pixel` is one of dozens of callers of a shared pair of functions that already collapse the
  DMA region's invariant into one hand-written assertion apiece; migrating only `surface_pixel`'s own
  call would have duplicated that invariant (a second `MappedWindow` beside the untouched
  `dma_write`/`dma_read`) rather than collapsed it, the wrong direction for this milestone. Migrating
  the two shared functions instead -- a `const WINDOW` over the whole DMA region
  (`DMA_FRAMES * 4096` bytes), with `dma_write`/`dma_read`'s bodies becoming `WINDOW.write`/
  `WINDOW.read` -- covers `surface_pixel` for free and bounds-checks every one of the few dozen
  virtqueue-field one-off writes in the file too, which round 3's own text flagged as "not the
  performance question and could be migrated independently": here it came along for free rather than
  as separate scope. 2 removed, 1 added, net -1.
- `display_terminal.rs`'s `paint`: a window constructed once in `_start`, sized to
  `gfx::SURFACE_BYTES` in `MODE_DISPLAY` (this process maps that many bytes itself) or to
  `stride * h` in `MODE_WINDOW` (the compositor's own published geometry, the same per-client
  reasoning as `window.rs`'s), held in the `Wiring` struct rather than declared as a file-level
  `const` for the same reason `window.rs`'s is not one. 1 removed, 1 added, net 0 -- flat by
  block count, still a real reduction by criterion 2 (raw pointer arithmetic replaced by a typed,
  bounds-checked abstraction), the same case `swish.rs`'s terminal pair and `smb_server.rs` were.

Combined round 4: 7 `unsafe {` blocks removed, 4 added, net -3. Measured from the diff against
this round's own base commit (`757562a3`), uncontaminated: nothing else landed on this branch between
the base commit and this reduction, so the tree-wide census confirms it exactly: 782 blocks outside
`arch/` at the base commit (88 per 10,000, matching round 3's own final reading despite 12 blocks of
unrelated tree growth landing in between, 770 to 782 -- density absorbed it the same way round 1 and
round 2's readings did), 779 after, exactly -3. Density moved 88 to 87 (truncated);
the line count moved by +19 net (mostly the new `SAFETY` comments explaining each window's
invariant), so the denominator barely moved this round, like round 3's.

The ratchet, cinched a fourth time: `<!--count-at-most:unsafe-density-outside-arch-->` lowered
from 95 to 94 in the same commit (`notes/unsafe-obligations.md`, `notes/counted-claims.md`,
`notes/register-of-measures.md`), keeping the same 7-point headroom every ceiling in this milestone
has carried, now above the 87 this round reached.

The framebuffer/graphics investigation is now settled, not just narrowed. `compositor.rs`'s
per-frame hot path still carries no per-pixel `unsafe` at all (round 3's finding, unchanged); the
five genuinely per-pixel sites round 2 and round 3 identified are now four migrations and one
structural non-issue (`compositor.rs` itself needed nothing). Nothing about this cluster remains
open.

## Round 5 (2026-08-24): `tock-registers` for the fixed-layout device blocks, and the ns16550
correction narrows it to a per-file, per-architecture call

Round 3 named `console.rs`, `input.rs` and `jh7110_trng.rs` as one follow-on, on the strength of
"`kernel/src/drivers/pl011.rs` and `kernel/src/drivers/ns16550.rs` already use
`tock_registers`... for the identical hardware." That premise was checked before migrating
anything (a lane cannot answer "is the premise true" by reading its own brief) and turned out to
be half wrong: see the correction above. `ns16550.rs` was never migrated, for exactly the reason
its own module doc gives, the runtime-variable register stride `register_structs!` cannot express.
That correction changed the unit of analysis from "three files" to "which half of which file has a
compile-time-fixed layout," since `console.rs` and `input.rs` each carry two architecture-gated
halves (an aarch64 PL011 driver and a riscv64 NS16550 driver) behind one file name.

`console.rs`'s and `input.rs`'s aarch64 halves: migrated. Both drive the PL011 at a fixed
offset table (`DR`, `FR`; `input.rs` also `IMSC`, `ICR`), the same fixed layout
`kernel/src/drivers/pl011.rs` already verifies at compile time for the identical hardware, with no
runtime knob analogous to the NS16550's stride. Each file gained a local `register_structs!`/
`register_bitfields!` block (private to that file, not shared between the two, matching how they
already did not share their hand-written offset constants either) and a single-pointer-cast
`unsafe` function replacing the hand-rolled `read_volatile`/`write_volatile` pair.

`console.rs`'s and `input.rs`'s riscv64 halves: genuinely unsuitable, the same finding
`ns16550.rs` itself already made. Both hard-code the NS16550 register layout at QEMU's one-byte
stride (`THR`/`LSR` in `console.rs`; `RBR`/`IER`/`LSR` in `input.rs`), with no `Shape` parameter
and no way to vary it at runtime today. That is not a reason to migrate them: it is a reason not
to. The underlying hardware fact `ns16550.rs`'s module doc names, that this device family's
register stride is a fact the JH7110's real silicon and QEMU's emulation disagree on and that only
a runtime value (the device tree, eventually `Shape`) can resolve, holds for these two files
exactly as it holds for the kernel's own driver; they simply have not grown a `Shape` parameter
yet to make the disagreement visible. Encoding today's QEMU-only assumption into a
`register_structs!` block would not fix that gap, it would fossilize it behind a stronger-looking
mechanism: a compile-time-checked layout that is confidently wrong the day this driver runs
against real JH7110 hardware, which is a worse failure mode than the honest hand-written offsets
that at least invite a reader to ask "is this the right stride." Left unmigrated, on purpose,
matching `ns16550.rs`'s own precedent rather than round 3's "identical hardware" premise.

`jh7110_trng.rs`: migrated. Checked against the crate's own sourced register file
(`crates/jh7110_entropy::regs`, transcribed from `jh7110-trng.c`) and the device-tree binding
(`starfive,jh7110-trng`, `reg = <0x1600C000 0x4000>`) before assuming this was the file round 3's
brief warned it might be ("the one most likely to have this problem," being real-hardware-specific
code for a board this tree has not yet run against, milestone 159, `NOT-STARTED`, gate HARDWARE):
the binding gives one `reg` window with no `reg-shift`/`reg-io-width` knob, unlike the NS16550's
binding, so there is no runtime-variable aspect to this device's layout for `register_structs!` to
be the wrong tool for. `r32`/`w32` (two hand-written volatile-access functions, one `unsafe` block
apiece) collapsed into one `regs()` pointer-cast function reused by every accessor; the register
block names only the registers this driver touches (`CTRL`, `ISTAT`, `RAND0..RAND7`), with `STAT`,
`MODE`, `SMODE`, `IE`, `AUTO_RQSTS` and `AUTO_AGE` as reserved padding, the same "not otherwise
used" status the crate's own `regs` module already gives several of them. This has not run against
real silicon either way (see the crate's and the program's own module docs); the migration changes
nothing about that gap, only how the register offsets are checked.

Measured from the diff against this round's own base commit (`757562a3`): 5 `unsafe {` blocks
removed, 3 added, net -2 (`console.rs` flat at 1 before and 1 after -- still a real reduction by
rounds 1-3's own criterion 2, raw pointer arithmetic replaced by a compile-time-checked typed
abstraction, the same "flat but real" case `smb_server.rs`, `fs_subtree_caretaker.rs` and
`swish.rs`'s terminal pair were; `input.rs` 2 removed, 1 added, net -1; `jh7110_trng.rs` 2 removed,
1 added, net -1). Tree-wide census, base commit versus this round's tree: 782 blocks outside
`arch/` in 88,761 lines (88 per 10,000) before, 780 blocks in 88,812 lines (87 per 10,000) after --
the line count grew despite the block count falling, because the compile-time-checked layout costs
more lines in doc comments and macro invocations than the hand-written offsets and SAFETY comments
it replaced, the mirror image of round 2's `asm!`-collapse where both moved together.

*`jh7110_trng.rs` is `components/src/jh7110_entropy.rs` now (milestone 175's split, then calef's
two 2026-09-13 rulings, `jh7110_entropy_source` and then `jh7110_entropy`; the second was performed
2026-09-14). This round's sections spell it as it was when the blocks were counted, so the -1 stays
checkable against base commit `757562a3`, and `notes/unsafe-obligations.md` spells it the same way.*

The ratchet, cinched a fourth time: `<!--count-at-most:unsafe-density-outside-arch-->` lowered
from 95 to 94 (`notes/unsafe-obligations.md`, `notes/counted-claims.md`,
`notes/register-of-measures.md`), keeping the same 7-point headroom every ceiling in this milestone
has carried, now above the 87 this round reached. One honest caveat this round's own report
should carry rather than let a merge discover: a separate, concurrently-running round-4 lane
(`milestone/139-round4-graphics`) is measuring and migrating a different candidate set from the
same base commit at the same time. Whichever of the two rounds' pull requests lands second will
find this ceiling arithmetic stale (both rounds subtracted from the same starting density) and
needs to re-measure from the merged tree rather than trust either round's own before/after numbers
in isolation, the same discipline round 2's own report names for tree-wide census under concurrent
growth.

## Round 6 (2026-08-25): the `user/` survey this milestone's BUGS section asked the first lane
for, finished, plus two more migrations and one honest count-regression

This milestone's own BUGS section, verbatim: *"`user/`'s 285 is unexplained... nobody has read
enough of it to say whether that number is raw shared-page handling, a missing safe wrapper, or
something else."* Five rounds had already done most of that reading as a side effect of chasing
specific clusters; this round did the rest: read every remaining `unsafe` block in `user/`,
confirmed the census against `script/lint`'s own regex rather than a hand grep, and closed the one
named gap ("what is still open"'s `login_test_client.rs`'s `PAGE_VA`) plus five more files reading
found in the same shape.

The re-measured count. `user/` carries 284 `unsafe {` blocks today (replicating
`script/lint`'s exact stripping-and-counting regex, scoped to `user/*.rs`), against the milestone's
own opening baseline of 285 (2026-08-18) and the 287 a mid-milestone reading of this same section
recorded. Roughly flat, and that flatness is itself the finding: five rounds removed real
duplication (net -21, -16, -6, -3, -2 across rounds 1-5, most of it in `user/`), and unrelated tree
growth in `user/` over the same five weeks (milestone 176's COM1 IRQ wiring, milestone 158's
kernel-object renames, and ordinary feature work) put almost exactly that much back. The
tree-wide density ceiling is the number that shows the reduction rather than hiding it: 93.4 to 87,
even while the raw count outside `arch/` grew from 799 to (as of this round's start) 792 net-lower
despite five weeks of concurrent development. `user/`'s own raw count is not the number to read;
the density is.

The breakdown, by what the first non-comment token inside each block is:

| shape | blocks | what it is |
|---|---|---|
| `invoke(cap, method, ...)` | 123 | a raw capability invocation: the userspace syscall itself |
| `read_volatile`/`write_volatile` | 36 | device registers and shared-frame fields with no further collapse available (see below) |
| `core::arch::asm!` | 16 | entry stubs, the trap, and a handful of driver-specific instructions (`wfi`, `fence`) |
| `core::slice::from_raw_parts[_mut]` | 12 | whole-page slice construction; down from ~30 before this round's own migration |
| everything else | 97 | `MappedWindow`/`RegisterBlock`-family constructors (new, mostly this round: see below), the C ABI shim (`c_shim.rs`, `malloc`/`free`, already documented per milestone 82's survey), deliberate-fault test programs (`flaky.rs`, `outlaw.rs`, `hello.rs`'s `.bss`/`.data` probes), and single one-off writes (`memory_grant_depleter.rs`, `swapper.rs`) |

Two clusters migrated this round, on `MappedWindow`, the same primitive round 1 built.

*`user_rt::initrd::initrd_bytes` (new, provisional name).* Seven programs (`builder`, `c_confiner`,
`hello`, `login`, `root_supervisor`, `swapper`, `timetable`) each declared their own `const
INITRD_VA: u64 = 0x2000_0000` and their own `unsafe { core::slice::from_raw_parts(INITRD_VA,
initrd_len) }`, one hand-written `// SAFETY:` comment per file asserting the identical invariant
("the kernel maps `initrd_len` bytes of the initrd, read-only, at this VA, before `_start` runs").
`timetable.rs`'s own comment had already named the duplication out loud ("the same contract
`components/src/builder.rs` is started under") without anyone lifting it out, the same shape `ntp.rs`'s
comment named for round 1's cluster. One `unsafe fn` in `crates/user_rt/src/initrd.rs` now holds
that assertion once. Measured from the diff: 7 `unsafe {` blocks removed at the seven call sites,
7 added at the same sites (calling the shared function) plus 1 added inside it, net +1. Flat at
the call sites, still a real reduction by this milestone's own criterion 1: seven independently
worded assertions of one invariant collapsed into one declaration plus seven one-line "forwarded
from `initrd_bytes`'s own contract" comments, the same "flat count, real reduction" shape
`smb_server.rs`, `swish.rs`'s terminal pair, `display_terminal.rs`'s `paint` and `console.rs`'s
PL011 half were in earlier rounds. Verified compiling clean on all three target architectures
(aarch64, riscv64, x86_64), since the seven call sites split across both boot paths.

*`MappedWindow::as_slice`/`as_mut_slice` (new methods on round 1's own type, provisional).*
`login_test_client.rs`'s `PAGE_VA` was this milestone's one explicitly named gap ("what is still
open", above): a `core::slice::from_raw_parts_mut` site the `read_volatile`/`write_volatile` grep
that found rounds 2-5's clusters could not see, being a different pattern. Reading it in context
surfaced five more files in the identical shape, none previously investigated:
`credentialer.rs` (5 sites: `PROV_VA` and `VERIFY_VA`, one shared page apiece, read in some
functions and written in others depending on protocol phase), `credentialer_test_client.rs` (5,
the same two pages from the client side), `identity_provisioner.rs` (4: `REQ_VA`, `PROV_VA`,
`FS_VA`), `session_reviver.rs` (2: `FS_VA`, already had a sibling `MappedWindow` for a different
page from an earlier round) and `smb_server.rs` (1: `CRED_VA`). Eighteen call sites total, each
independently asserting "this page is mapped read/write here, shared with exactly this peer,"
often in near-identical wording. `MappedWindow` gained two methods returning the whole window as
an ordinary (non-volatile) `&'static [u8]`/`&'static mut [u8]`, for the "hand the page to a parser"
call shape these eighteen sites all have, distinct from round 1's "read/write one typed field"
shape. Six new window constants (`PROV_WINDOW`, `VERIFY_WINDOW`, `PAGE_WINDOW` x2 in two files,
`REQ_WINDOW`, `FS_WINDOW`, `CRED_WINDOW`) now hold the eighteen invariants as six declarations;
`credentialer.rs`'s `wipe`/`publish` and `credentialer_test_client.rs`'s `request`/`page_is_clean`
changed from taking a raw `va: u64` to taking a `MappedWindow`, so the type carries the "which
page" fact through the call rather than a bare integer.

The honest count, which this round is not going to round off. Unlike round 1's `r8`/`w8`
cluster, this migration does not reduce the raw block count, and it should not be reported as
if it does. Round 1's `read`/`write` are ordinary safe functions: the bounds check they perform
(`offset + size <= len`) is a real runtime assertion that lets the call site drop `unsafe`
entirely. `as_slice`/`as_mut_slice` have no equivalent check to add: the risk they carry is not "an
offset out of range," it is "something else touches this memory while the slice is alive," which
no bounds check catches and no cheap runtime check can verify. So both methods stayed `unsafe fn`,
every one of the eighteen call sites still carries its own `unsafe {}` block (unchanged in count),
and centralizing the "which page" half of the invariant cost one new block per shared window (ten
constructors) plus two new blocks inside `MappedWindow` itself for the methods' own bodies.
Measured from the diff against this round's own base commit: 18 `unsafe {` blocks removed, 30
added, net +12. Combined with `initrd_bytes`'s +1, this round is net +13 against a
tree-wide `outside_arch` count of 792 at its own start, landing at 805 (89 per 10,000 lines,
truncated, against the still-unmoved ceiling of 94: 5 points of headroom, one point less than the
7-point cushion every prior round preserved, spent by unrelated growth and this round's own
choice rather than by a ceiling that fired).

What this round believes it bought for that cost, and why it might be the wrong trade. Eighteen
independently worded assertions became six canonical declarations plus eighteen one-line forwarding
comments: a reader auditing "is `PROV_VA` really exclusive to the provisioner while a request is
in flight" now checks one comment instead of five worded slightly differently across two files.
That is a real reduction in *the number of distinct claims a reader has to independently verify*,
which is the milestone's own stated test ("the test is not the token count, it is the number of
distinct invariants asserted by hand"). But it is not a reduction in the number the ratchet
watches, and a careful reader comparing this round to rounds 1-5 should notice that difference
rather than take "round 6" as more of the same shape. This is easy to revert: it touches
exactly six `user/` files plus two new methods on one type, none of it wire format, syscall
surface, or anything else two programs must agree on, so if calef would rather the raw count stay
the primary signal even at the cost of the six duplicate comments, reverting this specific piece
(not `initrd_bytes`, which is unambiguously a reduction) costs nothing but the six files' worth of
diff.

The one cluster this round did not touch, and the reason is a design fork rather than a
reduction this lane could invent. `invoke(cap, method, a0, a1, a2)` calls are 123 of `user/`'s
284 blocks, by far the largest single share (43%), and every one carries the identical comment
`invoke` itself carries: *"the kernel validates the capability and the method before acting; the
caller is trusting the kernel, not the other way around."* Milestone 134's own census already
named this precisely: *"a per-method obligation carried by a single all-methods signature... some
methods (`aspace::MAP_INTO` among them) can perturb the caller's own address space, so some
obligation is real,"* and its own conclusion was "neither is this milestone's work; the handoff in
its lane report proposes it." This round counted rather than proposed: at least 18 distinct
methods are invoked directly this way across `user/` (by literal `abi::module::CONST` at the
call site; calls that compute the method dynamically are not counted, so 18 is a floor, not a
ceiling), led by `RETYPE` (10 sites), `REPLY` (9), `page_frame::MAP` (8), `page_frame::REVOKE` (3),
`memory_region::DESTROY` (3), `irq::WAIT` (3), `address_space::MAP_INTO` (3), and nine more at one
or two sites each. `send`/`recv`/`reap`/`call` and the rest of `user_rt`'s existing safe surface
already cover the handful of methods common to nearly every program; what remains uncovered is
long-tail and program-specific: page-frame and address-space construction verbs mostly used by the
handful of programs that build child processes (`hello`, `builder`, `login`, the caretakers), and
IRQ and virtio methods used only by the drivers that own those devices.

Why this is an architect's call and not a migration to invent. Building a safe wrapper per
method the way `send`/`reap` already exist would need a decision this lane has no standing to make:
whether the per-method obligation is real (as `MAP_INTO`'s is, per milestone 134's own reading) or
vestigial (as most of `send`/`recv`/`reap`'s turned out to be), for each of at least 18 methods, and
whether the wrappers belong in `user_rt` (available to every program, growing that crate's surface
by a wrapper per verb) or in a smaller per-purpose crate (a construction-verbs module used only by
the handful of programs that build children). Getting this wrong in either direction costs more than
the code: too permissive and a genuinely dangerous method (one that perturbs the caller's own
address space) reads as safe; too conservative and the exercise reduces to renaming 123 identical
comments without moving the count, the exact "relocates unsafe... hides it behind a [wrapper]"
anti-pattern this block's own text refuses. Left as a named follow-on, not attempted here: the
next lane's job is not "wrap `invoke`," it is "decide, method by method, which of the 18-plus
obligations are real, the same reading milestone 112 (SAFETY comments) already
did for the four SAFETY comments that discharged onto nobody" -- and only then does a mechanical
wrapping pass become safe to write.

A realistic floor for `user/`, as this milestone's own BUGS section asked the first lane to
report rather than pick a target here. The `invoke` cluster is the whole question: it is 123 of
284 blocks, and the achievable reduction ranges from near zero (if most of the 18-plus methods
turn out to carry the real, per-call obligation `MAP_INTO` does) to on the order of 100 (if most
turn out to be the same non-obligation `send`/`recv`/`reap` already were). No number in that range
is more than a guess without the method-by-method reading above. Setting the `invoke` cluster
aside, the rest of `user/` (roughly 161 blocks: the `read_volatile`/`write_volatile`, `asm!`,
`from_raw_parts` and "everything else" rows above) is close to its practical floor already. Six
rounds have read essentially all of it: the `asm!` entries are ABI entry stubs and traps with no
further collapse available; the remaining `read_volatile`/`write_volatile` sites are device
registers this milestone investigated and deliberately left unmigrated (the NS16550 halves of
`console.rs`/`input.rs`, whose register stride is a runtime fact no compile-time layout can
express; `clock.rs` and `driver.rs`, each already collapsed to one function apiece); the remaining
`from_raw_parts` sites are deliberate-fault test programs (`flaky.rs`, `outlaw.rs`) and one-off
writes (`memory_grant_depleter.rs`, `swapper.rs`) this milestone's own text already names as not having a §94
shape to collapse; and `crates/inter_process_communication`'s three call sites are DECIDED as genuinely distinct (round 2).
So: no single number, but a bounded one -- somewhere between roughly 160 (if the `invoke`
cluster turns out to need no wrapper at all) and roughly 260 (if it turns out nearly all of it is
real per-call obligation and stays exactly as it is), and the only way to narrow that range further
is the method-by-method reading named above, not more reading of the kind this round and its five
predecessors already did.

The ceiling's own open question, answered as far as it can be from five weeks of data. BUGS
item 1 (below) asks whether the density ceiling fires on honest work. It has not: the density has
moved from 93.4 (this milestone's own start) through 90.8, 89, 88, 87, 87 (unchanged), and now 89
again after this round's own count-regression, always 5 to 7 points under whatever the ceiling was
at the time, across five weeks and both growth and reduction. That is not proof it never will, but
it is the honest answer available today: no evidence yet that 94 is too tight, and this
round's own +13 is the first commit in the milestone's history to spend headroom rather than widen
it, which is worth calef seeing plainly rather than folded into a paragraph that reads like every
other round's.

## Round 7 (2026-08-26): the `invoke` cluster, read method by method and mostly resolved

Round 6 read and counted the `invoke(cap, method, ...)` cluster (123 of `user/`'s 284 blocks, 43%,
its largest single share) and deliberately did not migrate it, naming the open question precisely:
*"deciding which of the 18-plus obligations are real... versus vestigial... is a design fork for
calef."* This round did that reading, method by method, and it resolves almost the whole cluster.

The re-read, replicating round 6's own count first. `user/`'s 123 `invoke(...)` call sites
(confirmed against round 6's number exactly; the 124th grep hit is a doc comment on
`os_primitives_benchmarker.rs`, not code) resolve, after following each file's own `use abi::... as
...` aliases (`ut` for `memory_region`, `fr` for `page_frame`), to 22 distinct methods, four more
than round 6's own floor of "at least 18" because round 6's count did not fully resolve aliases.
Grouped by method rather than by call site, the shape round 6 predicted is exactly what is there:
some methods have one or two call sites, and several have a dozen or more.

The reading, method by method, and the answer to round 6's own question. Every one of the 22
methods carries the *identical* safety argument `invoke`'s own doc already states: *"the kernel
validates the capability and the method before acting; the caller is trusting the kernel, not the
other way around."* That argument does not vary by method, because it is not about what the method
*does*, it is about what the syscall boundary *is*: a trap the kernel validates before touching
anything, on every one of the 22. Milestone 134's census read `abi::address_space::MAP_INTO` as
carrying a "real" obligation because it "can perturb the caller's own address space," which is true
of the *result* of a successful call, but that is exactly as true of `abi::page_frame::MAP`, which
round 1 already wrapped as `map_page_frame` (a fully safe function, no `unsafe` at any of its
seventeen call sites) without anyone treating that as unsound. The two methods carry the same shape:
the kernel checks rights and alignment and either performs the mapping or returns an error, and
nothing about the *call itself* can violate a Rust invariant the wrapper could have checked and did
not. What can go wrong after a successful call (aliasing a page a Rust reference already assumes is
private, racing a mapping change) is a caller-side correctness question every syscall in this cluster
already has, `map_page_frame`'s included, and it is the argument the raw call site's own SAFETY
comment already discharges onto "the caller is trusting the kernel." So round 6's "real vs.
vestigial" question resolves to: for Rust-safety purposes, all 22 are the `send`/`recv`/`reap` shape,
not the exception `MAP_INTO` was flagged as being. The genuinely separate question, "should a
supervisor be able to remap a child's memory out from under it without the child's cooperation," is
real, but it is a capability-policy question the kernel's rights model already answers (`WRITE` on
the address-space capability), not a Rust-safety gap a wrapper's absence was leaving open.

What that reading bought, mechanically. Three of the 22 methods already had a safe wrapper
sitting unused: nine `abi::reply::REPLY` call sites (three of them local one-line `fn reply(slot,
r0)` re-wrappers, in `fs_file_caretaker.rs`, `fs_nameset_caretaker.rs`, `fs_subtree_caretaker.rs`,
each duplicating [`reply`]'s own body) and one `abi::rendezvous::SEND` call site were migrated onto
[`reply`]/[`send`], which already existed and needed no change; seventeen `abi::page_frame::MAP` call
sites moved onto [`map_page_frame`], likewise pre-existing. That is 27 sites, zero new code.

One more method is the sharpest instance of the §94 shape this milestone keeps finding: `date.rs`
had a `granted(slot) -> bool` probe (invoke a method number no object type defines, so the call can
only be refused, and read *which* refusal came back to tell an empty slot from a real object),
documented as lifted from `clock_page`'s own probe. Four more programs (`pgrep`, `pmap`, `ps`,
`watch`) had each copied the identical function, byte-for-byte body and comment, one of them saying
so out loud ("`ps`'s and `date`'s probe, verbatim") without anyone lifting it out. [`granted`] is now
one declaration in `user_rt`; the five call sites are gone.

The remaining eighteen methods had no wrapper at all, and each got the thin, one-`unsafe`-block
wrapper round 1's `map_page_frame` already established the shape for: [`retype_page_frame`]
(`memory_region::RETYPE`, 15 sites), [`retype_object`] (`RETYPE_OBJ`, 7), [`split_region`] (`SPLIT`,
3), [`destroy_region`] (`DESTROY`, 4), [`map_region_page`] (`MAP`, 2), [`revoke_frame`]
(`page_frame::REVOKE`, 3), [`map_into`] (`address_space::MAP_INTO`, 9), [`tcb_cap_insert`],
[`tcb_configure`], [`tcb_start`] (`thread_control_block::CAP_INSERT`/`CONFIGURE`/`START`, 2 each),
[`irq_wait`], [`irq_ack`] (`irq::WAIT`/`ACK`, 7 and 6), and [`send_cap`] (`rendezvous::SEND_CAP`,
10). Four more (`virtio::READ_REG`/`WRITE_REG`/`SETUP_QUEUE`/`NOTIFY`, 18 sites across `display.rs`,
`entropy.rs`, `kbd.rs`, `net_transport.rs`) went into a new opt-in `user_rt::virtio` module rather
than the crate root, the same "scoped to the programs that actually touch this capability" shape
`mapped_window`/`initrd` already established, since only four of `user/`'s programs hold a `Virtio`
capability. Provisional names, all of them, per this milestone's own naming discipline: none is
ratified, and `calef`'s call on all fourteen (plus `granted`) is open.

One `tcb_start` correction found while wrapping it: `abi::thread_control_block::START`'s own doc
comment reads `invoke(cap, START, _, _, _)`, implying the three arguments are ignored, but
`kernel/src/syscall.rs`'s `START` arm forwards all three to `sched::start_thread_control_block`
unconditionally, and `builder.rs`'s own call relies on exactly that (`START`'s second argument is the
worker's input). The wrapper takes `(tcb_slot, x0, x1, x2)` to match what the kernel actually does;
the stale `abi` doc comment is a separate, smaller finding left for whoever next touches that file.

**Every one of the 122 migrated call sites lost its `unsafe {}` entirely**, not moved it: the
wrapper functions are ordinary safe `fn`s (each holding exactly one `unsafe { invoke(...) }` inside,
forwarding `invoke`'s own contract), so a caller with a valid capability slot number, of any value,
cannot violate a Rust invariant by calling one. That is what makes this a §94 collapse rather than a
relocation: eighteen new declarations (plus `granted`, plus four `virtio` functions) replace 122
independently-worded `// SAFETY:` comments that all said the same thing.

**The one call site left raw, and why it is not this shape.** `window.rs`'s `ROLE_PROBE_INPUT` path
calls `invoke(INPUT, abi::rendezvous::RECV, 0, 0, 0)` directly to read the raw negative `abi::Error`
a `RECV` against an empty capability slot returns. `user_rt::recv`'s own contract assumes success and
returns the three data words, discarding the syscall's own return code, so it cannot serve this call
site's actual purpose (proving the kernel refuses cleanly) without becoming a second, differently-
shaped `recv` written for one caller. That is not a reduction by this milestone's own test ("the
number of distinct invariants asserted by hand"), so it stays, with an expanded `# Safety` comment
recording why it is the one exception rather than an oversight.

**The honest count.** `user/` carries **162** `unsafe {` blocks (was 284; the exact regex from round
6, re-verified against `script/lint`'s own printed number before and after). Tree-wide, outside
`kernel/src/arch/`: **701** (was 805), density **77 per 10,000** (was 89), against the still-unmoved
ceiling of 94 -- **17 points of headroom**, the widest margin on record (round 6 had spent it down to
5). `crates/user_rt` itself grew from 31 to 45 blocks (`lib.rs`) plus a new 4-block `virtio.rs`,
which is the 18 wrapper bodies; that growth is inside the same directory this round's own reduction
counts against, so the 104-block net (122 removed from `user/`, 18 added to `user_rt`) is the real
tree-wide number, not the 122 `user/`-local one. Verified compiling clean and `cargo clippy` clean on
all three target architectures (aarch64, riscv64, x86_64), and the full `script/test` suite green on
all three (295/298/175 passed respectively, the x86_64 skips being the recorded hardware-scope gaps
already in the suite, none of them touching this round's own migration).

**What this round believes it resolved in round 6's own design fork.** The "which of the 18-plus
methods carry a real per-call obligation" question is answered: **for Rust-safety purposes, none of
them do, in the way `MAP_INTO` was flagged as possibly doing**; the obligation `invoke`'s own
contract already discharges is the whole of it, the same as it already was for the five methods
round 1-6 had already wrapped. The "where do the wrappers belong" question is answered by precedent
already in the tree: the crate root for anything more than one or two programs touch, a scoped
opt-in module (`virtio`) for a capability only a handful of drivers hold, exactly `mapped_window`'s
and `initrd`'s own shape.

**What is not resolved, and is worth naming rather than leaving implicit.** This round did not build
a generic, checked `invoke` entry point that all 22 (now: 1 unwrapped) methods share -- each wrapper
above is its own thin, per-method function, not a shared primitive underneath them. That is
deliberate: a cross-cutting `invoke` abstraction is new shared infrastructure every future capability
call would depend on, and building one without a proposal first is exactly what this milestone's own
brief asked not to do. **Nothing observed while doing this reading suggests such a primitive would
buy anything the fourteen per-method wrappers do not already buy** (there is no shared runtime check
across the 22 methods beyond `invoke`'s own trap, unlike, say, `map_page_frame`'s bounds check, which
IS method-specific and already lives in that one wrapper), so this round is not raising it as an open
question for calef; it is recording that the question was considered and the answer, on the evidence
gathered, is "the per-method shape is the right one and nothing here argues for more machinery."

**Provisional, easy to revert.** All fifteen new names (fourteen wrapper functions plus `granted`)
and the new `virtio` module are provisional, per this milestone's own naming discipline; `calef`'s
call. One naming tension worth surfacing rather than deciding here: `user_rt::virtio` shares its
last path segment with the pre-existing `crates/virtio` crate (the block driver, unrelated). The two
never collide in code (nothing in `user/` imports both unqualified in one file), but a reader
grepping "virtio" meets two different things under that name. `user_rt::virtio` reads correctly as
"the wrapper module for `abi::virtio`'s methods," which is the reason it was named that on first
pass, but a different name (`user_rt::virtio_cap`, or similar) would remove the ambiguity entirely if
calef would rather. Reverting any single wrapper (moving its call sites back to raw `invoke`) costs
nothing but that one function and its call sites; none of this touches the syscall surface, a wire
format, or anything else two programs must agree on.

## Round 8 (2026-09-01): the kernel, which seven rounds had never touched, categorised and two
clusters collapsed

Rounds 1 through 7 worked `user/` and `crates/`. **None of them went near `kernel/src`**, and by
2026-09-01 that had become the largest unworked pool in the tree: **242 blocks outside
`kernel/src/arch/`**, against `user/`'s 162 after round 7, `crates/user_rt`'s 64 and `crates/inter_process_communication`'s
44 (settled in round 2). This block's own opening measurement recorded the kernel at 203 on
2026-08-18; it had grown to 242 while six rounds went through userspace. It is also the part
DECISIONS §14 (the project's direction: a verified-Rust capability microkernel) calls verified, which is what this milestone's own "Why
it matters" says the number is measuring.

**Round 8 was possible in a way earlier rounds were not**, because milestone 193 (put `kernel/src`
within reach of the prover) landed the day before: `cargo kani` can compile the kernel now, so this
milestone's criterion 2 ("a typed abstraction whose invariant the compiler **or Kani** holds") is
available inside the kernel for the first time. In the event neither collapse below needed the
prover, because both turned out to be criterion 1, but the categorisation below says where the
prover is the tool for what remains.

### The per-category table for the kernel's 242, so nobody re-derives it

Categorised by what the first token inside each block is (the same stripping-and-counting regex
`script/lint`'s census uses, so these add up to its number rather than a grep's).

| shape | blocks | verdict |
|---|---|---|
| `core::ptr::write_bytes` | 41 | **the §94 shape; 37 collapsed this round** (see below). The 4 left have different provenance: `kmem.rs`'s pool recycle, `memory_region.rs`'s two retype paths, and one partial-page write in `user.rs` |
| `core::ptr::read_volatile` | 34 | mixed. MMIO (`ns16550.rs`, `plic.rs`, `pci.rs`, `non_volatile_memory_express.rs`, `virtio.rs`) is not a target; the rest read a shared frame the kernel just handed a process, in test fixtures that each read a different field |
| `core::ptr::write_volatile` | 23 | same split as the row above, same verdict |
| `&`-first (a reference built from a raw pointer) | 25 | mostly `sched.rs` (10) and the graphical test fixtures; each names a different object at a different lifetime |
| `(`-first (a call through a raw pointer or a cast) | 16 | 13 of them `sched.rs`'s thread-control-block pointer arithmetic; see the design fork below |
| `let`-first | 13 | multi-statement blocks, each doing something particular |
| `core::slice::from_raw_parts[_mut]` | 11 | whole-page and whole-region slices, one per distinct region |
| `log_page(...)` | 6 | **a real §94 shape, not taken this round**; see the proposed follow-on |
| `q.push_back` / `inbox.push_back` | 8 | `sched.rs`'s run-queue handoff; a design fork, see below |
| `mmu::activate_user` | 6 | test fixtures in `user/tests.rs`, which another lane holds this session |
| `dtb::Dtb::from_ptr` | 5 | **the §94 shape; all 5 collapsed this round** (see below) |
| `crate::stack::paint` / `high_water` | 9 | three stacks, three different facts each; the `crates/inter_process_communication` shape, no collapse available |
| `drivers::plic::init` | 3 | MMIO init, one per boot path |
| everything else | 42 | one-offs: `force_unlock`, `ManuallyDrop::drop`, `from_utf8_unchecked`, `assume_init`, the fastpath pad, `Thread::spawn_into` |

### What collapsed

**Page zeroing: thirty-seven sites, one declaration.** Every service, driver and test fixture that
allocated a frame went on to zero it by hand, each with its own `// SAFETY:` comment over
`core::ptr::write_bytes` asserting the same two facts: a frame just handed back by `memory::alloc`
or `memory::alloc_contiguous` is exclusively the caller's, and the direct map reaches it. That is
not thirty-seven facts, it is one fact about what the allocator returns, and the allocator is the
only thing in the tree that can actually check it. Two of the copies had already said out loud that
they were copies, without anyone lifting it out: `user/rmle_service.rs`'s *"`fs_service`'s own
`page_frame` does the same thing... this is a second copy of three lines"* and
`user/session_reviver_service.rs`'s *"matches `fs_service::frame`'s own shape"*, the same tell
`ntp.rs` carried for round 1's cluster and `timetable.rs` for round 6's.
`crate::memory::alloc_zeroed` and `crate::memory::alloc_contiguous_zeroed` (both new; **ratified by
calef 2026-09-01**) hold it once. Every migrated call site is ordinary safe code afterwards: a caller cannot
violate a Rust invariant by asking the allocator for a zeroed frame, whatever it then does with it,
which is what makes this a collapse rather than the relocation this block refuses. The four sites
left behind are named in the table above and are genuinely different provenance rather than an
oversight: `kmem.rs` zeroes a page recycled through its own pool (the ownership half of the
assertion is the pool's, not the allocator's), `memory_region.rs` zeroes a page carved out of a
memory region (likewise), and `user.rs`'s timebase arm writes `PAGE_BYTES` into a frame it already
holds rather than a whole fresh one.

**The device tree: five sites, one declaration.** `memory.rs`, `console.rs`, `pci.rs` and `smp.rs`
(twice) each took the boot pointer and handed it to `dtb::Dtb::from_ptr` under a hand-written
comment rewording the same two facts: it is the pointer firmware put in `x0`/`a1`, which
`kernel_main` stashes in `crate::DTB` as its first statement before any of these can run, and it is
physical, so the direct map names it. `crate::device_tree` (new; **ratified by calef 2026-09-01**) holds that
once, in the module the static lives in. It returns `Result` rather than panicking, because two
callers legitimately continue without a tree: the early RISC-V console keeps its defaults, and
`x86_64` stores a PVH `hvm_start_info` pointer in `crate::DTB` rather than an FDT, so `from_ptr`'s
magic check is what tells those apart from a real failure (`smp.rs`'s own fixture already documents
that hazard). Three functions (`memory::init`, `smp::read_cpu_list`, `console::configure_from_dtb`)
lost a `dtb_ptr` parameter that was always `crate::DTB`'s value at every one of their call sites,
which is the same one-source-of-truth gain one level out.

**Measured from the diff against this round's own base commit (`8fc30efb`): 42 `unsafe {` blocks
removed, 2 added, net -40.** `kernel/src` outside `arch/` goes 242 to 202; tree-wide outside
`arch/`, 738 to 698. Density 83 to 78 (78.8 exactly). The tree-wide census confirms the diff
exactly, nothing else having landed on this branch in between, the same discipline every round since
round 2 has used.

**The ratchet, cinched a sixth time**: `<!--count-at-most:unsafe-density-outside-arch-->` lowered
from 94 to 88 (`notes/unsafe-obligations.md`, `notes/counted-claims.md`), **ratified by calef
2026-09-01**, ten points of headroom over the 78 this round reached.

An earlier draft of this paragraph called that a departure from a "seven-point convention". There is
no such convention, and the correction is worth carrying because it makes the case simpler rather
than weaker: the headroom actually on record is six points at round 1 (100 against 90.8) and
seventeen at round 7 (94 against 77). What every round has really held to is this block's own rule,
that the ceiling falls when a real reduction lands and the headroom is argued beside the marker. So
this is that argument.

It rests on a measurement none of the earlier rounds had: round 7 reached 77 on 2026-08-26 and this
round found 83 on 2026-09-01, **six points of unrelated growth in six days**, the steepest stretch
on record. A ceiling seven points over the current density is therefore about one week of ordinary
lane traffic before it fires on honest work, which is the "only ever rejects legitimate work"
signature BUGS item 1 below exists to keep this milestone from producing. Ten points is roughly ten
days at the observed rate.

**And the gain is more than kept**, stated exactly because this is a measurement: the density fell
**five** points (83 to 78, truncated) and the ceiling fell **six** (94 to 88), one point further
cinched than was gained.

### What this round deliberately did not take, and why, so nobody re-derives it

- `kernel/src/arch/` (248 blocks, up from 141 on 2026-08-23). Not a target, per this block's own
  text. The growth is milestone 161's `x86_64` port, which is a third architecture's worth of
  assembly and system registers: exactly the population the measurement excludes on purpose.
- MMIO and device-register access (`ns16550.rs`, `plic.rs`, `gic.rs`, `pl011.rs`, `pci.rs`,
  `non_volatile_memory_express.rs`, `virtio.rs`). The same finding round 3 and round 5 reached for the userspace drivers,
  and for the same reasons: each block names a different device at a different offset table, and the
  one file that could carry a compile-time layout already does (`pl011.rs` uses
  `tock_registers::register_structs!`; `ns16550.rs`'s own module doc explains why it cannot, the
  runtime-variable register stride). No §94 shape.
- The three stack helpers (`crate::stack::paint`, `high_water`, 9 blocks across
  `interrupt_stack.rs`, `smp.rs`, `thread.rs`, `stack.rs`). The comments rhyme and the facts do not:
  an interrupt-stack slot, a not-yet-handed-out `KernelStack`, and a secondary core's boot stack are
  three different ownership arguments. The `crates/inter_process_communication` shape from round 2.
- `kernel/src/user/tests.rs` (14 blocks) and the `mmu::activate_user` fixtures. Left alone for a
  scheduling reason rather than a technical one: AGENTS.md names that file the tree's merge hotspot
  and another lane held it this session. A later round should read it; nothing here says it is
  irreducible.
- **The prover.** Milestone 193's harnesses reach `kernel/src/syscall.rs`; `arch/`, `asm!` and MMIO
  are still out of reach, and a stub is a hole in a proof, so criterion 2's Kani half was not
  available for either of the clusters this round found. It is the right tool for the `sched.rs`
  fork below, which is the reason that fork is worth raising rather than guessing at.

### Two things identified and not done, in the two shapes this project accepts

**A proposed milestone (provisional number, the integrator mints it at merge): give the revocation
log's page chain a safe iterator.** `kernel/src/revoke.rs` walks its per-space chain of log pages in
six places, every one of them `let mut page_phys = space.head; while page_phys != 0 { let page =
unsafe { log_page(page_phys) }; ... }`, and every one asserting the identical fact that `log_page`'s
own `# Safety` already states: the page is one this module linked into a chain, and `SPACES` is
held. Six hand-written copies of one invariant, the §94 shape exactly. The collapse is not the
trivial one, which is why it wants a lane rather than a paragraph here: making the call sites safe
means the *chain itself* has to carry the "`SPACES` is held" half, as an iterator borrowing the lock
guard and yielding `&mut LogPage`, and the six loops are not uniform (two mutate entries in place,
one carries an index cursor across calls, two break early). Wrapping `log_page` in another function
would move the assertion and not collapse it, which this block refuses. Worth roughly 6 blocks and,
more to the point, worth removing the one place in the kernel where a future edit could walk a chain
without the lock and nothing but a comment would notice.

**A recorded limitation, in this block's own BUGS section below rather than invented as scope:
`sched.rs` is 47 of the kernel's 202 and the largest single share, and it is a design fork rather
than a migration.** Its blocks are the run-queue handoff (`cpu::current().with_runq(|q| unsafe {
q.push_back(ptr) })`, 6 sites, plus 2 inbox pushes) and the raw thread-control-block pointer
arithmetic underneath it (13 `(`-first sites). Every one of the eight queue pushes asserts the same
sentence: *this thread is live, Ready, and on no other queue.* That reads like the §94 shape and it
is not, because unlike `alloc_zeroed`'s invariant, this one is not a fact the callee can establish;
it is a fact the **caller** established two lines earlier by transitioning the thread. A safe
`enqueue_ready(ptr)` would carry the identical argument in a different place, which is this block's
named anti-pattern. The reduction that would be real is rung one of AGENTS.md's ladder: make the
wrong state unrepresentable, so a thread pointer can only reach `push_back` by way of a type that
only a Ready-transition can mint. That is a scheduler-core typestate change, it touches the one
subsystem where a mistake is an intermittent hang rather than a compile error, and it is exactly the
kind of thing milestone 193's prover should be pointed at first. **An architect's call**, not a
lane's to invent.

## What is still open

**`crates/inter_process_communication`'s unsafe is settled**: read in full, genuinely per-call-site distinct, no further work
indicated there (see round 2 above).

**The broader `user/` survey this milestone's BUGS section calls for is still not complete**, and is
now a better-informed job than it was after round 1: the `read_volatile`/`write_volatile` grep
(rather than the round-1 name-based search) is the right net, and round 2's pass through its results
sorted the non-FS hits into rough categories a follow-on lane can use rather than re-deriving:

- `swish.rs`'s other two windows, `disk_surveyor.rs`'s `ROSTER_VA`, and `net_stack.rs`'s
  `a_r8`/`a_r16`/`a_w16`/`a_w8` cluster are all done (round 3, see above). Nothing else in these
  three files' shape remains outstanding.
- Framebuffer/graphics code is done (round 4, see above): `painter.rs`, `window.rs`,
  `display.rs` and `display_terminal.rs` are all migrated, and the measured bounds-check cost that
  round 3 left open (negligible at every volume this cluster sees, on both ISAs) is recorded there.
  Nothing else in this cluster remains outstanding.
- `heeder.rs` is done (round 2); nothing else in that shape remains outstanding there.
- Device register blocks: done for the files that had a fixed layout (round 5), and nothing
  else in this shape remains outstanding. `console.rs`'s and `input.rs`'s aarch64 (PL011) halves
  and `jh7110_trng.rs` are migrated onto `tock_registers::register_structs!`/`register_bitfields!`,
  matching `kernel/src/drivers/pl011.rs`'s own idiom. `console.rs`'s and `input.rs`'s riscv64
  (NS16550) halves are deliberately NOT migrated: round 3's premise that
  `kernel/src/drivers/ns16550.rs` already uses this idiom "for the identical hardware" was false
  (see the correction above), and the real fact it got wrong, that this device family's register
  stride is a runtime value no compile-time layout macro can express, applies to these two files'
  riscv64 halves exactly as it applies to the kernel's own NS16550 driver. `clock.rs`'s RTC drivers
  and `driver.rs` still need nothing, per round 3's reading. See round 5 above for the full
  per-file, per-architecture reasoning and the measured reduction.
- Deliberately not migration candidates, named so nobody re-derives them and wastes a look:
  `hello.rs` (tests `.bss` zeroing and `.data` writability on purpose; the raw access *is* the test),
  `flaky.rs` and `outlaw.rs` (deliberately touch a bad/unauthorized address to provoke a fault; a
  bounds-checked wrapper would defeat the point), `memory_grant_depleter.rs` and `swapper.rs` (single one-off
  writes, not a repeated hand-written invariant -- nothing to collapse).
- `login_test_client.rs`'s `PAGE_VA` is done (round 6), along with five more files in the
  identical `core::slice::from_raw_parts[_mut]`-over-a-whole-page shape that reading this one
  surfaced: `credentialer.rs`, `credentialer_test_client.rs`, `identity_provisioner.rs`,
  `session_reviver.rs`, `smb_server.rs`. See round 6 above, including the honest note that this
  migration is a net **increase** in raw block count (+12), unlike every prior round's.
- **The `invoke` cluster (123 of `user/`'s 284 blocks, the largest single share) is read, resolved,
  and migrated** (round 7): 22 distinct methods, all found to carry the same Rust-safety shape as
  the five methods already wrapped (`send`/`recv`/`reap`/`reply`/`map_page_frame`), none of them the
  exception milestone 134's census flagged `MAP_INTO` as possibly being. 122 of the 123 call sites
  now go through fourteen new thin wrappers, a new `granted` (the §94 shape, five programs'
  identical probe), and a new opt-in `user_rt::virtio` module; one call site (`window.rs`'s refusal
  probe) stays raw, with the reason recorded there. See round 7 above for the full per-method
  accounting.

- **The kernel outside `arch/` is read and categorised** (round 8, the per-shape table above), and
  two of its clusters are collapsed: 37 page-zeroing sites onto `memory::alloc_zeroed`/
  `alloc_contiguous_zeroed`, and 5 device-tree parses onto `crate::device_tree`. What is
  deliberately left there, and why, is the "did not take" list in round 8; what is identified and
  not done is one proposed milestone (`revoke.rs`'s log-page chain) and one recorded limitation
  (`sched.rs`'s run-queue handoff, a typestate fork for calef), both written up in round 8 above.

**This block still sets no target number**, per its own original text -- the ratchet moves by
measured reduction, not by picking a floor in advance. Round 6's own reading of what a realistic
floor looks like is above, and it is a range bounded by the `invoke` cluster's unresolved question
rather than a single number, per this milestone's own BUGS item asking the first lane to report
rather than pick one.

**In brief.** Reduce the hand-written `unsafe` this tree carries outside `kernel/src/arch/`, and lower
the ceiling after each reduction so the ground gained cannot be given back quietly.

## The measurement it starts from

Taken on `main` 2026-08-18 and to be re-taken by 134 rather than trusted here:

| | count |
|---|---|
| `unsafe { }` blocks, ours, `vendor/` excluded | 893 |
| `unsafe fn` | 53 |
| `unsafe impl` | 28 |
| `// SAFETY:` comments | 885 |

By location: `kernel/src/arch/` 139, kernel outside `arch/` 203, `user/` 285, `ipc` 44, `user_heap`
41, `user_rt` 31.

**`arch/` is not a target and this block will not accept a reduction there.** Rule 1 says
architecture-specific code lives under `arch/`, and unsafe is what architecture is made of. Driving
that number down means either writing the assembly wrong or moving it somewhere it does not belong,
and both are worse than the number.

## What counts as a reduction, which is the whole design of this block

**The metric is gameable and the obvious way to game it is invisible.** Moving three `unsafe` blocks
into one helper function reduces the count by two and reduces the risk by nothing: the same
invariants are asserted by the same code in a different place. A milestone that rewarded that would
make the tree worse while the graph went the right way, and the graph would be the reason.

**So the test is not the token count, it is the number of distinct invariants asserted by hand.**

**§94 is the worked example and it is what a real reduction looks like.** The trap instruction was
inlined at **48 sites in 7 variants** across 58 panic handlers, each one a hand-written assertion of
the same invariant. Lifting it into `user_rt::trap()` left one. That is 47 chances to write it
differently, removed. §94 states the general form: *a per-binary item whose body is copied verbatim
into every binary is not per-binary; only its declaration is,* and copying it is asserting the same
invariant N times by hand, which §61 says a `// SAFETY:` comment must never be.

**And §94 found the cost of the copy by counting the copies**: one of the 58 handlers was different.
`terminal_sink_caretaker` called `exit()` and never trapped, so a panic there reported `EVENT_EXIT`
where every other program reports `EVENT_FAULT`. **A supervisor would have been told a panicking
program finished cleanly.** That is the argument for this milestone in one incident: the duplication
was not only ugly, it was already wrong in one place and nobody knew.

So a reduction qualifies when it does one of these:

- Collapses N hand-written assertions of one invariant into one, the §94 shape. Best available,
  and the only one that reliably reduces risk rather than moving it.
- Replaces raw pointer arithmetic with a typed abstraction whose invariant the compiler or Kani
  holds, so the assertion stops being a comment. Rung one on the ladder.
- **Deletes unsafe that was never needed**, which is the cheapest and rarest.

It does not qualify when it merely relocates unsafe, wraps it in a function whose safety argument is
the same argument, or hides it behind a macro.

## The ratchet

After each reduction, **lower the ceiling to the new count in the same commit.** A ceiling left above
the true number is not a ratchet, it is a budget, and a budget gets spent by whoever arrives next
without knowing it was won.

## Why it matters

**The verified-Rust claim in this project's own thesis is measured here.** DECISIONS §14 calls this a
verified-Rust capability microkernel; `unsafe` is where that claim is suspended, and 203 suspensions
outside the architecture layer is the honest size of the gap. Every one of them is a place where the
proofs and the type system are standing aside and a person's comment is the whole argument.

## BUGS

- This block sets no target number. `script/lint` has already had three checks deleted for the
  signature "only ever rejects legitimate work", and a ceiling cinched past what the tree can
  sustain would be the fourth. **Round 6 answered the "does it fire on honest work" half**: across
  five weeks and six rounds of both growth and reduction, the density has stayed 5 to 7 points
  under whatever the ceiling was at the time (93.4 down to 87, back up to 89), with no near-miss on
  record. **Round 7 widened that margin to 17 points** (89 down to 77) by resolving the `invoke`
  cluster (below), the largest single reduction of the milestone's seven rounds. No evidence yet
  that 94 is too tight, and rather more evidence now that it has real headroom. **The "what floor"
  half is answered too, as far as `user/` is concerned**: round 7's reading found the `invoke`
  cluster's achievable reduction was not the 0-to-100 range round 6 could only bound, it was
  essentially all of it (122 of 123 sites), because the "real per-call obligation" round 6 held open
  turned out, on reading, not to distinguish any of the 22 methods from the five already-wrapped
  ones. `user/` now stands at 162 blocks; whether that is close to a practical floor for the
  *rest* of `user/` (the `asm!`, device-register and deliberate-fault categories) is still the
  reading rounds 1-6 already did, recorded below.
- `user/`'s 285 (then 284, now 162) is explained, closing this milestone's own original BUGS
  item and, as of round 7, resolving the one piece of the breakdown round 6 left open. The
  breakdown (round 6's own table, updated by round 7): 122 of the 123 raw `invoke(...)` calls (43%
  of the original total) are now behind fourteen new thin wrappers, `granted`, or the existing
  `send`/`reply`/`map_page_frame`, and one stays raw with its reason recorded at the call site (see
  round 7 above); 36 `read_volatile`/`write_volatile` (device registers and shared frames already
  investigated and either migrated or deliberately left, per rounds 1-5), 16 `asm!` (entry stubs and
  traps, no further collapse available), 12 `from_raw_parts[_mut]` (deliberate-fault test programs
  and one-off writes with no §94 shape to collapse), and 97 everything else (window constructors,
  the C ABI shim, deliberate `.bss`/`.data` probes) are unchanged by this round. What was "how much
  of the `invoke` cluster is real" is now answered: essentially none of it, in the sense that
  mattered for whether a safe wrapper could exist.
- `sched.rs` is the kernel's largest remaining share (47 of 202) and it is a design fork, not a
  migration. Its run-queue handoff pushes a thread-control-block pointer under eight
  hand-written copies of one sentence (*live, Ready, on no other queue*), which looks like the §94
  shape and is not: unlike the allocator's postcondition that round 8 collapsed, this invariant is
  established by the **caller** two lines earlier, so a safe `enqueue_ready` wrapper would relocate
  the argument rather than collapse it. The real reduction is rung one of AGENTS.md's ladder, a
  typestate that only a Ready-transition can mint, in the one subsystem where a mistake is an
  intermittent hang rather than a compile error. Named here rather than attempted; see round 8.
- **A reduction can be real and still not show in the count**, and round 6 is the sharpest example
  on record of the inverse: a real reduction (eighteen independently worded page-sharing
  invariants collapsed to six canonical declarations) that shows as a raw block-count **increase**
  (+12), because unlike round 1's `r8`/`w8` cluster, the "hand back a whole slice" call shape has no
  runtime check to add that would let the call site drop `unsafe` the way a bounds check did. Do
  not let the number alone decide which work is worth doing, in either direction: it under-credits
  round 6's collapse and it would over-credit a Kani-proved block that stays exactly where it is.
## Follow-on

- **Outstanding.** `kernel/src/revoke.rs` still walks its per-space log-page chain in six unsafe
  blocks, each restating what the helper's own safety comment says. Round 8 named it a proposed
  milestone and no block has been minted for it. Checked 2026-09-03.
- **Outstanding.** `kernel/src/sched.rs` is unchanged at 47 blocks, eight of them the run-queue and
  inbox pushes. The typestate that only a ready transition can mint is an architect's call and there
  is no file for it under `design/decisions/`. Checked 2026-09-03.
- **Outstanding.** `kernel/src/user/tests.rs` still carries 14 unsafe blocks, untouched. Round 8
  skipped it for a scheduling reason, another lane holding the tree's merge hotspot, and said
  explicitly that nothing makes it irreducible. Checked 2026-09-03.
- **Outstanding.** Round 7's fifteen provisional names and round 6's two are unratified. Only three
  entries in `notes/unsafe-obligations.md` carry a ratification; the fourteen wrappers and the two
  others do not. Checked 2026-09-03.
- **Outstanding.** Round 6's offer to revert the slice-accessor migration if calef would rather the
  raw count stay the primary signal is unanswered; both methods are still in
  `crates/user_rt/src/mapped_window.rs` and the six windows are still in place. Checked 2026-09-03.
- **Outstanding.** `crates/abi/src/lib.rs` still documents the start invocation as if its three
  arguments were ignored, which both the kernel's own arm and the builder's call contradict. Round
  7 found this and left it for whoever next touched the file; nobody has. Checked 2026-09-03.
- **Outstanding.** The ratchet watches unsafe blocks only. This block's opening census also counted
  53 unsafe functions and 28 unsafe impls; a grep today gives 76 and 28, and no round has re-taken
  or gated those two numbers even though rounds 6 and 7 added unsafe functions of their own.
  Checked 2026-09-03.
- **Done.** Round 5's caveat that whichever concurrent lane landed second would find the ceiling
  arithmetic stale was discharged by round 8, which re-measured from the merged tree on 2026-09-01
  rather than trusting either round's isolated numbers.
- **Done.** The `tock_registers` follow-on round 3 named was taken in round 5: `user/Cargo.toml`
  pins it against the kernel's matching pin, and the aarch64 PL011 halves of the console and input
  programs plus the TRNG driver are migrated.
- **Done.** The framebuffer and graphics cluster is settled rather than narrowed: round 4 measured
  the bounds check at 4 aarch64 ticks and about 0.6 riscv64 ticks per access, flat across volumes,
  then migrated all four sites round 3 named.
- **Recorded.** `crates/inter_process_communication`'s three production blocks each assert a different fact under a
  different caller contract, so there is no §94 shape to collapse and nothing further is indicated
  there.
- **Recorded.** The NS16550 halves of the console and input programs stay hand-written, because the
  register stride is a runtime fact no register-layout macro can express, the same reason
  `kernel/src/drivers/ns16550.rs` gives in its own module doc.
- **Recorded.** `hello.rs`, `flaky.rs`, `outlaw.rs`, `memory_grant_depleter.rs` and `swapper.rs` are
  deliberately not candidates: for three of them the raw access is the test, and the other two are
  one-off writes with nothing repeated to collapse.
- **Recorded.** No target number, by design. The ceiling stands at 88 in
  `notes/unsafe-obligations.md` and `notes/counted-claims.md`, ten points over round 8's 78, and
  there is still no near-miss on record after eight rounds.
- **Refused.** `kernel/src/arch/` is not a target and this block will not accept a reduction there.
  It is 258 blocks today, up with milestone 161's x86_64 port, and driving it down means writing
  the assembly wrong or moving it out of `arch/`.
- **Outstanding.** This block's own "What is still open" section opens by saying the broader `user/`
  survey is incomplete, which its rounds 6 and 7 completed and its own `BUGS` says was closed. The
  sentence and its refutation are 140 lines apart in one file. Checked 2026-09-03.

## Index row

First real reduction: seven programs' hand-rolled shared-page volatile-access functions collapsed
into `user_rt::mapped_window::MappedWindow`, net -21 unsafe blocks (32 removed, 11 added),
bounds-checked at every access instead of trusted by hand. Density 93.4 to 90.8; the `unsafe-density-outside-arch` ceiling cinched from 100 to 97 in the same commit. `user/`'s broader
unsafe survey and several named candidate clusters remain.
