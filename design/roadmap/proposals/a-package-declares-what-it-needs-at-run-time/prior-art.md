# How other systems package software across languages

An appendix of [a package declares what it needs at run time](../a-package-declares-what-it-needs-at-run-time.md),
written 2026-10-07 (UTC) on calef's request: *"I would like prior art as part of that review."* It
is a separate file because the table would put the proposal over §212 (a prose budget)'s cap.

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
| Android | an APK: `AndroidManifest.xml` beside `classes.dex`, signed together (manifest read; the signing detail unverified) | the manifest: `<uses-permission>`, `<uses-feature android:required>` and the minimum SDK level (read: app manifest overview) | implicit: every APK runs on ART | Take the manifest file beside the bytecode, fork A's second carrier, and the SDK level for versioning, fork 5. Refuse permissions granted by a prompt. |
| Fuchsia | a package holding binaries and compiled `.cml` manifests | `use` in the component manifest, satisfied by an `offer` and `expose` route (read: capabilities) | `program: { runner: "elf" }`; a runner is a capability a component can serve (read: runners) | Take nearly all of it: `runner` for fork B, `expose` for fork 2. Refuse routing through a component tree nife does not have. |
| Genode | a `pkg` depot archive listing `src`, `raw` and other `pkg` archives that belong together at run time (read: Genode Foundations, package management) | the `pkg`'s `runtime` file: `requires` and `provides` services, plus `ram`, `caps`, `binary`, `config` and the ROM modules it needs. `pkg/terminal/runtime` requires `gui` and provides `terminal` (read: `genodelabs/genode`, `repos/gems/recipes/pkg/terminal/runtime`) | the `binary`; an interpreter ships as its own archives and a script as a ROM module (unverified) | Take most of all. A per-package service `requires` and `provides`, computed into what a deployer reads, is this proposal. Refuse writing `requires` by hand beside a binary that already declares it, which is what fork 1 ruled out. |
| Redox | a `pkgar` built from a cookbook `recipe.toml`, whose build template may be `cargo`, `configure` or `python` (read: porting applications) | `package.dependencies`, installed with the package, beside `build.dependencies` (read) | none beyond the dependency | Take the split between build and run dependencies. Refuse: it names packages, not services, which is the gap this proposal starts from. |
| Haiku | an `.hpkg` | `requires` and `provides` over typed resolvables, `lib:`, `cmd:`, `app:` and `add-on:`, with versions (read: building packages) | a resolvable such as `cmd:python3` | Take typed resolvables: a contract and a runner are two types. Refuse `cmd:` paths as the runner's name. |
| WASI and the component model | a Wasm module or component | a WIT world's imports and exports; a component "cannot access" what it does not import (read: component model, worlds) | the host runtime, such as Wasmtime (unverified) | Take the world as the closest match to a nife manifest, and one WASI runtime as a runner for many languages. Refuse its ABI as nife's own: §84 (how we port) claims shape, not ABI. |
| OCI images | layers plus a config | `Entrypoint`, `Cmd`, `Env`, `ExposedPorts`, `Volumes`, `User`, and no required services or capabilities (read: image-spec `config.md`) | whatever the entrypoint names inside the image | Refuse as a model for needs: an image ships its whole userland and declares nothing. It is only a reference for layering. |

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
