---
status: BUILT
raised: 2026-08-23
built: 2026-09-26
---
# 154. A process that holds two directory capabilities

Built 2026-09-26 (PR #1346), for the mechanism. Minted 2026-08-23, proposed
independently by two milestones that converge on the same gap: milestone 47's `bind` ("It is
blocked on a second grant") and milestone 64's `File::open` fork ("tier two, anything that
traverses, needs a namespace to resolve *against*, and that is 47's unbuilt half"). Built
2026-08-23: one process holding and resolving against two directory capabilities, on the real
wire. §126 (a real, single, moving cwd), in
[design/decisions](../decisions/126-two-directory-cwd.md), decided 2026-08-25 that the holder has one
real, moving position. Built 2026-09-26: the real shell and its grant planner across two trees,
presented as calef ruled that day, "one tree with other trees mounted at names in it" ("The shell
across two trees" below).

Not wired into the boot shell, on calef's instruction: that lands with the first real second
filesystem, and it is
[proposals/a-second-filesystem-mounts-in-the-boot-shell.md](proposals/a-second-filesystem-mounts-in-the-boot-shell.md).

## The gap, in both milestones' own words

47: "A shell holds one directory capability, so a namespace assembled from what it holds has
exactly one member and every bind is an alias inside one tree. The interesting case, and the only
one that pays for the mechanism, is a union of two grants... Nothing in this system grants a
second directory capability to one process. `fs_service::start_granted_dir` starts one caretaker and
hands one endpoint; a second means a second caretaker, a second slot, and a spawn-protocol position
to say which is which."

64: "Tier two, anything that traverses, needs a namespace to resolve against, and that is 47's
unbuilt half. `Path::new("assets").join("x.png")`, an absolute path, or a program wanting two
directories all land here."

## The deliverable

Both milestones already name it identically: one process, two subtrees, `/a/x` and `/b/y` both
resolving, `/a/../b` refused, and neither caretaker able to see the other's tree. Concretely:

- A second `fs_subtree_caretaker` (or equivalent), a second cspace slot, and a spawn-protocol
  position to say which directory a grant is (an endowment question, per 47's own environment
  section, "expensive" in the same sense that section already prices).
- The negative control that only a union can state: `/a/../b` refused, proving neither subtree can
  name the other's parent.
- `caps` gains a namespace section with more than one row, which 47's own text says is currently
  empty precisely because one root has one row.

## What it unblocks

- Milestone 47's `bind` falls out as a name on a `Cwd` per entry once this exists; the mount
  table itself is "the cheap half" per 47's own finding, already priced.
- Milestone 64 (enough `std` to run somebody else's crate): its tier-two `File::open`, anything
  that joins a path or wants more than the one granted directory, gets a namespace to resolve
  against.
- Milestone 47's `PATH` work, which is the same question scaled to programs rather than files,
  needs this before its own four open sub-questions are worth deciding in detail.

## What this does not decide

The spawn-protocol encoding for "which directory is which" is a real wire-format question (two
programs, the shell and init, must agree), in the same category 47's environment-variable section
already prices as reversible-but-real. Left to whoever builds this, following the existing
`DIR_BIT`/`GRANT_WORDS` precedent rather than inventing a new shape.

## What was built

Two second-level pieces, host-tested and guest-tested rather than left as design:

- `grant_plan::nav::TwoRoots` (provisional name), pure and host-tested in `crates/grant_plan`:
  composes exactly two labeled directory roots. Each label selects one grant's root by an exact
  match on an absolute path's first component; everything after that resolves through the
  existing `Cwd::apply`/`Cwd::ascend`, unmodified. That is the whole mechanism, and it is why
  `/a/../b` refuses for free: selecting `a` leaves nothing above `a`'s own root to pop, so `b` is
  never reached to be a question. It is deliberately not `bind`: it composes two fixed labels,
  not an ordered, shadowable union, and 47's four open questions (shadowing, enumeration, whether
  `$PATH` survives as a string) are untouched.
- `kernel::user::fs_service::start_granted_two_dirs`, the endowment mechanism itself: wires a
  second `fs_subtree_caretaker` alongside the first, for one confined program, and delivers both
  narrowed endpoints into two distinct cspace slots (slot 0 is always the first grant, slot 1 the
  second, and that ordering is the spawn-protocol position this milestone decides, deliberately
  the smallest possible answer rather than a new wire word). Both caretakers share the one FS
  server a boot has, and both narrowed endpoints map the same shared file-channel frame, safe for
  the reason `narrow_dir`'s own doc already gives one level narrower: the confined program is one
  thread of control with at most one `CALL` in flight.
- The guest proof, `kernel/src/user/multi_dir_namespace_tests.rs` (one module for both ISAs,
  `dir_capability_tests`' reason): a new `fs_test_client` role (`ROLE_TWO_DIR`) holds both grants
  at once, told nothing beyond which cspace slot is which. It proves the deliverable literally:
  `/a/inner` and `/b/secret` (the roadmap block's `/a/x` and `/b/y`) each resolve through
  `TwoRoots` and then open for real over the caretaker that resolution named; `/a/../b` is refused
  by `TwoRoots::resolve` before any request is sent; and, independent of `TwoRoots` entirely, grant
  A's endpoint cannot open the name that exists only in grant B's subtree and the reverse, which is
  the wire-level witness that the endpoint is the boundary (notes/dir-capability.md's structural
  finding), demonstrated here with two live caretakers instead of inferred from one.

## What is now decided, and what is still open

Kept as the 2026-08-25 record. Its labeled presentation (`/a/...`, `/b/...`, two labeled `caps`
rows) was replaced on 2026-09-26 by calef's one-tree ruling; see "The shell across two trees".

[DECISIONS §126](../decisions/126-two-directory-cwd.md) closed the ambiguity the first bullet
below used to name. A two-grant shell gets a real, single, moving `cwd`: state `(which, pos)`
in place of one-grant `Holdings`' bare `Cwd`, a bare relative name resolves against `pos` inside
whichever tree `which` currently names, an absolute `/a/...`/`/b/...` path both resolves and moves
between trees, and `..` at either tree's own root refuses exactly the way one-grant `Cwd::apply`
already refuses at its own root today. That is a real, single answer, not a per-caller choice, and
it does not answer 47's four open questions (unions and shadowing across more than two labeled
sources, enumeration, the compile-time-set-to-runtime-lookup gap, whether `$PATH` survives as a
string): those stay exactly as open as they were, since this decision only ever concerned two
disjoint, individually-labeled trees with one position at a time.

- `caps`'s display and `Holdings` are extended, host-tested, to §126's shape (2026-08-25).
  `grant_plan::Holdings` gains `second: Option<SecondDir>` (provisional name), carrying both
  labels and `which` beside the existing `cwd`/`pos`; a one-grant `Holdings` (`second: None`)
  resolves and prints exactly as it always has, byte for byte (pinned by
  `a_one_grant_holdings_resolves_exactly_as_before` and
  `holding_a_directory_changes_exactly_one_line_of_the_endowment`). `Holdings::resolve` is the
  `(which, pos)` combinator itself, built on a new `nav::TwoRoots::resolve_from`/`apply_from` pair
  (relative stays in the current tree, absolute crosses by label, `..` refuses at either tree's own
  root; each a host test in `crates/grant_plan/src/nav.rs`). `crates/swish::write_holdings` prints
  two directory rows and a namespace section with both labels when `second` is `Some`, marking
  which tree `cwd` currently stands in and printing the other at its own root (there is only one
  remembered position, per §126's "real, single, moving" cwd): this is milestone 154's own line,
  "`caps` gains a namespace section with more than one row", made real and tested
  (`a_second_grant_prints_two_rows_and_a_namespace_section`).

  What this increment did not reach, per-command grants carrying their tree, was closed on
  2026-09-26 by "The shell across two trees" below.

- `crates/system_initializer::boot` can construct and deliver a second, disjoint directory
  capability to the shell at boot, mechanically, for real (2026-08-25). `boot` takes a new
  `second_dir: Option<SecondDirGrant>` parameter; `Some` builds a second `fs_subtree_caretaker`
  (`build_caretaker`, the same function `spawn_service`'s dynamic `rm`-style grants already use)
  narrowed to `SecondDirGrant::name`, and delivers its endpoint into the shell's capability table
  at the slot after the filesystem pair, pushing the clock (already told to the shell numerically
  rather than assumed, per its own existing convention) one slot further out. This is the real init
  both boards run (`user/src/system_initializer.rs`, `fixtures/src/hello.rs`'s `init_boot` role), not a
  kernel-side test harness.

  Both real entry points pass `None`. What the second subtree should *be* remains calef's
  boot-time policy call (DECISIONS §126), unanswered by this increment on purpose. Two further
  gaps, recorded rather than hidden. First, this exact path is unverified against a real boot:
  `script/swish-check` is the only thing that runs a real init, nothing types a second grant
  through it, and the capability-table headroom at the point this builds a caretaker is the same
  spot a past bug already found tight (`boot`'s own `# BUGS` note says so; watch for "reaches
  userspace and prints nothing" first). Second, and this is the sharper gap: nothing tells the
  shell process it has a second grant at all. `_start`'s three `START` words (role, argument,
  clock slot) are already fully spoken for, so a shell built with a second grant today would hold
  a capability its own `Nav` has no way to learn the label or slot of. `components/src/swish.rs`'s
  `holdings()` therefore still always reports `second: None`. Closing that gap is a real
  shell-to-init wire question of its own (a fourth `START` word, or packing the clock slot and a
  second-dir slot into the same word) and deserves its own decision rather than a quick encoding
  chosen under this lane's time pressure.

- The shell-to-init spawn-protocol encoding is built (2026-08-25), following `DIR_BIT`'s own
  precedent exactly as this block said it should: `grant_plan::spawnproto` gains `DIR2_BIT` and
  `Wiring::dir2`, a second bit rather than a count, round-tripping independent of the other six
  flags (host-tested, `the_wiring_flags_do_not_collide` extended and
  `a_second_directory_grant_is_a_second_bit_not_a_count` added). `crates/system_initializer::spawn_service`'s
  init-side decode is *not yet* extended to build a second caretaker when `dir2` is set: nothing on
  the shell side ever constructs a two-directory `Endowment` to set the bit with (no manifest declares
  two directory operands), so this is the wire format alone, ahead of an emitter, in exactly the
  shape `DIR_BIT` itself was built in before anything could construct a one-directory grant either.
## The shell across two trees, built 2026-09-26

Lane `milestone/154-two-directories`. The detail is [notes/two-trees.md](../../notes/two-trees.md)
(provisional name).

- The presentation is calef's 2026-09-26 ruling: one root, and the second tree at a mount point.
  `pwd` prints the mount path. `cd ..` from a mount point reaches its parent, which is this lane's
  choice pending calef (the written ruling leaves it open). An earlier cut of
  this lane printed per-tree labels (`/b/logs`) and was replaced the same day.
- One resolver. `grant_plan::Holdings::anchor` says where a path starts (where the shell stands, a
  bound name, or the root), `Cwd::apply` applies it, and `Holdings::locate` says which tree serves
  the result. The shell's walk, its `plan_path` and the planner's `designate` all use it, which
  also fixed `wc /recent/x` ignoring a bind that `ls /recent` honoured.
- Per-command grants carry their tree: `FileGrant` and `DirGrant` gain a required `which`.
- A walk carries its endpoint. `swish`'s `Nav` holds a second `Tree`, and every request on a walk's
  handles goes to that tree's slot, because two caretakers number handles independently.
- A correction: `nav::Cwd::ascend` left the popped component behind the depth, so the derived
  `PartialEq` called two routes to one position different.
- The guest proof is a new `swish` role (`ROLE_TWO_TREES`, provisional) over
  `start_granted_two_dirs`' caretakers:
  `a_shell_holding_two_trees_moves_between_them_and_crosses_neither`, an exact twelve-bit set with
  four never-allowed bits, on every ISA with an initrd.
- A surprise, recorded where it happened: a second, identical pair of caretakers filled the
  suite's 256-slot region table, and an unrelated later test failed to load. `start_granted_two_dirs`
  now reuses a pair asked for twice.

## Follow-on

- **Proposed.** Wiring a second filesystem into the boot shell at a mount point, with the transport
  calef chose (a named slot and a shared constant holding the mount path), the Gone-and-go-home
  behaviour for a dead mount, and `rm` under a mount. Its trigger is the first real second
  filesystem: `design/roadmap/proposals/a-second-filesystem-mounts-in-the-boot-shell.md`.
- **Proposed.** What `..` does at a mount point, which the written ruling leaves open: the shell
  goes to the mount point's parent, as Unix and Plan 9 do, and calef is asked to confirm:
  `design/roadmap/proposals/what-dot-dot-does-at-a-mount-point.md`.
- **Recorded.** `spawnproto::DIR2_BIT` has no emitter and no decoder in init, because no manifest
  declares two directory operands and §170 (how a foreign program is told what to do), as ruled
  2026-09-26, grants a named directory per the manifest. In `notes/two-trees.md`'s BUGS.
- **Recorded.** The planner passes an ~1 KiB `Holdings` by value through four frames, so the
  globbing witness needed a seventh stack page when it grew. In `notes/two-trees.md`'s BUGS.
- **Done.** `bind` was built by milestone 47 (navigation and naming) on 2026-08-26 and now points
  through a mount point: the two-tree witness binds a name to `/second` and lists it.
- **Milestone 47.** The four open namespace questions (shadowing across more than two labeled
  sources, enumeration, the compile-time-to-runtime lookup gap, and whether `$PATH` survives as a
  string) are untouched here and stay 47's, which its own block confirms.
- **Recorded.** The two-root type, the second-directory type and its grant ship as provisional
  names, said so where they live, and naming is calef's. So do this lane's: `Holdings::anchor`,
  `locate`, `place`, `SecondDir::mounted_at`, `swish`'s `Tree` and `ROLE_TWO_TREES`, the
  `fixture::twotrees` bits, and `notes/two-trees.md`.

## Index row

Milestones 47 (`bind`) and 64 (tier-two `File::open`) independently named the same missing
primitive: nothing granted a second directory capability to one process. BUILT 2026-09-26: one
process holding two trees, and the real shell presenting them as one tree with the second mounted
in it, per calef's ruling. The boot shell gets a second tree with the first real second filesystem.
