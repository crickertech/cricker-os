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

- Built by `helpers/build-ripgrep.sh`, which nothing in `script/test` or CI runs, for all three
  targets with `-Zbuild-std`, into `target/ripgrep/<triple>/rg`.
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

Without slot 4 the same binary prints `failed to get current working directory`. These needs are
written down only in that harness call.

From the prompt it would be worse. With no note, `grant_plan::image_manifest` (`:1477`) falls back
to `NO_NOTE_MANIFEST` (`:1397`), which is `uptime`'s: native layout, no words. A `std` binary built
in the native layout takes its output endpoint for an allocator (the `Runtime` doc says so). That is
read from the code and not run. The note `rg` needs is three fields: `runtime = Std`,
`arg = Words(ReadOnly)` and `output = Bytes`. `UNVOUCHED_STD_MANIFEST` (`:1574`) is already that
shape, and a word such as `.` then designates the directory, as §170 (how a foreign program is told what to do) rules.

## What a runtime need is here

A need is a capability, not a process: a program needs a `WRITE` endpoint that answers
`entropy_protocol`, not "the entropy package". And absence is supported, since every provider in
`system_initializer` is optional, so most needs are wants in systemd's sense, not requirements.

## Options and prior art

Prior art was read where a URL is given; the rest is marked recalled. How eleven systems package
software across languages, Genode and Fuchsia in particular, is in the
[prior-art appendix](a-package-declares-what-it-needs-at-run-time/prior-art.md).

| Option | Prior art | Rung |
|---|---|---|
| A. A comment in each package file | none | 3 |
| B. Hand-declared `requires` and `provides` per package | Debian: a dependency "may be satisfied by ... any other concrete package which provides the virtual package" ([policy ch. 7](https://www.debian.org/doc/debian-policy/ch-relationships.html)) | 2 for consistency, 3 for truth |
| C. Unit ordering | systemd: "requirement dependencies do not influence the order", which `After=` sets ([systemd.unit(5)](https://man7.org/linux/man-pages/man5/systemd.unit.5.html)) | none |
| D. Derive package needs from what binaries declare | Nix closures, found by scanning outputs for store paths (recalled) | 2, cannot drift |
| E. `use`, `offer`, `expose`, routes verified before run | Fuchsia: "there must also be a valid capability route from the consuming component to a provider" ([capabilities](https://fuchsia.dev/fuchsia-src/concepts/components/v2/capabilities)); `scrutiny` checks routes over an image (recalled) | 1 in tree, 2 at pack and install |
| F. A static assembly the compiler wires | seL4 CAmkES: `provides` and `uses`, connected in an assembly, glue generated before run time ([manual](https://docs.sel4.systems/projects/camkes/manual.html)) | 1 |

- B duplicates the manifest, so a gate can prove two hand lists agree and never that they agree with
  the code.
- A rots the first time a program gains a field. C does not apply: ordering is readiness, a
  provider answering `READY` before its endpoint is handed out.
- F fixes the image at build time, and §235 (the OS is built and updated from packages) and `jig`
  install onto a running system.
- D and E fit together. nife has Fuchsia's `use` (the manifest) and an implicit `offer` (the
  progenitor endows each service it built to each child that declared it). What is missing is
  `expose`, and a check that a route exists.

## Recommendation

Fork 1 is ruled, so needs live only in program manifests and the build derives the rest.

1. Use: `grant_plan::Manifest`, carried as in fork A. Add `Manifest::contracts()`, mapping each
   service field to a contract identifier. Write it as a destructure with no `..`, so a new field is
   a compile error until it is mapped (rung 1). A manifest also names its runner (fork B).
2. Expose: a `provides` table in a package file, covering contracts and runners (fork 2).
   `packages.py` checks each named program is a member, the `subtree_grants` precedent
   (`helpers/packages.py:449-458`).
3. Route check: `xtask` already links `grant_plan` and checks every archive it packs
   (`check_declared_programs`, `xtask/src/archive.rs:265`). It emits each package's derived needs,
   and `packages.py` applies the kind rules `depends` already has (fork 3).
4. Install: the derived needs go into the package metadata and the index, per fork 1.

`depends` stays link-only, so no package gains a `depends` on `system-log` by calling it. Counted
cost: seven service fields plus `file` and `dir` to map, a `provides` table in about six packages,
one function in `grant_plan`, one emitter in `xtask`, one rule in `packages.py`. Would I choose this
if B cost the same? Yes; B loses on drift, not effort.

The gate would fire on day one. `rm` (`core-tools`, `base`) declares a directory, which only a file
server answers, and the only file server's package, `redoxfs`, is `optional`. Fork 3 takes it.

System log: `system-log` provides the contract `system_log`. Its first client adds a manifest field
(`log`, provisional), links only `contracts`, and has no `depends` on `system-log`.

Ripgrep: `helpers/build-ripgrep.sh` links a note declaring `runtime = Std`, `arg = Words(ReadOnly)`
and `output = Bytes`, by the mechanism #1319 measured (`-Clink-arg=note.o`). The words grant then
derives the `filesystem` contract, which is slot 4 above. The lint sees it through fork 4's file.

## Every kind of language

calef, 2026-10-07 (UTC): *"What about dynamic languages, non-C languages, go, java? I'm trying to
make certain that we can package software comprehensively."* The question here is only where each
kind's manifest lives and who runs it. Porting each runtime is per-workload milestone work, not part
of this proposal.

| Kind | Examples | What the kernel runs | Where the manifest lives | In the tree today |
|---|---|---|---|---|
| Native compiled | Rust, C, C++, Zig, Swift, Go | the program's own ELF | an ELF note | Rust `std` (`patches/std-nife`); C behind a Rust shim |
| Interpreted | Python, JavaScript, Ruby, Lua, shell | the interpreter | a comment block after `#!` (fork A) | none |
| Bytecode VM | Java, .NET, WebAssembly | the VM | a custom section, or an entry under `META-INF/` (fork A) | none |

Native compiled. Each language needs its runtime ported, and the shape differs. C runs today only
as §31 (the foreign-language seam) builds it: `fixtures/c/c_seam.c` is linked into `c_shim`, a Rust
shell that holds every capability, and the C makes no syscalls. Full POSIX, §31's tier three, is
unbuilt. C++, Zig and Swift need a libc or their own runtime ported; nothing has started. Go is
different: its runtime makes system calls itself per `GOOS` rather than through a libc (recalled),
so Go needs a `GOOS=nife` port. `git grep GOOS` finds nothing. §83 (take the Rust one) still
applies, so a C path is for software with no Rust equivalent. A native program carries its note by
linking an object, the same as `rg`.

Interpreted. The interpreter is the ELF the progenitor builds, shared by every script. Its note
cannot carry a script's needs, because it would have to be the union of every script's, which is
ambient authority. So the manifest belongs to the script, and the grants attach to the
interpreter-plus-script pair: the interpreter's own needs plus what the script declares. That closes
§219 (how the shell names an installed program to the spawner)'s first recorded limitation, that a
vouched interpreter runs any script with all of its own authority (`design/decisions/219-naming-an-installed-program-to-the-spawner.md:45`).
§219 already names the shape: `interp build.nsh`, with the script as a read-only file grant (`:129`).

Bytecode VMs. The same as interpreters, one level down. The JVM and .NET assume threads, which
§105 (`std::thread::spawn` stays declined) declines for now, so they wait on that ruling as well.
WebAssembly is the cheap route. WASI's preopens hand a module its directories at start, which is
this system's directory capability already. §84 (how we port) records that alignment and is careful
that it is "of design shape, not of ABI". One WASI runtime ported as a nife program would run every
language that targets `wasm32-wasip1` behind one runner. That is a candidate milestone, not a
promise.

## Forks

Each is reversible while nothing outside the tree reads these files. A "no" leaves the gap as it is
and breaks nothing that runs.

1. Where a need is declared. Ruled by calef, 2026-10-07 (UTC): only in program manifests. The build
   computes them into package metadata (§197 (a package is one archive file)) and the index (§250 (an image names its
   distribution's package index)), where `jig` resolves them before download.

A. Where a manifest travels. Revised by calef, 2026-10-07 (UTC), on his question *"Could a script's
   manifest be part of the start of the script much like #! is used to identify the scripting
   language/executor?"*, answered *"Yes, that seems more elegant. Obviously jig would need to evolve
   as we expand language support."* The proposal now reads: a program carries its manifest inside
   itself, in its own format's metadata slot.

   | Format | Slot | Encoding |
   |---|---|---|
   | ELF | the note, milestone 597 (a program carries its manifest in an ELF note) | the note's binary descriptor |
   | Script | a comment block right after `#!` | the text form below |
   | WebAssembly | a custom section named `nife.manifest` | the note's binary descriptor |
   | Java jar | the entry `META-INF/nife/manifest` | the text form |
   | A format with no slot | a sidecar file, as the last resort | the text form |

   The marker. A block opens with a line made of the script's comment prefix, a space, `/// nife`,
   and closes with the prefix, a space and `///`. Every line between starts with the same prefix.
   The reader takes the prefix from the opening line, so `#`, `//` and `--` all work. This is PEP
   723's block shape with its own type word, so a Python script can carry `# /// script` for its
   packages and `# /// nife` for its grants side by side:

   ```python
   #!/usr/bin/env python3
   # /// nife
   # runner = python
   # entropy = true
   # ///
   ```

   The encoding. A text form inside text formats, the binary descriptor inside binary ones, and both
   decode to one `grant_plan::Manifest`. The text form is a strict subset of TOML: one `key = value`
   per line, keys from `Manifest`'s fields, values bare words, integers or booleans, and no tables or
   quoted strings. That keeps the reader the progenitor needs small enough to fuzz, while any TOML
   parser still reads it. A round-trip test (text, `Manifest`, descriptor, `Manifest`) keeps the two
   encodings equal.

   The consequence. One digest covers code and manifest together, for a script as for an ELF, so
   the earlier draft's separate manifest file and its own entry in the activation set go away. The
   cost. `jig` and the spawner each need one small reader per format, so `jig` grows as language
   support grows. That belongs to the package client's milestone (#1799 promotes it), and this
   proposal does not edit that block.

   Prior art, in the [appendix](a-package-declares-what-it-needs-at-run-time/prior-art.md): PEP 723,
   Cargo's script frontmatter, `nix-shell` `#!` lines, Deno's permission flags in a shebang,
   WebAssembly custom sections and the jar manifest. Refused: a sidecar for every format, which
   separates the manifest from the bytes §197's option M2 hashes, and a stub ELF per script, which
   repeats the interpreter in every package.

   Recommend this shape, with the marker and encoding above.

B. How a program names what runs it. Options:
   - `#!` and a path the shell resolves, Unix's and Linux `binfmt_misc`'s way (recalled). It names
     a path, which is ambient, and the package system cannot see it.
   - Bundle the interpreter into each package. Every package then ships its own copy.
   - `runner` as a kind of need. A manifest names a runner (`elf`, `python`), and an interpreter's
     package provides it. Fuchsia does this: `program: { runner: "elf", binary: "bin/example" }`,
     the ELF runner built in, and a component able to serve a runner capability
     ([runners](https://fuchsia.dev/fuchsia-src/concepts/components/v2/capabilities/runner)).

   Recommend `runner` as a need, with `elf` implicit for native programs and provided by `init`.
   Unlike a contract, a missing runner means nothing runs, so it is a requirement, not a want. A
   script keeps its `#!` line for other systems; nife reads the runner from the block, not the path.

2. How a provider is declared. Recommend a `provides` table in the package file covering both kinds,
   each naming the member program that serves it:

   ```toml
   [provides]
   contracts = { entropy = "entropy" }
   runners = { python = "python3" }
   ```

   A contract is named by an identifier, and the Rust crate (`entropy_protocol`) is one binding of
   it, so a Python or WASI client names the same contract. The alternative, a `Serve` declaration in
   the provider's own manifest, is stronger and waits on the boot services adopting manifests.
3. How hard an unmet need fails. Recommend: in the tree, a `base` program's contract or runner must
   be met by a `base` provider, and the gate fails otherwise. In `jig`, a missing runner is resolved
   or refused before download, and a missing contract is installed if it can be and warned if not,
   since a consumer degrades and says so. This fires on `rm` at once; recommend making `redoxfs`
   base, because a base image whose `rm` cannot run is the defect.
4. Whether `rg` becomes a package now. Recommend yes: `ripgrep`, kind `test` until `jig` installs it,
   its note written by `helpers/build-ripgrep.sh` (carrier: the ELF note; runner: `elf`), and a new
   `[[foreign]]` key, since `packages.py` admits only in-tree binaries:

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

   `xtask`'s `BUILT_ELSEWHERE` list is then read from `[[foreign]]` entries. A paths-only file would
   pass the lint today and read as complete when it is not, so the lane added none.
5. Versioning. Held for the package client's milestone, which #1799 promotes: a later version
   goes on the contract, as Fuchsia's API levels do, not a range on the providing package.

Names for calef, all provisional: `provides`, `contracts`, `runners`, `runner`, `[[foreign]]`,
`Manifest::contracts`, the package `ripgrep`, the manifest field `log`, and contract identifiers.

## BUGS

- Needs between boot services, such as `system_log`'s `WRITE` on the console, stay literals in
  `system_initializer`. Only `component_plan` components declare those.
- A contract names a kind of service, not an instance. Two file servers would both provide
  `filesystem`, and the progenitor still chooses.
- Prior art marked "recalled" was not reread for this file.
