# How other systems package software across languages

An appendix of [a package declares what it needs at run time](../0849-a-package-declares-what-it-needs-at-run-time.md),
written 2026-10-07 (UTC) on calef's request: *"I would like prior art as part of that review."* It
is a separate file because the table would put the proposal over §212 (a prose budget)'s cap.

## The package-level options

The main text's options A to F. Prior art was read where a URL is given; the rest is recalled.

| Option | Prior art | Rung |
|---|---|---|
| A. A comment in each package file | none | 3 |
| B. Hand-declared `requires` and `provides` per package | Debian: a dependency "may be satisfied by ... any other concrete package which provides the virtual package" ([policy ch. 7](https://www.debian.org/doc/debian-policy/ch-relationships.html)) | 2 for consistency, 3 for truth |
| C. Unit ordering | systemd: "requirement dependencies do not influence the order", which `After=` sets ([systemd.unit(5)](https://man7.org/linux/man-pages/man5/systemd.unit.5.html)) | none |
| D. Derive package needs from what binaries declare | Nix closures, found by scanning outputs for store paths (recalled) | 2, cannot drift |
| E. `use`, `offer`, `expose`, routes verified before run | Fuchsia: "there must also be a valid capability route from the consuming component to a provider" ([capabilities](https://fuchsia.dev/fuchsia-src/concepts/components/v2/capabilities)); `scrutiny` checks routes over an image (recalled) | 1 in tree, 2 at pack and install |
| F. A static assembly the compiler wires | seL4 CAmkES: `provides` and `uses`, connected in an assembly, glue generated before run time ([manual](https://docs.sel4.systems/projects/camkes/manual.html)) | 1 |

## Across languages

Each row covers a program that is not native code. It says how one is packaged, where its run-time
needs are written, and how its interpreter or runtime is named. The last column says what nife
should take or refuse. "Read" names
the primary source this lane read on 2026-10-06 or 2026-10-07. "Unverified" means recalled and not
reread, the way §197 (a package is one archive file) marks it.

| System | A non-native program is packaged as | Run-time needs declared in | Interpreter or runtime named by | Take or refuse |
|---|---|---|---|---|
| Debian | an ordinary `.deb` holding the scripts | `Depends`, partly generated: `dh_python3` fills `${python3:Depends}` from `Requires-Dist` and from shebangs, and `dpkg-shlibdeps` fills `${shlibs:Depends}` from each ELF's `NEEDED` (read: `dh_python3(1)`, policy ch. 8) | a rewritten shebang, `/usr/bin/python3` | Take the generation: needs computed from what the artifact says, which fork 1 ruled. Refuse the path as the name of a runner. |
| Nix and Guix | a derivation; `buildPythonPackage` builds a Python one | nowhere by hand: the closure is every store path the output references (unverified) | a shebang patched to the interpreter's store path (unverified) | Take "derived, so it cannot drift". Refuse scanning bytes for hashes, since nife has a declared note to read instead. |
| Flatpak | an app on exactly one runtime (read: available runtimes) | the runtime in the app manifest; SDK extensions such as `openjdk11` and `golang` are for the build only (read: extensions); sandbox holes as `finish-args` (unverified) | the runtime's id and version | Take: a runtime is a runner a package names. Refuse one runtime per app; a nife program names its own runner. |
| Snap | a snap on one base snap (unverified) | interface plugs, connected to slots another snap or the system provides (unverified; the docs sat behind a login) | the base, plus the language plugin used at build (unverified) | Take plugs and slots: they are `use` and `provides` under other names. Refuse auto-connection by store policy as the grant. |
| Android | an APK: `AndroidManifest.xml` beside `classes.dex`, signed together (manifest read; the signing detail unverified) | the manifest: `<uses-permission>`, `<uses-feature android:required>` and the minimum SDK level (read: app manifest overview) | implicit: every APK runs on ART | Take the SDK level for versioning, fork 5, and the manifest file only as fork A's last resort. Refuse permissions granted by a prompt. |
| Fuchsia | a package holding binaries and compiled `.cml` manifests | `use` in the component manifest, satisfied by an `offer` and `expose` route (read: capabilities) | `program: { runner: "elf" }`; a runner is a capability a component can serve (read: runners) | Take nearly all of it: `runner` for fork B, `expose` for fork 2. Refuse routing through a component tree nife does not have. |
| Genode | a `pkg` depot archive listing `src`, `raw` and other `pkg` archives that belong together at run time (read: Genode Foundations, package management) | the `pkg`'s `runtime` file: `requires` and `provides` services, plus `ram`, `caps`, `binary`, `config` and the ROM modules it needs. `pkg/terminal/runtime` requires `gui` and provides `terminal` (read: `genodelabs/genode`, `repos/gems/recipes/pkg/terminal/runtime`) | the `binary`; an interpreter ships as its own archives and a script as a ROM module (unverified) | Take most of all. A per-package service `requires` and `provides`, computed into what a deployer reads, is this proposal. Refuse writing `requires` by hand beside a binary that already declares it, which is what fork 1 ruled out. |
| Redox | a `pkgar` built from a cookbook `recipe.toml`, whose build template may be `cargo`, `configure` or `python` (read: porting applications) | `package.dependencies`, installed with the package, beside `build.dependencies` (read) | none beyond the dependency | Take the split between build and run dependencies. Refuse: it names packages, not services, which is the gap this proposal starts from. |
| Haiku | an `.hpkg` | `requires` and `provides` over typed resolvables, `lib:`, `cmd:`, `app:` and `add-on:`, with versions (read: building packages) | a resolvable such as `cmd:python3` | Take typed resolvables: a contract and a runner are two types. Refuse `cmd:` paths as the runner's name. |
| WASI and the component model | a Wasm module or component | a WIT world's imports and exports; a component "cannot access" what it does not import (read: component model, worlds) | the host runtime, such as Wasmtime (unverified) | Take the world as the closest match to a nife manifest, and one WASI runtime as a runner for many languages. Refuse its ABI as nife's own: §84 (how we port) claims shape, not ABI. |
| OCI images | layers plus a config | `Entrypoint`, `Cmd`, `Env`, `ExposedPorts`, `Volumes`, `User`, and no required services or capabilities (read: image-spec `config.md`) | whatever the entrypoint names inside the image | Refuse as a model for needs: an image ships its whole userland and declares nothing. It is only a reference for layering. |

## A manifest inside the program

For fork A as calef revised it on 2026-10-07 (UTC): systems that put metadata inside the file it
describes, in that format's own slot.

| System | Slot | Format | Read by | Take or refuse |
|---|---|---|---|---|
| Python, PEP 723 (Final) | a comment block, `# /// script` to `# ///`, typed by the word after `///` | TOML, with each line's leading `# ` removed (read: PEP 723) | uv (read: uv's scripts guide); pipx (unverified) | Take the block shape and its type word, so `# /// nife` sits beside `# /// script`. |
| Rust, RFC 3502 and RFC 3503 | frontmatter right after `#!`, between `---` fences, with an optional infostring such as `cargo` | TOML, a Cargo manifest (read: RFC 3503) | Cargo | Take "right after `#!`". Refuse the fences, which are Rust syntax and not a comment elsewhere. |
| `nix-shell` | extra `#! nix-shell` lines after the first, allowed anywhere, even inside block comments (read: `nix-shell` manual) | command-line flags | `nix-shell` | Take the proof that a second header survives languages whose comment is not `#`. Refuse flags as the encoding. |
| Deno | the shebang itself: `#!/usr/bin/env -S deno run --allow-env` (read: Deno's executable scripts example) | permission flags on the command line | the kernel's `#!` handling, then Deno | Take grants written in the script. Refuse the shebang as the carrier: it depends on `env -S` and names a path. |
| WebAssembly | a custom section, id 0, a name and uninterpreted bytes, "ignored by the WebAssembly semantics" (read: core spec, binary modules) | any bytes | whatever names the section | Take: a section named `nife.manifest` holding the note's binary descriptor. |
| Java jar | `META-INF/MANIFEST.MF`, `name: value` headers such as `Main-Class` (read: JAR file specification) | text headers; a signed jar's `.SF` file covers the manifest's digest (read) | the launcher, `java -jar` | Take `META-INF/` as the slot. Refuse adding keys to `MANIFEST.MF` itself; nife's goes in its own entry. |

## What the rows agree on

Three things, each already in the proposal.

- Needs are generated from the artifact where they can be: Debian's substitution variables, Nix's
  closures and Genode's deployer all compute them. That is fork 1.
- A runtime is named and resolved like a dependency, apart from libraries: Flatpak's runtime,
  Fuchsia's runner and Haiku's `cmd:` resolvable. That is fork B.
- The two systems built on capabilities, Fuchsia and Genode, name services rather than packages on
  both sides. That is fork 2.

Genode is the closest match and worth reading before milestone work starts. Its `runtime` file
already sits at the package level, holds `requires` and `provides` of services, and is what its
deployer routes from. The difference this proposal argues for is that nife writes the need once,
in the program's manifest, and builds the package-level record from it.

## An ad hoc script at the prompt, prior art for the trial

All read, for the `BUGS` entry's open usability risk: an installed program comes through `jig`
with its block reviewed, while an ad hoc script typed at the shell runs on a subset of the shell's
authority with no block.

- Deno prompts at run time for a permission no `--allow-*` flag granted.
- Android asks in a dialog for a manifest-declared permission when it is used.
- `ffx component run` starts a component in the `ffx-laboratory` collection, whose narrowed
  capabilities are unverified.
