---
status: DECIDED
raised: 2026-08-23
decided: 2026-08-23
ratified_by: calef
---

# 112. `touch`'s two behaviors need two rights: write covers "now", a separate right covers "arbitrary"

calef, 2026-08-23, agreeing with the recommendation on milestone 47 (`cd`, `pwd`, `ls`, `mkdir`, `rm`, paths, and environment)'s own named
fork: *"I agree with your recommendation."*

## The question

`touch` does two different things to a file's mtime: set it to *now*, and (`touch -t`) set it to
*whatever the caller says*. Milestone 47 names the second as the ability to lie about history,
significant for anything reasoning from mtime, backups included. The question left open: is "set to
now" the same right as "set to an arbitrary value," and does the file's write right already cover
both, or does the second need something more?

## The decision

**No, they are not the same right.** Plain write covers setting mtime to now. Setting an
arbitrary value needs a separate right, not folded into write.

## Why, and the two independent precedents that converge on it

POSIX already drew this exact line, checked rather than assumed: `utime()`'s semantics
distinguish setting to the current time (ordinary write permission suffices) from setting an
arbitrary timestamp (the caller must *own* the file; write permission alone is not enough). The
reason POSIX splits it this way is the same reason this milestone names: setting to now only records
something already true and independently observable; setting to an arbitrary value is an assertion
nothing bounds. [utime(3p) POSIX](https://www.unix.com/man_page/posix/3p/utime/),
[utime(2) Linux](https://www.man7.org/linux//man-pages/man2/utime.2.html).

And it lands exactly on this tree's own precedent, one level down, which is what the milestone's
own text predicted ("that is §43's asymmetry again... one level down"). §43 already treats reading
the clock as broadly grantable and setting it as a separate, more tightly held authority (a distinct
writable page from the read-only one). `touch`'s two behaviors are the same shape applied to one
file's timestamp: "now" is bounded by what the clock already says and cannot lie; "arbitrary" is
bounded by nothing.

## The shape of the fix

Follows the same separable-rights-ladder pattern this milestone already uses for a directory
capability (`enumerate`, `open` read/write, `create`, `remove`, each grantable independently: "a
program handed a directory to write logs into should not thereby be able to delete what is there").
`touch -t` needs its own grant, distinct from and in addition to `WRITE`, the same way `REMOVE` is
already distinct from `WRITE` on a directory rather than implied by it.

## What this does not decide

The exact name and wire shape of the new right (a new `fs_proto` bit, its place in the rights ladder
alongside `enumerate`/`open`/`create`/`remove`) is left to whoever builds `touch`, not decided here.

## What it unblocks

Milestone 47's `touch` section can now be built to a concrete spec: `CREATE` for the create-if-absent
half (already expressible today), `WRITE` for bumping mtime to now, and a new, separate right for
`touch -t`'s arbitrary-value half.
