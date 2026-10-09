# The `jh7110_entropy` name

*An appendix to [the naming record](../naming.md), moved from the crate's Name block by milestone
862 (comments state the constraint as it is now) on 2026-10-09 (UTC). Wording kept except where a
sentence had to split.*

Name: ratified 2026-09-13 (calef, working the unratified worklist). It replaces
`jh7110_entropy_source`, which he ratified earlier the same day and superseded on a second pass
through the same worklist. Kept as the crates.io name by calef on 2026-10-07 (UTC), pull request
#1806's publish-ours review. Its maintainer comment reads: "one milestone per crate, minted now,
each keeping its tree name (all free on crates.io as of today) [...] Names are ratified now and
permanent on first publication." His words: "Yes, one per crate". Milestone 818 (proven JH7110
TRNG logic, released on its own) publishes it.

The stem was settled on 2026-09-13 and is not reopened here. TRNG expands to true random number
generator. The expansion teaches, and that is the 2026-09-05 acronym rule which retired
`jh7110_trng`. What moved is the tail. `_source` appears **nowhere else in this tree**, so the
crate and its program were the suffix's only two instances. The chip prefix was already doing the
work it was doing: saying which of the two backends for one contract this is.

The peer backend, the virtio-rng one, is simply `entropy`, on milestone 63 (directory and package names: one spelling per
thing)'s resource-name pattern
of naming a service for the resource it hands you. Milestone 63 (directory and package names) cites that pattern as the one it
departed from for `credentialer`, on the ground that a credential service never hands you a
credential, and this one does hand you entropy, so the pattern holds. The chip prefix is settled
precedent in the other direction too: `jh7110_clock_and_reset` was ruled in the same pass, and
`jh7110` stays in both. A part number is a proper noun no expansion test reaches.

Refused `jh7110_entropy_driver` (it breaks the crate-and-program pair). This crate is not a
driver, it is the register decode, the device-tree query and the byte pool, host-tested and
Kani-reachable, and AGENTS.md says that shared name is worth seeing rather than splitting. That one was close,
because it would have joined the ratified driver family in `components/`. That family's own
ratification wanted a reader scanning the directory to meet programs that say what they drive. What
disqualifies it is the half of the pair that is not a program. Refused
`jh7110_true_random_number_generator`: 35 characters, and the spec's full name buys nothing, the
reasoning that also gave `executable_format` its name rather than ELF's. Bare `entropy_source`
lost too: this tree will have a second system on a chip, and the chip qualifier keeps two drivers
apart.

**The argument that lost**, when the stem was decided, was the external-standard exemption: that
`trng` follows `nvme` and `pci` as a spec-named device. The 2026-09-13 amendment to decision 113
ends that exemption for acronym crates. (`nvme` is kept here because it is the spelling the
argument was made in; DECISIONS §154 (the acronym test is whether the phrase is spoken) later deratified it and the crate is
`non_volatile_memory_express` now, which is that amendment arriving.) The name joins `entropy` and
`entropy_protocol` rather than colliding with them. They are the service, the wire contract, and
this, the hardware behind them.
