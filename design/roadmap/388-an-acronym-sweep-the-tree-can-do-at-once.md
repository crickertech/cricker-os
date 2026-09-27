---
status: NOT-STARTED
raised: 2026-09-05
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# 388. An acronym sweep the tree can do at once

Filed 2026-09-05 as an unnumbered proposal by milestone 264, which asked
the acronym question of sixty names and deliberately answered none of them; numbered 2026-09-19 by
milestone 433 (drain)'s drain of the proposal pile. Premise re-read against the tree on 2026-09-19. The
table below is already history rather than a worklist, and says so: all four rows were ruled by
calef on 2026-09-13 and performed by milestones 265, 290 and 298. What is left is the BUGS
section, and both halves of it are still live. `kernel/src/user/entropy_service.rs` still exports
`jh7110_trng_device` (line 300) and `jh7110_crg_window` (line 416), so the file still reads
`jh7110_entropy::discover` inside a function called `jh7110_trng_device`, and
`kernel/src/user/entropy_tests.rs` and `kernel/src/main.rs` still call both by those names. Of the
five names the 2026-09-05 rule deratified, three are still live crates carrying `provisional` and a
recorded refusal apiece (`dtb`, `gpt`, `ipc`); `dma` and `asid` are no longer names
`script/names` sees. *(Number provisional until the merge queue lands it.)*

an architect names things, and this is a list of names. The rule behind the gate is
[§154 (acronym)](../decisions/154-the-acronym-test-is-whether-the-phrase-is-spoken.md), cited here from
2026-09-19; the block argued from it throughout and never named it.

§154 is `DECIDED` and it already answers this block's hard half, which is why nothing new is
minted for it. calef ratified it on 2026-09-18: an acronym expands where its expansion is a phrase
people actually say, and stays whole where nobody says it, asked again of any acronym left inside
the expansion. Its own table rules four of the five names in this block's first `BUGS` entry by
name: `dtb` becomes `device_tree_blob`, `ipc` becomes `inter_process_communication`, `asid` becomes
`address_space_identifier`, `gpt` becomes `globally_unique_identifier_partition_table`, and `dma`
was already ruled on 2026-09-05. [§113](../decisions/113-kernel-object-plain-names.md)'s amendment
is the other half, since it ends the external-standard exemption the `jh7110_crg` row leaned on.

So what stands here is performance and one genuinely open name, not a fork. Applying §154 to
those four is a ratified rename per `design/naming.md`'s procedure and a sweep `ipc` makes large,
which is why this is still a milestone; it is not a question anybody has to answer first. The one
thing §154 does not settle is the second `BUGS` entry, `jh7110_trng_device` and
`jh7110_crg_window` in `kernel/src/user/entropy_service.rs`, where `design/naming.md`'s
*"abbreviation we receive rather than author"* clause may cover a name for a hardware block whose
device-tree spelling is the vendor's (`starfive,jh7110-trng`). That is a ruling per name on
`script/names`' worklist rather than a `design/decisions/` section, and this block is where it
waits.

All four rows are now answered, every one of them by calef on 2026-09-13, working the
unratified worklist: `jh7110_crg` is `jh7110_clock_and_reset`, `jh7110_trng` (crate and program) is
`jh7110_entropy`, by way of `jh7110_entropy_source`, which he ratified first and replaced later the
same day, and he ruled the `ntp` and `mdns` stems the same day. The rows stay in the table below
with their answers beside them, because the question each one asked is the half a future proposer
needs.

The two stem rulings were recorded and not performed, deliberately, and milestone 265 (proto) performed
them on 2026-09-14 along with its own suffix change, so that one crate was not renamed twice.
`ntp_proto` is `network_time_protocol`, `mdns_proto` is `multicast_dns_protocol`, and the two
siblings carrying the same stem moved with it. The nested `mdns` question this proposal called the
interesting one was answered rather than dissolved: calef ruled that DNS stops because it is the
`pci` case one level down, so `multicast_dns` is the whole expansion and
`multicast_domain_name_system` is not. The `ntp` program was to stay short and no longer exists
to: milestone 290 split it into `network_time_client`, `network_time_test_server` and
`unwritable_clock_witness` on 2026-09-14.

The gate no longer stands for the four names in the table. It stands for the BUGS below, which
are the larger half and were never in it.

## Why it cannot be done a name at a time

**An acronym is spelled out unless its expansion teaches nothing** (calef, 2026-09-05). The rule
deratified `dma`, `dtb`, `gpt`, `ipc` and `asid` in the sentence that set it, and design/naming.md
already says the sweep is its own milestone because `ipc` is load-bearing across the tree.

Milestone 264 found the second reason, which is smaller and sharper. **Several of these names exist
in matched pairs that a partial sweep would break.** The program `ntp` and the crate `ntp_proto` were
one word twice, and calef ratified the crate on 2026-08-23. Spelling out the program alone leaves the
pair disagreeing; spelling out the crate alone overturns a ratification as a side effect of tidying a
program. The same holds for `jh7110_trng`, which is a crate and the program built from it, a pairing
AGENTS.md describes as deliberate and worth seeing.

**That prediction was tested and held.** The `jh7110_trng` pair was ruled and performed as one
object both times, crate and program together, and the second ruling turned on the pair: bare
`jh7110_entropy_driver` was refused precisely because the crate is not a driver, which is a refusal
only visible if you are holding both halves at once. See `crates/jh7110_entropy/src/lib.rs`'s
provenance block.

## The names 264 surfaced, each with the question already asked

Every one of these carries the question in its own provenance block, recorded as open rather than
guessed at.

| Name | Expansion | The tension |
|---|---|---|
| ~~`ntp`, `ntp_proto`~~ | network time protocol | Expands into something more informative than itself, which is the deratified class. Against: it is the protocol's registered name and `ntp_proto` was ratified 2026-08-23. **Answered 2026-09-13: the stem is `network_time`**, deratifying that ruling; the crate is `network_time_protocol` since milestone 265 and the program was split away by milestone 290. |
| ~~`mdns_proto`, `mdns_config`, `mdns_responder`~~ | multicast DNS | The expansion contains a second acronym. A full spelling runs to `multicast_domain_name_system_proto` and has stopped teaching before it ends. **Answered 2026-09-13: `multicast_dns`**, DNS staying whole as the `pci` case one level down. Performed by milestone 265: `multicast_dns_protocol`, `multicast_dns_config`, `multicast_dns_responder`. |
| ~~`jh7110_trng`~~ (crate and program) | true random number generator | Expansion teaches, and the acronym is not one a reader outside hardware carries. **Answered 2026-09-13: `jh7110_entropy`**, via `jh7110_entropy_source` the same day. |
| ~~`jh7110_crg`~~ | clock and reset generator | Expansion teaches. Against: both device trees for this chip spell the blocks `syscrg`, `stgcrg` and `aoncrg`, so the acronym is the hardware documentation's own. **Answered 2026-09-13: `jh7110_clock_and_reset`**, the against-case overruled by the same-day amendment to decision 113, which ends the external-standard exemption for acronym crates. |

`cpu` and `icount` were asked and answered inside 264: `cpu` expands to something a reader already
has, and `icount` is a contraction rather than an acronym and is the exact string QEMU prints and
accepts.

## What settling it needs

**Settled for the four names in the table**, including the nested case (`mdns`), where the rule's own
asymmetry argument ran out: spelling out the outer acronym exposes an inner one, so the expansion
does not reach a word the newcomer knows either way. calef did not apply the rule harder; he drew a
line, and the reasoning is in milestone 265's block and in
`crates/multicast_dns_protocol/src/lib.rs`'s provenance (that crate and its two siblings were retired
on 2026-09-15 by milestone 298; the provenance is at commit `0652c981`).

What is left is the BUGS below: the five names the rule deratified by name, and the two kernel
function names. Both want a ruling per name, and `ipc` is the reason this is still a milestone
rather than an afternoon.

## BUGS

- The five names the rule deratified by name are not in the table above, because 264's scope was
  the sixty unrecorded ones and all five were ratified. They are the larger half of the work and
  `ipc` is the reason this is a milestone rather than an afternoon.
- **Two public function names in the kernel carry the acronyms the crates just shed, and nobody has
  ruled on them** (found 2026-09-14 by the `jh7110_entropy` rename, which deliberately did not touch
  them). `kernel/src/user/entropy_service.rs` exports `jh7110_trng_device` and `jh7110_crg_window`,
  and both survived the 2026-09-13 renames of the crates they call into, so the file now reads
  `jh7110_entropy::discover` inside a function called `jh7110_trng_device`. They are a genuinely
  harder case than the crates were and that is why they were left: unlike a crate name, each of
  these names the **hardware block**, whose device-tree spelling is the vendor's
  (`starfive,jh7110-trng`) and whose boot-log wording throughout the tree is "JH7110 TRNG", so the
  "abbreviation we receive rather than author" clause in design/naming.md may cover them where it did
  not cover the crate. A lane should not guess: AGENTS.md puts public function names in an
  architect's hands, and this list is where a name waits for one.

## Index row

An acronym is spelled out unless its expansion teaches nothing (calef, 2026-09-05), and the rule
deratified five names in the sentence that set it. It cannot be applied a name at a time for two
reasons: `ipc` is load-bearing across the tree, and several of these names exist in matched pairs a
partial sweep would break, since spelling out a program alone leaves it disagreeing with its crate
and spelling out the crate alone overturns a ratification as a side effect of tidying a program.
That prediction was tested and held, on the `jh7110_trng` pair, where bare `jh7110_entropy_driver`
was refused precisely because the crate is not a driver, a refusal visible only while holding both
halves. The four names milestone 264 surfaced were all ruled by calef on 2026-09-13 and performed by
milestones 265, 290 and 298, including the nested case, where he drew a line rather than applying
the rule harder: DNS stays whole because it is the `pci` case one level down. What is left is the
larger half that was never in the table: the five ratified names the rule deratified by name, three
of which (`dtb`, `gpt`, `ipc`) are still live crates carrying a recorded refusal apiece, and two
public kernel function names, `jh7110_trng_device` and `jh7110_crg_window`, which survived the
2026-09-13 crate renames and are a genuinely harder case, because each names a hardware block whose
device-tree spelling is the vendor's.
