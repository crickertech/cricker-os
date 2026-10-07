---
status: BUILT
raised: 2026-10-06
built: 2026-10-07
milestone_dependencies: 633
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 800. A non-Anthropic model attacks the confinement claim

Raised 2026-10-06 (UTC) by calef: "Tomorrow GLM 5.3 can do an attack. We should probably have a
milestone for that." Minted by milestone 633 (an outside agent attacks the confinement claim)'s lane
as the maintainer's delegate. The number 800 is provisional; other lanes are minting from 769 to 799,
so expect renumbering at merge. *(Title, slug and every name below are drafts.)*

**Reuse:** the counting rule and the report format are milestone 633's third pass, unchanged, so the
passes compare row for row. The isolation is not: this attack is informed.

## Index row

A model from another vendor, GLM 5.3 first, attacks the published confinement claims and counts an
attack only when it boots on three ISAs. It serves risk 7's green criterion (c).

## Why

Risk 7 (the confinement claim is false) is AMBER, re-affirmed by calef on 2026-10-06. Its criterion
(c) for green is two consecutive independent attacks with no escape on a shipped path. At least one
must be by a non-Anthropic model or a human. All three of milestone 633's passes were Anthropic
models, so shared training may mean shared blind spots. This is the first attack that can count
toward the non-Anthropic half. Ruled since (calef, 2026-10-07, PR #1798's thread): it does not
count; see the Result section.

## How calef runs it

calef launches GLM 5.3 with opencode, and opencode can read a milestone. So this block is the brief.

1. Open a lane: `script/claim milestone/800-a-non-anthropic-model-attacks-the-confinement-claim`.
2. `cd` into the worktree it prints.
3. Start opencode with GLM 5.3 and say: "read milestone 800 and do it".
4. When it finishes, push the branch and hand the pull request to the maintainer.

## The attack is informed

calef ruled on 2026-10-06 (UTC): "GLM should run informed. That's a more realistic attack for an
open source OS." So opencode runs in a normal checkout with the whole tree and its git history, as
any attacker of a public repository has them. Milestone 633's passes were blind to history by
request. This one is not, on purpose.

Informed changes what counts. Earlier passes found and fixed escapes, and their records are a map:
the git history, `design/audit-reports/`, the `notes/confinement-outsider-pass-*.md` notes and the
narrative below the claims table. A re-discovery of a fixed escape is reported, but scored
separately. Only a new finding counts toward risk 7.

## The attacker's brief

You are an outside security reviewer. Your job is to break a published security claim of an
operating system you did not write. Assume every claim is false and look for the case that makes it
so. An escape is the most useful thing you can find.

### The system

nife is a capability microkernel in Rust for aarch64, riscv64 and x86_64. Its
confinement claim: a confined component cannot reach an object it was not granted. The claim is
broken into numbered claims in the table under "## The claims" in `notes/confinement-claims.md`.

### Where to look

You have the whole repository and its git history. Use them as an attacker
would. Two searches, both required:

- **Variant analysis.** Every escape found and fixed so far is recorded: in the git history, in
  `design/audit-reports/`, in `notes/confinement-outsider-pass-*.md`, and below the claims table in
  `notes/confinement-claims.md`. For each, ask whether the fix closed the class or only the
  instance. Look for siblings: the same mistake on another path, another object type, another ISA.
- **New ground.** Attack where nothing has been found yet: claims no record mentions, services no
  pass drove from a hostile client, and surfaces with no claim at all.

### Scoring

Only a new finding counts. If you re-find an escape that is already fixed, report it in
its own section as a re-discovery, and check whether the fix still holds.

### What counts

An attack counts when it boots. Write an exploit program or a kernel test that runs
under QEMU on aarch64, riscv64 and x86_64 and observes whether confinement held. x86-only claims run
on x86_64 only. Reading code to find where to aim is expected. A verdict from reading alone is
graded `read`, the weakest grade. Where a claim concerns a userspace server, build a hostile client
that drives it. A host-crate proof is supporting evidence, not a booted attack.

### Mechanics

- Kernel and system tests live in `system_tests/src/user/`. Read `notes/scripts.md` and
  `notes/qemu.md` first.
- Run one test or module with `script/test --arch <aarch64|riscv64|x86_64> --test <substring>`.
- Run one QEMU or test command at a time. Never run `script/verify` or a mutation sweep.
- Any interactive QEMU run goes through `helpers/qemu-bounded.sh <seconds> <cmd...>`.
- When you finish, `pgrep -l qemu` must show nothing you started.
- Shape an exploit so a break ends the run (exit or fault). A break that hangs the watchdog is
  not observable.
- You may add fixtures anywhere in the tree when an attack needs one.

### Escapes

Write each escape as a test that is red because of the escape, and boot it to confirm.
Commit the failing test before anything else. Do not write the fix. If a red test would break the
default suite, make it opt-in (skipped unless named with `--test`) and say so in its doc comment. A
path that is one bug away, or holds by accident, is a near miss.

### Output

1. Your exploit tests, committed. Keep a passing test only if it proves something no existing test
   does.
2. Results in `notes/confinement-outsider-pass-4.md` (name provisional). One row per claim: the
   attack, the outcome, the evidence grade, and the test name. The outcome is exactly one of
   escape, near miss, held, or untestable with the reason. The grade is `booted <isa>`, `host` or
   `read`. Then one short section per escape and per near miss, a section of variants checked
   against each fixed escape, and a section of re-discoveries.
3. A refusal log, as the last section of that note, titled "Refusal log". Every step you declined,
   or were stopped from taking, gets one entry. An entry names the claim you were working on. It
   says what you were trying, in one sentence with no exploit detail. And it says whether the
   refusal was your own or an external safety system's. An empty log says "none" and is still present. A refusal
   is a finding, not a failure, so do not leave one out.
4. Write plainly. No em-dashes. No "delve", "comprehensive", "notably", "moreover", "furthermore" or
   "straightforward". Keep sentences under 40 words.
5. Commits: one purpose each, and the message says why. End every message with
   `Co-Authored-By: GLM 5.3 <noreply@z.ai>`. Do not use `git stash`. Never edit `design/`,
   `AGENTS.md` or `CLAUDE.md`.
6. Finish with a report: counts by outcome (new findings and re-discoveries apart), each escape and
   near miss in two lines, each test with the ISAs it booted on and the result, and the refusal log's
   entry count.

## The refusal log is a standing rule

calef, 2026-10-06 (UTC): "Add the refusal log." It binds every attack pass from this one on, not
only this milestone. A refusal names an attack path no pass has tested. So each entry becomes an
explicit target of a later pass, the fuzzing milestone (provisionally 779) included. Risk 7's
criterion (c) does not count a pass that leaves a refusal on a shipped path unexamined.

## Where the results go

Straight into the lane: `notes/confinement-outsider-pass-4.md` (name provisional) and any tests,
committed on this milestone's branch and reviewed as its pull request. The maintainer then gives each
finding a home, as milestone 633 did: a fix with a falsifiable test, a `BUGS` entry, or a proposed
milestone. Risk 7's appendix cites the pass through the maintainer under §216 (fatal-risk facts are
correctable, and verdicts are the architect's). Moving the color stays calef's.

## Done means

- Every claim has an attack, or a written reason it was not attacked.
- Every escape is a failing test committed before any fix.
- The pass-4 note exists, with its refusal log, and every finding has a home.
- Every refusal entry on a shipped path is examined by a later pass or recorded as a target.

## BUGS

- How opencode is configured to reach GLM 5.3 is calef's and is not recorded here.
- An informed pass cannot say whether a fresh mind would find what earlier passes found. That was
  the blind passes' question, and milestone 633 answered it three times.
- Gating in the lane takes the machine-wide `nife-dev` toolchain link, as every gating lane does.
  Relinking is the maintainer's at merge.

## Follow-on

- **Milestone 198.** A human review, or a public bounty once a stranger can install nife, is the stronger form. Both
  wait on milestone 198 (a package manager, and the trivial install that makes a second customer
  possible).

## Result (BUILT 2026-10-07, UTC, by this lane)

The pass ran informed, per the brief, and its note is `notes/confinement-outsider-pass-4.md`.

- **One booted re-discovery, red on all three ISAs**: a second client of a shared `Stack` endpoint
  pre-attaches its own frame at the victim's socket id and captures both directions of the victim's
  exchange. `ATTACH` is a `SEND_CAP` with no reply and the id namespace has no per-caller scope, so
  the victim's attach fails silently. Pinned while the pass ran by the opt-in red
  `net_confinement_tests::a_squatter_at_a_shared_stack_endpoint_cannot_capture_the_clients_traffic`
  and a BUGS entry in `net_stack.rs`. The fix shape was first ruled per-caller windows keyed
  by badge, the resolver's exact shape, refusals silent (calef, 2026-10-07 UTC, about 03:50Z,
  PR #1798). Superseded the same day (15:04Z): each socket is its own capability; the stack
  mints a badged endpoint per socket on open, every later call is made on it, and the `sid`
  leaves the wire. Built as milestone 649 in #1817 (merged 2026-10-07 UTC), which rewrote the
  server, the contract and this pass's pinned test. The test now runs green in the default suite
  on all three ISAs with a replayable falsification, and the BUGS entry went with the defect.
  Whether refusals also become answerable is a smaller second call, still open. Ruled the same
  day, tree-wide and still standing: per-caller scoping is a written rule for every multi-client
  window server, recorded as §256 (a server that keeps windows for many clients scopes each by the
  caller's badge). 649's lane read `system_log`, the file service and `name_resolver` (§255 (each socket is its own capability)); the
  rest of the audit is milestone 823 (every multi-client window server is audited for caller
  scoping). A
  second premise this pass settled by reading:
  the escape is on a shipped path. The interactive boot endows every network-declaring child with a
  copy of the one stack endpoint, unbadged, and the job pool holds six live jobs at once.
- **How the pass counts toward risk 7's criterion (c): ruled (b), an escape on a shipped path**
  (calef, 2026-10-07, PR #1798's thread). The two-consecutive count restarts at zero and stays
  there until 649's defect is fixed and a fresh pass comes back clean; a known unfixed escape
  counted as anything else would flatter the verdict. The finding itself was already recorded as
  milestone 649 (every client of a network stack shares its socket numbers), raised 2026-09-24,
  NOT-STARTED, no test; this pass's addition is the booted bidirectional
  capture, pinned red on all three ISAs. The capability ruling (calef, 2026-10-07 UTC, 15:04Z)
  resolves 649's option fork toward option 2, an endpoint per socket; #1817 updated 649's block,
  and 649 is BUILT. The count still waits on a fresh clean pass.
- **The claims row the fix will owe: ruled (i), twice.** The first wording (calef, 2026-10-07,
  PR #1798's thread) was the window-model sentence: *a socket's window is the frame its holder
  attached; a second client of a shared Stack endpoint can neither substitute nor capture
  another's socket traffic.* It described the window model and died with it. Re-ruled for the
  capability model (calef, 2026-10-07 UTC, PR #1798): *a program reaches only the sockets it
  holds; a socket moves only by its capability.* The closed-socket half of #1817's second test
  stays with claim 30's revocation row, where it belongs; the evidence column cites #1817's two
  rewritten tests and the decision file that PR carries. Added at merge as claim 34 of
  `notes/confinement-claims.md`.
- **The refusal targets' order: ruled (i)** (calef, 2026-10-07, PR #1798's thread). The redoxfs
  name-window TOCTOU boot is the next probe, minted at merge as milestone 825 (a hostile client
  races the file server's name window).
  It is the likeliest live escape and the only item that could change a verdict this week. The
  compositor respawn scrub waits for a reachable respawn path, since no red boot test can exist
  before one does. MSI confinement stays with the milestones that already own it. Kani re-runs in
  attack passes need no home: the pass brief already forbids them, which is a process rule, not a
  worklist item.
- **The retype-GRANT question, 633's carry, is ruled (a)** (calef, 2026-10-07, PR #1798's thread):
  the mint is the intersection of the budget's rights and `Rights::ALL`, so a `WRITE`-only budget
  mints `WRITE`-only frames and `GRANT` cannot be minted, only delegated. Not this pass's finding;
  presented here because the question was open. The fix is milestone 824 (a retype mints no right
  its budget lacks): the kernel intersects at retype, and
  `confinement_attack_tests::a_grant_less_budget_mints_a_grant_bearing_frame` flips from
  characterization to held assertion. Recorded at the test's doc and `cap.rs`'s BUGS in this
  branch, and in 633's block at merge.
- **The routed `chatty` reshape landed.** Claim 26's own test now fails rather than hangs: the
  operator retires the last receiver, a plant parks the marker, and a let-open `RECEIVE_CAP`
  returns it. Green on aarch64 (whole module) and riscv64, red at its own assertion under the
  recorded patch.
- Every fixed escape was checked for siblings (the abort-path `outgoing_cap` follow-up closes the
  sharpest); every claim has an attack or a written reason; the refusal log has four entries, each
  with a home or a target. Coverage was targeted boots, not the suite: two rows booted this pass
  (26 and the escape), 31 read.
- Done at merge, 2026-10-07 (UTC), by the maintainer session. §256 records the tree-wide scoping
  rule, and 633's block records the retype ruling. Claim 34 is the claims row. Milestones 823, 824
  and 825 are minted. Risk 7's page cites this pass under §216 with the count restarted, and its
  appendix has a section for it. Still open and non-blocking: whether the socket refusals become
  answerable.
