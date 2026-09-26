# What a nife package is, and what still cannot be done with one

Milestone 198 (a package manager, and the trivial install that makes a second customer possible)'s
rung 3a has two halves. This note is the producer half and the format both halves share, built
2026-09-23, and the first part of the consumer half, built 2026-09-24: a target fetches a package
over plain HTTP and accepts it only by a digest its own image vouches for. Since 2026-09-26 an
installed program runs by its bytes (DECISIONS §219 option D), and the target installs, removes and
rolls back packages itself, fetched by name. What is missing is in "Where this stops" below.

## The two decisions this is downstream of

DECISIONS §197 (a package is one archive file), in
[its own file](../design/decisions/197-a-package-is-one-archive-file.md), ruled the container: one archive file per package, identified by name and version, the shape
`.deb`, `.apk` and `.hpkg` all use. It accepted a cost out loud while doing it, because a container
is bytes a target must parse: *"the reader owes a fuzz target and the Kani treatment `crates/nifefs`
and `crates/elf` already carry."*

DECISIONS §195 (a reviewed recipe vouches for a package), in
[its own file](../design/decisions/195-a-recipe-vouches-and-the-owner-may-overrule.md), ruled trust: a package's digest lives in a version-controlled recipe
changed by human review, trust is scoped per source the owner opted into, and the owner may
overrule. That is Homebrew's arrangement, and Homebrew is also §197's worked example of the pairing.

Those two together say what a package *is* and what decides whether its bytes may run. What
installing one does was the third fork, and DECISIONS §208 (installing a package is granting it, and
the activation set is versioned) ruled it on 2026-09-23.

## The format

`crates/package_archive` holds it, and holds it once: the host tool writes a package with the same
code a target reads one with, which is the arrangement `crates/nifefs` has for the boot archive.
The crate's own documentation carries the byte layout and the reasons; three choices are worth
repeating here because a reader comparing this against `.hpkg` will ask about them.

- **Offsets are absolute**, so a reader never has to know where the table of contents ends to find
  data. `nifefs` made the same call about `start_block`.
- **Every member carries its own SHA-256**, where `.hpkg` carries none and keeps digests in the
  repository index. §195 puts a digest over the whole file in the recipe, which answers *are these
  the reviewed bytes*; the per-member digest answers *is this member the one that was reviewed*,
  which is what a spawner handed one program out of a package needs to ask. It is also the shape
  `crates/measured_boot`'s table already has, so a package's table of contents is that table
  travelling with its bytes.
- **Member bytes are 8-byte aligned.** The largest member is an ELF, and the cheapest way to be
  wrong later is to hand a parser an odd address.

The encoding is provisional and the crate's name is provisional. §197 ruled the container, not
these offsets. Its manifest question was ruled 2026-09-26: an ELF note inside the executable, not
a member. Whether the digest is a Merkle root is still open; this takes the plain SHA-256 §197
records as the default. §197 says the day somebody outside this repository fetches a package
is the day the format is fixed; nobody has.

## The producer

`cargo xtask package <recipe>` turns a reviewed recipe into one package file, a digest, and a
catalogue line. `packages/uptime.recipe` is the worked example and a real recipe rather than a
fixture.

```
$ cargo build -p components --bin uptime --target aarch64-unknown-none-softfloat
$ cargo xtask package packages/uptime.recipe
uptime 0.1.0 aarch64, 2 members, 90491 bytes
  uptime                       89168 bytes  d801cd2b65dbfbd4982226394c8ad5de471d4f782f39eb16c5ee06cd71bcef26
  uptime.licence                1067 bytes  dba2f854c33606c0a4f028f88baf9d9bcef8d30714e29c8f3314dd5eeebf1c14
digest d74f8eb3ecc14b43b9f55b2113c830ee856394eee4398b52da2da1614d426cd9  (the recipe records none; review it and add it)
wrote target/packages/uptime-0.1.0-aarch64.nifepkg
package: PASS
```

A program is packed stripped since 2026-09-24, the same bytes the image packs. The first run
above read 881,152 bytes for `uptime`, nearly all of it debug sections, against a job region of 40
pages (160 KiB): an installed program is bytes something on the target must read into memory to
build a process from, and nothing there reads DWARF. The digests in this note's history changed
with it; nobody had fetched one.

Three things in that run are the mechanism rather than decoration.

It reads its own output back with the target's parser before writing anything to disk. A
producer that could emit a file its consumer refuses would ship one, and the gate that would catch
it does not exist yet on the target side.

A recipe's recorded digest is checked before anything is written. A rebuild that does not
reproduce the reviewed line is exactly the failure §195's arrangement exists to make visible, so the
tool prints both digests and writes nothing. That ordering costs a rebuild to discover and is worth
it: a package nothing accepts, sitting on disk beside a catalogue entry vouching for it, would be
the tool disagreeing with itself.

The catalogue line is `measured_boot`'s manifest shape, a name, a space, 64 hex characters,
which is what the progenitor already reads to decide whether a program may run. §195 makes the
image's measurement table the first source of trust, so a package's entry looking like an entry in
that table is the point.

The bytes are a function of the inputs alone: no timestamp, no ordering pass, no non-zero
padding. A reviewed digest is worth nothing if two hosts building the same recipe disagree, and
`the_same_inputs_give_the_same_bytes` is the host test that says so.

`packages/uptime.recipe` deliberately records no digest, and the comment in it says why: the
program it names is rebuilt by this checkout whenever anything it links changes, so a recorded
digest would be a number that fails for the next reader. A recorded number that is wrong is worse
than an absent one. The line goes in when there is a release to pin it to, which is rung 4.

What proves the producer, and what running and installing cost, is in
[the appendix](packages/measurements.md).

## The consumer's first half: fetch, and verify against the image

Built 2026-09-24 by the rung 3a consumer lane. Three pieces, each doing one thing:

- The image carries its own package source. Every archive build (`cargo xtask initrd-aarch64`
  and `initrd-riscv`) runs every recipe under `packages/` for its architecture, writes the package
  to `target/packages/`, and packs the catalogue lines as the archive entry
  `package_archive::CATALOGUE` (provisional name), above the measurement table. So the kernel's
  trust root vouches for the catalogue, and the catalogue vouches for the package. That is §195's
  "the image's measured table becomes the first source" taken literally, and it is why plain HTTP
  is enough on this rung: the digest the client checks against never crossed the network. It also
  means the producer runs end to end on every build, which this note's BUGS said nothing did.
- A host on the network serves it. `helpers/package-http-peer` is a `guestfwd` peer at
  10.0.2.9:8080 in both QEMU runners, started by slirp once per connection with the connection on
  its standard input and output, exactly as the TCP echo peer at 10.0.2.9:7777 is a `/bin/cat`. A
  real HTTP/1.0 exchange with a real host process, and nothing binds a port on the machine or
  outlives QEMU. `GET /tampered/<name>` serves the same file with one byte flipped halfway through.
- The client hashes as it reads. `crates/http_response` (provisional name) writes the `GET` and
  reads the response a socket read at a time, keeping only the head, so the body goes straight
  into `measured_boot`'s streaming SHA-256 and a package costs the client one page of socket
  frame. The client is a mode of `net_stack`'s socket-contract client (`TEST_HTTP_PACKAGE`), because
  that is the only thing in the tree that holds a `Stack` capability.

### EXAMPLES

```
$ script/test --arch aarch64 --test package
running 1 of 359 tests (filter: package)
test kernel::user::tests::a_package_fetched_over_http_is_accepted_only_by_the_image_digest ... ok
test result: ok. 1 passed
$ cat target/packages/catalogue
uptime-0.1.0-aarch64 d74f8eb3ecc14b43b9f55b2113c830ee856394eee4398b52da2da1614d426cd9
```

The test fetches twice through one `net_stack`: the genuine package, which must be accepted, then
the tampered copy, which must be refused. The second fetch is what gives the first its meaning. The
tampered response is a complete, correct HTTP exchange of the right length; only the digest can
tell, and a client that accepted whatever arrived would pass the first half too. They share one
spawn because every `net_stack` a test starts holds a virtio slot for the rest of the boot; this
lane took the table's tenth bump (`MAX_DEVICES`, to 34) and filed the unregister it keeps deferring
as `design/roadmap/proposals/a-virtio-slot-comes-back-when-its-driver-dies.md`. riscv64 runs the
same test as a twin in `kernel/src/user/riscv_virtio_tests.rs`.

### The versioned table, as logic

`crates/activation_set` (provisional name) is §208's second clause as a pure, host-tested crate: a
generation is a text file of `<program> <package> <digest>` lines that is never rewritten, a
one-line `current` names the live one, and install, upgrade and remove each produce the next
generation. Its test `a_rollback_restores_the_whole_set` is the property calef asked for by name.
The progenitor reads and writes it on the target (below).

## Running what was installed, by its bytes

DECISIONS §219 (how the shell names an installed program to the spawner) was ruled on 2026-09-26:
option D, the executable's bytes as frames the caller owns, with gate D2. Both are built.

A command word with a `/` in it is a file. The shell binds the line against the manifest the file
carries as an ELF note, or `uptime`'s if it has none: milestone 597 (a program carries its manifest
in an ELF note). It opens the file and sends `spawnproto::request(len, ..)` with `IMAGE_BIT` set: word 0 is the byte length, then
one `SEND_CAP` per page narrowed to `READ`, then the grants as today, holding one frame at a time.

The progenitor maps each frame through the loader's never-reused scratch window, copies it into a
page of its own, and deletes the capability before taking the next. It copies because the caller
keeps a mapping of its frames and could change them between a hash and a build. It hashes the copy,
reads `activation/current` and then that generation through the file service it already held, and
looks the digest up (`activation_set::lookup_digest`). A hit is built from the copy and endowed from
its note. A miss runs only for a session presenting D2's capability, with its grants and two pages;
otherwise it gets `SPAWN_UNVOUCHED`. D2 is [packages/running-unvouched.md](packages/running-unvouched.md).

The digest is the member's, not the package's: the spawner is handed the executable, and the
package's table of contents already carries each member's digest. The recipe's digest over the whole file (§195 (a reviewed recipe vouches for a package))
is still what installing checks first; the activation table records the member's.

## Installing on the target

Built 2026-09-26. Each of three words is one request to the progenitor and one reply naming the
live generation:

```
$ package install downloads/tampered.nifepkg
  refused: this image's catalogue does not vouch for those bytes; nothing is installed
$ package install downloads/uptime.nifepkg
  installed; generation 1 is live
$ packages/uptime/0.1.0/uptime
  up 00:00:06
$ installed/unvouched
  network: refused (no capability at slot 10)
  entropy: refused (no capability at slot 9)
  domain: refused (no capability at slot 7)
  slots held: 0 1 2
```

And on the next boot, from the same disk:

```
$ packages/uptime/0.1.0/uptime
  up 00:00:01
$ package remove uptime
  removed; generation 2 is live
$ caps packages/uptime/0.1.0/uptime
  packages/uptime/0.1.0/uptime would grant the new process, and nothing else:
    ...
    provenance: unvouched (digest ...)
    runs on this session's capability to run unvouched bytes (slot 22)
$ package rollback
  rolled back; generation 1 is live
$ packages/uptime/0.1.0/uptime
  up 00:00:01
```

The progenitor is the installer, not a program, for §208's own reason: the authority that
decides which version is active should be the one that performs a swap, and §219 already made it
the reader of the table. It holds the file service with `WRITE`, the image's catalogue in its
archive, and the frame-staging path an image request built. A program would need all three
delegated, and an argument vector it does not have (milestone 205 (how a foreign program is told
what to do)). `spawnproto::ACTIVATION_BIT` (provisional) is the request.

Install stages the package exactly as an image is staged, so the progenitor checks its own
copy. `package_archive::installable` is the whole decision on bytes, host-tested: the file's digest
must be the image catalogue's line for the stem in its header, and the member named after the
package is the program. Its bytes go to `packages/<name>/<version>/<program>`, one component per
field because a prompt component is at most sixteen bytes. Remove writes a generation without
the entry and leaves the bytes, which is what lets a rollback bring them back. Rollback points
`current` at the generation one below the live one and writes nothing else.

All three end in one commit order: the new generation is created and written whole, `current` is
written to `current.next` and renamed over `current`, and the device is synced before the reply. A
generation is never rewritten, and `current` never names one that was not written whole.

### What proves it

`script/swish-check` boots twice per architecture against one disk, typing the first transcript
above and then the second (`SWISH_CHECK_AFTER_REBOOT`). The host puts a package on the disk and
installs nothing (`seed_installed` in `xtask/src/disk.rs`). The tampered copy has one program byte
flipped and its table of contents rewritten to agree, so the catalogue is the only thing that can
refuse it. Green on aarch64, riscv64 and x86_64 (under OVMF) on 2026-09-26. Each line was falsified
once on aarch64:

- skip the catalogue check, and the tampered package installs;
- record the program's digest with one bit flipped, and the installed program is refused;
- give the second boot a fresh disk, and every line after the reboot fails;
- make `remove` rewrite the old table, and the removed program still runs;
- make `rollback` stay on the live generation, and the last line is refused.

A first tampered copy that only flipped a byte stayed refused with the catalogue check skipped:
the program's own digest caught it.

## Fetching by name, and a program the image never carried

`package install <name>` fetches over the network and installs, and `greeting`, which no archive
packs, is what the gate fetches: [packages/fetching.md](packages/fetching.md).

## Where this stops

Rung 3a's exit criterion (fetched, verified, installed, run, kept across a reboot, rolled back,
removed) is met, and so is §219's gate D2. The boot prompt is the owner's console, which may write
`activation/` and vouch for a build: [packages/vouching.md](packages/vouching.md).

## BUGS

- The boot prompt can write `activation/` directly, through the root endpoint the progenitor
  writes through. §221 (the boot prompt is the owner's console) ruled that is the owner's right.
- Whoever holds the spawn endpoint (only the boot prompt) may install, remove, roll back and vouch.
- A vouch, the owner's or a package's, now grants whatever the bytes' own note asks, the network
  included (milestone 597 (a program carries its manifest in an ELF note)).
- Nothing collects `packages/`. A removed program's bytes stay, which is what rollback needs.
- Rollback is by number, to the generation one below the live one, as Nix's is (recalled, not
  read). Undoing a rollback is another install.
- The member named after the package is the program, by the installer's convention; the format
  marks no member executable. A package whose program has another name installs nothing.
- The `SYNC` before the reply is not falsified. QEMU keeps a killed guest's writes, so no gate
  here can lose a generation that was not synced.
- A note may declare only what an image request can carry (`grant_plan::image_can_carry`): no
  file, directory, input, option or `std` runtime yet. Milestone 597's BUGS has the list.
- Only a plain line runs an image. A path in a pipe or behind a redirection reaches the planner as
  a program name and is refused as "no such program", and `caps <path>` prints no `provenance:`
  row. `crates/grant_plan/src/spawnproto.rs`'s `BUGS` has the full list.
- The package source is compiled in: the runners' peer at 10.0.2.9:8080
  (`socket_protocol::fixture`). A booted system outside QEMU has no source to fetch from, and §195's
  per-source trust needs a way to name one.
- The progenitor fetches on socket 5 by convention: every client of the stack shares its socket
  numbers (milestone 590 (the booted system starts its network stack)'s BUGS). Another network
  program can fail a fetch, not pass one; the digest decides.
- The progenitor serves nothing else while it fetches, and a slow source makes the prompt wait.
- An HTTP reader runs in the progenitor before the digest check (above). It is fuzzed, not
  proved; `crates/http_response`'s BUGS says why.

- **No compression.** `.hpkg` chunks its heap with zlib and `.apk` is three gzip streams; this
  stores members whole. The first packages are ELFs that were about to be written to a disk anyway,
  and a compressor is a second hostile-input parser on the same path. The size cost is measured
  nowhere.
- **A package is bounded by `u32`** in both member length and file length.
- The catalogue is one file in `target/` and one archive entry, not a repository index. The
  image's own source is the only source; §195's per-source trust needs a catalogue per source the
  owner opted into, and a way to add one.
- x86_64 fetches nothing: its QEMU runner attaches no `-netdev`. It installs `greeting` from the
  disk instead. Milestone 494 (a driver for the network card a PC actually has) is where x86
  networking starts.
- `greeting` proves the path, not a useful program: it prints one line, with `uptime`'s manifest.
- The package peer is a `guestfwd` process, not a server on a LAN. It speaks HTTP to the guest
  over slirp's forwarding, which is enough to prove the client and not enough to prove a real
  network card or a host elsewhere on a network (rung 3b).
- Plain HTTP carries the package, and that is safe only because of the image's catalogue. A
  source whose digests arrive over the same connection would be worth nothing against a machine in
  the middle; that is what §196 (nife carries TLS)'s TLS is for on rung 3c.
- **A recipe cannot say where its source came from.** Homebrew's formula carries an upstream URL and
  a digest of the tarball; this carries neither, because every package that exists is built from
  this repository. The first out-of-tree package is what forces it.
- **`cargo xtask package` builds nothing.** A `program` whose ELF is not in `target/` is an error
  naming the file. Packaging and building are separate acts here for milestone 150 (adding a program should not need eight hand-maintained lists)'s reason: a tool
  that quietly rebuilt would hide which binary it had packed.
