---
status: BUILT
raised: 2026-09-05
built: 2026-09-14
---
# 265. `_proto` is a truncation, and it collides with the other word it could be short for

Built 2026-09-14. Minted 2026-09-05 by calef, on being shown `timebase_proto` for
ratification: *"I think `_proto` was lazy on my part. It should have been `_protocol` globally to
differentiate from prototype."* *(Number provisional until the merge queue lands it.)*

## The measurement, and this block's own numbers were wrong

Re-measured on 2026-09-14 against `origin/main` at `2487a1f7`: 15 crates carrying `_proto`, one
more crate and one program carrying a stem that had to travel with them, referenced across 424
files. The block said 14 crates and 349 files, and both figures are corrected here rather than
quietly: the crate count was written before `capability_demo_proto` arrived with milestone 291 on
2026-09-14, and the file count was never re-derived. The precise measurement is
`git grep -lE '\b(<the seventeen names>)\b'`, which is also the enumeration the rename was performed
from.

```
byte_sink_proto   capability_demo_proto  clock_proto      credential_proto  entropy_proto
environment_proto filesystem_proto       graphics_proto   login_proto       mdns_proto
ntp_proto         socket_proto           supervision_proto swap_proto       timebase_proto
```

Plus `mdns_config` (a crate) and `mdns_responder` (a program), which carry no `_proto` and travel
with the stem ruling below.

The 424 splits 252 code and configuration to 172 markdown, and only 108 of the markdown moved:
the status rule below is what decided the other 64.

## Five stems calef ruled on 2026-09-13, which this milestone carried

Working the unratified worklist, calef ruled the `mdns` family, `ntp_proto`'s stem and
`timebase_proto`'s. **The rulings were recorded and the rename was not performed**, deliberately:
doing it then would have meant renaming the same files twice, once for the stem and again for the
suffix. So this milestone grew by five names and the tree grew by none.

| Was | Is | Ruled |
|---|---|---|
| `mdns_proto` | `multicast_dns_protocol` | stem 2026-09-13, suffix by this block |
| `ntp_proto` | `network_time_protocol` | stem 2026-09-13, suffix by this block |
| `timebase_proto` | `counter_frequency_protocol` | stem 2026-09-13, suffix by this block |
| `mdns_config` | `multicast_dns_config` | 2026-09-13 |
| `mdns_responder` | `multicast_dns_responder` | 2026-09-13 |

The last two rows carry no `_proto` suffix and are here because they carry the same stem:
renaming the protocol crate and leaving its config and its responder spelled the short way would
split one protocol across two spellings, which is the state this milestone exists to end.

The third row was missing from this block and is the reason to enumerate rather than trust a
list. It said four stems; there were five. `timebase_proto`'s own provenance records calef ruling
`counter_frequency_proto` on 2026-09-13, the same day and the same worklist as the other four,
and it records `timebase_proto` as refused in the same breath. A lane working from this block's
table alone would have applied the suffix rule to a stem the tree had already said no to, and
produced `timebase_protocol`.

That is a narrower claim than it first looked and is worth stating precisely, because the first
draft of this paragraph overstated it. `timebase_protocol` is not itself on record as refused;
`timebase_proto` is. So `script/names --check`, which catches a name recorded as refused that is also
live, **would not have fired**: it matches the refused string, and the string a suffix sweep produces
is one letter different. The refusal is of the stem and the gate can only see the whole name. Nothing
mechanical stood between this milestone and a crate carrying a refused stem; reading the block did.

The sweep then did the other half of the same failure, and it is the blind-`sed` scar in its purest
form: it rewrote *"Refused `timebase_proto`, above"* into a name refusing itself, twice, and only
enumerating the diff found it.

The crate this milestone was minted over is therefore `counter_frequency_protocol`, not
`timebase_protocol`. The stem is an architect's and the suffix is this block's, applied in one pass
for the same reason the `mdns` and `ntp` rows exist.

`network_time_protocol` is also the answer to a stutter. Expanding the stem alone gives
`network_time_protocol_proto`, which says protocol twice. The suffix change removes the duplication
rather than adding to it, which is an argument for this milestone that its own block did not have.

The external-standard exemption is narrowed by this ruling, and that has to be said out loud.
Both crates' own provenance argued against expanding, and the argument was not weak.
`ntp_proto`'s said NTP is RFC 5905's own name for the protocol, *"the same external-standard
exemption `elf`/`pci`/`dtb`/`gpt` already carry"*. `mdns_proto`'s said the expansion does not stop
cleanly, since DNS is itself an acronym and a consistent spelling runs to
`multicast_domain_name_system_proto`. calef ruled against both on 2026-09-13, twice, having been
shown them.

So the exemption now reads: a standard's own name stays whole where it names a format or a piece
of hardware (`elf`, `pci`, `dtb`, `gpt`), and expands where it names a network protocol. That is a
line drawn rather than derived, and a reader is owed the reason: `elf` and `pci` are what the thing
*is* and have no useful longer form in a reader's head, where a protocol's expansion says what it
*does* (network time, multicast DNS) to someone who has not met the acronym. DNS stops because it
is the `pci` case one level down: domain name system teaches nothing a reader did not already
have.

The cost is honest and is this block's to carry: the exemption used to be one rule and is now a
rule with a boundary, and nothing mechanical can tell a format from a protocol. The next name that
tests it comes to calef.

The `ntp` program was to stay `ntp`, and the exception expired before this milestone ran.
Milestone 388, `design/roadmap/0388-an-acronym-sweep-the-tree-can-do-at-once.md`, named this exact pair as a
reason not to work one name at a time: *"Spelling out the program alone leaves the pair disagreeing;
spelling out the crate alone overturns a ratification as a side effect of tidying a program."* This
block accepted the disagreement, on the ground that `AGENTS.md` leaves the length of a typed command
to its author and `ntp` is what a person types.

Milestone 290 found that premise false on 2026-09-14 and dissolved the exception rather than
overruling it. Nothing types `ntp`: the kernel's wiring loads it from the archive by name. So the
program was split into `network_time_client`, `network_time_test_server` and
`unwritable_clock_witness`, all three already spelled the way this milestone would spell the crate,
and there is no `ntp` program left to disagree with `network_time_protocol`. The paragraph is kept
rather than deleted because the reasoning was sound and the fact under it was not, which is worth a
reader's minute.

## Why, and the rule it fails is the tree's own

**`proto` is not an abbreviation, it is a truncation.** `design/naming.md` already refuses the shape:

> Truncating a word you happen to be tired of typing is not abbreviation, it is shorthand, and
> shorthand is what the third principle ("a newcomer must be able to succeed without asking anyone")
> exists to refuse.

And the test it gives: *"would a competent stranger who has never read this tree recognise it?"*
`pci` passes that. `proto` does not, because it is equally short for `prototype`, and this tree
uses that word for a real thing. Milestone 263's spike built a prototype and deleted it on purpose on
2026-09-05, and wrote about doing so in a tree carrying fourteen `_proto` crates. The ambiguity is
live rather than theoretical.

It also fails the acronym rule set the same day, one category over. That rule asks whether an
expansion teaches: `pci` expands to peripheral component interconnect and the reader is no wiser, so
it stays. `proto` expands to protocol, which is exactly what these crates are and is the fact a
reader most needs, so it goes. The rule was written for acronyms and the principle is the same: keep
the short form when it teaches nothing, spell it when it teaches.

calef named it as his own laziness, which is worth recording because the convention was his and
because §75's own line is that the refusals are the valuable half. This one was never refused, only
never examined.

## What this is

A mechanical rename, tree-wide. Directory, package name, every `use`, every `Cargo.toml`
dependency, and the prose that cites them. `_proto` becomes `_protocol`; nothing else about these
crates changes.

Milestone 63 is the precedent and did about twenty names in one pass, including three directories
whose package names matched neither the directory nor the rule.

## Sequencing, which was the whole risk

It collides with almost everything, so it goes when the tree is quiet. Two lanes were named as
the constraint: milestone 264, writing provenance blocks for sixty unrecorded names, and milestone
91, which will touch nearly every documentation file.

**264 was satisfied. 91 was not, and this ran anyway, deliberately.** 264 is `BUILT` and landed; its
branch `milestone/264-name-provenance` is still on the remote and is stale rather than unlanded, the
merge queue having rebased it. 91 is `NOT-STARTED` with no lane on it, and waiting on an unstarted
milestone is waiting indefinitely. The collision with 91 is a merge conflict rather than a
correctness problem, and it is named here so that a future 91 finds it written down:

- Every documentation file 91 touches now spells these crates `_protocol`. The rename is on
  `main`, so 91 rebases onto it and sees no conflict from the spelling itself.
- **What 91 inherits is the 64 markdown files this deliberately did not sweep**, listed by rule
  below. Those still read `_proto`, correctly, because they are accounts. A documentation sweep that
  normalizes spelling across the tree will read them as rot and must not: the status rule is the
  reason, and `design/naming.md`'s rename procedure is where it is written down.
- **And it inherits one thing this milestone could not fix**: `design/decisions/` sections are
  closed decisions and keep `_proto` throughout, so a reader following a decision to a crate lands
  on a directory that no longer exists. Every such citation is prose rather than a checked path
  (`script/citations` checks glosses and block quotes, not backticked spans), so nothing is red.
  It is recorded in this block's `BUGS`.

**And `timebase_proto` was not ratified before this landed.** It was on `script/names --unratified`
as provisional, and ratifying it would have settled a name into a form this milestone was about to
change. calef held it back on 2026-09-05 for exactly that reason, and its successor
`counter_frequency_protocol` is still `provisional` for the same one: he ruled the stem and has not
been shown the whole name.

## `login_protocol` keeps a stem that is still open, and the second rename is accepted

`login_proto` became `login_protocol`, suffix only, stem untouched, and that will cost a second
rename. calef parked the whole `login` family on 2026-09-14 without ruling what it is named for.
The alternative was to leave one crate spelled `_proto` while fourteen siblings read `_protocol`,
which is exactly the split state this milestone exists to end, so the uniform suffix won and the
cost was taken with its eyes open rather than overlooked.

Where the stem question lived: a proposal carrying the measurement and the options, which took no
position. It was answered and deleted on 2026-09-15; see the correction at the end of this section.

**It was not on `main` while this milestone was performed, and that is why the cost is written twice
here rather than cited once.** It sat unlanded on the branch `maintainer/ratify-entropy` until PR
#861 merged partway through this lane, so for most of the work the only record a reader could reach
was this block and the crate's own provenance. That is the branch-as-record failure `AGENTS.md`
names, nobody reads branches, and it is worth leaving recorded now that it is closed: the duplication
above is not redundancy, it is what a lane has to do when the record it depends on is somewhere a
reader cannot get to.

What that costs, stated so nobody has to rediscover it: when the `login` stem is ruled, these
files move again. It is one crate, 27 files at the time of writing, and the compiler finds every
site because a crate rename is compiler-checked. The cost is the record rather than the code, which
is this tree's usual asymmetry: a second rename means a second set of provenance edits and a second
chance to sweep an account. The crate's own provenance block says all of this where a reader meets
it.

**Corrected 2026-09-15: the second rename will not happen.** calef ruled that the `login` stem stays
for the whole family, so the cost this section accepted is never paid and `login_protocol` is final.
The account above is kept as written, because it is what this lane knew and chose when it performed
the suffix; the argument for keeping the stem is in `design/naming/vocabulary-rulings.md`, "The
`login` stem stays".

## How it was performed, and what the sweep took

**The pattern was `git grep -lE '\b(<seventeen alternatives>)\b'`, word-anchored, never a substring
match**, and the file list was written out and read before anything was edited. The two habits
`design/naming.md` prescribes both earned their keep:

- Enumerate before sweeping caught the false positives. Thirty-four distinct `*_proto` tokens
  exist in the tree and only fifteen are live crates; the rest are predecessors kept on purpose
  (`fs_proto`, `cred_proto`, `gfx_proto`, `sink_proto`, `env_proto`), names of crates that do not
  exist yet (`smb_proto`, `nfs_proto`, `manifest_proto`), and `ntp-proto`, which is ntpd-rs's and
  belongs to somebody else. A substring sweep would have eaten all of them.
- **A program's name is not compiler-checked** caught the false negatives for `mdns_responder`. The
  string sites were enumerated rather than trusted to `cargo check`: the `[[bin]]` name and path, the
  archive tuples in `xtask` once per architecture, the kernel's `program(...)` lookups and test
  expectations, `crates/timetable`'s fixture strings, the shipped `components/*.conf` compiled in
  with `include_str!`, and one derived identifier, `mdns_responder_image`.

**The sweep took five records it should not have, and enumerating the diff is what found them.** All
five are restored, each with a sentence beside it saying what the thing is called now:

- A refusal. `timebase_proto`'s *"Refused `timebase_proto`, above"* became a name refusing
  itself, which is this tree's oldest naming scar exactly.
- Two records of what the 2026-08-23 renames produced. `graphics_proto`'s block and
  `user_mode_runtime`'s both cite `cred_proto` -> `credential_proto`; the sweep made that day produce
  a name that did not exist for another three weeks.
- The 2026-07-30 four-ways account, carried identically by `clock_proto`, `entropy_proto`,
  `supervision_proto` and `swap_proto`, where the whole point of the sentence is that two of the four
  spellings ended in `_proto`. Rewriting them left it listing four spellings, two of which no longer
  contain the word being contrasted.
- `socket_proto`'s 2026-08-01 ratification, which named `socket_proto`.

And one gate went blind rather than red. `script/lint` check 3 globs `crates/*proto` to select
the crates whose suffix it then judges. After the rename that glob matches nothing, so the loop body
never runs and the check passes by checking zero crates, which is the `--exclude`-goes-stale failure
`design/naming.md` lists one row over. It now globs `crates/*protocol`, and the hazard is written into
its comment because the next suffix change will meet it again.

The status rule decided the markdown, 108 files swept and 64 left alone: `BUILT`, `REMOVED` and
`RECORDED` roadmap blocks, `design/decisions/`, `design/audit-reports/` and the `BUILT` rows of
`design/roadmap/README.md` are accounts and keep the spelling they were written in. Live intent
moved: `PROPOSED`, `NOT-STARTED` and `PARTIAL` blocks, every file under
`design/roadmap/proposals/`, and `notes/`.

## BUGS

- This is a rename with no functional change, which makes it the kind of diff nobody reads
  carefully. The pattern used and what it was checked against are written out above, which is what
  the original wording of this bullet asked for.
- `design/decisions/` keeps `_proto` throughout, and that leaves paths a reader cannot follow.
  Twenty-two closed decisions cite `crates/filesystem_proto` and its siblings; those directories no
  longer exist. Keeping the old spelling is correct (a closed decision is what was decided, in the
  words used then) and the cost is real, so it is named here rather than hidden. Nothing is red,
  because `script/citations` checks glosses and attributed block quotes, not backticked spans.
- It does not touch `_rt`, `_cli` or any other suffix, and nobody has checked whether the tree
  carries other truncations of the same kind. That sweep is a different milestone and this block does
  not claim it. `_rt` is already gone, by milestone 285 and for the same reason.
- Fifteen crate names get five characters longer, and `nifefs` caps archive names at 32 bytes.
  Crates are not in the archive, so nothing here is bounded by it, but a program taking one of these
  names later would be. The longest name this milestone creates is
  `counter_frequency_protocol` at 26.
- **Five of the renamed crates are still `provisional` and this milestone did not change that.**
  `multicast_dns_protocol`, `multicast_dns_config`, `multicast_dns_responder`,
  `network_time_protocol` and `counter_frequency_protocol` carry stems calef ruled and whole names he
  has not been shown. They stay on `script/names --unratified` and that is the correct place for
  them.

## Follow-on

- **Recorded.** `design/decisions/` keeps the old spelling, so twenty-two closed decisions cite crate
  directories that no longer exist. The limitation is written beside the reader in this block's
  `BUGS` and in `design/naming.md`'s rename procedure, which is where somebody performing the next
  rename meets it.
- **Done.** `login_protocol`'s stem, left open here and expected to cost a second rename, was ruled on
  2026-09-15: the stem stays for the whole family, so there is no second rename. Recorded in the
  crate's provenance block and in `design/naming.md`.
- **Done.** The five stems calef ruled on 2026-09-13 were performed in this pass, including
  `counter_frequency_protocol`, which this block's own table had omitted. Carried by this lane's pull
  request.
- **Done.** `script/lint` check 3's glob, which this rename would have left selecting nothing. Fixed
  in the same pull request, with the hazard written into the check's comment.
- **Milestone 401.** The class that glob belongs to. One instance is a fix; the question of how
  many other gates pick their own subject with a pattern that is allowed to stop matching is a lane,
  and this milestone measured only what it tripped over. Numbered on 2026-09-19 by milestone 433's
  drain of the pile.
- **Recorded.** The other truncations nobody has swept for, in this block's `BUGS` where a reader
  meets the rename: `design/roadmap/0265-proto-is-a-truncation-not-an-abbreviation.md` says `_rt`,
  `_cli` and any other suffix are untouched and that nobody has checked. It is the same shape as the
  acronym question, and milestone 388 is the list that already exists for acronyms, with its `BUGS`
  carrying the names still waiting on an architect. **The disposition was `**Proposed.` until
  2026-09-19 and was wrong then: that word names the proposal file holding *this* work, and the
  file it named holds the acronym question instead, so the gate passed on the path's shape rather
  than on what it held.

## Index row

calef, 2026-09-05: it was lazy and should have been `_protocol` globally. `proto` is equally short for `prototype`, which this tree uses for a real thing, and design/naming.md already refuses truncation. The block's own numbers were wrong and are corrected in it: 15 crates, not 14 (`capability_demo_proto` arrived with milestone 291 after it was written), across 424 files, not 349. Five stems calef ruled on 2026-09-13 travelled in the same pass rather than renaming the same files twice, and the block's table listed four: `timebase_proto`'s provenance records him ruling `counter_frequency_proto` that day and refusing `timebase_protocol`, the name a lane working from the table alone would have created. `mdns_config` and `mdns_responder` moved with their protocol's stem. The sweep took five records it should not have (a refusal, two accounts of what the 2026-08-23 renames produced, the 2026-07-30 four-ways account in four crates, and `socket_proto`'s ratification), all restored by enumerating the diff, and `script/lint`'s `crates/*proto` glob would have gone blind rather than red. `login_protocol` keeps a stem calef has not ruled and will move a second time, accepted knowingly
