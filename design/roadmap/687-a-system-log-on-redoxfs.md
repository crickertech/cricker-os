---
status: NOT-STARTED
raised: 2026-09-27
promoted_from: a-system-log-on-redoxfs
milestone_dependencies: none
decision_dependencies: 242
machine_requirements: none
specific_machine: none
needs_person: no
---
# 687. The system log persists through RedoxFS: what a directory grant has to answer

Promoted from `design/roadmap/proposals/a-system-log-on-redoxfs.md` on 2026-10-03 (UTC). The number 687 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

<!-- writing-standards: exception. Granted 2026-10-03 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->

Raised by lane `proposal/system-log` (pull request #1423), from calef's request for a roadmap
proposal covering §242 (a system log)'s Question 5: storage is decided as "in memory now, RedoxFS
later through a directory grant," and this is what that later half needs to answer before a
building lane picks it up. Not a build; slug provisional.

## What persists, and where

Each stored record is a JSONL line, one converted F3 record per line: `seq`, `time`, `program`,
`user`, `severity`, `msg` (§242's ruling). The persisted form is a directory grant to the log
service, matching Fork 6 C's shape for a scheduled job's own directory grant, so the same mechanism
serves both.

**Decided, 2026-09-27: one store**, filtered at read time rather than split by user. The system log
and per-user logs share one directory of rotated files: the service writes every record it accepts
into the same files regardless of who wrote it, and a per-user read capability filters by the
`user` field it already stamps, exactly as the in-memory ring does today. Splitting into per-user
files on disk would mean every kernel and system-service line gets copied into files it does not
belong to, for a volume (below) that does not yet justify it.

Two known limitations follow from one store, where a reader of this proposal should meet them
rather than discover them later: **a shared quota lets one chatty user crowd out everyone's
history** (below), since the quota rotates and drops oldest-first without regard for whose lines
those are; and **deleting one user's records means rewriting files**, since a user's lines are
interleaved with everyone else's inside the same rotated file rather than isolated in one of their
own. Whether a busy durable session under milestone 152 (durable delegation) ever earns its own file
is left open; nothing here blocks adding it later, since the read side already filters rather than
trusting file layout.

## Handoff at boot

Before the grant arrives, records live where §242 already puts them: the kernel's 16 KiB ring and
the service's 64 KiB ring, both in memory. Once the FS server offers the grant, the service
flushes the buffered records in order, as JSONL, inside one transaction (see write amplification,
below, for why one transaction beats one per line). `seq` is monotonic from process start
regardless of whether storage exists yet, so a flushed batch's numbers slot in ahead of whatever
the service appends afterward with no renumbering.

**Decided, 2026-09-27: event-driven, with no timeout on the switch-over.** The service takes the
directory grant whenever the FS server offers it rather than polling or timing out, so there is no
window where a slow-but-working mount looks like a missing one. If the filesystem never comes up for
a boot, the log stays memory-only for that boot, structurally unchanged from today, since §242
already rules that persistence blocks nothing.

A fixed delay is used for exactly one thing, not for the switch-over itself: writing a marker line
the first time the grant is overdue that boot (not "never," since it may still arrive late), so a
reader of a memory-only log knows persistence was expected rather than inferring it from silence.

**Open item for the builder, not resolved here:** this proposal did not verify how the FS server
signals that a grant is ready. Establishing that signal is the first thing a building lane needs,
before the event-driven switch-over above can be wired up.

## Retention and rotation

**Decided, 2026-09-27: rotated files from the start**, not one growing file per boot. The proposal's
original reason was wrong and is corrected here: RedoxFS records are capped at 128 KiB regardless
of file size, so an append's recompression cost (below) is bounded either way, whether the log lives
in one growing file or many rotated ones. The deciding reason is retention instead: RedoxFS cannot
drop the front of a file without rewriting it, while deleting a whole rotated file costs almost
nothing. Roll to a new file at a size ceiling, and delete the oldest file once the directory exceeds
its quota.

**Decided, 2026-09-27: fixed bytes, not a percentage of the drive.** journald's own defaults scale
off filesystem size (`SystemMaxUse` at 10% of the filesystem, capped 4 GiB; `SystemKeepFree` at 15%,
same cap), which solves a problem nife does not have: the target drives (a single USB drive,
`design/decisions/34-redoxfs-primary.md`'s topology) are small and known ahead of time. The quota's
size is provisional, for the builder to set against `notes/redoxfs-audit.md`'s numbers: at about
3.5 MB a day (calef's figure, the raw-text rate measured below), 64 MiB holds about 18 days. A
builder sizing the actual quota should check it against the *stored* rate instead, since that is
what fills the quota: about 5.5 MiB/day once JSONL's overhead is added (also measured below), which
holds closer to 11-12 days in the same 64 MiB. A journald-style keep-free floor (`SystemKeepFree`)
was suggested alongside this ruling but not itself ruled on; it is the builder's to propose.

Who configures it: wherever the directory grant itself is configured, at the log service's launch,
not a manifest field on writer programs (§242 Q2's "programs declare nothing new" extends here).
What's dropped first: oldest complete file, the same oldest-out rule the in-memory rings already
use, with a dropped-count record for the gap. Age is checked only at a rotation boundary rather than
by scanning timestamps continuously, following logrotate's shape over journald's.

## Crash consistency

RedoxFS's own proven guarantee, from the amendment milestone 37 (prove RedoxFS's crash consistency)
earned in `design/decisions/34-redoxfs-primary.md`, measured rather than assumed: prefix
consistency. A fresh mount after any crash recovers exactly the state after some
non-decreasing prefix of committed transactions, and a torn or lost write is caught by its
`BlockPtr` checksum and refused, never returned as wrong bytes. Applied to a log file, that means
the failure mode at the file level is "ends after N whole lines," not a scrambled interior, so long
as the service commits a whole batch of complete JSONL lines inside one `fs.tx` closure and never
splits a line across two commits. That is a rule for the service to hold, not something RedoxFS
enforces on its behalf.

Where a torn last line can still reach a reader: not from RedoxFS's crash recovery (the header
generation ring already refuses a torn header and falls back to the previous valid one), but from a
copy taken outside that recovery path, such as `redoxfs_host extract` reading a live or
inconsistent image, or the service itself crashing mid-write in violation of the one-transaction
rule above. JSONL's shape is the defense for exactly that case: a reader parses forward and, on the
final line only, tolerates and reports a parse failure rather than refusing the whole file, which
is JSONL's ordinary property and part of why it was chosen for the stored form over a fixed binary
frame (§242 Q4).

## Write amplification and cost

Read from `vendor/redoxfs/src/transaction.rs` (`write_node_inner_records`, around line 1882), not
recalled: writing into an existing record reads the *whole* record, patches the touched bytes in
memory, and, once the record exceeds one raw block, recompresses the *whole* record with lz4 and
replaces it if the result is smaller. Record size grows with the node up to `RECORD_LEVEL_MAX = 5`
(128 KiB, `BLOCK_SIZE << 5`), the same figure `notes/benchmarks/filesystem-throughput.md` measured
for reads. So one appended line into a log file whose current record already sits at that ceiling
costs a read, an lz4 pass and a rewrite of up to 128 KiB, plus the tree-block and header-generation
commit every RedoxFS write pays, §34 (RedoxFS is the primary filesystem). That cost is not a log's
invention, but a log's pattern
(many small appends) meets it far more often per byte than the sequential 4-64 KiB writes
`notes/benchmarks/filesystem-throughput.md` measured.

**Recommended: batch a transaction to the service's own 64 KiB ring rather than committing per
line**, flushing on whichever comes first between the ring filling and a time bound. That amortizes
both the record recompression and the header commit over a batch instead of paying each per line,
and it needs no new buffer, since the ring §242 already specifies is the batch.

Bytes per day, derived from §242's own soak measurement (`bench/radon-2026-09-25/soak-8h.log`:
6,046 lines, 1,152,699 bytes over 8 hours, one chatty reporter): scaled to 24 hours, about 18,138
lines and 3.3 MiB of raw text. Converting each line to JSONL (computed here, not measured on a real
build: `{"seq":...,"time":"...","program":"...","user":"...","severity":"...","msg":"..."}`) adds
roughly 117-125 bytes of fixed skeleton per line depending on the program name's length, landing
around 5.5 MiB/day of stored JSONL at that one reporter's rate, about 65% more than the raw text.
That is the honest cost of Question 4's structured-later tradeoff landing on disk rather than only
on the wire. A system logging only at boot-time rates (146 lines, 8,791 bytes a boot) adds
negligible daily bytes; the number that matters is set by whichever component logs most, not by the
system as a whole.

## Prior art, read where possible

Read live on 2026-09-27; one line below is marked as memory because the fetch failed.

- **journald**, read live (`man7.org/linux/man-pages/man5/journald.conf.5.html`). `Storage=`
  defaults to `persistent`, preferring `/var/log/journal` with a fallback to `/run/log/journal`
  (memory-only) before persistent storage exists; `journalctl --flush` or `SIGUSR1`, automated at
  boot by `systemd-journal-flush.service`, moves the volatile journal to persistent storage once it
  is available. That is nife's own "memory now, RedoxFS later" shape, including the same
  flush-on-arrival mechanism, which is evidence the pattern is sound rather than untested. Retention:
  `SystemMaxUse` (10% of the filesystem, capped 4 GiB), `SystemKeepFree` (15%, same cap),
  `SystemMaxFileSize` (an eighth of `SystemMaxUse`, capped 128 MiB), `SystemMaxFiles` (100),
  `MaxFileSec` (rotate by age, default one month), `MaxRetentionSec` (delete by age, off by
  default).
- **logrotate**, read live (`man7.org/linux/man-pages/man8/logrotate.8.html`). `size`, `maxsize`
  and `minsize` govern rotation by size, alone or combined with age; `maxage` deletes rotated logs
  past an age, checked only when a rotation happens; `rotate` bounds how many old copies survive
  before the oldest is dropped, which matches the oldest-first rule recommended above.
- **syslog / RFC 5424**: already read for §242's own prior-art table; not re-read here.
- **Fuchsia persistent logs**: *(memory, unverified)*. A live fetch of Fuchsia's diagnostics pages
  found how logs are generated (LogSink, debuglog) but no page on whether Archivist logs persist
  across reboot; a linked persistence page 404'd. Treat any claim about Fuchsia log durability as
  unconfirmed until someone reads the source.

## Decided

calef ruled all four questions this proposal raised on 2026-09-27 (UTC), each as a comment on pull
request #1423 (`gh pr view 1423 --comments` has the full text); the sections above carry each
ruling's reasoning where a reader meets the design it settles.

1. **Rotated files from the start**, not one growing file, corrected to retention rather than
   record-recompression cost as the reason (Retention and rotation, above).
2. **One store**, filtered at read time by the stamped `user` field, with two known limitations
   recorded where they apply (What persists, and where, above): a shared quota lets one user crowd
   out everyone's history, and deleting one user's records means rewriting files.
3. **Event-driven, with no timeout** on the switch-over; a fixed delay is used only for the
   persistence-expected marker line. The FS server's grant-readiness signal is an explicit open item
   for the builder (Handoff at boot, above).
4. **Fixed bytes**, with the quota's size left provisional at about 64 MiB (roughly 18 days at the
   measured 3.5 MB/day rate) for the builder to set; a journald-style keep-free floor was suggested
   but not ruled on (Retention and rotation, above).

## Index row

DECISIONS §242 stores the system log in memory now and in RedoxFS later through a directory grant. Proposed: the later half, with what the directory grant has to answer before a building lane picks it up.
