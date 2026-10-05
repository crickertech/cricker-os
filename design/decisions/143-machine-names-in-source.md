---
status: DECIDED
raised: 2026-09-03
decided: 2026-09-03
ratified_by: calef
---

# 143. A machine's name is not a hardware fact, and source comments should say the hardware

calef, 2026-09-03, asking who a comment naming `radon` is written for:
*"But I'm thinking of readers that are not me. How are they to know what radon is? Is it relevant to
them at all?"* *(Number provisional until the merge queue lands it.)*

## What is being decided

The three bench machines have names (`argon`, `radon`, `xenon`; also `patagonia` and `cordoba` for
the development and always-on hosts), ratified and recorded in notes/target-hardware.md. Where may
those names be used, and where must the hardware be named instead?

## The answer

A fact about hardware names the hardware. A fact about one machine over time may name the
machine.

- Source comments (`kernel/`, `crates/`, `user/`, `uefi_loader/`) name the SoC or board:
  JH7110, Jetson TX1, OptiPlex 7050. A machine name may appear only as a trailing gloss, never
  leading, and only where the instance is genuinely the point.
- `notes/` and `design/` keep the machine names. They are recording measurements, and there the
  name does something a model number cannot.

## Why the split falls there, and it is not a style preference

**For a stranger reading source, the machine name is never the relevant fact.** Whether an interrupt
arrives on line 32 is a property of the JH7110; it is equally true of their VisionFive 2 and of ours.
The name adds a term with no referent in their world, which is cognitive load with no payoff. That is
AGENTS.md's third principle exactly: *"could a competent stranger, with only this repository, get to
a passing build and a correct mental model without opening a chat window?"*

**And it is not hypothetical.** Measured 2026-09-03: twelve uses in source against about a hundred
in `notes/` and `design/`. Nine of the twelve already gloss the name with its hardware, which is
the tree reaching for this rule without stating it. Two do not:

- `kernel/src/soak.rs`: *"on radon, in `wake_load_aware`"*
- `kernel/src/arch/riscv64/timer.rs`: *"recorded that as unknown on radon and it is still
  unknown"*

Both are really claims about JH7110 silicon, and the second is really a claim about *real silicon
versus emulation*, which is the distinction a reader needs and the name obscures.

The ordering was also wrong where the gloss exists. `"on radon (the StarFive VisionFive 2)"`
leads with the household word and puts the useful fact in parentheses. The relevant thing leads.

## Where a machine name earns its keep, because it is not merely tolerated there

A measurement series needs to assert identity, and a model number cannot. *"These nine boots were
the same physical machine, the same card and the same firmware"* is the claim that makes a
distribution mean anything. notes/soak.md's fifteenfold spread over nine boots is only evidence
because the machine was held constant, and `radon` is what says so.

So the names are load-bearing in `notes/` for the same reason they are noise in `kernel/src`: one is
recording what happened to a particular thing, the other is stating what is true of a class of
hardware.

## What this does not decide

**It does not touch the names themselves**, which are ratified (notes/target-hardware.md) and are
good: an element per architecture, sortable in conversation, and unambiguous in a way "the board"
never was.

It does not reach `script/` or the bench procedures, where a name addresses the person at the
bench and is exactly right: `script/board-console` talks to one machine and the reader is holding it.

And it does not apply to a program's own example data. `crates/mdns_proto`'s doc examples use
`host: "patagonia"` as a sample value. That is not a claim about hardware; it is a string in an
example, and a stranger loses nothing. It is worth changing for a different and weaker reason (a
household name in a public crate's documentation), and that is a preference rather than this rule.

## BUGS

- Nothing gates it. A check would have to tell a hardware claim from a provenance claim in prose,
  which is the same 82%-false-positive problem AGENTS.md prices for `git grep -w TODO`. This is rung
  three: the record sits where the reader meets it, and the next person to touch one of those
  comments is already reading it.
- The boundary between "a fact about hardware" and "a fact about one machine" is a judgement, and
  the honest cases are at the edges. *"Unknown on radon"* is a fact about our firmware revision as
  much as about the SoC; naming both is usually the answer, and naming only the machine is the error.
- `notes/target-hardware.md` is indexed as "Where nife could actually run", which does not
  advertise that it defines the names. A stranger who meets `radon` and wants to resolve it has no
  obvious route, and fixing that index line is the smallest part of acting on this section.
