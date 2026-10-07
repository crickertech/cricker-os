---
status: DECIDED
raised: 2026-08-23
decided: 2026-08-23
ratified_by: calef
---

# 111. Inert configuration is a read-only page, and each declared key is validated against a closed domain

calef, 2026-08-23, on milestone 47 (`cd`, `pwd`, `ls`, `mkdir`, `rm`, paths, and environment)'s environment-variables fork.

## The question

Milestone 47 splits what Unix puts in one string-to-string map into three parts: inert configuration
(`TZ`, `LANG`, `TERM`, "genuinely just data, no authority in it"), names (`PATH`, `HOME`, already
answered by the namespace work), and secrets (already answered, §41's broker). Only the inert third's
wire encoding was undecided, priced as three options: more `SEND`s on the spawn endpoint, a read-only
page (§15's `BootInfo` shape), or an endpoint to a configuration service. The doc's own recommendation
was the page, calling the choice of encoding reversible (a userspace spawn-protocol change between two
programs) but flagging the page's *layout* as the one piece that needs care before anything is built,
since both `user_rt` and the `std` PAL will depend on it once shipped.

## The decision

**The page, as recommended**, for inert configuration only. Names stay capabilities (already
decided); secrets stay endpoints (already decided, §41).

**The layout, settled in the same conversation that surfaced a real gap in the original framing**:
each declared key is checked against a closed, validated domain at assembly time, not accepted as an
arbitrary string. `TZ` must parse as a real IANA timezone identifier; `LANG` as a real locale code;
`TERM` as a real, known terminal type. A value that doesn't parse as a member of its key's domain is
refused when the page is assembled (by whoever builds it, init or a provisioning step), the same
shape as a manifest mismatch being "a refusal at the prompt rather than a mystery later."

## Why validation, not just "the page"

The question that surfaced this: calef asked whether "inert configuration" leaves room for
someone to store a secret there out of convenience and expose it more broadly than intended, the
exact class of accident that makes `AWS_SECRET_KEY`-as-env-var a real, repeated problem in practice
(and one this tree's own survey of Unix's environment-variable history, `LANG`/`NLSPATH`-driven
message-catalog loading and `TERM`/terminfo path bugs, shows is not hypothetical).

Capabilities alone don't answer that question, and it is worth being precise about why. A
capability governs reach, not meaning; once a value is bytes on a page, nothing about the capability
model can tell a password from a timezone. The classification mistake happens before the data becomes
bytes, which is a different problem than anything a capability can gate.

What does answer it is this tree's own strongest tool, applied to value shape instead of authority:
make the wrong state unrepresentable. A byte sequence has to parse as a member of a specific
known-safe set to go through this channel at all. An API key doesn't parse as `America/Los_Angeles`,
so it cannot ride through disguised as one. This closes the door for every key whose domain is worth
validating, which is effectively all of "inert configuration" as this milestone names it.

A cheap, complementary catch for anything a domain hasn't been written for yet: `caps run prog`
already previews what a program will see before it runs (decided elsewhere in this milestone). Extend
that preview to print the actual values of declared inert config, not just the key names, so a
misclassified value is visible to whoever is about to run something, not silently embedded in a page
nobody looks at again.

## What this does not claim

Neither mechanism prevents someone from *inventing* a new inert-config key with a deliberately loose,
unvalidated shape and putting a secret in it. What they remove is the two most likely accidents:
reusing a well-known key for something it was never meant to carry, and an unreviewed launch silently
carrying a wrong value. A new config key's validated shape is a decision made on purpose when it's
added, the same discipline milestone 129 already applies to widening a scheduler's `Held`.

**This does not reproduce, and is not meant to reproduce, milestone 56's actual secrets mechanism**
(checked but never readable back, `PROVISION` then `SEAL`). Nothing here builds a second way to hold
real secrets safely; the fix is entirely about stopping a secret from ending up in the *wrong*
mechanism, not an alternative to the right one.

## What this unblocks

Milestone 47's environment-variable section can be built to a concrete spec: the page (one frame per
process, a shared crate both `user_rt` and the `std` PAL depend on, following `DIR_BIT`/`GRANT_WORDS`
precedent for how it's announced on the spawn wire), each declared key backed by a validated domain,
and `caps`'s existing preview extended to show inert-config values.
