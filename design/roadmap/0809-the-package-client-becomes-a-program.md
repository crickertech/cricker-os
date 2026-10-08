---
status: NOT-STARTED
raised: 2026-10-06
promoted_from: the-package-client-becomes-a-program
milestone_dependencies: 205
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 809. The package client becomes a program, `jig`, with the verbs an index needs

*(Promoted from the proposal pile on 2026-10-07 (UTC), calef: "Promote jig to a milestone." The
number is provisional until the merge queue lands it. `jig` is ratified; the title, slug and verb
spellings are drafts. Later rulings are in the work list and the forks table.)*

*Dependencies corrected 2026-10-07 (UTC) by this lane: 198 was dropped. 198 is the umbrella over 802,
which names this block, so the three waited on each other. What this block uses from 198 is rung 3a,
built inside 198 by 2026-10-05 with no milestone of its own.*

calef ruled on 2026-10-06 (UTC) that the package client must be a program, not a shell builtin, and
named six things it must do.

The program is `jig`, the basalt package manager, ratified 2026-10-06 by calef: the guide that makes
flat-pack assembly come out the same every time. Refused: `package`, `pkg`, `bpm`, `knap` and
`flatpak`; `kit` and `cam` collide with shipped commands. When built, this record becomes the
`Name:` block in its module doc, where `script/names` reads it.

calef also ruled, the same day: *"We abbreviate on the command line."* The verb spellings below
stay provisional and are to be revisited under it.

## What calef asked for

His wording is the intent.

| Intent | Spelling | Today |
|---|---|---|
| List available packages, from the local copy of the index | `jig list` | nothing |
| Install a package from the index | `jig install <name>` | the builtin, fetched by the progenitor |
| Install a local package, an archive already on the system | `jig install <path>` | the builtin; a word with a `/` is a file (§219 (how the shell names an installed program to the spawner)) |
| Add another index | `jig add-index <url>` | nothing |
| List what is outdated | `jig outdated` | nothing |
| Update the local copy of the index | `jig update` | nothing |
| Remove a program | `jig remove <program>` | the builtin |
| Roll back a generation | `jig rollback` | the builtin |

`update` refreshes the index and installs nothing, as in apt, pkg and Homebrew (recalled, not
read). No upgrade verb is proposed.

*Amended 2026-10-07 (UTC), calef on #1805: `jig` must update base packages; only the slot reboots. See §159 (only a new kernel needs a reboot).*

## The premise, checked

The builtin's doc comment gave two reasons. The progenitor alone reads the activation set and holds
what installing needs: still true, but a reason for the *installer* to stay there, never for the
client to live in the shell. And a program could not be told which package: stale since milestone
205 (how a foreign program is told what to do) built `ArgSpec::Words` on 2026-09-27. The lane
corrected that reason in all three places it sat.

## What moves, and what stays the progenitor's

Out of the shell: `Command::Package`, `PackageVerb` and `package_verb` in `crates/grant_plan`,
`package` in `components/src/swish.rs`, and `PACKAGE_USAGE` in `crates/swish`. The builtin and the
program go in one change, so the owner never has two clients for one store. Builtins match before
program names, so if the program were named `package`, a builtin left behind would shadow it on
every line ([the naming appendix](../naming/programs-scripts-and-directories.md#shell-builtins)
refused `doc search` for that reason).

Out of the progenitor: the fetch. Today `Activation::Fetch` has the progenitor open a socket, send
`GET /<stem>.nifepkg` and parse the reply with `http_response`, before any digest check.
`notes/packages.md` lists that parser as a limitation: network input read by the most trusted
process. Under §195 (a reviewed recipe vouches for a package) the digest decides, so whoever
carries the bytes need not be trusted. The program fetches them and sends them as an ordinary
`Activation::Install` with frames, exactly as a local file goes today. `Activation::Fetch` retires,
and `http_response` leaves the progenitor's dependency graph.

Stays the progenitor's, unchanged:

- the activation set, `activation/` and its generations, and the one rename of `current` that
  changes what runs (§208 (installing a package is granting it, and the activation set is
  versioned));
- the digest check, `package_archive::installable` on the progenitor's own staged copy;
- the image's catalog, measured in the archive, which is what it vouches against today.

`vouch` is not on calef's list and stays a builtin. It is the same request one verb over, so a
`jig vouch` later is a small and reversible change.

## The capabilities the program holds

| Capability | Verbs that use it | How it is granted | Exists today? |
|---|---|---|---|
| A client view of the network stack (`NETWORK_SLOT`) | `install <name>`, `update` | `Manifest::network` | yes (milestone 590 (the booted system starts its network stack)) |
| The index directory, read and write | `list`, `outdated`, `update`, `add-index`, `install <name>` | a directory grant; path provisional, `packages/indexes/` | the grant kind, yes; the directory, no |
| `activation/`, read only | `outdated` | a directory grant | yes, the kind |
| The file a line names | `install <path>` | `ArgSpec::Words` designation (§170 (how a foreign program is told what to do)) | yes |
| A request to the installer | `install`, `remove`, `rollback` | new, below | no |

The last row is the one new grant. Today the activation request rides the spawn endpoint, which
only the boot prompt holds, and §221 (the boot prompt is the owner's console) made holding that
prompt the definition of the owner. Handing a program the whole spawn endpoint would let it spawn
anything. The plan is an endpoint the progenitor badges, which speaks install, remove and
rollback and nothing else. It is granted through a new `Manifest` field, provisionally
`activation`, in a named slot like `NETWORK_SLOT`. It does not speak `Vouch`, which would let the
program vouch for any bytes. Only the owner's console may grant it. A `login` session holds
no spawn endpoint and so could not pass one on, which keeps today's rule that only the owner
installs. The wire format is `spawnproto`'s activation request unchanged; only the endpoint it
arrives on is new. This is a new grant inside the capability model, not a syscall. CLAUDE.md asks
that its semantics be recorded in `design/decisions/`, which the integrator mints at merge.

The program is `Runtime::Std`, because `ArgSpec::Words` requires it.

## One program with verbs: ruled

calef, 2026-10-06 (UTC): *"One program."* That refuses V3. V1 against V2 is detail within it, and
this block keeps V2.

calef's principle, from §226 (`pidwait` takes tids): a program does one thing. Milestone 281
(`watch` holds exactly what `ps` holds) turned that into a test a capability system can measure. Two programs are two
programs when they hold different authority. By the table above, these verbs do. `list` needs one
directory, `remove` needs only the installer, and `update` needs the network and a write.

| Option | What it is | Verdict |
|---|---|---|
| V1. One program holding the union | Every line holds the network, the index, `activation/` and the installer | Refused. `jig list` would carry authority it never uses: 281's own counterfactual, where `ps` would always carry a clock it rarely needs |
| V2. One program, granted by verb | The manifest maps each verb to its grants, and the planner reads the first word | Recommended |
| V3. Split by authority | A reader (`list`, `outdated`), an index keeper (`update`, `add-index`) and an installer client (`install`, `remove`, `rollback`) | Refused by calef's ruling |

V2 has a precedent in this tree. `rm -r` hands over more than `rm`, through `subtree_flag` in
`rm`'s manifest: the extra grant exists only on the line that asks for it. A verb table is the same
idea keyed by a word instead of a flag. The planner already classifies the builtin's tail
(`package_verb`), so what moves is the table's home, from the shell's parser into the manifest.
The cost is one new `Manifest` field and a planner that consults it.

V2 gets V3's least authority under one name. V1 is cheaper, so V2 is not about effort, and it is
reversible: a lane can build V1 first with a `BUGS` line.

## Who writes the index copy: ruled, `jig`

calef, 2026-10-06 (UTC), choosing I2: *"I would think jig."* Once milestone 801 (packages over the internet)'s index replaces
the image's catalog, whoever writes the index copy decides which digests the progenitor accepts.

| Option | What it is | Cost |
|---|---|---|
| I1. The progenitor fetches the index | `jig update` is one more activation request | TLS and an index parser in the most trusted process (§196 (nife carries TLS)) |
| I2. The program fetches and writes the copy | The progenitor trusts the copy in a directory only the owner's console grants | A client defect becomes a defect in what runs; provenance becomes "the owner's client said basalt vouched" |
| I3. A signature over the index | The progenitor verifies the copy against a key in the image | A long-lived key, which §195 clause 4 deferred and named irreversible |

I2 widens nothing, since the owner may already vouch for any bytes (§195 clause 3, §221). Until
I3, `jig` is inside the trusted base for installs; I3 waits on a key-custody ruling. The image's
own catalog is untouched.

## Many indexes per machine: ruled

calef, 2026-10-06 (UTC): *"A machine may write many indexes."* So a machine holds several, as apt's
`sources.list` does; details below.

§250 (an image names its distribution's package index) fixes only an image's one index, basalt's;
an index the owner adds is the machine's. §195 clause 2 scopes trust per source, and §196 clause 4
holds roots per source.

To settle before `add-index` is built:

1. How is a second index authenticated? §250 authenticates basalt's index by TLS alone, pinned to
   ISRG Root X1. A second index needs a root or key the owner supplies at `add-index`. There is no
   system store to fall back on, by §196 clause 4's design.
2. Two indexes may vouch for different bytes under one name. The tree's analogous case is milestone
   614: a bare name the catalog vouches for at several versions is refused as `Ambiguous`, and
   `name@version` picks one. The same refusal, with an index-qualified name to pick, is one answer.
   First-index-wins is another, and it is the shape behind dependency-confusion attacks.
3. The activation set records `OWNER` for a vouch. It would need to record which source vouched,
   or a rollback cannot say what it is undoing.
4. Whether a second index may carry §250's "moved to" field, and whether the client follows it.

## Other package managers, against the same contracts

calef asked whether other package managers could emerge. This milestone makes it a design
property: `jig` uses only public contracts, with no private channel to the progenitor.

- The package format is `crates/package_archive`, per §197 (a package is one archive file).
- The index format is milestone 801's crate, by rule 7.
- The request to install, remove or roll back is `spawnproto`'s activation request, per §208.
- The installed table is `crates/activation_set`.

A second manager on those crates can do what `jig` can.

Who may install means which programs the owner's console has handed the installer capability. A
gate can hold the property: no `jig` dependency outside those crates and `user_mode_runtime`.

calef accepted how managers coexist, 2026-10-06 (UTC): *"Yes."*

- By default only `jig` holds the installer endpoint, and the owner's console is what grants it.
- Other tools stay user-local. They run by path or are vouched, like `pipx` or `~/.cargo/bin`, and
  never touch the system activation set.
- Each activation set row records which manager installed it. `activation_set::Entry` gains that
  field. If the owner ever grants a second manager the endpoint, `jig` refuses to remove or roll
  back a row it did not install.

## How this splits against milestones 801 and 802

Milestone 801's items 1 and 2, the index split and the index format crate, need neither name
resolution nor HTTPS, and are what `list`, `outdated` and `update` consume. Promotion did not move
them here, so item 6 waits on 801. Milestone 802 (the trivial install) depends on this block so
its stranger meets a program, not a builtin about to go.

## The work, in the order it can land

1. The installer capability: the badged endpoint, the slot and the `Manifest` field, granted only
   by the owner's console. Falsify it: a program without the grant is refused, and a `login`
   session cannot grant it.
2. The program, with `install` (both forms), `remove` and `rollback`, ported from `swish::package`.
   The builtin goes in the same change. `script/swish-check`'s transcripts and `notes/packages.md`
   change from builtin to program.
3. The fetch moves into the program, and `Activation::Fetch` and `http_response` leave the
   progenitor. The gate that fetches `greeting` today passes unchanged in what a person types.
   Per calef's ruling on #1796 fork 4 (2026-10-07), the same change deletes `system_initializer`'s
   `http_response` exception in `packages/init.package.toml`.
4. Grants by verb (V2), or V1 with a `BUGS` line if V2 is refused.
5. `list`, reading the image's catalog until an index exists, and saying so.
6. Milestone 801's items 1 and 2 if moved here, then `update` and `outdated` over the LAN fixture
   source, plain HTTP, as rung 3a, with `jig` writing the index copy.
7. `add-index`, once the details of many indexes are settled.
8. Before installing, `jig` checks three things. The archive's digest matches the index (§250 (an
   image names its distribution's package index)). The index's needs match the archive's metadata
   (§197 (a package is one archive file)). The metadata matches the manifests in its ELF notes,
   recomputed after download. A mismatch refuses (calef, 2026-10-07, "Yes"). The needs
   come from [#1797's fork 1 ruling](https://github.com/nifeos/nife/pull/1797#issuecomment-6029565630):
   written only in program manifests, computed by the build into metadata and index. These checks
   keep resolution correct and records honest; they are not the security boundary. Enforcement
   stays at spawn, where a program gets only what its manifest names and the grant plan approves
   (§208).

Items 1, 2, 3 and 5 wait on nothing.
Every item is proved on aarch64, riscv64 and x86_64 by the same gate, per rule 5.

Reuse: all inside the tree. `package_archive` (`catalogd_stem`, `installable`, `installable_as`),
`http_response`, `socket_protocol`'s fixture peer, `activation_set`, `spawnproto`'s activation
request, swish's frame sender and milestone 205's argv and designation. Writing the program rather
than adapting one is forced: no existing client speaks this capability ABI.
Outside it, `rustls` arrives through milestone 501 under §196. The verb spellings lean on apt,
pkg and Homebrew from memory. The building lane owes a read of their manuals under §46
(thin primitives or whole subsystems).

## Plan, 2026-10-07 (UTC)

Written by this lane, stopped before building. The block has no
exit section, so this proposes one, on all three architectures under QEMU. `jig install` (both
forms), `remove` and `rollback` pass `script/swish-check`'s transcripts with the builtin gone.
`greeting` is fetched by `jig`, and the progenitor has no `http_response` or `Activation::Fetch`.
A program without the installer grant is refused. That is items 1 to 5 and 8's digest check. Items
6, 7 and the index half of item 8 need 801's index crate.

Forks not yet ruled, each with a recommendation:

1. The installer endpoint's semantics. A badged endpoint speaking install, remove and rollback is
   a new method in the capability model, so a `design/decisions/` section is owed before item 1
   (`decision_dependencies: unwritten`). Recommend the block's design: `spawnproto`'s activation
   request unchanged, a new slot, no `Vouch`.
2. V2's verb table is a new `Manifest` field, so its encoding in 597's ELF note is a format the
   build and the planner agree on. Recommend a list of (verb, grants) pairs; calef rules.
3. The `Entry` field naming a row's manager changes the persisted activation set. Recommend adding
   it before item 1 lands; how older rows read back is unmeasured.
4. Verb spellings under "we abbreviate on the command line". Recommend keeping the full words for
   the first build and ruling abbreviations separately; nothing else depends on them.
5. Whether 801 items 1 and 2 move here. Recommend yes, if 810 is the next customer step: its exit
   reads basalt's index.

Rough size: items 1 to 3 are a large lane (a new grant, a new program, a retired wire format);
4, 5 and 8 a medium one. 810 needs items 1 to 3 and 8, plus 801's
HTTPS and name resolution for `basalt.nifeos.org`, which its `milestone_dependencies` does not list.

## Forks

| Fork | State | Blocks |
|---|---|---|
| The name | ratified 2026-10-06: `jig` | nothing |
| One program or several | ruled 2026-10-06: one program; V2 kept | nothing |
| Who writes the index copy | ruled 2026-10-06: `jig` (I2); I3 the follow-on | nothing |
| Many indexes | ruled 2026-10-06: a machine may write many; details open | item 7 |
| A row records its manager | ruled 2026-10-06: yes; `jig` alone holds the endpoint by default | item 1 |
| Who fetches | ruled 2026-10-07, #1796 fork 4: `jig` | nothing |
| Install-time checks | ruled 2026-10-07: item 8's three | nothing |
| Needs against what is installed | open, #1797 fork 3 | nothing yet |

## BUGS

- Until an index exists, `list` can only read the image's catalog. That is a list of what this
  image vouches for, not of what a distribution offers.
- Retiring `Activation::Fetch` changes a wire format that only this tree speaks.
- Until I3, `jig` is inside the trusted base for installs, and the activation set's provenance for
  an index install is `jig`'s claim.

## Index row

The package client is a shell builtin, and the progenitor fetches packages, so the most trusted
process parses HTTP. calef ruled the client becomes one program, `jig`, with seven verbs and an
installer endpoint only the owner's console grants. The fetch moves into `jig`, retiring
`Activation::Fetch` and `init`'s `http_response` exception.
