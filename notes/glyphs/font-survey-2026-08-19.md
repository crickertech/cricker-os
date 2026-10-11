# The font options, with pictures (2026-08-19, lane `bench/font-options`)

An appendix to [notes/glyphs.md](../glyphs.md), which keeps the font that ships and why. This file
holds the survey that preceded it: the license correction and reversal, the specimen tool, every
candidate with its cost and obligations, the measurements, and the screen constraint. It also holds
the survey tool's own limits.

## A correction, and a reversal

First, a correction to the framing the parent once had. Its paragraph about Terminus and Spleen read
as though the font were a compromise forced by licensing. It is not: `font8x8` is public domain, which
was verified for this survey by opening the upstream project rather than by recalling it. Its
`README` says, of the header files this table came from:

```text
Author: Daniel Hepper <daniel@hepper.net>
License: Public Domain
```

The credits below that carry Marcel Sondaar's original header, which says the same thing, and names
IBM's public-domain VGA fonts as the source. Nothing in this tree owes an obligation to anyone for
the letters on the screen. So a change of font would be a decision about how it looks.

And a reversal on top of that correction, which is calef's (2026-08-19). The old rule refused any
font with an attribution obligation, on the ground that a bitmap font is compiled into the image, and
its license therefore travels with the artifact. The first half of that reasoning stands; the
conclusion does not. He read OFL 1.1, and his verdict was that the obligation "doesn't look onerous".
So obliging licenses are in scope, and priced rather than refused. That matters, because the
public-domain corner of this field is small, and the well-drawn fonts mostly live under the OFL.

## The specimen sheet

The specimen sheet is how the aesthetic question gets answered. The crate's design claim is that the
expected picture is a pure function. That is what lets the terminal, the kernel test and the
host-side scanout check agree about a letter. Spend the same property on the choice itself, and a
font stops being an argument:

```text
cargo run -p bitmap_font --example specimen                          # what ships
cargo run -p bitmap_font --example specimen -- --dots                 # one character per pixel
cargo run -p bitmap_font --example specimen -- --font ter-u16n.bdf --name terminus-16
cargo run -p bitmap_font --example specimen -- --font bench/font-options/hand-drawn-8x8.art
cargo run -p bitmap_font --example specimen -- --font crates/bitmap_font/kaypro-style-7x8.art
```

It reads the three formats a bitmap font arrives in. They are `.hex` (GNU Unifont), and `.bdf`
(Adobe, which is what Terminus, Spleen and every X11 bitmap font ship as). The third is the `.art`
`#`/`.` picture, the only sane way to author one by hand. Every font gets the same sample text, chosen for where small
fonts fail. That is `Il1|` and `O0` (the confusions that ruin a hex dump); `rn` against `m` (the one
that makes prose *wrong* rather than ugly); the descenders `g q y p j`; and two lines of ordinary
prose and ordinary code. A font that looks good on a pangram and bad in a sentence is bad.

## The candidates, what they cost, and what they oblige

"Grid" is the decisive practical column: characters by rows on the display ladder's 128x64 scanout,
which was 16x8 then.

| Font | Cell | Table | Grid | License | Reserved name |
|---|---|---|---|---|---|
| **kaypro-style (ships)** | **7x8** | **1024 B** | **18x8** | **ours** | none |
| `font8x8` (shipped until 2026-08-20) | 8x8 | 1024 B | 16x8 | Public domain | none |
| hand-drawn | 8x8 | 1024 B | 16x8 | ours | none |
| `unscii-8` (+`-alt`, `-thin`, `-mcr`) | 8x8 | 1024 B | 16x8 | Public domain / CC0 | none |
| `spleen-5x8` | 5x8 | 1024 B | **25x8** | BSD-2-Clause | none |
| `terminus-12` | 6x12 | 1536 B | 21x5 | OFL-1.1 | **"Terminus Font"** |
| `terminus-14` | 8x14 | 1792 B | 16x4 | OFL-1.1 | **"Terminus Font"** |
| `gohufont-14` | 8x14 | 1792 B | 16x4 | WTFPL v2 | none |
| `kaypro-ii` (`81-146a`) | 7x8 | 1024 B | **18x8** | **none stated** | **excluded, see below** |
| `terminus-16` | 8x16 | 2048 B | 16x4 | OFL-1.1 | **"Terminus Font"** |
| `spleen-8x16` | 8x16 | 2048 B | 16x4 | BSD-2-Clause | none |
| `unscii-16` | 8x16 | 2048 B | 16x4 | Public domain / CC0 | none |

Where each license was read, since a claim from memory is a claim to mark as such:

- `font8x8`'s `README` at `github.com/dhepper/font8x8`;
- unscii's `README.md`, whose line 18 says "You can consider it Public Domain (or CC-0) except for the
  files derived from ... Unifont (unifont.hex, hex2bdf.pl, unscii-16-full.*) which fall under GPL",
  an exception that does not touch `unscii-8` or `unscii-16`;
- Terminus's own `OFL.TXT` inside `terminus-font-4.49.1.tar.gz`, which opens "Copyright (C) 2020
  Dimitar Toshkov Zhekov, with Reserved Font Name "Terminus Font"";
- Spleen's `LICENSE` at `github.com/fcambus/spleen`, two-clause BSD;
- and gohufont's `COPYING-LICENSE`, whose entire terms are "0. You DO WHAT THE FUCK YOU WANT TO."

What an obligation would cost us, in the order that matters:

- The Reserved Font Name is the expensive clause, and only Terminus has one. Being picky about fonts
  means eventually fixing a glyph. Under the OFL, the moment a glyph changes the table is a Modified
  Version, which may not carry the reserved name without written permission. So adopting Terminus
  means either never touching it, or renaming our copy. Spleen (BSD-2) and gohufont (WTFPL) reserve
  nothing, and a redrawn glyph costs nothing beyond the notice.
- The OFL has no cure period. Its own words are that the license "becomes null and void" if a
  condition is not met. So shipping the notice has to be a mechanism rather than an intention.
- The notice would live in three places, because the obligation attaches to the image and not to the
  source tree. The font's source and its `LICENSE` go in `vendor/`, registered in `vendor/README.md`
  the way the RedoxFS pin is. The identifier goes in `deny.toml`'s shared license policy, with the
  honest caveat that `script/supply-chain` checks the cargo graph. So a font transcribed into
  `crates/bitmap_font/src/glyphs.rs` is on the register rather than on the gate. And a page goes in
  the documentation store of milestone 40 (documentation as a system service), so a machine running
  nife carries the text it owes.

What is still excluded. The Linux console's `lib/fonts/font_8x16.c` is the familiar IBM VGA shape,
and its first line is `// SPDX-License-Identifier: GPL-2.0`. Copyleft on a table compiled into every
binary is a different question from attribution, and it is out. Fixedsys Excelsior is called public
domain by unscii's `README`. But that is a third party's summary, and `fixedsysexcelsior.com` does not
resolve (checked 2026-08-19), so the claim cannot be read at its source. Ambiguous is treated as
obliged.

## What the shapes measure

Over the 52 letters, straight from the tables:

| Font | Ink per letter | Left edge sigma | Width sigma | Cap | x-height | Descender |
|---|---|---|---|---|---|---|
| **kaypro-style (ships)** | **13.7** | **0.23** | **0.54** | 7 | 5 | 1 |
| `font8x8` | 24.6 | 0.27 | 0.79 | 7 | 5 | 1 |
| hand-drawn | 15.5 | 0.19 | 0.61 | 7 | 5 | 1 |
| `unscii-8` | 23.2 | 0.44 | 0.61 | 7 | 5 | 1 |
| `unscii-8-thin` | 14.4 | 0.27 | 0.77 | 7 | 5 | 1 |
| `spleen-5x8` | 11.8 | 0.23 | 0.30 | 6 | 5 | 1 |
| `terminus-12` | 15.6 | 0.34 | 0.52 | 8 | 6 | 2 |
| `terminus-14` | 20.0 | 0.41 | 0.80 | 10 | 7 | 2 |
| `gohufont-14` | 19.1 | 0.44 | 0.74 | 9 | 7 | 3 |
| `kaypro-ii` | 13.6 | 0.27 | 0.48 | 7 | 5 | 1 |
| `terminus-16` | 20.1 | 0.41 | 0.80 | 10 | 7 | 3 |
| `spleen-8x16` | 33.6 | 0.46 | 0.74 | 10 | 7 | 3 |
| `unscii-16` | 34.6 | 0.38 | 0.52 | 11 | 7 | 3 |

The drawn font measures like the machine it is drawn after, which is the check on whether the style
survived the drawing. It has 13.7 ink per letter against the ROM's 13.6, a left-edge sigma of 0.23
against 0.27, and a width sigma of 0.54 against 0.48. It is lighter than everything here but Spleen
and `unscii-8-thin`, and more evenly fitted than everything but Spleen. The one number that moved the
wrong way is the width sigma, and the reason is deliberate. `Il1|` were given four different widths
so they cannot be confused, where a font that padded them all to the grid would score better and read
worse.

Ink per letter is weight. The two sigmas are consistency, which is what the eye reads as rhythm rather
than as any one glyph. The descender column is how many rows a `g` gets below the baseline an `x`
sits on. Descender depth is what separates the sizes, and it is why every 8x8 font here, ours
included, has a cramped `g`: one row against three.

## The cell size is a screen decision before it is a taste decision

The screen turned out not to be free to move. The scanout was 128x64. So an 8x8 cell gives the 16x8
grid then recorded under Honest limits, and any 8x14 or 8x16 font gives 16x4. Four rows of text is
not a terminal. Two candidates dodge that entirely, by being narrower rather than shorter: Terminus
6x12 gives 21x5, and Spleen 5x8 gives 25x8, which is more columns than then at the same number of
rows. A narrower cell costs nothing but `GLYPH_W`, which is a constant in `crates/bitmap_font` that
three parties read.

Growing the scanout is blocked, and it is blocked on the capability model rather than on memory. That
was measured on 2026-08-19, when a lane tried to build the chosen font onto a terminal-sized surface.
A `Frame` capability names exactly one page, and each one occupies a capability table slot. The
capability table has sixteen slots, and the virtio-gpu driver's DMA region already uses nine of them.
The hard ceiling is `SURFACE_FRAMES <= 9`, which is 36,864 bytes. Every non-square shape inside it
(128x72, 144x64, 192x48) gives five text rows or fewer at 8x14. So there is no scanout reachable
today on which gohufont-14 is a terminal. 800x600, which is 100x42 characters and the size calef
picked, needs 469 frames. The build fails rather than the boot, because
`display_service::DRIVER_SLOT_DMA` carries a `const` assertion for exactly this. The fork, its three
priced options and the sizing arithmetic that goes with it are in notes/frames.md's `BUGS`. Until it
is answered, gohufont-14 is the right font for a screen this tree cannot yet make. (The scanout has
since grown; see the parent's Honest limits for the grid today.)

## Authoring our own, priced

It was raised as an option, and no existing sample can answer it.
`bench/font-options/hand-drawn-8x8.art` is 95 printable glyphs drawn for this survey, in the `#`/`.`
format the specimen tool reads. It is at exactly the cost of any other 8x8 font: 128 glyphs by 8 rows
is 1024 bytes, and 8x16 would be 2048. The drawing cost was one lane for 8x8. 8x16 would be more than
twice that, because sixteen rows is where drawing skill stops being hidden by the grid. The honest
assessment of the result is below, and the short version is that it is consistent and plain rather
than good.

## The survey's own limits

- The hand-drawn candidate is competent, not good. `bench/font-options/hand-drawn-8x8.art` is
  consistent (the tightest left sidebearing in the survey) and light. Three glyphs are weak enough to
  name where a reader meets them. `$` is mushy where the stem crosses the S, `&` reads as a blob, and
  the shoulder of `r` sits a pixel clear of its stem, so the arm looks detached. It is a drawn
  candidate for comparison, not a proposal, and nothing in the tree uses it.
- The specimen tool clips wide glyphs rather than refusing them. Both the `.hex` and `.bdf` readers
  keep the leftmost byte of a row. So a 16-pixel-wide glyph is shown as its left half, which looks
  like a clipped font instead of an error. The `.hex` height is taken as the commonest row count among
  the letters, because `unscii-8.hex` stores `U+0000` with sixteen rows in an eight-row font, and the
  maximum is therefore a lie. The `.bdf` reader uses the bitmap and the bounding boxes only. `SWIDTH`,
  `DWIDTH` and the property block are ignored, which is right for a fixed-pitch cell and wrong for
  anything else. Half-block output is only faithful in a terminal that draws `U+2580`/`U+2584` at full
  cell height; `--dots` has no such dependency and is the tie-breaker.
- A font narrower than the cell is drawn at its own advance, and that is a choice worth knowing.
  Spleen 5x8 and Terminus 6x12 are narrower than eight pixels. Drawing them on an eight-pixel pitch
  makes them look loose, in a way that is the tool's fault rather than the font's. The tool takes the
  advance from the font's own bounding box. What it does not do is prove that `crates/bitmap_font`
  would work at that width. `GLYPH_W` is a constant three parties read, and moving it is a change to
  the crate rather than to a table.
- The 8x16 authoring option is priced but not drawn. There is a hand-drawn 8x8 to look at and no
  hand-drawn 8x16. So the "author our own at twice the height" row in the survey is an estimate, where
  the 8x8 row is a specimen.
