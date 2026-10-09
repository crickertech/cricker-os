# The `capability_plan` name, ruled and unperformed

*An appendix to [the naming record](../naming.md), moved from the crate's Name block by milestone
863 (the comment-block sweep continues, worth three) on 2026-10-09 (UTC), wording kept except
where a sentence had to split. The crate is still `component_plan` until the rename is performed;
milestone 23 (a capability-routed component OS with live replacement)'s block tracks that.*

calef ruled **`capability_plan`** on 2026-09-13, working the unratified worklist. `component`
named the customer; the crate is about capabilities. calef, shown the old name and the two
alternatives on offer: *"Neither component nor substitution plan convey to me in the name what
this thing is. The word capability seems important to it."* It is: §41 (the endpoint is the broker)'s test is that any program
that speaks the protocol and holds the right capabilities is the component. The protocol half has
always been a crate. This crate is the capability half, lifted out of the literal arrays
milestone 23's block calls "endowments are literals in the operator's source".

The argument that lost, recorded because it is a real cost rather than a bad idea. A grant *is* a
capability, so `capability_plan` and `grant_plan` read as near-synonyms while the two crates are
siblings rather than one a subtype of the other. That distinction is the header's to carry rather
than the name's, which is where it already was. `supervision_plan` was the alternative that kills
the synonymy by naming who wires, and it was refused for dropping the word that makes the crate
legible.

A second reason the old name had to go, which arrived on its own. Milestone 175 (split `user/`: `components/` for services, `fixtures/` for test programs) landed on
2026-09-13 and split `user/` into `components/` and `fixtures/`, so `component` now also names a
directory. Those are different senses: §41's component is a contract a program satisfies, and
175's is a place a file sits. The collision did not decide the ruling and would have needed
recording either way.

Refused `component_manifest` and `manifest`: a second `Manifest` beside `grant_plan::Manifest`
is the comprehension disaster the crate's own header argues against. Refused
`capability_declaration`: a declaration is only half of it, and the wiring is the other half and
is where the checking lives, which is why the ratified name keeps `plan`.
