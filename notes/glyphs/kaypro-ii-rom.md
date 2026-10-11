# The Kaypro II character ROM: found, rendered, and excluded

An appendix to [notes/glyphs.md](../glyphs.md), whose shipped font is an original drawing in this
ROM's style. This file holds the ROM's identification, its geometry, the license question in three
parts, and the comparison that preceded the poll.

## Which machine, and how that is known

calef owned a Kaypro II and asked what its font was. It was not a typeface with a designer; it was a
chip, and the chip has been dumped. The dump renders, it is genuinely the Kaypro II's, and it is
excluded on license. That is the same answer the survey gives Fixedsys Excelsior, and for the same
reason.

MAME's `src/mame/kaypro/kaypro.cpp` gives the `kayproii` machine one `"chargen"` region holding
`81-146.u43`, 2048 bytes, `CRC(4cc7d206)` `SHA1(5cb880083b94bd8220aac1f87d537db7cfeb9013)`. Don
Maslin's archive at `retroarchive.org/maslin/roms/kaypro/` lists `81-146A` as "Kaypro II/4/83
character generator". `github.com/ivanizag/kaypro-disassembly` carries `chars/81-146a.bin`, whose own
`README` says "81-146a: Kapyro II/83, downloaded from Retroarchive". That file is bit-identical to
MAME's: 2048 bytes, the same SHA-1 and the same CRC-32, checked 2026-08-19. So the artifact is the
Kaypro II's own, not a later model's, and two archives and one emulator agree on it.

The distinction matters because the line ran on. The only Kaypro font in circulation under a clear
license is VileR's `Kaypro2K`, in the Ultimate Oldschool PC Font Pack (CC BY-SA 4.0). Its own entry
says it is the Kaypro 2000, a 1985 PC-compatible laptop: a different machine, a different decade, and
a different font. A dump labeled "Kaypro" is not automatically the II.

## The geometry, which is a fact about the video board rather than about the file

A character generator has no header, so it is read by knowing the wiring. MAME's `kaypro_v.cpp`
`screen_update_kayproii` indexes it as `m_p_chargen[(chr<<3) | ra]` with `ra < 8`. It shifts out
`0, BIT(gfx,4), BIT(gfx,3), BIT(gfx,2), BIT(gfx,1), BIT(gfx,0), 0`. Its comment says:

> The first half of the character generator is blank, with the visible characters in the 2nd half.
> During the 'off' period of blanking, the first half is used. Only 5 pixels are connected from the
> rom to the shift register, the remaining pixels are held high.

Reading the bytes says the same thing: the first 1024 are uniform, and the font is the second 1024.

So the cell is 7 pixels wide by 10 scanlines. The ROM supplies 5 ink columns by 8 rows, and the
hardware holds the rest blank, on an 80x24 display. The specimen tool derives all of that from the
bytes, and prints `7x8 cell, 1024 B table, 94/94 printable drawn, 18x8 on 128x64`.

## The license, which is three questions and not one

They are answered separately because they have different answers. Collapsing them is how a tree ends
up shipping something it cannot account for.

1. The 1982 bits. A copyrighted work of Kaypro Corporation, which went through Chapter 11 and no
   longer exists. No release, dedication or grant was found. There is a real argument that the
   subject matter is not protected at all. 37 CFR 202.1 lists as material not subject to copyright
   both "mere variations of typographic ornamentation, lettering or coloring" and, flatly, "(e)
   Typeface as typeface". That is a defense and not a license, and it has never been tested on this
   artifact.
2. The dump. Somebody's labor, and nobody's stated terms. Retroarchive publishes it with no license
   statement. `ivanizag/kaypro-disassembly` has no `LICENSE` file, which under GitHub's own terms
   leaves it all-rights-reserved. MAME records the hash and does not distribute the bytes.
3. A recreation. None exists for this ROM. The one clearly-licensed Kaypro font is the wrong machine,
   as above.

Ambiguous is treated as obliged, which is the survey's existing rule. So the ROM is not in this
repository, and the font is not a candidate. What *is* a candidate, and what now ships, is an original
drawing in its style. Answer 1 above says the look is not the protected part, and answer 2 says the
file is. See the parent's font section. What is committed is the reader and the recipe. The specimen
tool grew a `.rom` format, and the commands below reproduce everything above from a file you fetch
yourself. That is MAME's own posture, which is to ship the hash and not the bits.

```text
curl -O https://raw.githubusercontent.com/ivanizag/kaypro-disassembly/master/chars/81-146a.bin
shasum -a 1 81-146a.bin   # 5cb880083b94bd8220aac1f87d537db7cfeb9013
cargo run -p bitmap_font --example specimen -- --metrics \
    --font 81-146a.bin --name kaypro-ii --font gohufont-14.bdf --name gohufont-14
```

## How it looks next to `gohufont-14`

`gohufont-14` was the pick at the time. The two are drawn interleaved by giving `--font` twice,
because a specimen in a section of its own is an inventory.

The Kaypro is the most evenly-fitted font in this survey bar one. Its left-edge sigma is 0.27 and its
width sigma 0.48, against gohufont's 0.44 and 0.74; only Spleen 5x8 is tighter. That evenness is real,
and it is the thing a person remembers about the machine. But it is the evenness of a constraint
rather than of a design. Five columns leaves exactly one way to draw most letters, so the regularity
is enforced rather than chosen, and it is paid for twice. `M` and `W` come out as exact vertical
mirrors of each other, distinguished only by which end the middle spike sits at. And the descender is
one row against gohufont's three. So `g p q y j` all descend, but by a single row each: a hook where
gohufont has a tail.

It is light, 13.6 ink per letter against 19.1. That is the weight that suited a green phosphor tube
whose bloom filled the strokes in. On a modern panel with no bloom it reads thin.

The verdict, plainly: gohufont-14 is the better font, and it is not close on the letterforms. Nine
rows of cap against seven, three rows of descender against one, a `g` with a real tail rather than a
hook, and an `M` and `W` that cannot be confused. (The two draw `a` the same way, so that is not one
of the differences, and an earlier draft of this paragraph said it was.) Its higher sigmas are the
good kind of variation: letters given their own width rather than padded to a grid, which reads as
typographic where the Kaypro reads as gridded.

And the survey's verdict was overruled, on purpose and by the right person (2026-08-20). calef ran a
poll, the Kaypro's font won it, and the tree now ships a drawing in its style. The paragraph above
stands as written rather than being quietly softened. The letterforms it describes are genuinely
better than what ships, and a reader deciding whether to change the font again should have the
argument against the current one in front of them. What the poll settled is a question of taste that
measurement was never going to answer. The screen argument below is the reason the taste and the
engineering agree here.

The one place the Kaypro wins is the screen, and it wins decisively: 18x8 against 16x4 on the 128x64
scanout. That is more columns than the shipped `font8x8` and twice the rows of anything 14 tall. The
survey's point about four rows not being a terminal applies to `gohufont-14` and not to this. That is
a genuine tension, and it is not resolved by the license answer, because the license answer only
removes this particular font. A 7x8 or 5x8 cell is what the scanout wants, and Spleen 5x8 is the
candidate that offers it under a license we can take.
