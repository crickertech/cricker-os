---
status: PARTIAL
raised: 2026-07-31
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 47. Navigation and naming: `cd`, `pwd`, `ls`, `mkdir`, `rm`, paths, and environment

The token read `IN-PROGRESS` with no branch anywhere and nobody holding it, found
2026-08-17 by the status-accuracy sweep. Two claims in the "still to do" sentence below were also
false, and both are corrected there rather than here: the glob caretaker was built 2026-07-31, and the
`std` PAL's three namespace verbs were bound 2026-08-04.

Discharged 2026-08-18. The navigation half is built. The namespace half (absolute paths,
environment, `PATH`, and `bind`) had no forcing use case from the shell, and this block's own
sequencing was to let milestone 64 measure first so a real crate's demands could size the remaining
scope. That measurement has landed and it did its job, so the gate it was waiting for is
discharged rather than merely aged.

Where this stands, 2026-09-26 (UTC), `milestone/47-navigation`. Every item below was checked
against the tree that day. One was built: `caps` prints the inert-configuration values a child
will read, from the boot shell's own read-only view of the same frame (DECISIONS §111 (inert configuration is a validated page)'s preview;
notes/env-config.md). One was settled elsewhere: the function-call syntax fork was refused by
§141 (application is grant) on 2026-09-03. Completion was built the same day, once calef ruled
§227 (how Tab reaches the shell) as option D: the shell edits its own line over raw mode, and Tab
finishes a program or file name from its own authority (notes/shell-line-editing.md). The other
two calls were ruled the same day and are work to build:

- A set grant at the prompt, which is what `xargs <program>` has been waiting on and, found
  today, what refuses a plain `rm *.txt` over two files: `spawnproto` cannot carry a set. Ruled
  2b in §228 (how a set of matched names reaches the progenitor): a page the shell fills
  (notes/a-set-grant-at-the-prompt.md).
- `PATH` for installed programs: the manifest question that blocked it is answered by §208 and
  §219, and what remains is how a bare name reaches an installed program. Ruled B2 in §229 (how a
  bare name at the prompt reaches an installed program): the activation set, never an owner's
  vouch.

The secrets third moved out of this milestone: it is §41 (the endpoint is the broker)'s endpoint, milestone 65 (a secrets service)'s service and
§165's open question, and no program on nife needs one (Follow-on, below).

<!-- prose-budget: exception. 17,386 words (wc -w, this marker included) against a 3,000-word
     cap. Ratified by calef on 2026-09-25 (UTC), who ruled that this block is not split. Reason:
     large and rarely referenced; the maintainer counted 7 files linking it that day. Roadmap blocks
     as a class stay under the cap, per §212 (a prose budget), which refused a class exemption.
     Marker syntax is PROVISIONAL until the prose-budget gate exists. -->

Absolute paths were built 2026-08-18 (`milestone/47-namespace`), the first piece of the namespace
half and the one the other three lean on: `/` is the root of your own namespace, Plan 9's answer,
in the shell and in the `std` PAL together so that one fork was answered once.

Environment variables' inert third was built 2026-08-23 (`milestone/47-env-config-page`), the
second piece of the namespace half: a read-only page carrying `TZ`, `LANG` and `TERM`, each
validated against a closed domain before it is ever written (DECISIONS §111 (inert)). See "Environment
variables" below and notes/env-config.md. What is built is end to end for a std program
(`env_proto`, the kernel-side assembly and mapping, the `std` PAL's `sys/env::seed`), proven by
`std_exerciser` on both ISAs; what remains is a shell-facing customer to declare wanting it (no
`grant_plan::Manifest` field exists yet, the position `clock` was in before `date`), which is what
would also make the `caps` preview extension §111 asks for meaningful. The other two thirds of
what Unix calls "environment" stay where this milestone always put them: names (`PATH`, `HOME`)
wait on `bind`, and secrets are answered elsewhere by an endpoint (§41).

That shell-facing customer was built 2026-08-26 (`milestone/47-remainder-round2`): `printenv`
and `grant_plan::Manifest::config`, wired into real init on both boards, not only the kernel test
harness that stood in for it above. See "The shell-facing customer was built 2026-08-26" under
"Environment variables" below.

`bind` was built 2026-08-26 (`milestone/47-namespace-remainder`), closing the gap this block's own
"blocked on a second grant" section named: milestone 154 supplied the second grant, and this lane
found the doc's own sketch ("a bind entry is a value, not a capability... a `nav::Cwd` under a
name") held up against the real, now-built mechanism without change. `crates/grant_plan::nav::Bindings`
(provisional name) is a small fixed table, up to four entries, mapping a name to a `(Which, Cwd)`
position this shell reached some other way; `Holdings::resolve` tries a grant label first
(when this shell holds two), then the bind table, then, for the one-grant shell every real boot is
today, the same literal walk from the sole root it always did, so `bind` is additive and a shell
with nothing bound resolves byte for byte as before. `Holdings::bind` is the mutator, refusing a
name that collides with a grant label in addition to everything the table itself refuses (bound, full, unnameable). Wired into the real interactive `swish` as a builtin, `bind <target>
<name>`, in `mkdir`'s category (mints no capability, spawns nothing): `target` resolves through the
same bind-aware `plan_path`/`walk` every other verb now shares, so binding a name under an existing
bind composes rather than needing a special case, and `..` past a bound name climbs the *real* tree
the bind points at rather than stopping at a boundary invented at the alias (the same "misdirect,
never grant more" property §50 (namespace) gives symlinks-as-bind). Proven over the real wire, both ISAs,
extending `kernel::user::shell_navigation_tests`' navigating witness: a bound name lists the real
directory it points at (`ls`), three `cd ..`s from inside it reach the real parent, the real root,
and refuse there exactly where a direct walk to that depth would.

What this increment does not reach, honestly. `bind` in a *two-grant* shell (composing two
disjoint trees under more names than the two grant labels) is host-tested in `grant_plan` but not
guest-provable, because milestone 154's own gap is still open: nothing tells a real, live `swish`
process it holds a second directory capability at all (`_start`'s three `START` words are spoken for), and both real init entry points still pass `second_dir: None`. That gap is 154's, not
this lane's to close: it is a spawn-protocol wire decision 154 itself declined to make "under this
lane's time pressure," and what the second directory should even *be* is an architect's boot-time
policy call (DECISIONS §126 (moving `cwd`) made the harder judgment call
next to it, the real single moving cwd, without answering this one). This lane did not invent an
answer to either.

`ln`'s symlink half is retired from "still to do" rather than built as a separate command: DECISIONS
§50 settled that symlinks-as-stored-paths are superseded by `bind`, not implemented beside
it, so `bind` landing *is* that half of `ln` landing, under Plan 9's name rather than Unix's. Hard
links stay declined (DECISIONS §110 (hard), want of a customer); see the `ln` section below, kept as
history, with this line added rather than rewritten.

Completion and `PATH` were investigated on 2026-08-26 and neither was built. Each turned out to
have a real fork underneath, and their own sections below carry the findings and the options
(cut from here on 2026-09-26, where they were repeated word for word in substance).

`RMDIR` and `rm -r` were also already built, found 2026-08-22 by the same kind of status check
that caught the `IN-PROGRESS` token above: the code (`components/src/rm.rs`, `fs_proto::fs::RMDIR`), the
decision (`DECISIONS §49`) and the concept note (notes/rm.md) all say `Built 2026-07-31`, but this
roadmap block never got the matching annotation, so the "rmdir and rm -r" section below read as
still-undecided years after the design it describes shipped. Recorded here rather than left for the
next reader to rediscover in the git log.

`touch` is fully built. The create half landed 2026-08-22: `touch <name>` makes an empty file if
the name is not there and does nothing if it is, using `fs_proto::fs::CREATE` (already built for
milestone 31 phase 2) through the same shell-builtin shape `mkdir` already has. The mtime half
landed 2026-08-24 once DECISIONS §112 settled the authority question: bare `touch` bumps to now
(`fs::SETMTIME`, needs only `dir::WRITE`), `touch -t <RFC-3339-instant>` asserts an arbitrary one
(`fs::SETMTIME_AT`, needs `dir::WRITE | dir::SETTIME`, the seventh rung on the directory rights
ladder). See the `touch` section below and notes/touch.md.

`bind` was built 2026-08-26, and it is the mechanism environment's "names" third pointed at
("`PATH`... and `HOME`... wait on `bind`"), but it does not by itself close that third: `bind`
files a *directory or file position* under a name, which is the whole of what `HOME` ever was (a
directory capability wearing a string costume) and nothing consumes it specially yet, no `cd` with
no args reads a bind the way Unix reads `$HOME`. `PATH` is the harder half of "names" and stays open
for the deeper reason below. What remains is in "Where this stands, 2026-09-26" at the top.
`ln`'s symlink half is retired rather than unbuilt: DECISIONS §50 (namespace composition) already settled that symlinks-as-stored-paths are superseded by `bind`, not built beside it, so
`bind` landing is that half landing. Hard links stay declined for want of a customer (DECISIONS
§110).

64's second pass (2026-08-18) reports that nothing in this milestone was ever waiting on 64, and
hands over the sized demand this block asked for: named customers at ranks 16, 18 and 27 plus
path-joining, and one 64 did not expect: a *seeded* environment has to arrive from this
milestone's endowment, because `std::env` on nife starts empty by construction and 16 direct
consumers want otherwise. `env::var` was rank 4 with no PAL at all, and the shape of its absence is
the warning worth carrying here: `getenv` answered `None` harmlessly while the same fallback's
`env()` was `panic!`, so `std::env::vars()` aborted the process and compiled perfectly. The
dangerous refusal is the one that answers.

One sequencing question is open and deliberately not made a gate. Milestone 122 (`OPENDIR`
reaches the PAL) is `NOT-STARTED`, and some of the namespace half may want it; the 2026-08-17 status
sweep raised that and declined to rule, and this line does not rule either. Recording it as `NONE`
with the question named is the honest state: a lane can start, and the first thing it should
establish is whether its piece needs 122. Turning an unestablished dependency into a gate is how a
milestone sits blocked on nothing, which is what the twelve days behind this block's own corrected
sentence cost.

In brief. A navigation model for a system with no global namespace. Keep the Unix command names
and behaviour wherever they can work honestly; diverge only where the capability model forces it, and
say why each divergence is earned. The keystone is built (the directory capability and its
rights ladder, six rungs at the keystone and a seventh added by DECISIONS §112 for `touch -t`,
DECISIONS §47 (the directory capability's rights), notes/dir-capability.md), and so are the five commands, on
both ISAs: `cd`, `pwd`, `ls`, `mkdir` and `rm` as shell builtins, `..` clamped at your root by
popping the stack of capabilities the shell descended through, `pwd` relative to that root, and a
name on a command line resolved against the shell's position at the moment the grant is made, so
a child holds a capability to one file and cannot re-resolve anything. `rm` is `UNLINK`, added to
`fs_proto` here and separated from revocation in the contract's own words; revocation is not offered,
because the FS server's handle table is per *server* and it cannot enumerate the clients holding
handles. The headline is proven with the real shell binary: two shells rooted in two subtrees, each
told nothing about which it holds, and neither can name the other's files (notes/shell-navigation.md).
The glob caretaker is built too, which this sentence claimed as outstanding for seventeen days:
built 2026-07-31 (merge `5e48826c`, branch `milestone/47-glob-wiring`, wiring commit `00f4e277`; no
pull request, it predates the workflow), and proven end to end on both ISAs by
`what_a_shell_shows_is_what_a_set_grant_takes_away`
(`kernel/src/user/glob_grant_tests.rs:29`, no arch `cfg` at `kernel/src/user.rs:2259`), which runs the
real `swish` binary expanding one pattern two ways and the real `rm` binary behind a real
`fs_nameset_caretaker` holding only `REMOVE` over the two matched names. `rm` is then pointed at a
name one directory entry outside the set and gets `ENOENT`. Witnessed from the host by
`xtask::redoxfs_glob_grant_took_exactly_the_match` (`xtask/src/main.rs:4360`). See notes/glob-grant.md,
which has said "Built 2026-07-31" the whole time, and the sections below at "Built 2026-07-31: the
matcher, then the grant", which contradicted the sentence from inside this same file.

Still to do: see "Where this stands, 2026-09-26" at the top of this block, which replaced a
longer list here that had drifted from the sections below it. The `std` PAL's `rename`, `unlink`
and `rmdir` were bound by milestone 64 (enough `std` to run somebody else's crate) on 2026-08-04 (pull request #113); they answer `Unsupported`
only when the process holds no filesystem capability at all.

Why it matters. calef's framing, and it is the governing constraint: *"I hate Windows/DOS
specifically because they went differently than virtually every other OS I've used."* Gratuitous
divergence taxes every user forever. So the bar is not "is this more capability-pure", it is "does
the model actually force this." Three divergences clear that bar; the rest of Unix's surface should
survive unchanged.

## The reframe: `cd` was never the problem

A working directory, in capability terms, is *a directory capability the shell holds, used as the
default base for resolving names*. Held by the shell that is entirely legitimate, the same as its
untyped budget. The badness in Unix is three specific things, none of which is `cd` itself:

1. Children inherit it silently, so every process gets a starting point nobody granted it.
2. Relative paths resolve implicitly, so a program's reach depends on invisible state.
3. `..` walks out, so the cwd bounds nothing.

Fix those three and the command is fine.

## `cd`, `pwd`, `ls` are shell builtins, not programs

The same category as `caps`, which already prints the shell's whole endowment: they spawn nothing,
need no grant, and confer no new authority, because the shell is reading and rebinding what it already
holds. This also retires a worry raised while designing `ls`: that a listing program would be
over-granted, holding the power to read everything it lists. It is not a program.

The cwd stops at the process boundary. `wc report.txt` resolves the name against the
shell's current directory *at the moment the grant is made*, and the child receives a capability to
that one file. The child has no cwd, inherits nothing, and cannot re-resolve anything. The convenience
is the shell's; the authority is explicit.

## The earned divergences: three, then two

- ~~No global absolute paths.~~ Retired 2026-08-18, and it was never a position: it was the
  honest state of a system that had no namespace to root a path in. See "Absolute paths: Plan 9's
  answer, not DOS's" below, which is now built. The `InvalidFilename` refusal it describes survives
  for the two cases that still name nothing, `..` and a Windows-shaped prefix.
- `..` stops at your root. You descend from what you hold and never ascend past it. This is
  chroot's shape arrived at from the other direction.
- `pwd` is relative to your root, because naming anything above it implies a namespace that does
  not exist.

What that buys, and Unix cannot: every shell has its own root. Two shells can hold different
subtrees and neither can name the other's files, not by policy but because no capability reaching them
exists.

## `mkdir` and `rm`

`mkdir` is the same verb family as descending: it mints a directory node and hands back a
capability to it, exactly as `CREATE` already returns a file handle. `mkdir` is descend-with-creation,
and the two should be designed together rather than separately.

`rm` is where Unix conflated two operations. `rm` unlinks a name; the data survives while anyone
holds a descriptor, and the blocks survive after that, so it cannot promise what people mean when they
delete something sensitive (and `shred` only pretends to on a copy-on-write filesystem like ours).
Separate them:

- Unlink: remove a name from a directory; existing capability holders keep reading. Unix's
  semantics, and genuinely useful (atomic replace and the temp-file idiom both depend on it).
- Revoke: the object dies and *every* capability to it goes stale.

The second is not exotic here: §13 revokes frames, §16 revokes objects, and generational names
(`crates/slots`) make a stale capability fail safely rather than point somewhere wrong. One
implementation caveat to design rather than gloss: the FS server validates handles against its own
table, so invalidating them is mechanically easy, but that table is per-session and the server does
not track all outstanding sessions today.

The rights ladder becomes explicit: a directory capability needs separable enumerate, open
(read versus write), create and remove. A program handed a directory to write logs into should
not thereby be able to delete what is there. `FileSpec` already makes this split for files, where the
manifest declares direction and the human designates the file without typing a mode.

And one safety property falls out free. `rm -rf /` is bounded here by what your directory
capability reaches, structurally. A shell rooted at a subtree cannot recursively delete the system,
because no capability naming those files exists in it. Not a guard rail, not a confirmation prompt,
not a check that could be wrong: there is nothing to name.

## `rmdir` and `rm -r`: Unix already made the safe choice (decided and built 2026-07-31)

### Built 2026-07-31, and the annotation added 2026-08-22. See DECISIONS §49 and notes/rm.md.

The code, the decision and the concept note have all said "Built" since the day this section was
written; this roadmap block did not, and that gap outlived the design it describes by three weeks.
The section below is kept as written, because the reasoning is the reasoning that shipped; treat it
as history rather than as an open question.

`mkdir` shipped in §48 (navigation) with no way to remove what it makes: `rm` answers `EISDIR` and there is no
`RMDIR`. The lane declined to add one, on the grounds that "a verb that removes whatever it finds is
how one word takes a subtree away". That objection is right about a *recursive* verb and does not
apply to Unix's, which is the point.

`rmdir(2)` removes only an empty directory, and that is the whole safety property. The recursion
in `rm -r` lives in userspace, as a loop of individually safe single-step operations: walk, unlink
files, remove empty directories bottom-up. No single call in the contract can take a subtree away.

So: `RMDIR` requiring `REMOVE` on the parent, refusing non-empty with `ENOTEMPTY`, and explicitly
not revocation, for §48 (navigation is the shell rebinding what it holds)'s reason: the handle table is per server, so handles cannot be
invalidated for clients the server cannot enumerate.

The recursion is bounded by construction, which Unix cannot say. `rm -r` needs `ENUMERATE` to see,
`DESCEND` to recurse and `REMOVE` to delete, *at every level*, so the walk stops exactly where the
capabilities stop. Unix bounds `rm -rf /` with a permission check per file, which is a check that can
be wrong and famously has been. This milestone's existing note stands: not a guard rail, not a
confirmation prompt, "there is nothing to name".

`rm` is a program, not a builtin, and that is Unix's shape rather than a divergence from it.
`cd`/`pwd`/`ls` are builtins here because the shell is rebinding what it already holds; `rm -r` is a
destructive loop, not a rebinding. A builtin would run with the shell's entire endowment, while a
program takes an explicit attenuated grant, so `caps rm -r logs/` prints the subtree at risk before
anything happens, and a bug in the recursion can only reach what it was handed. Same shape as
globbing below: attenuate, then hand over.

`-f` stays, with Unix's semantics (calef, 2026-07-31). An earlier draft of this section argued it
should not exist, on the reasoning that with no prompting its only remaining meaning is suppressing
errors, which §42 (a filesystem declares what it offers) forbids. That was wrong about what `-f` does. It means *ignore nonexistent files
and do not prompt*: a permission failure on a file that exists still reports. Its real value is
idempotency: `rm -f maybe-there` succeeding is what makes a script re-runnable, and "absence is
the desired state" is not a lie about failure. The divergence did not earn its keep.

Reporting is Unix's, and it is quieter than an earlier draft of this section claimed. Checked
against `rm(1)` rather than remembered: silence on success, `-v` exists precisely because the
default prints nothing ("be verbose when deleting files, showing them as they are removed"). Failure
is a diagnostic plus exit status: "exits 0 if all of the named files or file hierarchies were
removed… If an error occurs, rm exits with a value >0." So a partial `rm -r` says what it could not
do and exits non-zero, and says nothing about what it did. An earlier draft here said it should
"report what it removed", which is the `-v` behaviour, not the default.

`-f` is also broader than that draft assumed: "attempt to remove the files without prompting for
confirmation, regardless of the file's permissions. If the file does not exist, do not display a
diagnostic message or modify the exit status." So it suppresses the missing-file diagnostic *and*
its effect on the exit status. The claim that a permission failure still reports under `-f` was wrong.

One thing to settle when building it. A `rm -r` interrupted halfway leaves a partial tree, and
there is no transaction spanning requests: adding one would mean the server holding a transaction
open across receives, which conflicts with the serve-loop-runs-one-request-to-completion property §47
relies on for concurrency atomicity. Partial, with failures reported and a non-zero exit, is the
answer, and it happens to be exactly what Unix already does.

Worth noticing while copying Unix here: `rm(1)` says "it is an error to attempt to remove the
files `/`, `.` or `..`". That is a literal special-case guard for `/`, shipped in the utility,
precisely the "guard rail, a check that could be wrong" this milestone contrasts itself against. We
need no such case: a shell holding a subtree cannot name the root, so there is nothing to special-case. And `rm` on a directory stays a refusal (`EISDIR`) rather than a silent
escalation to recursive removal, which is Unix's behaviour and worth keeping for the same reason
`rmdir` is empty-only.

## `ln`: hard links make it not a tree, and symlinks stop being an escalation

Two verbs with very different stories. Neither is built. Hard links are declined (calef,
2026-08-23, DECISIONS §110): no customer needs them, `mv`/`RENAME` already covers the atomic-replace
idiom they're usually reached for, and offering them would cost an audit of every place subtree
reasoning quietly assumes a tree rather than a DAG, for a feature nobody's asked for. Symlinks'
mechanism question is separately settled (§50, below: `bind`, not stored paths).

Hard links are mechanically easy. RedoxFS keeps link counts, and §48's deferred-delete fix
already depends on them: "the last link goes" is exactly what made `rm` an unlink rather than a
revoke. A second name for one node is a short step from there.

The problem is structural, and it is ours rather than Unix's. §47 justified `DESCEND` as a
separate right because otherwise "the shape of the tree would decide how much authority a grant
carried". Hard links make it not a tree. A file reachable from two directories sits in two
subtrees, so "this subtree" stops having a clean boundary: you granted a name, and the node is also
reachable through one you did not mention. That is not automatically wrong (the grant was the name),
but every piece of subtree reasoning written so far quietly assumes a DAG cannot happen, and that
assumption should be made explicit before it is falsified. Unix forbids hard links to *directories* to
prevent cycles; the argument is stronger here, where a cycle also defeats `rm -r`'s bottom-up
termination.

Symlinks are the interesting one, and the answer is a real result. A symlink stores a path,
resolved at open time, and this milestone already decided paths resolve in the client, against the
holder's own position, with `..` clamped at the root (§48). So: resolved against *whose* namespace?

Resolve against the holder's, and it follows that a symlink cannot escalate. It can only name
what the resolver could already reach. Unix's symlink attacks: the `/tmp` races, the confused-deputy
TOCTOU classics: work because resolution happens against a *global* namespace carrying the
*victim's* authority. There is no global namespace here and no borrowed authority, so a symlink can
misdirect but cannot grant. Same shape as the `PATH` result above: the escalation vector closes
because there is nothing ambient to point into.

The cost is that one symlink means different things to different holders. That sounds alarming and is
exactly Plan 9's per-process namespace behaviour, so it is a well-explored place to stand rather than
a novel one.

Hard links: decided, declined (§110 (hard links declined)). What remains, for symlinks: what a stored path containing
`..` means when the holder's root is shallower than the creator's. §48 clamps, so it should clamp
here too rather than erroring, but that is a decision, not yet made.

### ~~Open fork~~ SETTLED 2026-07-31: `bind`, not stored paths (DECISIONS §50)

calef chose namespace composition. The analysis below is kept because the naming search is the
evidence for the decision rather than a digression: twenty-eight-plus candidates, terminating without
a winner, which is what a construct that does not fit any familiar relationship looks like. `bind`
needed no search: Plan 9 and `mount --bind` already named it. See §50 for the decision, what it
costs, and the inert-stored-path escape hatch if milestone 55 turns out to need on-disk fidelity.

#### The analysis that settled it: was the mechanism right, and if so what is it called? (raised 2026-07-31)

Not decided. Two questions, in this order, because the second keeps answering the first.

Mechanism first. Plan 9 has no symlinks: it has `bind`. Per-process namespaces made them
unnecessary: you do not need a stored path that resolves oddly per holder when you can compose the
holder's namespace directly. This milestone already took Plan 9's answer for absolute paths and for
`PATH`; taking Unix's here, renamed, would be the inconsistent choice. Settle whether we want
namespace composition instead before settling a noun.

Then the name, because "symbolic link" fails §39 (a component is named for what it is) on both halves. "Symbolic" is defined *against*
"hard", so if hard links are declined the adjective contrasts with something that does not exist.
"Link" is worse: it links nothing. The by-name-ness is the entire content: there is no object
identity, and two holders may resolve the same entry to different files or to nothing.

The criterion, which rules out most candidates at once. A name here must not imply object
identity, must not imply a connection, and must not collide with "reference": in a
capability system a reference is unforgeable and holder-independent, the exact inverse of this. That
disposes of `link`, `reference`, `shortcut` and `pointer`.

Worked, and rejected with reasons rather than by taste:

| Candidate | Why not |
|---|---|
| `alias` | Semantically closer than `link`: a shell alias is stored text, expanded at use, meaning what the current environment makes it mean, with no identity claim. But taken twice: zsh's `alias` (which this milestone tracks, so we would collide with ourselves), and macOS "aliases", which store a file ID and survive the target moving: they track the object, the inverse of ours. Borrowing a Mac term for its opposite is a poor trade on a project whose first real user is a Mac |
| `costume`, `disguise` | Both imply an underlying thing being dressed or concealed, reinstating exactly the object identity the word must avoid. `disguise` also claims intent to mislead, naming into existence a danger this design removes: a stored name here cannot escalate, because it resolves only within what the holder already reaches |
| `projection`, `shadow` | Honest about viewpoint-dependence without implying concealment, and still metaphors. This project names descriptively (`net_stack`, `compositor`, `line_editor`), which is §39's doing; `link` got away with a false claim partly *because* it was a metaphor |
| `mirror` family (`erised`, `matsuyama`) | The best framing anyone found, and the only family to pass all three tests: a mirror shows something viewer-dependent, implies no object identity, implies no connection, and does not collide with "reference". It fails on the word rather than the idea. In computing a mirror is an identical replica at another location: "same content, elsewhere", which is the identity claim we are trying to avoid. The literary instances add their own wrong axis: Erised shows what you desire (ours shows what your namespace resolves to, often nothing), and the Matsuyama tale is about a mistake (the deception axis where `disguise` failed). Both also need a decoder ring, and `design/naming.md` sets the bar at names that parse without prior exposure |
| `fsalias` | Fixes the zsh collision, and prefixes are in-style here (`fs_file_caretaker`, `fs_subtree_caretaker`, `c_confiner`). But "filesystem alias" is exactly what Finder calls a macOS alias (the object-tracking one), so the prefix picks the *wrong* one of the word's two meanings. And prefixing to fix a collision is a smell: it answers *which* alias, where the objection was that alias claims another name for the same thing |

The descriptive candidate, if the mechanism survives: a third entry kind beside file and
directory: a `path`. A directory entry names a file, a directory, or a path; it stores a path and
the holder resolves it, which is the whole description. It also reads correctly when it fails: *"that
entry is a path that does not resolve"* is what happened, where *"that link is broken"* implies
something was once connected. The verb becomes writing a path into a directory rather than "linking",
which retires the `ln -s` shape and its trailing-slash footgun with it.

A further seven produced no new failure modes (`speculum`, `glass`, `scryer`, `mimic`, `imitate`,
`parallel`, `echo`), which is what an exhausted search looks like. They re-derive the four already
listed: `speculum` and `glass` and `scryer` are the mirror family with added baggage (a medical
instrument, a *material* that only means mirror with "looking" in front, and a word naming the
person looking rather than the thing looked into, plus divination); `mimic` and `imitate` reinstate
an original being imitated, which is where `costume` and `disguise` died, and `imitate` is a verb
besides; `echo` collides with a shell builtin we already have, exactly as `alias` collides with
zsh's; and `parallel` means concurrency, in a system with four cores and per-CPU run queues.

Two later candidates are worth their own line. `harmonic` clears all three tests: the stored path
as fundamental, the holder as resonator, and fails on the direction of causation, a failure mode
none of the others had: a harmonic is *determined by* its fundamental, whereas our resolution is
determined by the namespace, not by the stored name. The metaphor points the causal arrow
backwards. (`harmony` is simply the wrong axis: it means concord, where ours may resolve to nothing.)

`reflection` is the best of the mirror family, better than `mirror` itself, because a reflection is
explicitly *not the thing* where a computing mirror implies an identical replica, and its causation is
right, since what you see depends on the mirror *and* where you stand. It fails on a harder collision:
in programming, reflection is runtime introspection of types, which is precise, universal, and in
our own domain.

And that is the pattern behind the whole family. `mirror` → replica, `reflection` → introspection,
`echo` → a shell builtin we ship, `parallel` → concurrency. Physical-optics vocabulary has been
comprehensively borrowed by computing for unrelated meanings, so the one metaphor that actually fits
this construct is the one whose every word is already spent. That is not bad luck; it is why a flat,
non-metaphorical name is the likely answer if the mechanism survives at all.

That the naming is this hard is itself evidence. Seventeen candidates, the first eight failing for a
*different* reason each and the rest finding almost none, and the one that passed every test failed on the word being occupied by its own inverse. The
construct is the only thing in this design whose meaning depends on who is looking, and the vocabulary
has no slot for that. Plan 9 hit the same wall from the same premises and answered with a different
mechanism rather than a better noun, which is why this fork is mechanism first.

### Where `rm` meets them, which is where the sharp edges are

`rm` on a symlink removes the link, never the target. `rm(1)` says so outright: "the rm utility
removes symbolic links, not the files referenced by the links", and it is right for the reason §48
already established: `rm` operates on a name in a directory, and a symlink is a name.

`rm -r` must not descend through a symlink, and our reason differs from Unix's in a way worth
recording. Unix declines because following would escape: a symlink to `/` inside a directory
would turn `rm -r` into `rm -rf /`. Here it could not escape: a symlink resolves in the holder's
namespace, and `rm`'s namespace is the granted subtree with `..` clamped at its root (§48), so a
symlink cannot name anything outside the grant. We keep the behaviour and lose the reason. The
behaviour still earns its place: following would delete a different set of names than the grant
named, and "surprising but bounded" is still surprising.

`rm` on a hard-linked file removes one name and the data survives. That is not a special case, it
is exactly §48's unlink-versus-revoke distinction, and the mechanism is already built: RedoxFS's
deferred delete (`on_open_node` / `on_close_node` and the release list) is what makes the last link,
not the first, the one that frees.

The sharp one: `rm -r subtree/` where a file inside is also linked from outside. The subtree goes
away and the data does not, because the outside name still holds it. That is correct: you removed the
names you were granted, and you were never granted the other one, but it means "I deleted the
subtree" and "that content is gone" stop being the same statement. For a backup target (milestone
55) that distinction is worth stating rather than discovering.

And the cycle, which is a termination argument rather than a taste one. `rm -r` works bottom-up,
so a hard link making a directory its own descendant does not merely confuse it: it does not
terminate. Unix forbids hard-linked directories for this reason among others; here the same
prohibition is load-bearing for a verb we have already shipped the recursion for.

One footgun inherited if symlinks land: `rm -r link` versus `rm -r link/`. The trailing slash
changes whether the target's contents are in scope, which is a real source of accidents in Unix.
Decide it explicitly rather than letting the path parser decide by accident.

## `touch`, both halves now built. See notes/touch.md.

### The create half was built 2026-08-22.

It splits the way `mv` and `rm` did, and the split held: creating an empty file if absent needed
nothing this milestone had not already built (`fs_proto::fs::CREATE`, milestone 31 phase 2). This
section originally expected that half to reach for §49's `DirSpec`, the same program grant `rm`
takes, on the reasoning that it is "a program granted the directory a name lives in". Building it
found a cheaper answer: `touch` needs no more than `CREATE` on the directory the shell already
holds, which is `mkdir`'s right and not a new grant, so it shipped as a builtin in `mkdir`'s
category rather than a program in `rm`'s. `rm` needed a program because `-r` is a destructive
recursive walk that should not run with the shell's whole endowment; `touch` recurses over nothing
and destroys nothing, so the reason that moved `rm` out of the builtin set does not apply here.

### The mtime half was built 2026-08-24, once the authority question was decided.

Updating the modification time of a name that is already there was not built for two days, and
the open decision this section used to name (is "set to now" the write right already held, or a
separate authority) is exactly what stopped it, not effort: the create half took an afternoon once
the decision to split it was made, and the mtime half took about as long once DECISIONS §112 settled
the question. The reason it was expressible at all: the `std` PAL records that "the server keeps an
mtime but the contract does not carry one". RedoxFS tracks it; `fs_proto` did not expose it. It
does now: `fs::GETMTIME`, `fs::SETMTIME`, `fs::SETMTIME_AT` (all three provisional names).

The justification for the old refusal had gone stale. `notes/std.md` refused file times partly
because "there is no wall clock to interpret it against anyway": true when written, and false since
milestone 51 landed the clock (§43, RTC drivers on both ISAs, `date`). Same shape as §43's own
untestability note, which milestone 47's `date` work disproved: a scope note outlives the condition
that justified it.

The authority question was decided (calef, 2026-08-23, DECISIONS §112 (`touch`'s two rights)): no, they are not the
same right. `touch` does two different things to a timestamp: set it to *now*, and `touch -t` set
it to *whatever you say*. The second is the ability to lie about history, which matters for
anything reasoning from mtime, backups included. That is §43's asymmetry again (reading harmless,
setting an authority), one level down, and two independent precedents converge on the same split:
POSIX's own `utime()` requires only write permission to set the current time but ownership to set an
arbitrary one, and §43 itself already separates reading the clock (broadly grantable) from setting it
(a distinct, more tightly held authority). Plain `WRITE` covers "now"; a new, separate right, not
folded into `WRITE`, covers "arbitrary", the same separable-rights-ladder pattern this milestone
already uses for `enumerate`/`open`/`create`/`remove`, now a seven-rung ladder, `dir::SETTIME`
(provisional) the seventh, DECISIONS §47 extended by §112.

Built to that spec, exactly. Three verbs rather than one with a flag, because
`filesystem_protocol::verb::TABLE` encodes one fixed rights requirement per opcode and the two halves
need different ones; `GETMTIME`/`SETMTIME` need `dir::READ`/`dir::WRITE` respectively, resolved
directly under a directory handle like `UNLINK` (neither opens what it acts on), and `SETMTIME_AT`
needs `dir::WRITE | dir::SETTIME` with the caller's asserted seconds riding in the second word,
`TRUNCATE`'s reason for using `w1` over the length field. `touch -t` takes an RFC 3339 instant
(`date`'s own output format), not Unix's compact `[[CC]YY]MMDDhhmm[.ss]`, an earned divergence
because this tree already had an RFC 3339 parser (`calendar`) and no reason to build a second date
grammar for one flag; see notes/touch.md's `BUGS`.

Proven over the real wire, not only in `fs_server`'s host tests. Extending
`filesystem_protocol::verb::TABLE` past `STATFS` also closed a latent gap the extension itself
required fixing to stay contiguous: `SYNC` (milestone 55) had never been given a row, so every
caretaker refused it with `EINVAL` and a program confined to a subtree could never `SYNC` through
it. `kernel::user::shell_navigation_tests` now proves the two-right split against a real,
narrower-than-`dir::ALL` grant (`TOUCH_NOW_NEEDS_ONLY_WRITE` / `TOUCH_AT_REFUSED_WITHOUT_SETTIME`)
and the round trip against a real command line (`TOUCH_MTIME_ADVANCED` /
`TOUCH_AT_ROUND_TRIPPED`), on both ISAs.

## Globbing, which decides how every multi-file operation grants

### Built 2026-07-31: the matcher, then the grant. See notes/glob.md and notes/glob-grant.md.

The decided answer is implemented rather than revisited: `rm *.txt` grants a directory capability
attenuated to a name set, served by `components/src/fs_nameset_caretaker.rs`. Four things this section
did not predict, and one it did:

- It predicted the shape of the change to `grant_plan`, and that is exactly what happened.
  `plan_against` fills its slots by index now, and takes an `Expansion` keyed to that index,
  because the endowment is the set rather than the pattern. `DirGrant.name` became `DirGrant.names`,
  which is the finding in the type system: a literal operand is the set of one.
- The caretaker is a third one, not a generalization of `fs_file_caretaker` or a mode on
  `fs_subtree_caretaker`. `fs_file_caretaker` serves the *file* protocol, so teaching it a set
  would be writing a directory caretaker; and `fs_subtree_caretaker`'s design property is that it
  performs no checks at all, which a name filter (on seven name-taking verbs) would end. The
  grants also have different shapes: a name rides in registers, a set needs a frame.
- An empty match is a refusal, zsh's answer. The obvious argument for bash's pass-through was
  checked and is wrong: nothing here refuses `*` in a component, so passing the pattern through
  builds a grant whose namespace is a name nobody has, and which acquires a referent the moment
  somebody creates that file.
- `ARG_MAX` landed at eight names, set by a stack overflow rather than by reasoning. Sixteen was
  the number the argument produced; the shell ran off the bottom of its stack planning one grant,
  twice. Exceeding the bound is a loud refusal at the prompt, never a truncation.
- Qualifiers and `**` stayed out, for notes/glob.md's reasons, which are authority questions and
  not scheduling ones. `xargs` was not built when this was written, so the answer at the bound was a
  refusal; milestone 109 built it on 2026-08-04, as a shell prefix word rather than a program, and the
  refusal is still what happens for `xargs <program>` because the shell cannot yet ask init to mint a
  per-batch caretaker. That missing delegation chain is this milestone's, and 109's block says so.

Tests: `grant_plan` and `fs_proto` host suites; `kernel::user::glob_grant_tests` on both ISAs (a real
shell expanding one pattern two ways, then `rm` as its own attacker behind a real
`fs_nameset_caretaker`); and `xtask::redoxfs_glob_grant_took_exactly_the_match` reading the image
from outside the guest.

zsh's glob engine is the best thing in the shell (`**/*.rs`, and qualifiers: `*(.)` for regular
files, `*(om[1])` for newest, `*(Lm+1)` for over a megabyte). The mechanism is unremarkable here
because a glob is an enumeration, and the rights ladder above already separates `enumerate` out.
The fork is not how to match. It is what a match grants.

`rm *.txt` with five hundred hits, four candidate answers:

| Answer | Verdict |
|---|---|
| Grant 500 file capabilities | Honest, and it exhausts capability slots |
| Grant the directory plus a name list | Cheap, and it over-grants catastrophically: `rm` could touch anything in that directory, which is the thing this whole model refuses |
| Make `rm` a builtin so the shell deletes and nothing is granted | Dodges the question, and costs `rm` as a program |
| A directory capability attenuated to a name set | The principled one |

The last is a smaller change than it looks, and that is the finding. `fs_file_caretaker` today
serves "a namespace of exactly one name"; globbing generalizes it to a set of names. Same
caretaker, same `fs_proto` protocol above and below, wider namespace. Nothing new in the kernel,
and the attenuation stays checkable from outside the confined program exactly as it is today.

The property worth demonstrating: the expansion you see is the grant. `echo *.txt` prints
literally the authority that `rm *.txt` would transfer, because the matched set *is* the namespace
the caretaker will serve. Unix cannot make that claim, since `rm`'s authority never came from the
command line at all; the glob merely told it which of its existing powers to use.

Who expands. The shell, before planning the grant, which is also what Unix does, so there is no
divergence to earn. The structural consequence is that `grant_plan::plan` must see the expanded set rather
than the pattern, since the endowment is the set.

Two costs to design rather than gloss.

- Qualifiers are not free. `*(.)` and `*(om[1])` need type, mtime and size *per candidate*, so one
  `enumerate` becomes N `FSTAT` calls, and they need a read right beyond enumerate. Decide whether
  qualifiers are in scope at all before building the matcher around them.
- `ARG_MAX` becomes a capability limit rather than a buffer limit. Unix's "argument list too long"
  is why `xargs` exists; here the ceiling is that you cannot hand a child a hundred thousand
  capabilities. The same failure with a more honest cause, and it wants the same answer (batching),
  so `xargs` earns its place for a better reason than Unix had.

Completion shares this mechanism and should be designed with it, not after it: tab completion is
also an enumeration, so the completion menu is a rendering of your authority and cannot offer a path
no capability reaches.

## Absolute paths: Plan 9's answer, not DOS's

### Built 2026-08-18. The recommendation below was taken, and it cost less than it priced.

`/` is the root of your own namespace, in the shell (`grant_plan::nav::Path::from_root`,
`swish::Nav::walk_from`) and in the `std` PAL (`sys/fs/nife.rs`'s `count_names`, where a leading `/`
joins `.` as a component that names the base rather than a place) together, which is what this block
asked for when it said the resolution fork should be one fork answered once.

The resolver is the client's, as recommended, and it turned out to already be there. A grant
records a *position* rather than a token (`grant_plan::designate` resolves once, at plan time) and
`swish::open_at` re-walks that position from the root at run time, so rooting a token at
`Cwd::root()` instead of at the shell's position was the whole of the change on the planning side.
The server still sees a single component against a handle it was given, and §27 (filesystem) is untouched.

Four things this section did not predict.

- The forcing case was `pwd`, not a program. `Cwd::render` has printed `/logs/2026` since the day
  it was written, because a position relative to your own root is the only honest rendering, and
  typing that token back was a refusal. A round trip that does not close is §71's promotion trigger
  met exactly, and it was sitting in the tree for eighteen days.
- It grants nothing, and that is measurable rather than arguable. `/a/b` is `cd` to your root
  followed by two descents. The guest suite asks the same two probes with and without the slash from
  two shells rooted in two subtrees (`navscape::ABSOLUTE_REACHED_INNER` / `ABSOLUTE_REACHED_SECRET`)
  and each reaches exactly the file its own root holds; `/..` is refused exactly as `..` at the root
  is, because your root is the only root there is.
- The `std` half is smaller than Plan 9's and should be described that way. A nife process holds
  one directory capability, so the slash selects nothing: there is nothing else to select. What
  it buys is that `current_dir()` can answer (`/`, and `Unsupported` for a process that holds no
  directory), that `temp_dir()` and `current_dir()` finally name the same place, and that a crate
  which builds a path from `current_dir().join(..)` gets a path that resolves.
- A `Dir` handle is its own root, which is a deliberate divergence from `openat`. POSIX makes an
  absolute path ignore the `dirfd`; here `Dir::open_file("/x")` resolves under that `Dir`. The Unix
  rule exists because `/` names one global thing and a `dirfd` is a shortcut into it, and neither
  half is true here. It cannot widen anything either way, since a process holding a `Dir` holds the
  root it descended from.

The honest cost this section priced is unpaid so far, and it should be watched rather than
declared avoided: "two processes seeing different files at one path is powerful and confusing". With
one capability per process the confusion has nowhere to live yet. It arrives with `bind`.

Distinguish a path as *authority* (`open()` resolving against a namespace nobody granted you: out
permanently) from a path as a *name* (a string, and a name is not a capability). The syntax can
survive even though the semantics cannot.

Plan 9 kept absolute paths and made `/` the root of *your* namespace, assembled from what you were
given, so two processes can both open `/lib/foo` and get different files. That is the counter-example
to gratuitous divergence: the system that took namespaces furthest did not abolish paths, it made them
personal. It also lines up with "every shell has its own root" above, which is not a coincidence.

The real decision is where the resolver lives, and it changes the security story:

- *In the FS server*: it accepts multi-component paths and walks them. Workable, but it puts
  path-walking back into a server, against §27's discipline that open-by-path exists only inside the
  server relative to one bound directory.
- *In the client's runtime* (`user_rt`): a small table of prefix to directory capability, granted at
  spawn, resolved locally and privately. The server still only ever sees a single-component name
  relative to a capability presented to it, leaving §27 (the filesystem service) intact.

Recommendation: the client's runtime. It yields absolute-looking paths with no server learning a
name it did not already own, and the namespace becomes another endowment, inspectable in `caps`,
which Unix cannot do, since you cannot enumerate what your paths could reach. The honest cost is that
two processes seeing different files at one path is powerful and confusing, and Plan 9 users will
attest to both halves.

The `caps` half of that recommendation is not built, and it is worth saying why it would be
empty: a namespace with one root has one row, which `caps` already prints as the directory grant. It
becomes a real surface the moment there is more than one entry, which is `bind`'s question below.

## Environment variables, which are the same question wearing a string costume

Clean slate: there is no `argv` and no `envp` today. `notes/abi.md` is explicit: "no libc, no
`argv`/`envp` array, no dynamic loader, no `main` wrapper", so a program gets argument words in
registers and a populated cspace. Nothing has to be undone, and §15 already carries the natural seam
as a deferred item: a BootInfo page, "a structured block the loader hands the program".

Unix puts three different things in one string-to-string map, which is why environment variables are
both indispensable and a security disaster:

- Inert configuration (`LANG`, `TZ`, `TERM`). Genuinely just data, no authority in it.
- Names for finding things (`PATH`, `HOME`). This is namespace, and therefore *this milestone's*
  question: `HOME` is a directory capability wearing a string costume, and `PATH` is "the set of
  directories I may spawn programs from", which is a set of capabilities.
- Secrets (`AWS_SECRET_KEY` and friends). These are authority badly encoded as a bearer
  string. In a capability system a credential is a capability to a service, not a value you can
  print, log, or leak into a crash dump.

So the three go three different places: data stays data, names become capabilities (the work above),
and secrets become endpoints.

The property worth designing for is not secrecy, it is that environment is an *open channel*. In
Unix anyone can set any variable and hope the program reads it, which makes every process carry an
unbounded implicit input. `LD_PRELOAD`, `IFS`, `PATH` and a long tail of library-specific variables
are attacks that work because a program can be influenced by something it never asked for and does not
know exists.

Invert it: a program declares the configuration it reads, and undeclared variables cannot reach
it. That is not a new mechanism, it is exactly what the SHILL-style manifest already does for
capabilities: a program declares its expected endowment, the manifest is checked at spawn, and a
mismatch is a refusal at the prompt rather than a mystery later. Configuration is the same shape, and
declaring it closes the entire `LD_PRELOAD` class by construction rather than by blocklist.

And no inheritance. Unix's environment is inherited by default, which is exactly why a secret in a
shell leaks into every child including those with no business seeing it. Here it is granted like
everything else: at spawn, explicitly, visible in `caps`. The honest tension is the governing
constraint above: environment variables are convenient *because* they are inherited, and full
explicitness is verbose. Proposed middle ground: inheritance with visibility. The shell holds a
default config set and passes it, but the passing is explicit and inspectable, so `caps run prog`
shows exactly what that program will see before it runs. Convenient in the common case, never
invisible.

One thing to decide deliberately rather than drift into. If configuration is declared in the
manifest, the manifest grows from "what capabilities do I need" into "what do I need at all". That is
a larger claim than it makes today, and it is the sort of scope creep that is easier to accept early
than to reverse later.

### What it costs, measured 2026-08-18 rather than asserted (the absolute-paths lane)

This section argues the shape well and never prices the wire, which is the half that cannot be
undone. Four facts, each a lookup rather than an opinion, and the first changes the category:

- The spawn protocol is a userspace protocol, not the syscall surface. `spawnproto`'s own header
  says so: *"The kernel routes these words the way it routes any IPC; it never reads them. Adding a
  field is a change here, not to the syscall surface."* So an environment endowment is a change two
  programs agree on (the shell and init), which is still the irreversible category and is a rung
  below §10 and §16.
- The protocol already carries data rather than capabilities, and there is a precedent to copy.
  `DIR_BIT` announces "expect two more `SEND`s" and `GRANT_WORDS` carries three opaque words each,
  which is exactly the shape a bounded environment would take. Nothing new has to be invented to
  announce one.
- A page-shaped endowment already exists twice: the clock page a shell is granted read-only at a
  slot init names, and §15 (the native ABI)'s deferred BootInfo page, described there as "a structured block the
  loader hands the program". Init is the ELF loader and already maps pages into a child before
  starting it, so it is the one component that can place a table without a new mechanism.
- The receiving side is built and empty. `std::env` on nife is a process-local table
  (`sys/env/nife.rs`), and `notes/std.md` already named this milestone's namespace as where a real
  endowment would come from to seed it, without changing the table's shape once it did. `temp_dir`
  already reads `TMPDIR` from it, so one variable steers a real behaviour the day anything writes
  one. (Prediction borne out: see the "Built 2026-08-23" subsection below, and
  notes/env-config.md.)

Three encodings, with what each costs. They are not equivalent and the choice is an architect's,
because the shell and init both read whatever is chosen and a stranger's program is written against
it.

| Encoding | What it costs | Where it fails |
|---|---|---|
| More `SEND`s on the spawn endpoint, the `GRANT_WORDS` shape | No page, no capability, no new VA. Three words a message, so ~24 bytes a pair and a fixed maximum count | A `PATH`-shaped value does not fit in 24 bytes, and raising the count means more round trips per spawn |
| A read-only page init maps (§15's BootInfo) | One frame per process, one fixed VA constant, and a parser crate both `user_rt` and the `std` PAL depend on (rule 7: two binaries agree on it, so it is a crate) | A page is 4 KiB and an environment is unbounded in principle; the page is a fixed cost even for the programs that read nothing |
| An endpoint to a configuration service | The most machinery by far: a server, a protocol, a slot | It is the right answer for the secrets third of this section and the wrong one for `TZ` |

Decided (calef, 2026-08-23, DECISIONS §111): the page, for the inert-configuration third only,
with the declaration in the manifest that closes the `LD_PRELOAD` class. The other two thirds stay
answered elsewhere in this milestone: names become capabilities (the namespace above), and secrets
become endpoints (§41's broker shape).

The page's layout, the irreversible part, is also settled: each declared key is checked against a
closed, validated domain at assembly time rather than accepted as an arbitrary string (a real IANA
timezone identifier for `TZ`, a real locale code for `LANG`, a real terminal type for `TERM`), so a
value that doesn't parse as a member of its key's domain is refused when the page is built, not
silently carried. This closes a real gap the recommendation above didn't originally address: a value
misclassified as "inert" (a secret typed into the wrong bucket by mistake) cannot ride through
disguised as configuration, because it cannot parse as one. `caps run prog`'s existing preview is
also extended to print inert-config *values*, not just key names, catching anything a domain hasn't
been written for yet. See §111 for the full reasoning.

### Built 2026-08-23, for a std program end to end. Not yet built: a shell customer. See notes/env-config.md.

The page and everything it needed to reach a real process: `crates/env_proto` (the layout, the
three closed domains, `PageBuilder` and `ConfigPage`, host-tested), a kernel-side assembler and
mapper (`kernel/src/user/std_service.rs`, the clock page's own shape: a `Frame` with `READ` at a
named slot, a fixed VA the loader maps read-only), and the `std` PAL's read side
(`patches/std-nife/overlay/std/src/sys/env/nife.rs::seed`, called once from `pal::nife::init`
before `main` runs). `std_exerciser`'s transcript proves the whole path on both ISAs: the kernel
assembled and validated `TZ=UTC`, `LANG=C`, `TERM=dumb`, mapped the page, and the program read
those exact three values out of `std::env` without doing anything to ask for them.

What this section predicted correctly. All four "facts" the 2026-08-18 pricing found hold
without adjustment: the wire change is between two programs and not the syscall surface, no new
mechanism was needed beyond a page and a slot (`DIR_BIT`/`GRANT_WORDS`'s "announce, then read"
shape did not even need announcing, since the receiving program's manifest already says whether it
wants one, the same way `wants_clock` does today), and the receiving side really was "built and
empty": `sys/env/nife.rs`'s `ENV` table needed one new function, not a redesign.

What it did not predict. The manifest declaration this section's "Invert it" paragraph and
DECISIONS §111 both call for does not exist yet, because nothing in `grant_plan::Prog` has a
reason to read `TZ`/`LANG`/`TERM`. `date` did not read the clock page until `date` existed to be
its customer; the config page is in that exact position. Concretely this means: no
`grant_plan::Manifest::config` field, no wire bit on `spawnproto`, and the `caps run prog` preview
extension §111 asks for has nothing to preview. What *is* built (the kernel unconditionally
granting a std program a config page, the way it unconditionally grants one a clock) is a
demonstration harness standing in for the shell's own "inheritance with visibility" default, not
that mechanism itself: there is no shell-held default config set yet, only `std_service.rs`'s one
fixed default (`UTC`/`C`/`dumb`).

### The shell-facing customer was built 2026-08-26 (`milestone/47-remainder-round2`): `printenv`, `Manifest::config`, and real init wiring

This turned out to be exactly the wiring task the previous round's text predicted, with no fork
underneath, unlike completion and `PATH` below: everything needed was `date`'s own shape, one
field over, and every mechanism it reaches for already existed.

- `grant_plan::Manifest::config: bool` (provisional field name), `clock`'s twin field for field:
  a program declares it, the shell cannot, and init reads the declaration to decide who gets the
  page. No wire bit on `spawnproto`, confirmed rather than merely predicted: `wants_config` is
  read straight off `prog.manifest().config` in `crates/system_initializer`'s spawn loop, the same
  line `wants_clock` already was.
- `Prog::Printenv` (provisional name, Unix's own for exactly this, a term of art already right
  per this tree's naming convention for standard terms), `components/src/printenv.rs`: reads the page at
  a fixed VA (`CHILD_CONFIG_VA`, `crates/system_initializer`), prints `KEY=value` for a declared
  key and `KEY (unset)` for one the page is valid but does not carry, using the same
  probe-before-touch shape `date` already uses for the clock (`granted(slot)` before building the
  pointer, so a process holding no capability at all never faults reading `CONFIG_VA`). `PROG_COUNT`
  moved 11 to 12, exercising `grant_plan`'s own round-trip sweep test as designed.
- Real init wiring, on both boards, not only the kernel test harness. `boot_config_page()`
  (`kernel/src/user.rs`), `boot_clock_page`'s own twin minus the service (nothing runs; the page is
  assembled once with the same `UTC`/`C`/`dumb` defaults and handed to init unconditionally, ahead
  of the filesystem pair, so its slot number is fixed on every boot whether or not a disk is
  attached). `BootEndowment::config_page` is a new field on both real entry points
  (`fixtures/src/hello.rs`, `user/src/system_initializer.rs`), which is why the filesystem pair's slot
  numbers moved by one on each board (documented at each call site; nothing else's numbering
  changed, since config is granted unconditionally like the clock and cannot depend on whether a
  disk was attached). This is the wiring the 2026-08-23 lane named as missing: "there is no
  shell-held default config set yet, only `std_service.rs`'s one fixed default" is no longer true
  for a std program's *test harness alone*: the real, both-board init path now grants the same
  default to any child whose manifest asks, `printenv` being the first and, for now, only asker.
- `caps printenv` prints a `config` row, `clock`'s row with the same one adaptation `date`'s
  page already required: presence only, not the three values. Printing values needs the shell to
  hold its own default config set to preview from, which is the "inheritance with visibility"
  middle ground this section's "Decided" subsection above named and which nothing has built yet;
  see "What remains" below.
- Proven with a real `printenv`, spawned directly with the grant `crates/system_initializer`'s
  wiring would make (`kernel/src/user/printenv_tests.rs`, both ISAs): the page's own three values
  round-trip unchanged from a real assembled page (deliberately not the boot defaults, so the test
  cannot pass by coincidence), a key the page never declared reads as `(unset)` rather than an
  empty string, a page nobody assembled (a zeroed frame) reads as no configuration at all rather
  than three empty strings, and a process granted no capability answers without touching
  `CONFIG_VA`. Also proven over the real interactive prompt: `script/swish-check`'s canned
  script now runs `printenv` and `caps printenv` against a real, both-board boot
  (`xtask::SWISH_CHECK_SCRIPT`), the same gate `date`'s own wiring is proven against, because
  `crates/system_initializer`'s per-command spawn logic (the `wants_clock`/`wants_config`
  match arms, the `caps`/`maps` assembly) is reachable only from a real init and is excluded from
  the host pass for that reason (`crates/system_initializer`'s own module doc says so; nothing in
  `script/test`'s automated kernel suite goes through `crates/system_initializer::boot` at all,
  every lighter guest-test harness reimplements its own bespoke spawn loop instead).

Running that gate is what caught a real bug, and it is exactly the bug `script/swish-check`
exists to catch. The first swish-check attempt failed silently before any prompt: init's own
capability table has sixteen slots, and this file's own comment ("milestone 50 added two more
kernel grants... the shell's `build_child` had no slot left") already named that margin as having
broken once before. A fourth permanently-held kernel grant (the config page, beside the clock page
and the filesystem pair) reproduced it. Fixed by moving `for_test_roles`'s deletion from after
console, line discipline and input are all built to before any of them are, since nothing in
between ever reads those capabilities' contents and the slots they freed were sitting idle through
the three builds that needed the room most (`crates/system_initializer/src/lib.rs`, the comment at
the new deletion site carries the account). Verified by re-running `script/swish-check` on both
boards after the fix. This is the argument for the gate, made concrete: nothing in the
automated `script/test` suite would ever have caught this, because nothing in it boots a real init.

What remains, honestly: printing the three inert-config *values* in a `caps` preview, which
DECISIONS §111 also asked for and which needs the shell to hold a default config set of its own
(unbuilt); and the "inheritance with visibility" mechanism itself, which is the same unbuilt thing
one level up. Neither has a forcing customer yet, `printenv` being read-only and diagnostic rather
than a program whose *behaviour* would visibly change with a different default.

### Environment's secrets third: still not built, and the reason is sharper than "no PAL yet"

Read closely this round rather than re-asserted. The previous lane's line ("no PAL and no shell
support, unchanged") is accurate but understates why: inert configuration had a straightforward
build path the moment it had a customer, because the *mechanism* (a validated, read-only page) was
already fully designed by DECISIONS §111 and this section's own 2026-08-18 pricing table; nothing
about secrets is in that position. "Secrets become endpoints (§41)" names the *shape* of the
answer, not a mechanism ready to extend: `credential_protocol`'s `PROVISION`/`VERIFY` split (milestone
56) is the one concrete instance of that shape in this tree, and it is purpose-built for *login*
(an identity and a password-shaped secret, checked but never read back). A generic secret,
`AWS_SECRET_KEY`'s own case, is a different shape again: the *bytes* themselves are often needed
client-side (to sign a request), not merely a yes/no over them, so the credential service's whole
security property (a client can use a secret and never read it) does not transfer. Generalizing
`credential_protocol` into "any named secret, capability-gated" is itself an undecided design question
with real alternatives (a single broker service keyed by name and an ACL, versus per-secret minted
endpoints an admin tool hands out, versus folding it into milestone 49's login/session work since
that is what would actually *grant* a secret to a session in the first place) and, per this
milestone's own governing rule, there is still no forcing customer: nothing in this tree today
needs a bearer secret the way a real network client eventually will. Building a generic mechanism
now would be inventing infrastructure against a hypothetical, which this milestone has correctly
declined to do for `PATH` and `bind` alike. Left exactly where it was, with the reason on record
rather than only the fact.

## `bind` is not blocked on a mount table. It is blocked on a second grant (found 2026-08-18)

### Built 2026-08-26 (`milestone/47-namespace-remainder`), against the real mechanism. See the Status block for what it does and does not reach.

The section below is kept as written: the sketch it ends on ("`bind` then falls out as a name on a
`Cwd` per entry, and `caps` gains a namespace section with more than one row in it") is exactly what
got built once milestone 154 supplied the second grant, checked rather than assumed. What changed
in the checking: the sketch's "per entry" turned out to mean a *table* of entries rather than one
(`nav::Bindings`, up to four, provisional), because a shell with one grant still wants to file more
than one shortcut, and the table composes with `nav::TwoRoots`'s two fixed labels rather than
replacing them (a grant label always wins, so a bind can never shadow `a` or `b`). `caps` now prints
a bound name's own row (`bind <name> -> <real position>`), beside the two-grant namespace section
154 already built rather than folded into it, since a bind is additive to whatever grant rows
already print.

DECISIONS §50 chose namespace composition over stored paths and priced the unbuilt half as "a mount
table per process and resolution through it. That is real work". Building absolute paths priced it
again, from inside, and the mount table is the cheap half.

- A bind entry is a value, not a capability. Everything downstream of the shell's planner already
  re-walks a *position* from the root (`swish::open_at`), so a bind is a `nav::Cwd` under a name: no
  cspace slot, no handle to leak, no lifetime. And it cannot name above the root by construction
  rather than by a check, because `Cwd` has exactly two constructors, `root()` and a `descend` that
  refuses a bad component, and `ascend` returns false at depth zero. That is the ladder's first rung,
  and it is why this half warrants no proof harness: a Kani harness would restate the type.
- What is missing is something to bind. A shell holds one directory capability, so a
  namespace assembled from what it holds has exactly one member and every bind is an alias inside one
  tree. The interesting case, and the only one that pays for the mechanism, is a union of two
  grants: `/photos` from one caretaker and `/backups` from another, in one process, with neither
  able to name the other's parent.
- Nothing in this system grants a second directory capability to one process.
  `fs_service::start_granted_dir` starts one caretaker and hands one endpoint; a second means a
  second caretaker, a second slot, and a spawn-protocol position to say which is which. That is an
  endowment question, which is the category this milestone's own environment section says is
  expensive, and it is where the work actually is.

Minted as milestone 154 (a process that holds two directory capabilities) on 2026-08-23. The
deliverable is the wiring plus the negative control that only a union can state: one process, two
subtrees, `/a/x` and `/b/y` both resolving, `/a/../b` refused, and neither caretaker able to see the
other's tree. `bind` then falls out as a name on a `Cwd` per entry, and `caps` gains a namespace
section with more than one row in it. Until that exists, building the mount table would be machinery
whose one interesting case is missing.

## Milestone 64 is what forces the namespace half of this milestone

Recorded here as well as in 64, so neither is picked up without it.

What remains open in this milestone is the namespace machinery: ~~absolute paths~~ (built
2026-08-18), environment variables, `PATH`, and `bind`, whose real blocker is the section above
rather than the mount table §50 named.

None of it has a forcing use case from the shell. `swish` works with per-shell roots; `bind` is
a mechanism nobody currently has to have. That is why this milestone has sat IN-PROGRESS with its
navigation half done and its namespace half designed.

Milestone 64 supplies the missing demand. `std::fs::File::open` takes a path, and a `std`
program is not a shell: it cannot be handed a root and told to `cd`. A crate that writes
`Path::new("assets").join("x.png")` is a concrete request for per-process namespace resolution, which
is exactly what `bind` is for. The `PATH` conclusion below, that a program namespace is an
endowment, gets its first real customer at the same moment.

The sequencing this implies, and it runs the other way from the obvious: let 64 measure first.
Its probe crates will report what a real dependency actually needs, and that evidence is what this
milestone's remaining scope should be sized against, rather than building the general namespace and
hoping it fits. `File::open`'s resolution is then one fork answered once, spanning both
milestones, instead of a PAL trick here and a design there.

## `PATH`: there is no search, because there is no ambient namespace to search

The absolute-paths section above takes Plan 9's answer for paths in general; `PATH` is that same
question narrowed to programs, and Plan 9 answers it the same way. Plan 9 has no `PATH` variable
at all. `/bin` is bound per-process, union-mounted from whatever that process's namespace assembled,
so what you can run is what is bound. Taking the same answer here is consistency, not a new idea.

`PATH` is two bad things at once. It is a *search*, over a namespace you have *ambient access to*.
The search makes the order of a string into a security boundary: a writable directory ahead of a
system one, or `.` anywhere in it, and someone plants an `ls` that you then run. The ambient access is
why the order matters at all, since `PATH` never controlled *access* (permissions did), only which of
your already-reachable options wins. The tell is that `which` exists as a whole command whose job is
answering "which one did I actually get?".

So the program namespace is the endowment, and a name binds to exactly one thing in it. The
property that follows is the same class as `rm -rf /` above: `PATH` injection is structurally
impossible rather than mitigated. No search order to manipulate, no `.` to include by accident, no
writable directory that can precede a system one, because there is no search.

The distinction that makes it work. A shell may extend its program namespace only with
capabilities it already holds, so extending is a naming convenience and never an authority increase.
Unix nominally has this property too and loses it in practice: ambient authority means everyone can
read `/usr/bin`, so `PATH` order becomes the de facto security boundary. Here it cannot be, because
naming and access are separate things.

Four open questions, none decided:

- Unions and shadowing. A namespace unioned from several sources brings first-match-wins back,
  which is the ambiguity just removed. Plan 9 chose ordered union with explicit before/after on
  `bind`; the alternative is to refuse ambiguity (an error when two sources offer `ls`), which is
  more honest and probably more irritating.
- Enumeration. "What can I run?" is enumeration of the namespace, the same insight as globbing and
  completion above and bounded the same way: completion cannot offer a program no capability reaches.
- Compile-time set to runtime lookup. `Prog` is a closed enum with `from_name` today, and init
  already loads from the initrd by name, so half the mechanism exists. What is missing is enumeration
  and not being a fixed set.
- Does `$PATH` survive as a string? If the namespace is a capability, `echo $PATH` has no
  referent, and that is a divergence on one of the most-referenced variables in shell scripting. It
  looks earned (the variable's two real uses, inspect and modify, become `caps` and a grant) but the
  cost is real and should be named rather than glossed.

Two milestones this reaches into. Milestone 49 (users, login, and attribution) is what *hands* a
session its program namespace, so "who gets which capabilities at startup" includes which programs.
And milestone 39 (repository structure and the road to a distribution) inherits the sharper
consequence: installing a program becomes granting it into a namespace, which is a materially
different packaging story and is worth being on the record before anyone designs a package manager
around the assumption that installation means writing into a globally readable directory.

### PATH, sized rather than merely found (investigated further 2026-08-26, `milestone/47-remainder-round2`). **PROPOSED, not decided.**

The 2026-08-26 lane found the manifest-provenance blocker and correctly declined to invent an
answer. This lane went one step further, on the instruction to price rather than merely name a
fork: is carrying a manifest for a runtime-discovered program a contained extension, or does it
force the real packaging system? Both, depending on which half. Storage and trust turn out to
be cheap, almost free; the spawn protocol does not, and neither does the scope question, so this is
still a fork, just a better-priced one.

What this lane found that the previous one had not looked at: `nifefs` needs no format change at
all, and the trust chain already covers an arbitrary name.

- A manifest can be an ordinary sibling archive entry (`printenv.manifest` next to `printenv`,
  say), not a new field on `nifefs`'s directory entry. `nifefs` is a flat name-to-bytes store
  (`crates/nifefs`'s own module doc); a manifest is just bytes under a name, exactly like an ELF or
  `program_measurements` itself already is. No format bump, no `MAGIC` version, no `DIR_BLOCKS`
  change.
- `measured_boot::PROGRAM_MEASUREMENTS` is already generic over arbitrary names. It is a
  build-time text table, one `name sha256` line per packed archive entry
  (`crates/measured_boot::manifest_entries`), and `crate::trust::require(name, bytes)` checks any
  name against it. A manifest file packed at build time would get a line in this table for free,
  with zero new kernel mechanism: the same `require` call that already vouches for a program's
  bytes before `Elf::parse` touches them would vouch for its manifest's bytes before anything parses
  those either. Rule 7's "a shared definition is a crate" is the only new thing this half needs: a
  small, dependency-free crate (provisional name `manifest_proto`, `environment_protocol`'s own shape)
  defining `Manifest`'s on-disk encoding, so the build tool that writes the bytes and the shell that
  reads them share one definition and cannot drift.

What does not get cheaper, and is the reason this is still a fork. `grant_plan::Prog::id()` is
a small integer, 0..`PROG_COUNT`, and init decodes it by indexing a `[Option<Elf>; PROG_COUNT]`
array it built once at its own startup by walking every *compiled-in* `Prog` variant's name through
`program()`. A name absent from that fixed table has no id to send and no slot for init to have
prepared, whatever a shell learns from a manifest file. Reaching a runtime-discovered program
therefore needs `spawnproto`'s request word (today: program id, integer argument, memory-grant
page count, `spawnproto`'s own header) to carry a name instead of, or alongside, a small int, so
init can look the ELF up by string the way it already looks up every compiled-in program
(`nifefs::Fs::read`, already string-keyed; the previous lane's own finding). Mechanically this is
the same shape `DIR_BIT`'s "expect two more `SEND`s" already establishes for data that does not fit
one word, so a `NAME_BIT` (provisional) carrying a length-prefixed name over N more `SEND`s is not
a new *kind* of mechanism. But it is a new wire message two programs (the shell and init) must
agree on forever, `spawnproto`'s own header says so explicitly ("adding a field is a change here,
not to the syscall surface... [still] the irreversible category"), and this milestone's environment
section already priced the identical category of change for the config page. That places it
squarely in the "move fast on what can be undone" tenet's expensive column: reversible in principle,
but every future program and every future shell reads are written against whatever shape is chosen.

The scope question, which is sharper than the wire-format one. Even with both of the above
built, a program still reaches a running system only by being packed into the initrd at build
time: `nifefs` has no live write path, and the archive is mapped read-only into init. So this
mechanism would decouple "add a program" from "edit `grant_plan::Prog`'s Rust source and recompile
every consumer of it" (a real, meaningful simplification: authoring two files and re-packing the
archive, rather than a source change across `grant_plan`, `swish` and every match on `Prog`), but it
would not decouple "add a program" from "rebuild and re-flash the boot image", which is what
`PATH` usually evokes (drop a binary onto a live machine and run it). Building the name-carrying
wire change now, without deciding whether that partial win is worth having on its own, risks being
exactly what this section's own closing paragraph already warned against: designing a package
manager's shape by accident, one wire message at a time, before milestone 39 decides what
"installing a program" means here.

Three options, costed rather than merely listed (the six-questions discipline: what else was
considered, what this tree already does, what each costs, how reversible):

| Option | What it buys | What it costs | Reversibility |
|---|---|---|---|
| **Do nothing further** (status quo) | Nothing changes; `Prog` stays closed | Zero | N/A |
| **Build the manifest-as-data half only** (a `manifest_proto` crate, a build-time convention for pairing a program with its manifest file, `require`-checked the same as any program) with **no spawn-protocol change**, so nothing can actually be spawned from it yet | A `manifest_proto` crate other tooling (`caps`, a future installer) can already read and validate against, landed cheaply and reversibly, and it de-risks the harder half by proving the encoding before the wire change is designed | Builds a crate nothing consumes yet, which this tree's own convention (a lane ships nothing speculative) argues against unless a near-term consumer exists | Fully reversible: an unused crate costs nothing to delete |
| **Build both halves** (the wire change too) | Real decoupling of "add a program" from "recompile grant_plan" | A new wire message two programs must agree on forever, the expensive, hard-to-undo category this project reserves for calef, and it commits to a scope answer (build-time-only PATH) before milestone 39 has decided whether that is the right shape at all | The wire format, once shipped and depended on, is exactly the "who else has already acted on this" case the tenet asks about: every future program written against it inherits the shape |

Recommendation: neither yet. Per this project's own limit on when a fork earns a lane
("recommend on reversible forks; give options only on irreversible ones"), this is the irreversible
kind, so options rather than a push toward one. If forced to rank: the middle option is the
defensible next step if and only if a near-term consumer is already planned (otherwise it is
exactly the speculative crate this tree's own convention refuses); the full wire change should wait
for milestone 39 to at least sketch what "installing a program" means, since building the spawn
protocol first risks answering that question by accident, one opcode at a time, which is the
failure this section's closing paragraph already named before this lane existed.

What this does not decide. Whether `PATH` is worth building at all before it has a forcing
customer (unchanged: none exists today), and whether the manifest encoding, if built, belongs in
`grant_plan` itself or a new crate (a naming question, calef's per this tree's own rule, not sized
further here).

### The manifest question was answered elsewhere (checked 2026-09-26). **PROPOSED: how a bare name reaches an installed program.**

The table above is overtaken. §208 (installing is granting) made the activation set the record of
what is installed, digest and manifest together. §219 (how the shell names an installed program to
the spawner) ruled option D: the shell reads a program's bytes and sends them as frames, and the
progenitor finds the manifest by digest. §197 (a package is one archive file) puts the manifest in an ELF note (ruled 2026-09-26; the build is in flight as PR #1338). No
name crosses the wire, so the `NAME_BIT` priced above is not needed, and the "build-time only"
scope objection is gone: `package install` changes what runs without a reflash.

What is left is this block's own first open question, unions and shadowing. Today an installed
program runs only by path (`packages/uptime/0.1.0/uptime`, `components/src/swish.rs`, `run`: a
token with a `/` is a file's bytes), and its bare name is refused as "no such program"
(notes/packages/fetching.md). Three options:

| | A bare name that is not in the image | Cost | Reversibility |
|---|---|---|---|
| **Paths only** (status quo) | Refused | Zero. `bind` already shortens a path | Nothing to undo |
| **The live activation set**, ambiguity refused | Resolves to the live generation's entry of that name. An installed name equal to an image program's is refused at `package install` | One lookup in a table the shell already reads for `caps <path>`. No search order exists, because each name has one entry | Cheap now; once scripts name installed programs bare, removing it breaks them |
| **A bound directory** (Plan 9's `/bin`) | Tried as `<bound name>/<name>` after the image | A convention for which bind is searched, and with two sources, an order | An order is the thing this section says `PATH` gets wrong |

Leaning toward the activation set, because it is "the program namespace is the endowment" with no
search at all: installing is what puts a name in it (§208). It is a name the shell and every script
will depend on, so it is an architect's.

## Completion: built 2026-09-26 as §227 option D

Two lanes on 2026-08-26 found that Tab is dropped by the terminal's line discipline, and priced a
round trip on the terminal wire (a reply flag and a resume opcode) to reach the shell mid-line.
That pricing is now DECISIONS §227 (how Tab reaches the shell), whose option D needed no wire at
all: milestone 169 (`kilo`, the smallest real text editor)'s raw mode already existed. calef ruled D on 2026-09-26.

So the shell turns raw mode on at the prompt and runs `line_editor::LineDisc` itself. Tab is
`Event::Tab`, and `swish::complete` finishes the word from what the shell can name: a builtin or an
image program in command position, and otherwise an entry of the directory the word leads into,
listed with the same `ENUMERATE` `echo *` needs. `^C` at the prompt is a byte the shell's editor
turns into a discarded line. A supervised job still takes `^C` through the terminal's count
(§24 (interrupting the foreground process)), so the shell leaves raw mode just before it spawns one.
Proven by host tests in `crates/line_editor` and `crates/swish`, and at a real prompt on all three
architectures by `script/swish-check`, which types a Tab twice and a `^C` once. The binary cost and
the per-keystroke cost are measured in notes/shell-line-editing.md, with its BUGS. fish's extras
are proposals (Follow-on, below).

## `file:` and `run` are not earned, and come out (decided 2026-07-30)

### Built 2026-07-31: the grammar change, ahead of the commands.

`run` and `file:` are gone from `grant_plan` and the shell. A bare program name spawns it (`worker 9`,
`budgeter --mem 16`, `date`); a bare token in a file position designates the file, and the manifest
still declares the direction. `--mem N` stays, and is now accepted on either side of the program
name because with the verb gone a leading flag reads wrong.

The change the analysis above did not anticipate: the parser stopped classifying tokens at all.
`RunSpec` keeps the positionals in the order typed and `plan_against` places them into the slots the
manifest declares, which is what makes "the manifest says what it is" true in the code rather than
only in the prose. A shape-based rule (a number is the argument, anything else is the file) would
have read `wc 2026` as a missing file.

`caps <command>` is the preview's new spelling: the tail is the command you would have typed, so
what you inspect and what you run cannot drift apart, and it is the Unix prefix-word idiom (`time`,
`nice`, `env`) rather than new grammar. The refusals moved from "drop the `file:` designator" to
positional wording, and one refusal order changed on purpose: a program's own declaration is
checked before what the shell holds, so `worker report.txt` answers "takes no file; drop the name"
(true whatever this shell holds) rather than "you hold no such capability" (an accident of this
boot). The consequence, recorded rather than glossed: no shipped program declares
`FileSpec::Required`, so the headline "no such capability" refusal is no longer reachable from the
prompt, only through `plan_against` in the host tests.

`date` came along with it, because with `run` gone `date` is exactly what a person types, and
the shell had never heard of it (`Prog` knew four programs). It has a `Prog` entry and an all-
`Forbidden` manifest; the shell spawns it with the register defaults, since `ArgSpec` has no
position or arity yet. It is the first program whose whole authority the command line cannot
name: a read-only mapping of the clock page, which init endows. This boot starts no clock service,
so it prints "the time is unknown: this process holds no clock capability", and `caps date` says so
before you run it. What a shell that could delegate a clock would need is assessed in
notes/grant-expression.md and is its own lane: kernel boot wiring on both ISAs, a spawn-protocol
position, both inits, and nothing in the suite boots the interactive shell to prove any of it.

Tests: `crates/grant_plan` host suite, 34 cases. Notes: grant-expression.md, program-manifest.md, date.md.

calef asked to be convinced they were worth the typing. They are not, and the case against each is
stronger than the case that put them there.

`run` fails on consistency, the DOS objection turned inward. This milestone adds `ls`, `cd`,
`pwd`, `mkdir`, `rm`, and nobody would type `run ls`. So builtins become bare words while programs
need a verb, and a user has to know *which class a command is in* to know how to type it. That is
precisely the arbitrary divergence this milestone exists to refuse. Milestone 50 (pipes and
redirection) finishes it: `run a | run b` is indefensible. The lookup that replaces it already
exists (`Prog::from_name`, and `dispatch`'s `Unknown` arm), so `run` is phase-1 scaffolding from
when there were two programs, not a design position.

`file:` fails because it announces the wrong half of the grant. `wc file:report.txt` reads and
`tee file:report.txt` writes: identical syntax, opposite authority. Direction lives in the manifest
by design (milestone 31, a capability shell, took the SHILL shape deliberately), so the prefix marks
the part already visible and stays silent on the part that matters. The safety argument fails too,
on inspection: `worker 5 extra` is refused as unplaceable because worker's manifest says
`FileSpec::Forbidden`, not because of any prefix. The manifest was doing all the work and the
prefix was taking credit.

The reason it cannot carry the thesis is deeper than either. The capability claim is about absence,
not presence. That a filename grants access to that file surprises nobody; what `wc report.txt`
proves is that wc got that file *and nothing else*, and that claim lives in the tokens which are not
on the line. A prefix decorating a token that *is* present cannot express it. `caps run <cmd>` can,
including direction, which makes it the visibility mechanism and an argument for making it good.

What survives: the manifest declaring direction (load-bearing, untouched); `caps` as the sole
visibility surface; `--mem 16`, a real grant with no Unix analogue spelled as an ordinary flag.

Do it in this milestone, because the window closes. No program today takes both an argument and a
file (worker takes an int, budgeter takes memory, heeder and spinner take neither), so positional
resolution is at most one bare token and the manifest says what it is. Once a program wants both
(`grep pattern file.txt`), `ArgSpec` has to grow position and arity. The cost is that this changes
grammar in milestone 31 (a capability shell), which is built and host-tested, so the refusal wording changes from
"drop the `file:` designator" to something positional. Those tests are the work; it is a contained
edit, not a redesign.

## Open fork: should the shell be function calls rather than whitespace? (raised 2026-07-30)

Settled 2026-09-03 and refused, in DECISIONS §141 (application is grant), which kept this
section's one lasting idea as the system's mental model and measured each notation against the
tree. Kept below as history.

calef proposed `wc(cat(this-file.txt))` or `cat(this-file.txt).wc()`, on the grounds that shells lean
too hard on whitespace to tell a name from its arguments. Not decided. Recorded because the idea
contains one thing worth keeping whatever the syntax ends up being.

The diagnosis needs adjusting first. Whitespace is not ambiguous about which token is the
command; position handles that and always has. The real pathology is that a value containing a space
is silently re-split into two arguments after substitution, and then `IFS`, `"$@"` versus `$@`,
and glob expansion firing at the wrong moment. Call syntax does cure it, but so does never
re-splitting a value, which costs no syntax and which we can adopt freely having no legacy.

The two proposed forms are not equivalent. `wc(cat(f))` is command substitution, not a pipeline:
the inner call must complete and return a value, which buffers the whole output. `cat(f).wc()` reads
in the direction data flows and is genuinely pipe-shaped, which is why `|>` exists in Elixir, F# and
OCaml. But a method implies an object with a type, and milestone 50 currently carries bytes; over
bytes, `.wc()` is `| wc` with more punctuation, promising something the substrate lacks. Typed
pipelines are a separate and larger fork and should be decided in milestone 50 on their own merits,
not smuggled in through notation.

The part that is genuinely ours: application is grant. `f(x)` means "spawn f, grant it x", so in
`wc(cat(f))` the nesting *is* the authority tree, and the delegation structure can be read straight
off the syntax. No other shell can say that, because in Unix both `f(x)` and `f x` mean "f can
already reach everything, here is a string". Worth writing down as the mental model regardless of
which surface wins, and it is a better answer than `file:` ever was.

Three objections. It costs more keystrokes than the `file:` this same milestone just deleted for
costing five. Bare `ls` becomes `ls()`, miserable interactively, so both spellings get allowed and
commands acquire two classes, which is the *same* objection that killed `run`. And shells are
optimised for typing where languages are optimised for reading; Oil/YSH, Elvish and Nushell all ran
at this, and Plan 9's `rc` is the one that worked, precisely by fixing quoting and word splitting
while keeping the terse surface.

The recommendation, if this is settled without further design: kill word splitting outright,
keep whitespace application with parentheses for grouping only (`wc (cat report.txt)`, the ML and
`rc` answer), and record "application is grant". That takes what the idea is pointing at and drops
the notation.

## The finding that should drive the build order

`cd`, `mkdir`, and per-process namespaces each converge on the same missing primitive: a verb that
returns a directory capability rather than bytes. It would be the first place this contract hands
back authority instead of data, and it deserves the care `Endpoint::REAP` got (§32): what rights does
the child directory carry, can they ever exceed the parent's, and who may call it. Build that first;
the commands are the easy part once it exists.

Sequencing. After milestone 37 (RedoxFS's crash consistency), which owns the FS server's block path. Effort: 2 lanes
estimated, and the second estimate proved low: the namespace lane of 2026-08-18 spent itself on
absolute paths alone and left environment, `PATH` and `bind` untouched, the last of those blocked on
an endowment question rather than on effort (one for the descend/create verb and the builtins, one for namespaces), noting that
estimates for unbuilt work are guesses on a scale calibrated from history, not measurements.
## Follow-on

- **Done.** The shell's heap: fallible, capped at 32 KiB, and no substituted value re-split
  (notes/heap.md).
- **Done.** The sequencing question this block leaves open, that milestone 122 (`OPENDIR` reaches
  the PAL) is `NOT-STARTED` and some of the namespace half may want it, is stale: 122 is BUILT
  2026-08-18 (pull request #320), both options, proven on both ISAs by `std_exerciser`. Nothing
  here is waiting on it.
- **Done.** "The `caps` half of that recommendation is not built" was true when written and is not
  now. `crates/swish/src/lib.rs` prints a bound name's own row and writes it, with a test asserting
  `bind recent -> /logs/2026`. The block's own `bind` paragraph already says so; this sentence
  never got the correction.
- **Done.** Tab completion, built 2026-09-26 under §227 option D: see "Completion: built
  2026-09-26" above and notes/shell-line-editing.md.
- **Proposed.** fish's extras, recorded rather than built: suggestions from history
  (`design/roadmap/proposals/the-prompt-suggests-from-history.md`), colouring the first word by
  whether it can run (`design/roadmap/proposals/the-prompt-colours-what-it-can-name.md`), a live
  `^R` search (`design/roadmap/proposals/a-live-history-search-at-the-prompt.md`), and argument
  completion from the manifest (`design/roadmap/proposals/argument-completion-reads-the-manifest.md`).
- **Outstanding.** `PATH`, now buildable: calef ruled
  `design/decisions/229-how-a-bare-name-reaches-an-installed-program.md` B2 on 2026-09-26. An
  installed program still runs only by path, checked 2026-09-26. The lane must meet the two
  checks on owner-vouch entries that §229 names.
- **Milestone 65.** Environment's secrets third, moved rather than built. A secret here is an
  endpoint (§41), the service that holds one is milestone 65's (BUILT), and where a stored secret comes from is §165 (PROPOSED). §220
  (signed builds) met the one candidate customer, a signing key, on 2026-09-26 and recommended
  building no signer on nife until a build made on nife must run elsewhere. There is nothing for a
  navigation milestone to build here. Checked 2026-09-26.
- **Done.** Built 2026-09-26: printing inert-config values in `caps`. The progenitor places the
  frame it endows children with in the boot shell, `READ` only, at `grant_plan::SHELL_CONFIG_SLOT` and maps
  it at `SHELL_CONFIG_VA` (both provisional), and `caps printenv` prints the three values from it.
  Host-tested in `crates/swish`, proven at a real prompt by `script/swish-check`. A shell choosing a
  *different* set per child is not built and has no customer; it and a `login` session's blind
  preview are in notes/env-config.md's BUGS.
- **Milestone 154.** `bind` in a two-grant shell stays host-tested only. Both real entry points
  still pass no second directory (`components/src/swish.rs`, `user/src/system_initializer.rs`), and
  `crates/system_initializer` calls the path unverified against a real boot.
- **Outstanding.** An architect's call. The delegation chain `xargs` needs, and it is wider than
  `xargs`: `spawnproto` cannot carry a set of names, so the progenitor builds only the one-name
  subtree caretaker and every pattern matching two or more names is refused at a real prompt,
  `rm *.txt` included. The options, the costs and a prerequisite that was never given a milestone
  (the shared-page audit's frame per client channel) are in notes/a-set-grant-at-the-prompt.md,
  PROPOSED. Checked 2026-09-26.
- **Refused.** The two symlink questions this block leaves open, what a stored `..` means to a
  shallower holder and the `rm -r link/` trailing slash, are moot:
  `design/decisions/50-namespace-composition.md` chose composition over stored paths, so there is
  no stored path to clamp or to slash.
- **Decision.** Hard links were considered and declined, written up as
  `design/decisions/110-hard-links-declined.md`: no customer, `RENAME` already covers the
  atomic-replace idiom, and a DAG would cost an audit of every subtree argument in the tree.
- **Refused.** The function-call shell syntax fork calef raised 2026-07-30: DECISIONS §141
  (application is grant), 2026-09-03, recorded the model and refused each notation with a measured
  reason. The line above said it lived nowhere else; it has since 2026-09-03.

## Index row

divergence from Unix must be earned, never stylistic. Keep the commands; change only what the
capability model actually forces, and get one missing primitive right
