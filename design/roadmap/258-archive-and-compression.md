---
status: NOT-STARTED
raised: 2026-09-05
milestone_dependencies: none
decision_dependencies: 146
machine_requirements: none
specific_machine: none
needs_person: no
---
# 258. Archive and compression: `tar` and zlib, because `git` cannot read its own objects without them

Minted 2026-09-05 by calef, on finding that no milestone, proposal or line
of code in this tree covered archive or compression of any kind. *(Number provisional until the
merge queue lands it.)*

For half of it. `tar` is nobody's call but a lane's and can start today. The
compressor's write-or-take question is §146 (archive and compression: which pieces we write, which we take, and which we refuse),
which is `PROPOSED` and calef's, because taking a dependency is a decision under
[§46](../decisions/46-dependency-rule.md). A lane may build the whole of part 1 and must stop at
part 2's fork rather than choosing it.

## Why this exists, and it is not "a tree should have archive utilities"

**Milestone 99 (`git` on nife) counts a compressor it does not have.** Its own argument for why
`git` is a better first real workload than Vaultwarden:

> `init`, `add`, `commit`, `log`, `status`, `diff` are a filesystem, a hash, a compressor, a clock,
> and a place to put bytes. Every one of those is something this tree either has or is building.

Every git object is zlib-compressed, and so are packfiles, so `git` cannot read its own object
database without inflate. The emphasized clause is false for the compressor: there is no
compression code anywhere in this tree, in any form, and nothing is building any. Checked
2026-09-05 across `crates/`, `kernel/src/` and `user/src/`; the only greps that matched were a
comment about load inflating a median and `crates/globally_unique_identifier_partition_table`'s note naming the zlib CRC-32 polynomial.

That matters beyond bookkeeping, because 99's whole case is that unlike Vaultwarden it needs nothing
that does not exist. One of its five things does not exist.

Milestone 198 (the package manager) will need a container and probably a compressor, and says
outright that it *"does not decide the format, the activation shape, or the repository split."*
AGENTS.md makes 198 a precondition on the ranking function itself, since no third party sees nife
before a package manager and a trivial install exist. So this sits underneath the thing that makes a
second customer possible, rather than beside it.

## Scope, and the refusals are part of the deliverable

Part 1: `tar`, written here. A `ustar` header is 512 bytes of fixed-offset fields with an octal
checksum and the format is a loop over headers. Pure logic, so it belongs in `crates/` where it is
host-testable and Kani-reachable, which is what AGENTS.md prescribes and what `globally_unique_identifier_partition_table`, `elf`, `device_tree_blob`
and `nifefs` already are.

The security-relevant decisions in `tar` are policy rather than specification, and they are the
reason this is not a dependency: `../` traversal, absolute paths, and symlink escapes are questions
about what an extraction may reach. In a capability system that answer should be structural, and
this tree already has the shape for it in the `fs_subtree_caretaker` family. An extractor that
cannot name anything outside its subtree does not need to check for `..` at all, and demonstrating
that is worth more than the parser.

Part 2: zlib inflate, and enough deflate for `git`. Gated on §146. That section recommends
writing inflate and emitting only stored (uncompressed) blocks for the compressing direction,
which is legal zlib that any conforming reader accepts, and which makes real compression a measured
optimization later rather than a prerequisite now. It also states the case for taking `miniz_oxide`
instead, and neither is chosen here.

Refused, in writing, so nobody re-derives it: `bzip2` (no consumer, displaced everywhere that
had a choice), `xz`/LZMA (no consumer, and §146 records the 2024 xz-utils backdoor as a cost that
belongs in a dependency decision), and `7z` (no consumer, inherits the LZMA argument).

Deferred against a consumer: `zstd` (take it if 198 picks it; FSE and ANS entropy coding is a
genuine subsystem this project would gain nothing from owning) and `zip` (a container plus DEFLATE
plus CRC-32, and this tree already has the CRC-32, so it is close to free once inflate exists).

## The part that is more interesting than plumbing

A decompressor over untrusted input is the best adversarial parser this tree could acquire.
Milestone 191 found that no Kani harness here had ever caught a defect after the day it was written,
and milestone 193 fixed the reachability that caused it. A real attacker-facing parser is what that
machinery is for, and the properties are stateable: *the output never exceeds the declared bound*,
*the window is never read outside itself*, *a truncated stream terminates rather than looping*.

It is also the shape [§145](../decisions/145-compartmentalization-at-process-cost.md) proposes as
a possible bounded customer: hand a confined domain untrusted bytes, let it reach nothing, take the
output. Risk 7's adversarial half is unbuilt, and a decompression bomb is an adversary that needs no
outside researcher to supply it.

## The proof that this milestone worked

**`tar` round-trips an archive this tree produced, extracted by a program that could not escape its
subtree if the archive told it to**, with the traversal case tested rather than argued. That is part
1 and it does not wait on §146.

For part 2, when it is unblocked: an object written by real `git` is inflated correctly here, on
a fixture rather than on a running `git`, since milestone 99 is `NOT-STARTED`.

## BUGS

- Nobody has read git's object reader to confirm that stored-block deflate is accepted in
  practice rather than only by the zlib specification. §146 carries the same caveat and it is the
  cheapest thing to check before part 2 starts.
- **This block does not price part 2.** §146 judges inflate to be small from the shape of RFC 1951
  and not from a line count, and it says so in its own BUGS.
- **The capability answer to path traversal is asserted, not built.** `fs_subtree_caretaker` exists
  and nothing has yet used it to confine an extractor, so "structural rather than checked" is a
  design intent in this block rather than a demonstrated property.
- Compression is not on the customer path, because there is not one. It is on the path to
  milestone 198, which is what makes a customer path possible, and that is a longer chain than the
  ranking function usually rewards. Saying so plainly is what keeps this from looking like a
  utilities shopping list.

## Index row

there is no compression code anywhere in this tree, and milestone 99's case for `git` as the first
real workload counts a compressor among the things it says we have or are building. `tar` is a
lane; the compressor's write-or-take call is §146
