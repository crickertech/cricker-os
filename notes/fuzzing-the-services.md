# Fuzzing the services' request handlers

*Built by `lane/fuzz-service-handlers` on 2026-10-04 (UTC), part (a) of the proposal on #1592,
which calef ruled yes the same day. Every target, helper and module name here is provisional.*

`notes/fuzzing.md` covers the six parser targets. This note covers the three that replay a
session of requests against a service built on the host, and check a stated rule after every
reply. A confinement breach answers a request with success and crashes nothing, so a target that
only waits for a panic would miss the class it exists for.

## The targets

- `redoxfs_server_session` drives `Server::handle`, the file server's dispatch. A revoked or
  unknown badge succeeds at nothing. A bound badge acts only through `ROOT` or its own handles, and
  reads nothing outside its grant.
- `system_log_session` drives `Log::handle` and `Log::fill`. Every read is whole lines of exactly
  `json::render`'s shapes. A record's `program` and `user` are names the spawner registered for its
  source badge. A per-user reader sees only its own user. Sequence numbers rise inside the read.
- `compositor_session` drives `commit_damage` and `composite`. A client's damage lands inside its
  own window, and `STATUS_OK` means the rectangle was inside its surface. No pixel outside the
  damage changes, and every pixel inside it is the topmost committed window's own, or the
  background.

Two extractions made the first and third drive the code the binaries run rather than a copy.
`Server::handle` is the file server's 285-line request match, moved from the binary with its IO
behind a `ServeEdges` trait. `compositor::commit_damage` is the ten-line damage decode from
`serve_frame`. Neither changed behaviour.

## Rates

Sessions per second on an M-series Mac, from an empty corpus, with the sanitizer `cargo fuzz` builds:

| target | before | after | what changed |
|---|---|---|---|
| `redoxfs_server_session` | 688 (proposal: 190) | 2,180 in a minute; 1,250 over ten | the 4 MiB image is formatted once and reopened per input through a copy-on-write disk |
| `system_log_session` | | about 15,000 | |
| `compositor_session` | 153 | about 1,680 | paint the first whole screen once and copy it; check the far field once per session by row slice |

The "before" for the file server is this target with a fresh format per input, measured the same
way; the proposal's 190 was a different scratch target. The rate falls as the corpus grows and
sessions lengthen.

## Planted defects

Each ships as a patch in `fuzz/falsifications/`, replayed by hand (the header of each says how):
`script/falsifications` sweeps proofs and tests, not fuzz targets. Times are wall clock to the
first crash from an empty corpus, three seeds, build excluded.

| target | the defect | found in |
|---|---|---|
| `redoxfs_server_session` | milestone 726 (an unknown badge fails closed in subtree_scope)'s fail-open arm restored | 0.7 to 1.5 s, by the admission rule |
| `system_log_session` | `OPERATION_READER` reads its scope with `<=`, so every reader is a system reader | 1.3 to 1.8 s, by the scope rule |
| `compositor_session` | `damage_to_screen` loses its clip to the client's surface | 0.5 to 0.7 s, by an overflow in `Rect::translate`; with overflow checks off, by the window rule after about 2,000 sessions |

## What the file server's target found

Four real defects, all in `redoxfs_server`, each with a host test that fails without its fix:

1. A grant's root could be closed out from under its binding. Handles are per server, so an
   open client could `CLOSE` the handle a bound badge's `ROOT` resolves to; the next open reused the
   slot and the bound badge's `ROOT` named that object, outside its grant. First seen while writing
   the content rule, then found by the target in 512 s (702,000 sessions) with the fix reverted.
   `Server::close` now refuses a live grant root with `EINVAL`.
2. A write whose end wraps 64 bits killed the server. RedoxFS adds offset and length unchecked;
   the inline-data path then sliced backwards. Found in about three minutes. Refused with `EFBIG`.
3. A file end past the engine's node tree killed the server. Upstream's `NodeLevel::new` admits
   twelve level-4 entries where the node holds eight. Found after about 400,000 sessions. The server
   bounds a file at `MAX_FILE_END` with `EFBIG`; the vendored fix is #1606 (`lane/redoxfs-level4`).
4. Shrinking a huge sparse file stalls the server, walking every record pointer in between. Not
   fixed: recorded in `Server::truncate`'s `BUGS` section and proposed as
   `design/roadmap/proposals/a-client-cannot-stall-the-file-server-with-a-sparse-file.md`. The
   target folds sizes between 64 MiB and `MAX_FILE_END` below 64 MiB so it does not re-find it on
   every run.

`EFBIG` is a new reply on the file-service wire, for cases that used to be a dead server.
The system log and compositor targets found nothing in two minutes each.

## What CI pays

The CI `fuzz` job runs `script/fuzz --time 60`, so each target adds a minute of fuzzing: three
minutes for the three. Building them, measured locally into an empty target directory, took 8 s on
top of the 24 s the six parser targets took, almost all of it the RedoxFS engine. A runner is
slower; at three times this machine the build adds under half a minute. So the job grows by about
3.5 minutes, from milestone 721 (each merge-group CI job has a 20-minute budget)'s 7.0-minute median
to about 10.5, against the 20-minute budget. The first merge-group runs with these targets are the
measurement that settles it.

## What the other handlers need

`net_stack` has no host part. Its request dispatch (`components/src/net_stack.rs`, lines 227 to
398) and the helpers under it (400 to 855) are in an EL0 binary, which cannot build for the host
because `user_mode_runtime` carries EL0 assembly. Giving it a host part means a sans-IO crate,
measured by reading rather than by doing it:

- about 630 lines moved, with seven helper signatures taking `net_transport::VirtioNet` that would
  take smoltcp's `phy::Device` instead, so a host test supplies a loopback device;
- the clock (`instant` at 7 sites, `now`) and the two blocking waits (`wait_for_nic`,
  `service_until` at 8 sites, which sleep on the NIC interrupt and a timer) become an edge trait,
  `ServeEdges`'s shape;
- the per-socket windows (`MappedWindow`, 5 sites) become slices, and the delegation path
  (`attach_page_frame`, `cap_delete`) stays in the binary;
- the fuzz workspace then links smoltcp, already in the tree's graph, so no crate new to the tree.

Estimate: a lane of its own, about a day; the oracle worth checking is the listen grant
(`grant_allows`) and per-socket window isolation. Recorded as a `BUGS` entry in the binary.

The caretakers (`fs_file_caretaker`, `fs_nameset_caretaker`, `fs_subtree_caretaker`, 926 lines
together) are the same shape: their decode is inline in EL0 binaries with no library half. Each
carries a `BUGS` entry saying so.

## See also

- `notes/fuzzing.md`: the parser targets, the corpus discipline and the CI job.
- `redoxfs_server/src/dispatch.rs`: the dispatch the file server's target drives.
