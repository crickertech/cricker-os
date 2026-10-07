---
status: PROPOSED
raised: 2026-10-06
milestone_dependencies: 597, 611
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# A package declares what it needs at run time, apart from what it links

calef asked on 2026-10-06 (UTC) how a nife package says what it needs at run time, as distinct from
what it links. Written by lane `package-runtime-requirements`, which built nothing but this file.
Every name below is provisional. The forks at the end are what calef is asked to rule.

**Reuse:** the per-program half reuses `grant_plan::Manifest` and `crates/manifest_note` as they
stand, and the gate extends `helpers/packages.py`. No outside tool fits: Debian, Nix and Fuchsia
each solve this inside their own package formats, so the ideas are taken and the code is not.

## The gap

A package has one relationship to another today, and it is about linking.

- `depends` is checked only for link edges across a package boundary. `helpers/packages.py:410`
  allows a link if the target crate is an `interface` or its package is in `depends`, and nothing
  else reads `depends` except the kind rules below it. `script/lint:3298-3313` runs it.
- So a client of a service never declares the service. `system_log`'s clients link
  `system_log_protocol`, which `contracts` exports as an interface, so calef's ruling on #1796 (fork
  1, 2026-10-07 UTC) added no `depends` edge to `system-log` anywhere. That is right for linking. It
  also means no record anywhere says "this program needs a log service running".

What a program needs at run time is declared, but per program and in code, never per package:

- `grant_plan::Manifest` (`crates/grant_plan/src/lib.rs:1731`) has a field per service a program may
  be endowed with: `clock`, `domain`, `config`, `entropy` (`:1836`), `network` (`:1857`), `machine`
  and `share`. Its `runtime` field (`:1888`) says whether capabilities land in native order or at
  `std`'s fixed slots. An archive program's manifest is a `match` arm. A foreign program's travels
  as an ELF note (milestone 597 (a program carries its manifest in an ELF note), `crates/manifest_note`).
- Who provides each service is a string at a call site. `crates/system_initializer/src/lib.rs:1168`
  looks up `"system_log"`, `:1190` `"entropy"`, `:1194` `"net_stack"` and `:1244` `"redoxfs_server"`.
  Each is optional by design: a missing provider costs a feature, and a declaring child gets an empty
  slot (`:3550`, `:3559`) and says so.
- The log service has no client side at all yet. `:1728` records that nothing registers a log writer,
  so there is no manifest field for it either.
- `component_plan::Requirements` (`crates/component_plan/src/lib.rs:355`) is the closest thing to a
  provider declaration in the tree: a `contract`, capabilities tagged `Serve` or `Use` (`:216`), and
  `depends_on` (`:396`) naming contracts. Only milestone 23 (a capability-routed component OS with live replacement)'s swappable components use it. The boot's
  own services are built from literals in `system_initializer`.

Nothing joins these. No gate knows that `uuid` (in `core-tools`) declares `entropy`, that the
`entropy` package provides it, or that an image without the second still ships the first.

### Ripgrep, verified

`rg` is unmodified ripgrep 14.1.1 from crates.io. It is in no manifest and no recipe.

- Built by `helpers/build-ripgrep.sh`, which nothing in `script/test` or CI runs. It downloads the
  crate into `$TMPDIR/nife-ripgrep`, builds for all three targets with `-Zbuild-std` against the
  `nife-dev` farm, and copies each binary to `target/ripgrep/<triple>/rg`.
- Packed by `xtask` iff that file exists: `xtask/src/farm.rs:71` names the path, and
  `xtask/src/archive.rs:653` (aarch64), `:354` (riscv64) and `:525` (x86_64) push it as `rg`. It is
  one of six names in a hand list, `BUILT_ELSEWHERE` (`:236`).
- `packages/*.package.toml` cannot name it. `helpers/packages.py:325` refuses a `programs` entry that
  is not a binary target in this tree, and `rg` never is. No `*.recipe.toml` names it either. The
  script itself is claimed only by `packages/homes.toml:46`'s blanket `helpers/` entry.
- It carries no manifest note. `helpers/build-ripgrep.sh:67-69` says so: "ripgrep carries no note
  today", and a foreign program would carry one by linking an object nothing here writes yet.

What it is handed, and by whom: the only thing that runs it is the kernel's test harness, not the
progenitor. `system_tests/src/user/ripgrep_tests.rs:71` calls `fs_service::start_std_full`, which
starts a RedoxFS server if none is running and spawns `rg` with three capabilities
(`kernel/src/user/fs_service.rs:2005-2014`):

| Slot | What | Why `rg` needs it |
|---|---|---|
| 0 | a memory region, its heap | `std`'s allocator grows from it |
| 1 | `WRITE` on a rendezvous | stdout and stderr |
| 4 | the file server's endpoint, as a directory | `std::env::current_dir()` and every `std::fs` call |

Slots 2 and 3 stay empty on purpose, so `std::net` can tell it has no network. Without slot 4 the
same binary prints `failed to get current working directory`. The needs are real and specific, and
they are written down only in that harness call.

From the prompt it would be worse. With no note, `grant_plan::image_manifest` (`:1477`) falls back
to `NO_NOTE_MANIFEST` (`:1397`), which is `uptime`'s: native layout, no words. A `std` binary built
in the native layout takes its output endpoint for an allocator (the `Runtime` doc says so). That is
read from the code and not run. The note `rg` needs is three fields: `runtime = Std`,
`arg = Words(ReadOnly)` and `output = Bytes`. `UNVOUCHED_STD_MANIFEST` (`:1574`) is already that
shape, and a word such as `.` then designates the directory, as §170 (how a foreign program is told what to do) rules.

## What a runtime need is here

Three facts shape the answer, and each is already true of the tree.

1. A need is a capability, not a process. A program does not need "the entropy package". It needs a
   `WRITE` endpoint that answers `entropy_protocol`. That is why calef's #1796 ruling has clients
   name the contract and not the implementation.
2. The per-program half exists and is bound to the bytes. §197 (a package is one archive file)'s option M2 put the manifest in the
   ELF, inside what the progenitor hashes. Anything declared again in a package file could disagree
   with it.
3. Absence is a supported state. Every provider in `system_initializer` is optional, and a consumer
   is told it has nothing. So most needs are "wants", in systemd's sense, not "requires".

## Options and prior art

Prior art was read on 2026-10-06 where a URL is given; the rest is marked as recalled.

| Option | Prior art | What it does here | Rung |
|---|---|---|---|
| A. Record it in comments in each package file | none needed | a `# needs entropy` line | 3, a record |
| B. Hand-declared `requires` and `provides` per package, checked by `packages.py` | Debian virtual packages: "the dependency may be satisfied by ... any other concrete package which provides the virtual package" ([policy ch. 7](https://www.debian.org/doc/debian-policy/ch-relationships.html)) | consistency gate over two hand lists | 2 for consistency, 3 for truth |
| C. Unit ordering | systemd: `Wants=` is weak, `Requires=` strong, and "requirement dependencies do not influence the order", which `After=` sets ([systemd.unit(5)](https://man7.org/linux/man-pages/man5/systemd.unit.5.html)) | nothing at package level; ordering is code in `system_initializer` | none |
| D. Derive package needs from what the binaries declare | Nix closures: runtime dependencies are found by scanning the output for store paths, never declared (recalled) | union each package's program manifests into contract names | 2, and cannot drift |
| E. `use`, `offer`, `expose` with routes verified before boot | Fuchsia: `use` "declares capabilities that this component requires in its namespace at runtime", and "there must also be a valid capability route from the consuming component to a provider" ([capabilities](https://fuchsia.dev/fuchsia-src/concepts/components/v2/capabilities)). Its `scrutiny` tool verifies routes over a product image at build time (recalled) | per-program `use`, per-package `expose`, image-level route check | 1 in tree, 2 at pack and install |
| F. A static assembly the compiler wires | seL4 CAmkES: components `provide` and `use` interfaces, an assembly connects them, and glue is generated before run time ([manual](https://docs.sel4.systems/projects/camkes/manual.html)) | the whole system fixed at build | 1 |

Why each loses or wins:

- A is where things stand plus prose. It is rung 3 and would rot the first time a program gains a
  field.
- B is the obvious port and the trap. It duplicates the manifest in a second file, so a gate can
  prove the two package lists agree with each other and never that they agree with the code. It
  also turns every new service into an edit in two places.
- C answers a question nife does not have at package level. Ordering is already readiness: a
  provider answers `READY` before its endpoint is handed out. Keep only its vocabulary, wants against
  requires, for fork 3.
- F fixes the image at build time. §235 (the OS is built and updated from packages) and milestone 198 (a package manager) install packages on a
  running system, and `jig` exists to do it, so a static assembly refuses the thing being built.
- D and E together fit. nife already has Fuchsia's `use` (the manifest) and an implicit `offer` (the
  progenitor endows every service it built to every child that declared it). What is missing is
  `expose`, and a check that a route exists. Fuchsia also keeps packages and capability routes apart:
  a package carries component manifests, and package membership is not a routing edge (recalled).
  That is the split calef's #1796 ruling already drew.

## Recommendation

Declare a need once, per program, where it is today. Declare a provider once, per package. Derive
everything else, and check the routes in a gate.

1. Use: unchanged. `grant_plan::Manifest` for an archive program, its ELF note for a foreign one.
   Add `Manifest::contracts()`, which maps each service field to the contract crate that answers it:
   `entropy` to `entropy_protocol`, `network` to `socket_protocol`, a `file` or `dir` grant to
   `filesystem_protocol`, and so on. Write it as a destructure with no `..`, so a new field is a
   compile error until it is mapped. That is rung 1.
2. Expose: a new key in a package file, naming the contract and the member program that serves it.

   ```toml
   provides = { entropy_protocol = "entropy" }
   ```

   `packages.py` checks that the contract is an interface of `contracts` and the program is a member
   of this package. This is the `subtree_grants` precedent (`helpers/packages.py:449-458`): a package
   key about run-time behavior, held to the code both ways.
3. Route check: `cargo xtask` already links `grant_plan` and checks every archive's programs as it
   packs (`check_declared_programs`). It emits each package's derived contracts, and `packages.py`
   applies the kind rules `depends` already has: a `base` package's need is met by a `base` provider,
   and nothing but `test` relies on a `test` provider. Fork 3 sets how hard.
4. Install: the catalog line `jig` reads gains the derived contracts, so `jig install` can say what
   a package needs that the running system does not provide. The digest already covers the note, so
   the claim is vouched for by the same hash.
5. Boot services: `system_initializer`'s `measured(&fs, table, "entropy")` strings become lookups by
   contract through `provides`. That is a later milestone and not required for the gate.

`depends` keeps its meaning, link edges only. No package gains a `depends` because it calls a
service, which is what #1796's ruling asked for.

What it costs, counted rather than measured. There are seven service fields to map, plus `file` and
`dir`. About six packages gain a `provides` line: `entropy`, `network`, `redoxfs`, `time`, `init`
(for the pages the progenitor endows itself) and `system-log` once it exists. The code is one
function in `grant_plan`, one emitter in `xtask`, and one rule in `packages.py` with a planted
selftest.

The gate would fire on day one, and should. `rm` (`core-tools`, `base`) declares a directory, which
only a file server answers, and the only file server's package, `redoxfs`, is `optional`. Either
`redoxfs` is base, or `rm` is a base program that some images cannot run. That is a real question
the current lint cannot see, and it goes in fork 3.

Would I choose this if B cost the same? Yes. B is less work, and it loses on drift, not effort.

### System log under this proposal

The `system-log` package declares `provides = { system_log_protocol = "system_log" }`. Its first
client, when one exists, adds a manifest field (`log`, provisional) mapped to `system_log_protocol`.
That client's package still links only `contracts`, still has no `depends` on `system-log`, and the
gate now knows it wants a log service and which package provides one.

### Ripgrep under this proposal

Two changes, neither in this pull request.

1. `helpers/build-ripgrep.sh` links a note declaring `runtime = Std`, `arg = Words(ReadOnly)` and
   `output = Bytes`. #1319 measured the mechanism (`-Clink-arg=note.o`). `Manifest::contracts()`
   then derives `filesystem_protocol` from the words grant, which is the slot 4 the harness hands it
   by hand today.
2. A package file the lint can see, which needs one new key for a program built outside the tree:

   ```toml
   name = "ripgrep"
   kind = "test"
   home = { status = "undecided", reason = "upstream is BurntSushi/ripgrep; we ship a build, not a fork" }
   paths = ["helpers/build-ripgrep.sh", "notes/ripgrep-on-nife.md"]

   [[foreign]]
   program = "rg"
   source = { crate = "ripgrep", version = "14.1.1" }
   build = "helpers/build-ripgrep.sh"
   ```

   `packages.py` checks that `build` is a member path and that no in-tree binary shares the program
   name. `xtask`'s `BUILT_ELSEWHERE` list is then read from `[[foreign]]` entries instead of being a
   second hand list.

The lane did not add this file. `packages.py` refuses an unknown key, so the cheap version is
`paths` alone, which would place `rg` in the table without saying it is a program. That reads as a
complete record and is not one, so the lane left the file for the ruling.

## Forks

Each is reversible while nothing outside the tree reads these files. A "no" leaves the gap as it is
and costs nothing that runs today.

1. Where a run-time need is declared. Recommend: only in the program's manifest, and derive the
   package's needs from it (D). Refused: a hand-written `requires` per package (B), because it can
   disagree with the note the progenitor actually endows from.
2. How a provider is declared. Recommend: a `provides` key in the package file, naming a `contracts`
   interface and the member program that serves it (key name provisional), checked like
   `subtree_grants`. The alternative is a `Serve` declaration in the provider's own note, which is
   stronger and waits on the boot services adopting a manifest at all.
3. How hard an unmet need fails. Recommend: the in-tree gate fails on a `base` need with no `base`
   provider, and `jig install` warns rather than refuses, since every consumer degrades and says so.
   This fires at once on `rm` and `redoxfs`. Recommend making `redoxfs` base, because a base image
   whose `rm` cannot run is the defect, not the rule.
4. Whether `rg` becomes a package now. Recommend: yes, named `ripgrep` (provisional), kind `test`
   until `jig` installs it from the index, with a new `[[foreign]]` key, and with its note written by
   `helpers/build-ripgrep.sh`.

Names for calef, all provisional: `provides`, `[[foreign]]`, `Manifest::contracts`, the package
`ripgrep` and the manifest field `log`.

## BUGS

- Services that boot services use from each other, such as `system_log`'s `WRITE` on the console,
  stay literals in `system_initializer`. Only `component_plan` components declare those today.
- The derived set says which contract, not which instance. Two file servers would both provide
  `filesystem_protocol`, and which one a program gets is still the progenitor's choice.
- Prior art marked "recalled" was not reread for this file.
