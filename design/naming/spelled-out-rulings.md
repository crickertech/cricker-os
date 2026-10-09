# Spelled-out rulings

*An appendix to [`design/naming.md`](../naming.md), which is the rule. It holds rulings that spell
out a contraction nothing outside the tree owns, after the `receive` ruling in
[vocabulary-rulings.md](vocabulary-rulings.md), which sat in a document with no room left under the
prose budget. The file and its stem are provisional, minted 2026-10-04 by the lane that performed
the first of them; naming is an architect's.*

## The tree spells `operation`, never `op`

calef, 2026-10-04 UTC, after the `receive` ruling: *"Seems like OP should be OPERATION and RECV
should be RECEIVE."*

> `operation` is spelled out in every identifier this tree owns (`OPERATION_READLINE`,
> `request_operation`, `operation(w0)`). `op` is not a ratified abbreviation for it. An identifier
> that mirrors an external API, or where `op` means something else, keeps its spelling.

The rule is the one the `receive` ruling applied: spell out a contraction nothing outside the tree
owns. The `OP_` prefix on request-code constants became `OPERATION_`. The decoder `op(w0)` became
`operation(w0)`. The names `req_op`, `fs_op`, `op_code`, `op_in`, `OPS`, `PROOF_OPS` and `ops`
followed. No constant value, opcode or wire layout changed.

Refused: keeping `OP_` because the prefix is short and every constant carries it, since length is
the cost the tree already pays for `RECEIVE_CAP`. Also refused: `opcode` or `OPCODE_`, which names
the number rather than the thing requested and would be a second ruling.

Kept, because someone else owns the spelling or the word means something else:

- `core::ops` and `std::ops` (`Deref`, `Range`), and every `no-op` or `no_op`, an English idiom.
- `unsafe_op_in_unsafe_fn`, a rustc lint.
- `op` as an instruction mnemonic in `script/stack-depth-check` and `script/fastpath-footprint`,
  and as the TFTP opcode field in `script/board-netboot`.
- DHCP's `op` field (`op = BOOTREQUEST` in `crates/virtio`).
- `is_op` and `is_diag_op` in `crates/grant_plan/src/line.rs`, and the `op` loop in a swish help
  test. Both mean a shell *operator* (`|`, `>`, `2>`).
- `vendor/redoxfs`, a permission-bit `op` that is not ours to edit.

`OPERATION_SENDTO` keeps the POSIX `SENDTO` spelling in its suffix, pending a separate ruling on
whether the tree spells that `SEND_TO`. This ruling does not touch it.

Closed records (`BUILT` roadmap blocks, `design/decisions/`, dated audit reports) keep the name they
used, so a grep for `OP_` finds them.

The live notes and roadmap documents that cited `OP_` were swept on 2026-10-04 UTC (lane `records/rename-pointer-sweep`).

## The tree spells `capability`, never `cap`

calef, 2026-10-06 UTC: *"I don't think we should abbreviate capability as cap."* When the
maintainer proposed how to apply it, he answered: *"Record the ruling. We've almost got a quiet
tree so we can run that shortly."*

> New code spells `capability` out in every identifier. `cap` is not a ratified abbreviation for
> it, as a whole word or as a component (`cap`, `Cap`, `CAP`, `caps`, `cap_slot`, `SEND_CAP`,
> `irq_cap`). The existing names are renamed in one sweep when the tree is quiet.

The rule is the `operation` ruling's. Its exceptions are carried over as proposals, not yet ruled:
an identifier that mirrors a name a specification coined keeps it, and so does `cap` where it is
the English word for a ceiling. Comments and prose are outside the rule, since "a cap on lane
count" abbreviates nothing.

`script/lint` holds the line until the sweep, through `helpers/cap_abbreviation.py`: a ratchet
over Rust identifiers that fails when the count rises, 2,890 on the ruling's day. The census, the
kept names and a proposed spelling for each public name are in
[capability-worklist.md](capability-worklist.md). The proposals are not ruled. The ABI names
(`SEND_CAP`, `RECEIVE_CAP`, `CAP_INSERT`, `SYS_CAP_DELETE`) and the `Cap` type go last, as one batch.

## The tree spells `non_volatile_memory_express`, never `nvme`

Moved from the crate's Name block by milestone 863 (the comment-block sweep continues, worth
three) on 2026-10-09 (UTC), wording kept except where a sentence had to split.

Name: ratified 2026-09-17 (calef, DECISIONS §154 (the acronym test is whether the phrase is
spoken)), deratifying the crate's own 2026-08-23 ratification to do it, and performed on
2026-09-18, a day apart on purpose. Milestone 320 (every PCI bus the machine has) was a live
lane in `kernel/src/pci.rs` and `crates/pci`, and landing a tree-wide rename underneath it would
have handed that lane a conflict in the files it was rewriting. Kept as the crates.io name
by calef on 2026-10-07 (UTC), pull request #1806's publish-ours review: "one milestone per crate,
minted now, each keeping its tree name [...] Names are ratified now and permanent on first
publication." Milestone 814 (proven NVMe queue logic, released on its own) publishes it.

What the new ratification overturned was the 2026-08-23 kernel-dependency review's reading: "the
specification's own name for the device family, the same claim `pci` and `virtio` make." That
exemption was never the acronym test AGENTS.md states, and the acronym test is what this name
fails. An acronym is spelled out **unless its expansion teaches nothing**. "Peripheral component
interconnect" leaves a reader no wiser and `pci` stays. "Non-volatile memory" tells a reader the
crate is about **storage**, which `nvme` says to nobody who has not already met the word.
`virtio` is not an acronym at all, so it was never evidence for this one. The deciding precedent
is in this tree: `network_time_protocol`, `filesystem_protocol`, `graphics_protocol` and
`credential_protocol`. NTP is at least as famous an acronym as NVMe and this tree spells it out.

This was the name milestone 265 (`_proto` is a truncation) said would come. Its 2026-09-13 boundary held that a standard's
own name stays whole where it names a format or a piece of hardware and expands where it names a
network protocol, and closed by saying nothing mechanical can tell the two apart. The maintainer's
first reconciliation put NVMe on the protocol side without the line moving, since its
specifications define how host software communicates with storage across several transports.
§154 (2026-09-18) then replaced that boundary with one question, whether the expansion
is a phrase people say. Under it `gpt` expanded after all and `pci` stays for a different reason:
not because it names a bus, but because nobody says "peripheral component interconnect". This
crate's name is unchanged by the switch, since "non-volatile memory" is spoken under either test.

Refused `nvm_express`, which the maintainer recommended as the faithful spelling, since "NVM
Express" is what the standard's consortium calls itself. calef's ruling is that the rule is about
the reader rather than the vendor, and `NVM` is four letters a newcomer bounces off whether or not
the specification chose to keep them. Refused `nvme_driver` and `nvme_server` for the program,
both of which carry the same unexpanded word.

Known cost, recorded rather than hidden: every datasheet, every error message and this kernel's
own boot output say `nvme`, so a reader grepping the word the machine printed will not find these
identifiers. That is the price of the rule and it was weighed.

Four things carried the name and all four moved: the crate, `kernel/src/nvme.rs`, the `Nvme`
type inside it, and the program. The program took the crate's own name rather than a `_server` suffix
(calef, 2026-09-18; the argument is in `components/src/non_volatile_memory_express.rs`). What
stayed was deliberate. `crates/pci`'s `CLASS_NVME`, `PciNvmeDevice` and `find_nvme_device` name
the PCI class code the specification defines, so they keep the spelling the way `satp.ASID` did.
QEMU's `-device nvme`, the `NIFE_NVME` variable and `target/nife-nvme.img` are names this tree
does not get to choose. Three of the four senses of the word are not identifiers at all: the
standard as a proper noun, bench transcripts, and `BUILT` blocks' accounts. Measured across the
performed rename: **615 occurrences in 93 files before, 477 in 87 files after**, so 138 moved.
Of what stayed, 241 are the standard in prose, 71 are QEMU's device name or a build artifact,
and 68 are the old name inside an account or quotation. Another 30 are spec citations, 26 are a
decision or roadmap slug, and 26 are `crates/pci`'s class-code identifiers. AGENTS.md's own scar is the blind
`sed` that rewrote the row recording a name's refusal.
