# Fetching a package by name

The appendix to [notes/packages.md](../packages.md) for milestone 198 (a package manager) rung
3a's fetch, built 2026-09-26. An operand with no `/` is a name, by the prompt's rule for a command
word:

```
$ package install nosuch
  refused: this image's catalogue names no such package, so nothing was fetched; generation 1 is live
$ package install uptime
  refused: this image's catalogue does not vouch for those bytes; generation 1 is live
$ package install greeting
  fetched and installed; generation 2 is live
$ packages/greeting/0.1.0/greeting
  hello from a package this image never carried
```

## How the progenitor fetches

The request is `spawnproto::Activation::Fetch` (provisional), and `fetch` in
`crates/system_initializer` serves it. It asks the image's catalogue first
(`package_archive::catalogued_stem`). A name the image vouches for nothing by costs no network.

Then it splits one page from a region of its own and hands it to the stack it built at boot. Over
that page it sends `GET /<stem>.nifepkg` to the package source and reads the reply with
`http_response`. The body lands in the staging window a file install uses (`receive_image`'s).
From there it is an ordinary install with one more check: the package must be the one asked for
(`package_archive::installable_as`). A source can serve a *different* package the catalogue also
vouches for. Destroying the socket page's region at the end revokes it out of the stack.

## Why the progenitor, and what it costs

The alternative was a fetching program at the prompt. It would declare `network` and write the
package to a file for `package install <file>`. It lost because nothing can tell a program *which*
package: there is no argument vector (milestone 205 (how a foreign program is told what to do)).
That is the installer's own reason.

Were both equally possible, the program would be the better shape. It would keep a parser of
network input out of the progenitor. As built, `http_response`'s head reader runs there before any
digest is checked: a fixed 2 KiB buffer, host-tested and fuzzed but not proved. `package_archive::Package::parse`
already ran there on unvouched bytes, so this is the second such parser. The choice is reversible:
once milestone 205 lands, a program can take the fetch and the progenitor keeps the install.

## A program no image carries

`greeting` (`fixtures/src/greeting.rs`) prints one line. `fixtures/Cargo.toml` lists it as
`packaged_only`, which `xtask`'s `declared_programs` reads. So it is built with every fixture and
packed by no archive, and `packages/greeting*.recipe.toml` package it for all three architectures.

The prompt cannot show its absence. It is no `grant_plan::Prog`, so its bare name is refused either
way. `script/swish-check` reads the archive on the host before the boot instead, and stops if the
archive has it.

## The gate starts the package source

`script/swish-check` fills `target/package-source-<arch>/` with two packages. One is `greeting`.
The other is the tampered `uptime` the disk also gets, served under the genuine name. The gate
points `helpers/package-http-peer` there with `NIFE_PACKAGE_SOURCE`, which QEMU passes to the peer
it starts per connection. So a lying mirror is one line: a well-formed exchange of a well-formed
package that only the catalogue can refuse.

x86_64 has no NIC. It installs the same `greeting` package from the disk
(`downloads/greeting.nifepkg`) and omits the two fetch lines, each with its reason in
`swish_check_omits`. After the reboot, removing `uptime` leaves `greeting` running.

## What proves it

Green on aarch64, riscv64 and x86_64 (OVMF) on 2026-09-26. Each change below was made once on
aarch64, and each turned its line red:

- skip the catalogue lookup: `nosuch` reaches the source and gets a 404;
- skip the fetched package's digest check: the lying `uptime` installs;
- drop the body: `greeting` is refused;
- place the program with a byte flipped: it is refused when run;
- make `remove` drop every program: `greeting` is refused after the reboot;
- give the second boot a fresh disk: every line after the reboot fails;
- empty `packaged_only`: the seed refuses the archive.

On x86_64, seeding no `greeting` package failed its disk line. A first line, `greeting` typed bare
and expecting "no such program", stayed green with `packaged_only` emptied. That is how the archive
check replaced it.
