---
status: BUILT
raised: 2026-08-19
built: 2026-09-29
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 141. A palette worth looking at, and a gate that lets it be one

Minted 2026-08-19 by calef, on seeing that the terminal's colours were
chosen as a test instrument: *"can we have an option at some point to make it pretty and not just
good for tests?"*

The first piece was a check nobody had written, and it needed no decision.

In brief. The sixteen-colour palette in `crates/video_terminal` was picked so that a corrupted
pixel is a detectably wrong colour rather than a different legal one. That was a good reason and it
made the screen ugly. Both halves are in the tree, in the order the finding demanded: the gate
came first, and then the palette was free.

## The finding that makes this cheap

The palette does not deliver the property it is ugly for. Its own comment says the entries "have
all three channels distinct in most entries", and measured on 2026-08-19: no entry has three
distinct channel values, and eight pairs are related by a channel permutation. Entry 1 is
`0xcd0000` and entry 2 is `0x00cd00`, so swapping the red and green channels turns red into green,
which is a legal palette colour and passes every check.

So the tradeoff everyone assumed, pretty against testable, was never being paid for. The screen is
ugly and the swap it guards against is undetected.

## Why this is a property rather than a palette

The test's requirement is a property of the set, not a specific list of colours, and that is the
whole reason this milestone is possible:

1. Every entry has three distinct channel values. Then a swapped channel changes the colour.
2. No two entries are permutations of each other. Then a swap cannot land on another legal
   colour.
3. No entry is saturated at `0xff` in a channel that another entry saturates, which is what the
   present palette actually buys and should keep: a dropped shift or a saturating write lands
   off-palette.

Any palette satisfying those three is as good a test instrument as this one and better than it,
because this one fails the first two. And those constraints leave enormous room: they rule out pure
primaries and near-duplicates, and they permit essentially every considered terminal palette a
person would recognise.

## The order

1. Write the check. Done 2026-09-26, watched failing first. Aimed at the xterm palette that
   shipped, the gate killed the build with the very panic it raises today ("PALETTE fails milestone
   141's palette check"); the palette changed only after that was seen. It is `palette_faults`, a
   `const fn` over `PALETTE`, host-tested against the xterm set kept as its known-bad witness, and
   the gate is a `const` assertion: a palette that stops being a test instrument does not compile.
2. Choose a palette that passes. The choice was calef's and was already made: §104 (the
   rich-text font is DejaVu Sans Mono, and the palette is Solarized) ratified canonical Solarized
   Dark on 2026-08-20, and predicted this gate's one complaint. Published Solarized fails property
   1 on exactly one entry (base1 is `93,a1,a1`, a repeated channel), so entry 14 ships `0x93a1a0`,
   one unit of blue down. A test holds the shipped table to the published one but for that unit,
   and the nudge is recorded where a reader meets the palette, as §104 (the rich-text font is
   DejaVu Sans Mono, and the palette is Solarized) asked.
3. Only then consider an "option". Considered. The nicer-default reading is what shipped: the
   default is Solarized. The configurable reading is runtime state three parties would have to
   learn, and calef has not said which he meant, so it is deliberately unbuilt (see Follow-on).

## BUGS

- This block assumes the three properties above are the right ones. They are this tree's
  reconstruction of what the original comment was reaching for, not a specification anybody wrote
  down. A fourth failure mode nobody has named would not be caught by them.
- A palette that passes the gate can still be ugly, and no gate can fix that. The check makes an
  attractive palette *admissible*; it does not make one appear.

## Follow-on

- **Decision.** The nudge makes entry 14 `0x93a1a0` rather than Schoonover's `0x93a1a1`, and
  whether a nudged palette is still the name "Solarized" is calef's question, held open where he
  raised it: `design/decisions/104-the-font-and-the-palette.md`.
- **Recorded.** Bold is bright, and Solarized's bright slots are greys, so bold green, yellow, blue
  and cyan lose their hue. The limitation is in the PALETTE BUGS in `crates/video_terminal/src/lib.rs`,
  and the options with a recommendation are in `notes/solarized-and-bold-is-bright.md`.
- **Recorded.** A configurable palette was never built, on purpose: calef's word was "option", the
  nicer-default reading is delivered, and the runtime-state reading waits on which he meant. The
  order above holds the reasoning; nothing is proposed until he asks.

## Index row

Minted by calef on 2026-08-19: the terminal's colours were chosen as a test instrument and it
shows. The finding that makes it cheap is that the palette does not deliver the property it is
ugly for: no entry has three distinct channel values and eight pairs are channel permutations of
each other, so swapping red and green is undetected. Gate the property, then any palette that
passes is free to be pretty.
