# What the drawn font does better than the ROM, and what the grid would not allow

An appendix to [notes/glyphs.md](../glyphs.md), which keeps the font's geometry and provenance. This
file holds the drawing choices: where it departs from the Kaypro II's ROM, and where it cannot.

The brief was a font in the Kaypro's style, not a forgery. So where the machine is weak for reasons
the grid does not force, this is not.

- One baseline for every glyph. `g p q y` sit on row 6 like `o`, and descend into row 7. The usual
  8-row compromise, which the earlier hand-drawn candidate took and named, is to raise the descender
  bowls a row so the tail gets two. That leaves `p` visibly shorter than `o`. Here the tails are one
  row, and the bowls are not raised.
- `Il1|` are four different shapes, deliberately. `I` has serifs at both ends. `l` has a flag at the
  top left and a tail at the bottom right. `1` has a flag and a flat foot. And `|` is the only glyph
  that runs the full eight rows.
- A slashed zero, which the ROM has, and which is the reason a terminal font is usable in a hex dump.

And what is kept because it is the grid rather than the drawing:

- `M` and `W` are near mirrors. Five columns leaves one way to draw each, so they differ only in which
  end the middle spike sits at. That is the Kaypro's own failing, and it is not fixable at this width.
- The descender is one row. Eight rows with a seven-row cap height leaves exactly one, so `g` has a
  hook rather than a tail.
- The underscore does not join. `_` is five ink columns with a gutter each side, so a run of them is
  dashed rather than continuous. The ROM had the same property, for the same reason.
