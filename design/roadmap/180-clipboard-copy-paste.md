---
status: NOT-STARTED
raised: 2026-08-26
milestone_dependencies: none
decision_dependencies: 169
machine_requirements: none
specific_machine: none
needs_person: no
---
# 180. Copy and paste: a clipboard in a system with no ambient authority

Minted 2026-08-26, calef, checking the roadmap for a gap that turned out
not to be a gap at all: nothing in `design/` or `notes/` mentions a clipboard, not even as a
named limitation the way mouse support is (see milestone 179). This file is that gap's first record.

Not a driver or a wiring question, as milestone 179 (pointer input) was: a clipboard is shared,
mutable state that more than one principal reads and writes, and this tree has a standing rule
against exactly that shape (`AGENTS.md`, milestone 126: "enumeration is itself authority," the same
posture that refused a hung-thread lookup as "list everything and find the match"). A conventional
OS clipboard is ambient: any process can read whatever the last process wrote, with nothing proven.
That is precisely the authority model this system exists to not have, so this cannot be scoped
before that tension is resolved, and resolving it is an architect's, not a lane's. The fork is
§169 (whether a clipboard exists here, and what it
is scoped to), written up 2026-09-19 by milestone 435's lane; before that this gate named no
decision, which for a block whose whole finding was that nothing in the tree mentions a clipboard
left the record one rung lower than it should have been.

## What this is, in brief

Unix's `xclip`/`pbcopy`, conceptually: select text somewhere, retrieve it somewhere else, as two
acts separated in time with no direct connection between the programs on either end. The terminal
(milestone 142) and a pointer (milestone 179, for drag-to-select) are the two obvious sources; a
paste target is anything a shell or an editor (milestone 169's `kilo`) accepts text into.

## The tension a conventional clipboard runs into here

**Every existing multi-client object in this tree is minted, held, and delegated explicitly.** A
directory capability is handed to exactly the principal it names (DECISIONS §117); a channel
endpoint is minted per connection (milestone 49's login front door, `login_protocol::CONNECT`); even
the compositor's shared windows are each their own object, not one global surface every client can
address. A clipboard, in the form anyone has ever used one, is the opposite of all of that: one
slot, last-writer-wins, readable by whoever asks, with the OS deliberately not checking who "whoever"
is. That is the entire feature. A capability-shaped clipboard that required proving you were the
same principal who copied something would not be a clipboard; the whole point is that the pasting
program is not the copying program and the system introduces them anyway.

## Shapes worth considering, none decided here

- **A single systemwide object, held by whichever component owns the terminal/compositor, offered to
  any client that asks.** The closest thing to what every other OS calls a clipboard, and the
  starkest version of the tension above: it is ambient authority by definition, scoped only by
  "anything running on this machine can read the last thing copied," which is a real, named
  exception to the standing rule rather than an accidental one if it is chosen. AGENTS.md's own
  ladder has a place for exactly this ("an exception is allowed and must say so").
- Scoped to a login session (milestone 49's identity model), so a clipboard exists per
  authenticated principal rather than machine-wide, and reading it costs the same proof anything else
  under that principal's subtree already costs. Closer to this tree's own grain, and it means a
  clipboard cannot exist at all until milestone 49's login-boot-wiring piece does, which today it
  does not.
- **Not a clipboard at all, but a targeted transfer.** Some terminal emulators already do this for
  one narrow case (OSC 52, "copy to the host's real clipboard"): the copying program names its
  destination explicitly rather than writing to an ambient slot a third party reads later. This sidesteps
  the tension by refusing the shape that causes it, at the cost of not being what "select, then paste
  somewhere else" actually means to a person using the machine.

## What this unblocks

Nothing yet: this is the first record of the gap, not a scoped increment. Real terminal use (copying
a path from one pane, an error message from a build) is the actual motivation, the same "would
someone live in this" axis milestone 142 already named for the display half.

## Prior art

X11's `PRIMARY`/`CLIPBOARD` selections (ambient, exactly the ownerless-buffer shape this file's
tension section describes) and Wayland's data-device protocol (still ambient, but requires an
explicit client offer/request handshake rather than X11's poll-on-demand model, which is closer to
this tree's own "delegate explicitly" grain without fully escaping the tension). Both are the shape
to read before designing this, not to copy outright.

## BUGS

Not started; nothing built yet to carry its own BUGS section. This file's own "tension" section is
the gate.

## Index row

Not a driver gap: a clipboard is shared, mutable, ambient state, and this tree's whole capability
model exists to not have that shape (milestone 126, "enumeration is itself authority"). Three
candidate shapes named, none decided: a systemwide object as a named exception, scoped to a login
session under milestone 49's identity model, or a targeted transfer that never has an ambient slot
at all. Gate: DECISION.
