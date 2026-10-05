---
status: DECIDED
raised: 2026-08-23
decided: 2026-08-23
ratified_by: calef
---

# 115. No `sysctl`: each subsystem's tuning goes through its own service, not a bolted-on aggregator

calef, 2026-08-23, on milestone 126 (who else is running, and who is allowed to ask)'s `sysctl` fork: *"Agreed, go with option
B."*

## The question

`procps` ships `sysctl`, which writes machine-global kernel tunables, in the same package as `ps`.
Milestone 126 named the fork without a design for it: either build `sysctl` as a program holding a
capability per tunable subsystem, or decline it and let each subsystem's own service carry its own
tuning surface, with no unifying program at all.

## The decision

No `sysctl`. Each subsystem that grows a runtime tunable exposes it through that subsystem's
own service and its own capability, the same shape `notes/net/prior-art-and-the-contract.md`'s `/net/tcp/clone` already uses. There
is no program on this system that holds a bag of capabilities spanning multiple subsystems for the
purpose of retuning them.

## Why, and the precedent that made this an easy call

This tree already decided the identical question once, for control rather than configuration.
`pkill` was declined on 2026-08-17 with the ruling "a domain names its members and does not act on
them": authority stays with whoever already legitimately holds a resource, never centralized into
a generic tool, even at the cost of a hole in `procps`'s coverage claim. `sysctl` is the same shape
one layer over -- a single program reaching across subsystems it does not otherwise touch -- and
the ambient-tunables-namespace `sysctl` would need is exactly the kind of thing this system exists
to refuse.

And this tree already *built* the alternative, favorably, before this question was ever asked.
`notes/net/prior-art-and-the-contract.md` already routes configuration through the specific resource's own control surface
(`announce 80` written to `/net/tcp/clone`), citing Plan 9's per-resource `ctl` file convention over
a global panel. Declining `sysctl` is not a new design, it is applying a shape this tree already
committed to once, to the one remaining place Unix's packaging would have asked for the opposite.

**The premise was checked and is not yet live.** No subsystem in nife today exposes a
runtime-adjustable tunable of any kind, so this decision sets posture rather than unblocking
running code. Reversible if a future case for a unifying tool becomes overwhelming, since nothing
today depends on either shape.

## What this does not decide

The wire shape any individual subsystem uses for its own future tunables (a `fs_proto`-style verb,
a dedicated control page, or something else) is left to whoever builds that subsystem's service.

## What it unblocks

Milestone 126's statistics stratum can proceed without `sysctl` blocking it, and the package's
`BUGS` section records the gap plainly, the same way `pkill`'s absence is already stated rather
than glossed over: `procps` ships without `sysctl`, and a reader who expects to retune the kernel
through one program will not find one.
