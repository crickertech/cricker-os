---
status: NOT-STARTED
raised: 2026-09-14
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 405. The nine `INIT` roles, and the archive entry `spawn_hello` picks

Filed 2026-09-14 as an unnumbered proposal by milestone 291's lane, which
split twenty-two of `fixtures/src/hello.rs`'s thirty-one roles into programs and stopped at these
nine on purpose; numbered 2026-09-19 by milestone 433's drain of the proposal pile. **Premise
re-read against the tree on 2026-09-19 and still true, to the constant**: `fixtures/src/hello.rs`
still declares exactly those nine roles (`INIT` 20, `CHILD` 21, `DEV_CHILD` 22, `INIT_DEV` 23,
`INIT_CONSOLE` 24, `INIT_IRQ` 25, `IRQ_CHILD` 26, `INIT_LEAST_AUTHORITY_DEMO` 28, `INIT_COREMARK`
29), `ROLES_ENTRY` is still `"hello"` and is still read by `init_build` and one other parent to
re-enter this binary's own image, `kernel::user::spawn_hello` still takes a role and always enters
`HELLO_ENTRY`, and `crates/capability_witness_protocol` is there for the agreed words. The gate was
`MILESTONE 268` and it is cleared: milestone 268 (every architecture boots the same way) turned
BUILT on 2026-09-19, so nothing is rebuilding the boot sequence any more and this is ready to start.
**This block also absorbs** milestone 399 (the six `init` roles in `hello`), which proposed
renaming six of these nine constants and is `SUPERSEDED` for the reason 399's own successor gave
first: after the split the six parents are programs with their own names and the constants are gone,
so the rename would be a naming decision spent on an interim.
*(Number provisional until the merge queue lands it.)*

*(Cleared 2026-09-19 by milestone 268's own lane, because `script/roadmap` refuses
a gate on a BUILT milestone and 268 is now one. The reason it existed is kept: that lane was
rebuilding the boot sequence on all three architectures while 291 ran, and the change proposed here
is in `kernel::user::spawn_hello`. Two lanes in that function is the collision this tree already
knows how to avoid.)*

## What is left, and why it did not come apart with the rest

Nine roles of `hello`: `INIT` (20), `CHILD` (21), `DEV_CHILD` (22), `INIT_DEV` (23),
`INIT_CONSOLE` (24), `INIT_IRQ` (25), `IRQ_CHILD` (26), `INIT_LEAST_AUTHORITY_DEMO` (28) and
`INIT_COREMARK` (29). Six of them are a userspace parent that parses an ELF and builds a child;
three of them *are* the child that parent builds.

The other twenty-two came apart cheaply because the kernel spawns each of them directly, with
`run(image, Spawn { .. })`, so the only change was which bytes the caller passed. These nine do not,
for two reasons:

1. **`spawn_hello` always re-enters `HELLO_ENTRY`, since milestone 166 moved the boot role's own
   entry out to `boot_progenitor`. Six parents becoming six programs makes that choice a table, or
   makes the entry a parameter the six call sites supply. The second is the smaller surface and is
   probably right; it is still a boot-path signature change.
2. **The parents find their children by looking themselves up. `ROLES_ENTRY` is the string
   `"hello"`, and `init_build` reads *this binary's own ELF* out of the archive and re-enters it at
   a different role. Split, each parent names its child's archive entry instead, which is the
   shape `init_console`, `init_least_authority_demo` and `init_coremark` already have (they load
   `"console"`, `"least_authority_demo"` and `"coremark"` by name). So the target shape exists in
   the same file; it is the three roles that re-enter their own image that have to move.

## What it would take

- Three child programs, provisionally `reporting_child`, `device_identifying_child` and
  `interrupt_waiting_child`, packed in all three archives. Their two agreed words (`CHILD_WORD`,
  and the interrupt child's reported word) join `capability_witness_protocol`, which milestone 291
  created for exactly this.
- Six parent programs, each naming its child's entry.
- `spawn_hello` taking the entry name, or a role-to-entry table beside `PROGENITOR_ROLE`.
- Six `spawn_hello` test call sites updated. The role numbers themselves stay:
  [the progenitor's grant order](301-one-grant-order-for-the-progenitor.md) recorded that those six
  tests name them, and the grant *order* the roles share is the thing that proposal is about.

## What it would settle

**The name.** calef deferred naming whatever remained of `hello` until there was something settled
to name (2026-09-14: *"If there is anything left then we can consider a name for what remains"*).
After 291 what remains is nine roles; after this, either six programs and three children with their
own names, or nothing at all, and `hello` is deleted rather than renamed. That is the cleaner
answer and it should be checked before anybody spends a naming decision on the interim.

## What it would not settle

The six parents would still dispatch on nothing: each would be a whole program with one `_start`.
But three of them differ from `INIT` by a single capability in the endowment (`INIT_DEV` adds the
UART, `INIT_IRQ` adds the interrupt), which is an argument that they are one program parameterised
by what it was granted rather than three. **That is a design question this proposal does not
answer**, and answering it by splitting first would be the wrong order: a program that reads its own
capability table to decide what to do is the capability-system-native shape, and it may make three
of these nine unnecessary rather than separate.

## What it inherits from milestone 399

Milestone 399 proposed renaming six of these nine constants to `PARENT_*` and is `SUPERSEDED` by
this block, on the sequencing 399's own successor stated first: after the split the six parents are
programs with their own names and the constants are gone, so the rename is a naming decision spent
on an interim. Its refusals are worth reading before anybody names the six programs, because they
are refusals of names this work will reach for: `PROGENITOR_*` (false, the progenitor is one program
under one archive entry), `LOADER_*` (names one of three things the roles do), `BUILDER_*` and
`SPAWNER_*` (each already a program in this tree).

**One thing 399 carried that this block does not, and it should be picked up here.** The six role
numbers are written down twice, in `fixtures/src/hello.rs` and again in `kernel/src/user/tests.rs`
(`INIT_ROLE = 20`, `INIT_DEV_ROLE = 23`, `INIT_CONSOLE_ROLE = 24`, `INIT_IRQ_ROLE = 25`,
`INIT_LEAST_AUTHORITY_DEMO_ROLE = 28`, `INIT_COREMARK_ROLE = 29`), and nothing gates the two against
each other. This work touches both files and both sets of numbers, which makes it the moment to fix
that rather than to carry it across.

## Index row

Milestone 291 split twenty-two of `fixtures/src/hello.rs`'s thirty-one roles into programs and
stopped at nine on purpose. Six of the nine are a userspace parent that parses an ELF and builds a
child, and three of them are the child that parent builds. The other twenty-two came apart cheaply
because the kernel spawns each directly, so the only change was which bytes the caller passed; these
nine do not, for two reasons. `spawn_hello` always re-enters `HELLO_ENTRY`, so six parents becoming
six programs makes that a table or makes the entry a parameter the six call sites supply, which is
the smaller surface and is still a boot-path signature change. And the parents find their children
by looking themselves up: `ROLES_ENTRY` is `"hello"` and `init_build` reads this binary's own ELF
out of the archive to re-enter it at a different role, where `init_console`,
`init_least_authority_demo` and `init_coremark` already name their child's archive entry instead, so
the target shape exists in the same file and only the roles that re-enter their own image have to
move. What it would settle is the name calef deferred on 2026-09-14 until there was something
settled to name: after this there are either six programs and three children with their own names,
or nothing at all and `hello` is deleted rather than renamed. What it would not settle is whether
three of the six parents are one program parameterised by its endowment, since `INIT_DEV` and
`INIT_IRQ` differ from `INIT` by a single capability, and a program that reads its own capability
table to decide what to do is the capability-native shape; answering that by splitting first would
be the wrong order.
