---
status: DECIDED
raised: 2026-09-19
decided: 2026-10-05
ratified_by: calef
---

# 166. The rasteriser dependency, and whether the glyph atlas ships one face or four

Raised 2026-09-19 by milestone 435 (forty-five milestones are gated on a decision nobody wrote down)'s lane, which found milestone 142 (a text display good enough that people use it instead of a GUI) gated on
`DECISION` naming no decision, when half of what its gate describes was decided a month ago.
[§104](0104-the-font-and-the-palette.md) (the rich-text font is DejaVu Sans Mono, and the palette is
Solarized) took the font and the palette on 2026-08-20; the gate never said so. This is the half
§104 did not take. *(Section number provisional until the merge queue lands it.)*

## The ruling

calef, 2026-10-05 UTC, on the faces: *"four faces, italic as oblique"*. Then on the rasteriser:
*"rasteriser A"*.

- The rasteriser is `ttf-parser` 0.25.1 plus `ab_glyph_rasterizer` 0.1.10: two crates, about 23.7k
  lines (23,196 in `ttf-parser`'s `src/` and 555 in the rasteriser's, counted 2026-10-05 UTC),
  Apache-2.0 for the rasteriser and MIT OR Apache-2.0 for the parser. It is a host-only build tool
  and never enters the shipping graph.
- The atlas ships four faces: regular, bold, italic and bold italic. DejaVu Sans Mono has no true
  italic, so italic means its Oblique face, and bold italic its Bold Oblique.
- At the 14x26 cell and 479 glyphs the atlas is 174,356 B for one face and 697,424 B for four
  (479 x 14 x 26, then times four). This is host-side build output, and the 479 is the ruling
  session's glyph count, not a figure found elsewhere in the tree.
- Both halves of the original "bold is bright" reason expire at an anti-aliased cell, so that
  limitation retires when the faces land (milestone 142, increment five).

## What is being decided

1. The dependency that renders the font, which §46 (thin primitives or whole subsystems) makes a
   decision rather than a convenience.
2. Whether the atlas ships one face or four, which §104 (the rich-text font is DejaVu Sans Mono, and
   the palette is Solarized) is silent on.

## Question 1: the rasteriser

The determinism question was measured twice, by two investigations that did not see each other. A
peer session measured cross-architecture output stability on 2026-08-20 at raw `f32` coverage, over
95 ASCII characters at nine sizes across four fonts, on x86_64 and aarch64 with both `std` floats and
`libm`:

- `ab_glyph_rasterizer` is byte-identical across every combination.
- `fontdue` 0.9.4 is not, at default features. Its `simd` feature is on by default and compiles an
  SSE path on x86 only. The four-wide prefix sum reorders float additions against the scalar
  accumulation. One pixel in 151,414 differed on JetBrains Mono.

A second investigation reproduced it by a different route: 58,708 renderings across fifteen fonts,
identical between aarch64 and x86_64 for `ab_glyph`, and 136 of 47,166 differing for `fontdue`, each
by exactly one pixel and exactly one 255th. It pinned the mechanism: lane 3 of the prefix sum
computes `(a3+a2) + (a1+a0)` where the scalar path computes `((a0+a1)+a2)+a3`, and float addition
is commutative but not associative.

Three parties compute the picture without talking to each other: the terminal draws it, the kernel
test predicts it through the direct map, and the host checker grades QEMU's `screendump`. They
compare every pixel, and the negative control is a single changed letter. A rasteriser that disagrees
between architectures breaks that agreement outright.

| | option | outcome |
|---|---|---|
| A | `ttf-parser` plus `ab_glyph_rasterizer`, pinned at 0.1.10 | Decided. Smallest graph and the only one measured byte-identical across architectures. 0.1.4 through 0.1.8 panic with an index out of bounds on some in-bounds-adjacent geometry, fixed in 0.1.9, which is why the pin is specific. |
| B | `fontdue` 0.9.4 | Refused, see below. |
| C | Write the rasteriser and the parser | Refused, see below. |

Quantize to `u8` at the boundary and never persist raw `f32`. `ab_glyph_rasterizer` 0.1.5 removed
the `1.0` cap on coverage and the raw hash changed; the 8-bit hash did not, because Rust's
float-to-integer cast saturates. That is the difference between a table that survived a patch bump
by luck and one that survives it by construction, and it costs nothing because the table holds 8-bit
coverage anyway.

### Why B lost

The original table said B meant "fewer crates", and that is false. Measured 2026-10-05 UTC with
`cargo tree` in a scratch crate:

| | crates in the graph, counting `fontdue` |
|---|---|
| `fontdue` 0.9.4, default features | 8 (`allocator-api2`, `core_maths`, `equivalent`, `foldhash`, `fontdue`, `hashbrown`, `libm`, `ttf-parser`) |
| `fontdue` 0.9.4, `default-features = false` | 4 (`core_maths`, `fontdue`, `libm`, `ttf-parser`) |
| option A | 2 |

`fontdue` depends on `ttf-parser` itself, so it is option A plus more. The reason it lost is the
one the measurements give: its x86-only SIMD path risks different atlas bytes between the Mac and
x86 CI, which would break the regenerate-and-compare gate. Turning the feature off is a note
somebody has to remember at every call site. Its `FontSettings::scale` also feeds curve
linearisation at load time, so the same glyph at the same size renders differently depending on a
number set elsewhere.

### Why C lost, and a correction to the old argument

The original text said §46's first rule (write it if it is on the verification path) applied to the
rasteriser. That was wrong. The rasteriser is a build tool that runs on the host; only the generated
table is verified, by regenerating it and comparing bytes. Milestone 142's block says so (the
verification-path argument at lines 476 to 481, and increment three at lines 577 to 578, as of
`f7dec0e65`). The same shape is `script/vendor-verify` for RedoxFS.

The real cost of C is the font parser, not the rasteriser. `ab_glyph_rasterizer` is 555 lines of
source; `ttf-parser` is 23,196. Writing a TrueType and OpenType parser to avoid a host-only
dependency buys no verification and costs a subsystem.

### The caveat, corrected

The old caveat said no rasteriser had been run on either target. That is moot: the rasteriser runs
only on the host and never on aarch64, riscv64 or x86_64 targets. What matters is determinism
across the hosts that regenerate the table, the Mac and x86 CI. Neither project promises
bit-stability across versions, so the measurements are observations; the exact pins and the
regenerate-and-compare gate are what hold the property.

## Question 2: one face or four

`crates/video_terminal` said "bold is bright", and the reason was recorded: a bold weight needs a
second font, and at 8x8 a bold face is a smudge. At an anti-aliased cell both halves of that reason
expire. A bold face is legible and the atlas has room.

| | option | outcome |
|---|---|---|
| A | One face, "bold is bright" stands as a recorded limitation | Refused. Smallest table, but a terminal that cannot render italic, and the case for it was table size, which is an effort argument. |
| B | Four faces: regular, bold, italic, bold italic | Decided. What calef's word "rich" asks for. |

The family constraint was checked: DejaVu Sans Mono ships Bold, Oblique and Bold Oblique but no true
italic, which is why the ruling reads "italic as oblique". A terminal grid does not need a cursive
italic, and the slanted face keeps every glyph on the same cell boundary.

## Where the atlas lives, and what stays

The atlas is a format two programs agree on. The terminal draws from it, and the kernel test and the
host checker predict pixels from it. Under codebase rule 7 in `CLAUDE.md` (anything two binaries
must agree on is a crate, never a `#[path]` module), it belongs in a crate under `crates/`, with the
generator under `tools/` or `xtask`. Names are provisional until an architect ratifies them.

`bitmap_font` stays. The kernel's early-boot and panic output need a font with no parser and no
table to load, and a bit table does that.

## Why the console differs from Linux and Redox

Recalled, not read: Linux's console uses a bitmap font compiled into the kernel (the `fbcon` and
`vt` code), and Redox's console is likewise a bitmap font drawn by a program near the kernel. Both
are a last-resort console that lives in or near the kernel, so a bitmap font is the right trade
there.

Nife's goal is milestone 142's: a text display good enough that people use it instead of a GUI. Its
terminal is a userspace component, not kernel code. The precomputed atlas keeps a font parser out of
the shipping graph altogether, and run time needs only integer blending against 8-bit coverage.

## How reversible it is

The dependency is the expensive half (§46: adding one is a morning, removing one after a subsystem
is built on it is a project), though a host-only tool is cheaper to swap than a runtime one. The face
count is expensive because the atlas is a checked-in generated artifact and a cell-attribute layout,
so changing it regenerates the table and touches the terminal's own storage. Both were ruled before
the atlas was generated, which is the cheap moment.

## What this unblocks

Milestone 142's increments three to six no longer wait on this section. Increment six's palette was
already built, and increment five's faces can now land with the atlas.
