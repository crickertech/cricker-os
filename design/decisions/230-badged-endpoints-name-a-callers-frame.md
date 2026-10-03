---
status: DECIDED
raised: 2026-09-26
decided: 2026-09-26
ratified_by: calef
---

# 230. Badged endpoint capabilities: how a server learns which client's frame a request is in

*Section number provisional until the merge queue lands it. §221 (the boot prompt is the owner's
console) landed with #1340, and §222 to §229 are claimed by open pull requests (#1350, #1359,
#1361), so this took the next free number on 2026-09-26 and may move at merge. The file name is a
maintainer's coinage and provisional too.*

Raised 2026-09-26 by lane `milestone/fs-client-page` (pull request #1358), for its milestone, a
frame per filesystem client channel. That milestone is provisionally numbered 599 and its block is
not yet on `main`. The fork, its premise check, the analogous cases in the tree and the cost figures
are in `notes/a-frame-per-filesystem-client-channel.md`, which lands with that lane's [pull
request](https://github.com/nifeos/nife/pull/1358). It is not linked relatively because it is
not on `main` yet; whoever lands the second of the two can make it a relative link. This section
records the ruling and does not restate the note.

## The ruling

calef, 2026-09-26 (UTC), asked "A, B or C?": *"A"*. Recorded by the maintainer at 17:27Z the same
day; that is the time of recording, not of the ruling.

Option A is badged endpoint capabilities, seL4's answer:

- A new method mints a badged endpoint capability: a derived endpoint capability carrying a badge
  word that its holder cannot change. The method's name is provisional: `BADGE`, which §148 (a
  supervisor restarts by asking, and resolves by asking the kernel) refused for its own use to keep
  it free for this.
- `RECV_CAP` delivers the badge of the capability the caller invoked as a fourth return value,
  beside the first word, the Reply slot and the second word it already returns.
- The file server maps K windows and reads request `b` from its own window `b`. The progenitor holds
  a pool of K (badged endpoint, frame) pairs, hands one to each caretaker chain and takes it back at
  reap. `filesystem_protocol`'s layout and opcodes do not change.

This is a change to the syscall surface under §10 (process model: capability-based, microkernel),
and it fits that model. The badge is a property of a capability, minted by a holder with the right
to derive one, and nothing about IPC gains a side effect. Each new method's semantics are recorded
here or in the section the implementing lane owes, per §10's rule, before the method ships.

## What this settles, and what it amends

- §27 (the filesystem service). §27 recorded badged endpoints as the alternative to a receive
  over a set of endpoints and did not take them. This ruling takes them for the case §27 could not
  cover, several concurrent clients of one server. §27's caretaker stands: the badge tells the
  server which window to read, and the caretaker is still what narrows a client to its subtree.
- §101 (notification objects). §101 named badged capabilities as a later, separate fork. This
  ruling decides that fork for endpoints. Whether a notification capability carries a badge too, so
  a signal's identity is kernel-stamped, is not decided here; it is the natural next use, and it
  stays §101's open item until someone raises it.
- The network stack's shared socket numbers. This also answers option 1 of
  [every-client-of-a-network-stack-shares-its-socket-numbers](../roadmap/649-every-client-of-a-network-stack-shares-its-socket-numbers.md):
  the mechanism that proposal needed now exists. Choosing option 1 over that proposal's other
  options is still a separate ruling on `socket_protocol`, and this section does not make it.

## What it costs

From the note, computed from constants in the tree and not measured on a boot:

- The server maps K windows of 64 KiB (`fs::TRANSFER_PAGES` = 16 pages each). K = 8 is 512 KiB,
  inside the 8 MiB above `BLK_PAGE` the server does not otherwise use.
- Each concurrent client costs one 64 KiB channel of physically contiguous memory, where today the
  whole boot shares one.
- K is a fixed bound on concurrent filesystem clients per boot, and the pool is new state in the
  progenitor. Accepting windows at run time would need an attach verb, which is a wire change on top
  and is not part of this ruling.
- Per request, nothing is added: no extra rendezvous and no copy. The fourth return value is one
  more register on `RECV_CAP`'s return; the icount tripwire will price it when it is built.

## Refused

- B, the kernel remapping the receiver's window at `CALL`. It gives IPC a side effect on the
  receiver's address space, which is outside the model §10 established. It also puts a page-table
  rewrite and a TLB invalidation on the IPC fastpath at a cost nobody has measured.
- C, private client frames and a staging token. It loses at equal cost: more moving parts, a
  copy of every byte each way, and a cooperative rule that fails silently when one holder skips the
  token. Its only case is effort (no kernel change), and this tree does not choose on effort.
- The note's listed refusals, for the reasons it gives: a channel index in the request word, a
  file server per client, copying the name out first, notification objects on their own, and
  keeping clients serialised by construction.

## What happens next

The lane's milestone block (provisionally 599) is not on `main`; it lives on the lane's branch. Updating its gate to name
this ruling is left to that lane. The lane builds the mint method and the `RECV_CAP` return under provisional names, on every
supported architecture per §19 (architectural parity is a tenet). Milestone 47 (navigation and naming)'s set grant at the prompt is unblocked once it lands.
