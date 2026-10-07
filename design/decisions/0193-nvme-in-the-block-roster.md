---
status: PROPOSED
raised: 2026-09-19
---

# 193. What a block-roster entry calls an NVMe disk, and whether it carries more than virtio does

Raised 2026-09-19 by the maintainer, at calef's instruction, after he asked
whether milestone 421's `DECISION` gate had a decision behind it. It did not: the ask lived only in
that block's own *What is needed from calef* section, which is one rung above a chat message and
below where AGENTS.md says an open decision belongs. *(Section number provisional until the merge
queue lands it.)*

## What is being decided

Two questions, both narrow and both permanent, because `crates/block_roster` is a page the kernel
writes and a `no_std` program reads:

1. What the new transport kind is called, and whether adding it forces its two neighbors to be
   renamed under the acronym test.
2. **Whether an NVMe entry carries anything a virtio entry does not**, which is a question about
   `ENTRY_BYTES` rather than about a name.

## Why now

[§86](0086-el0-nvme-driver.md) listed this under *what is blocked until it is answered*, with the
reason stated precisely: *"small, but its wire shape depends on who owns the controller."* That
dependency was real, because a roster entry for a kernel-resident driver and one for a confined EL0
server differ in what a holder may then ask for. Milestone 261 answered it by putting the data
plane in a process that serves `filesystem_protocol::blk` on a request endpoint, and that lane
deliberately did not take this work, correctly: a transport kind is a wire value two programs read,
which is the category AGENTS.md prices as expensive and routes here.

So nothing is blocked on research any more. What is left is two spellings and one layout, and both
are the kind that cannot be un-shipped.

## The tree as it stands, read rather than recalled

`crates/block_roster/src/lib.rs` documents an 8-byte entry, two little-endian `u32`s:

```
an entry:
0..4     ordinal: which device of this transport, 0-based
4..8     transport: TRANSPORT_MMIO or TRANSPORT_PCI
```

`TRANSPORT_MMIO = 0`, `TRANSPORT_PCI = 1`, `ENTRY_BYTES = 8`, `HEADER_BYTES = 16`. The crate's own
header states the roster's purpose in a sentence this decision leans on twice below: **"Deliberately
not a handle. Holding this tells you a device exists; it does not let you touch it."**

And `kernel/src/user/non_volatile_memory_express_service.rs` "IDENTIFYs the namespace and creates the
one", so there is exactly one namespace in this tree today.

## Question 1: the name, which §154 has mostly already answered

The acronym test is `design/naming.md`'s, ratified by calef on 2026-09-18: *"An acronym is
spelled out where its expansion is a phrase people actually say, and stays whole where nobody says
it."* That ruling deratifies `nvme` by name, alongside `dma_validator`, `gpt`, `dtb`, `ipc` and
`asid`, while re-ratifying `pci` and `elf` because nobody says "peripheral component interconnect".
The kernel service is already spelled `non_volatile_memory_express_service` under it.

| | spelling | argument |
|---|---|---|
| **A** | `TRANSPORT_NON_VOLATILE_MEMORY_EXPRESS` | §154 applied with no exception, and consistent with the service crate already renamed under it. Thirty-seven characters for a constant naming the value `2`. |
| **B** | `TRANSPORT_NVME` | An exception, argued on wire-constant terseness and on sitting beside `TRANSPORT_PCI`, which §154 re-ratified. Costs an unmarked exception in a file a newcomer reads to learn the convention. |

Recommendation: A, on the grounds that B's argument is about the length of one identifier and
§154's is about what a newcomer can read, and this tree has already paid A's price once for the
service crate. **If B, it must be written down as an exception and as a foot gun**, which is the
ladder's rule: an unmarked exception reads as a design and the next person extends it.

Adding the third kind exposes a question about the first, and it should be answered here rather
than discovered. `TRANSPORT_MMIO` is not on §154's deratified list and was never tested against it.
"Memory-mapped I/O" is a phrase people say, so the test points at `TRANSPORT_MEMORY_MAPPED_IO`.
Whether that rename rides with this change or is refused is calef's, and refusing it is defensible:
§154's own wording says *"this rule is not a licence to rename everything."*

## Question 2: whether the entry grows

| | layout | cost |
|---|---|---|
| **A** | unchanged, 8 bytes: ordinal and transport | Nothing to un-ship. A holder that needs a namespace asks the server that owns it. |
| **B** | 12 bytes: add a namespace id | Every reader and the kernel writer change together, `ENTRY_BYTES` is part of the format, and it encodes a distinction the tree cannot yet make. |
| **C** | 8 bytes now, a second entry kind later if needed | Keeps A's cheapness and admits the format may need to grow, at the cost of a versioning question nobody has asked yet. |

Recommendation: A, and the argument is the crate's own stated purpose rather than economy.
A roster says *a device exists*, explicitly not what it is or how to reach it; a namespace id is a
property of a device you have been given, and asking for it is one message to a server that already
answers `SIZE`. And there is exactly one namespace, so B would add a field that can only ever
hold one value, which is the speculative generality this tree refuses elsewhere.

What would reverse it: a machine with two namespaces on one controller, at which point the
ordinal stops identifying a disk and the entry has to say which. That is the trigger to write down,
and it is a measurement rather than an argument.

## What this does not decide

Nothing about who gets the endpoint. §86 and milestone 57 already hold that: the kernel decides
which program gets which device's endpoint at wiring time, and nothing a program computes from a
roster changes it. This decision is only about what the page says.

## What is blocked until this is answered

Milestone 421, which is gate `DECISION` for exactly this and nothing else. Its other dependency
is closed: QEMU's NVMe is attached on every leg of all three runners (`NIFE_NVME`) and the
surveyor's two clients run there today, so the work starts the day this is answered.

Nothing else. The roster is correct as it stands for the two transports it names; what it cannot do
is name the one device in this tree whose driver is a confined process, which is the irony milestone
421's index row records.
