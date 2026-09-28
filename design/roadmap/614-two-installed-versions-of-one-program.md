---
status: NOT-STARTED
raised: 2026-09-27
milestone_dependencies: 198, 47
decision_dependencies: 208, 219, 229, 241
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 614. Two installed versions of one program, each runnable, and a caller granted the one it needs

Minted 2026-09-27 (UTC) by a maintainer-delegated lane, from calef's ask the same day: *"One
limitation of our model is that we cannot support multiple versions of the same binary on a system.
This is important to developers that might run different versions of their frameworks because
different packages have different dependencies on upgrading a dependency. We should mint a milestone
to address that."* *(Number provisional until the merge queue lands it. Title and slug are drafts.)*

## Why two versions cannot coexist today

The bytes already can. Install writes a program to `packages/<name>/<version>/<program>`
([notes/packages.md](../../notes/packages.md)), and remove leaves them there. What cannot hold two
is the table that says what may run:

- A generation of the activation set, per §208 (installing a package is granting it), is `<program> <package>
  <digest>` lines with no version field. `activation_set::with_entry` keeps one entry per program
  name and replaces it in place, which is how an upgrade works.
- Running by path is not a way round it. §219 (how the shell names an installed program to the
  spawner) sends bytes, and the progenitor runs them only if their digest is in the live
  generation (`activation_set::lookup_digest`). The old version's digest left the table when the new
  one replaced it, so `packages/uptime/0.1.0/uptime` gets `SPAWN_UNVOUCHED` once 0.2.0 is installed, unless the session
  holds the capability to run unvouched bytes, which is a developer's escape hatch and not an
  install.
- §229 (a bare name reaches an installed program) resolves a bare word through one entry per name,
  with no search order, by calef's ruling. Pull request #1374 (milestone 47, a bare word runs an
  installed program) adds the refusal that another package may not take a name already provided.
- Until §220 (signed builds) is built, a new version of any package is a new image catalogue, and
  so a new boot slot: §241 (a threadbare base), "What is being decided", item 2.

Libraries are not the problem here. There is no dynamic loader and no shared library
([notes/abi.md](../../notes/abi.md)), so each program links its own dependencies at build. Where
nife's "framework" versions collide is in programs and the services they talk to.

## Prior art

Read 2026-09-27 from each project's own documentation, except where marked.

- **Nix.** Every build lands at `/nix/store/<hash>-<name>-<version>`, so versions never collide. A
  profile generation is a tree of links into the store; the bare command is whatever the current
  generation links, and rollback swaps generations
  ([store paths](https://nix.dev/manual/nix/stable/store/store-path.html),
  [profiles](https://nix.dev/manual/nix/stable/package-management/profiles.html)). Guix is the same
  design (from memory). nife's versioned activation set is already Nix's generations; what it
  lacks is Nix's store, where many versions stay live.
- **Homebrew.** Each version is a keg at `Cellar/<formula>/<version>`; `opt/<formula>` and `brew
  link` point at the active one. A second major version is a second formula, `python@3.12`, left
  unlinked and reached by its `opt/` path ([FAQ](https://docs.brew.sh/FAQ),
  [Formula Cookbook](https://docs.brew.sh/Formula-Cookbook)).
- **Debian alternatives.** A generic name links through `/etc/alternatives/` to one registered
  alternative, highest priority wins unless an administrator sets one
  ([update-alternatives(1)](https://manpages.debian.org/bookworm/dpkg/update-alternatives.1.en.html)).
  Versioned package names such as `python3.11` beside `python3.12` carry the rest (from memory).
- **Fuchsia.** A package is identified by the Merkle root of its metadata.
  `fuchsia-pkg://<repo>/<package>?hash=<root>#meta/<component>.cm` pins exact bytes; omit `hash`
  and the resolver picks the current revision
  ([packages](https://fuchsia.dev/fuchsia-src/concepts/packages/package),
  [component identifiers](https://fuchsia.dev/fuchsia-src/concepts/components/v2/identifiers)).
- **asdf and rustup.** Versions install side by side behind a shim or proxy. asdf reads
  `.tool-versions`, walking up from the working directory
  ([asdf](https://asdf-vm.com/manage/versions.html)). rustup takes `+toolchain`, then an
  environment variable, then a directory override, then `rust-toolchain.toml` walking up, then the
  default ([overrides](https://rust-lang.github.io/rustup/overrides.html)). Both are search orders.
- **Capability routing.** A Genode child asks for its binary as a ROM session by label, and the
  parent's `<route>` decides which file answers, per child, so two children asking for one name
  can get two files
  ([init](https://genode.org/documentation/genode-foundations/26.05/system_configuration/The_init_component.html)).
  Plan 9's `bind` puts a different binary at `/bin/prog` in each process's own namespace
  ([bind(1)](https://9p.io/magic/man2html/1/bind)). Neither has a global answer to "which
  version": the caller holds what its parent gave it.

## Options

| option | what it is | verdict |
| --- | --- | --- |
| A. The version goes in the name | `python@3.12`, Debian's `python3.11`: a second package with a second program name | Refused as the answer, kept as a fallback. It is what calef called the limitation; `uptime` at two versions becomes two programs |
| B. One active version, switched | Debian alternatives, `brew link`: many installed, one live at a time, chosen system-wide | Refused. Two consumers needing different versions at once is the whole ask |
| C. A file in the working directory picks the version | asdf's `.tool-versions`, rustup's `rust-toolchain.toml`, both behind a shim | Refused. It is a search order keyed on ambient state, which §229 ruled out, and the directory rather than a grant decides what runs |
| D. The table admits every installed version | Nix's and Fuchsia's arrangement: a version is identified by its content, and many can be live | **Recommended, as the table half.** Install of a new version appends; the bare name keeps one entry, the default |
| E. The caller is granted the version it needs | Genode routes a child's binary by label; Plan 9 binds a per-process `/bin`; Fuchsia routes a capability to a component pinned by hash | **Recommended, as the selection half.** In a capability system "which version" is "which file the caller holds", and §219 already made the spawner send bytes rather than a name |

**The recommendation is D with E.** The table stops forgetting a version when another is installed.
The bare word still means one version, the default, so §229 is untouched. A specific version is
reached by path, or by a directory capability that a project, a session or a package's recipe is
granted, in which `uptime` means 0.1.0. No search order and no ambient lookup is added: the
answer falls out of the model rather than fighting it. D alone would leave a developer typing store
paths. E alone cannot work, because the progenitor would refuse the old version's digest.

Would it still be chosen at equal cost? Yes. A is cheaper and is refused on what it does to names,
not on effort.

## Open questions for an architect

None of these is decided here, and a building lane must not decide them.

1. **How a version is named in a grant.** By its version string (`uptime/0.1.0`), by its digest
   (Fuchsia's `?hash=`), or both, with the digest authoritative. A string is readable; a digest is
   what the progenitor already checks.
2. **The on-disk and table format.** Does a generation line gain a version field, or does the table
   key on digest with the name as a column? Either changes a format the progenitor and the host
   tool both read (§208's generation file), which is a wire decision.
3. **Which version the bare word means**, and who changes it: the newest install, the first, or an
   explicit `package default` step (name provisional). Today it is implicitly the last install.
4. **What a per-project selection is called and what it holds.** A directory capability, a line in
   a recipe, or a session's grant. This is the one new name the milestone mints.
5. **Remove and rollback semantics** when several versions are live: does `package remove uptime`
   take one version or all, and what a rollback restores.

## Dependencies

- Milestone 198 (a package manager, and the trivial install), PARTIAL: install, remove, rollback
  and the activation set this changes.
- Milestone 47 (navigation and naming), pull request #1374: the bare-name resolution and install
  refusals this must keep.
- The proposal
  [install-time-signature-verification.md](proposals/install-time-signature-verification.md)
  (§220). Not a hard dependency, since two versions can be proved with two catalogue entries, but
  without it a developer cannot install a second version without a new boot slot.

## What this unblocks

- A developer running two projects that need different versions of one program or service at once,
  which is calef's case.
- Staged upgrades: install the new version beside the old, move one consumer, then the rest. §208's
  live-swap intent (*"packages that can live load to replace an older version and then roll back
  if it breaks"*) wants the old
  version still runnable while the new one is tried.

## Done means

- Host tests in `activation_set`: installing 0.2.0 over 0.1.0 leaves both digests live; the bare
  name resolves to whichever version the ruled default names; removing one version leaves the
  other; a rollback restores the whole set, as `a_rollback_restores_the_whole_set` does today.
- A `script/swish-check` transcript on aarch64, riscv64 and x86_64 per §19 (architectural parity is
  a tenet): install two versions of one fixture, run each by path and by the ruled grant in one
  session, and show the bare word running the default.
- The names and the format above ratified by an architect, recorded in `design/decisions/` by the
  integrator.

## BUGS

- Services, not only programs, will want two versions at once (two versions of a file service
  behind two projects). This milestone covers programs a spawner runs. A running service at two
  versions is the same table change plus two endpoints, and is left to the building lane to
  scope or split.

## Index row

Two versions of one program cannot be installed together: the activation set holds one entry per
program name, and the progenitor refuses bytes whose digest has left it, so an upgrade makes the
old version unrunnable even by path. The recommendation is the Nix and Fuchsia table (every
installed version stays live, identified by content) with the capability answer to selection: a
caller is granted the version it needs, and the bare word keeps one default. How a version is
named in a grant, the table format and the default rule are an architect's calls.
