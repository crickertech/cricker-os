---
status: DECIDED
raised: 2026-08-26
decided: 2026-08-26
ratified_by: calef
---

# 131. The competitor question: hold at rung two, prove text-mode usefulness first

calef, 2026-08-26: *"I think we want to stick with a text mode OS until we've
built something useful on text mode. Computers operated for decades without graphics."* And,
sharpening it in the same conversation: *"I want a kick ass text mode. Something I'll love working
with."* [Milestone 34](../roadmap/34-gpu-acceleration.md)'s own gate named this the live question
once [the display ladder](../display-ladder.md)'s rung two (the compositor, milestone 33) landed;
this is that call, made. It is not only a hold: it names an affirmative direction in the same
breath.

## The question

[`design/competitor-question.md`](../competitor-question.md) parked one decision: when the
demonstrator becomes a competitor, if ever. §14 keeps a general-purpose competitor as an explicit
later optionality, triggered by two conditions holding together: a verified core running a real
workload (milestone 19, BUILT), plus a reason the world needs another OS that the demonstrator has
by then proved. The display ladder's own governance note is explicit that rungs three (real
applications: iced, cosmic-text) and four (milestone 34: GPU acceleration via virtio-gpu 3D, the
Venus path) are exactly the competitor-shaped work this parks: "broad driver coverage, a full Linux
ABI, a package ecosystem" is what the ladder's next two rungs start pulling toward. Rung two's
completion made the first half of the trigger real. The second half, a market or product reason,
is not something more code produces, and nothing in the tree was positioned to answer it.

## The decision

Hold at rung two. Rungs three and four (milestone 34 among them) stay `NOT-STARTED`,
deliberately, until something genuinely useful has been built and proven on text mode. This is not
a technical finding; it is a ranking call, and it agrees with one this tree has already made rather
than introducing a new principle. [DECISIONS §14](14-project-direction.md) and `AGENTS.md`'s own
ranking function already say the customer path (milestone 55, Time Machine) is what orders work here,
and that customer is entirely headless: a backup server has no display, no GPU, no application UI.
Every hour spent on rung three or four is an hour not spent on the thing a customer would actually
run. calef's own historical framing, computers ran productively for decades without graphics, is the
same point stated the way engineers who lived it would put it.

## The affirmative direction, not just the hold

"Prove something useful on text mode" names work, not only a boundary. This tree already has a
concrete, already-minted shape for exactly that: milestones 169 through 174, the self-hosting line
(`kilo`, then `nano`, real screen editors on this system's own primitives; `git` core plumbing;
`cargo`'s subprocess needs without fork/exec; full local `rustc`/`cargo`/LLVM self-hosting; and a
thin development client for daily use). That line was scoped, before this decision, around a single
test: developing nife *on* nife. Held against "something I'll love working with," it is the same
test restated. A kick-ass text mode is not a separate ambition competing with the customer path; a
shell, an editor and a toolchain calef genuinely reaches for daily is itself a real workload,
proven the way this tree already insists on proving things, by someone running it.

`design/roadmap/142-a-text-display-worth-living-in.md` ("a sibling of rung three that rung three
then consumes") is the adjacent case worth naming for the same reason it is named in "what this does
not decide" below: good typography on this system's own terms, gated on milestone 141 and a smaller
decision of its own, not on this one.

## What this does not decide

- Rungs one and two stay built and unchanged. This is not a retreat from the display work
  already proven (framebuffer contract, compositor, VT engine, virtio keyboard); it is a hold on
  climbing further, not a reason to strike what exists.
- **It does not answer the competitor question itself, only defers it again**, on the same terms
  §14 originally parked it: until the demonstrator has proved something a customer would want, not
  until a calendar date or an amount of code.
- **It does not touch milestone 142** (a text display good enough that people use it instead of a
  GUI) or any other rung-adjacent work that is explicitly *not* GUI-toolkit work.
  142's own doc frames itself as "a sibling of rung three that rung three then consumes," useful
  independent of whether rung three ever proceeds. Text-mode quality-of-life work is exactly what
  this decision asks for more of, not less.

## What reopens it

The same trigger `competitor-question.md` already named, read literally now that its first half is
satisfied: a reason the world needs another OS, proved by something useful built and running on
text mode, not merely asserted. Two candidates now stand for that proof, not one: milestone 55
(Time Machine: SMB3 with Apple's extensions), which was the original customer path, and the
169-174 self-hosting line (a daily-driver text environment). Whoever eventually reopens rungs three and four should point at what got built and
used, not just at time having passed.
