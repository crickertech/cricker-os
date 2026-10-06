# Falsification records: what a harness proves it can catch

Milestone 194, building DECISIONS §134, rescoped by milestone 212. The convention, the tooling,
and what running it on six real harnesses actually found, which was more than the mechanism was
expected to show.

`notes/verification.md` has said the rule since milestone 35:

> Falsify a property before believing it. Break the code the harness guards and confirm the harness
> fails. Every milestone 35 property was falsified this way, and one falsification corrected a claim in
> the code (the load-bearing guard was not the one the comment pointed at). A harness that cannot be
> made to fail is not evidence.
>
> -- notes/verification.md

Rung four, honored by whoever remembered; milestone 191 (did the proofs catch the bugs?) measured it: 141
harnesses, none recording what was done to falsify it. This note is the record's home.

## Inductive Validity Cores: checked first, and they are not available

§134 named IVCs as the cheaper family and said the implementing milestone should look before
building anything, because a yes would make most of the section unnecessary. The answer is no, and
the reason is structural rather than a feature Kani has not got round to.

An IVC is the minimal set of model elements a proof actually needed. It falls out of the
unsatisfiable induction step of an inductive proof, which is why the technique belongs to
k-induction and IC3 model checkers over Lustre models: JKind, Kind2, Sally. Kani is a bounded
model checker. It unrolls loops to a depth, compiles the whole thing to one SAT formula, and hands
it to CaDiCaL. A bounded proof has no induction step, so there is nothing for a core to be minimal
with respect to.

What was actually looked at, on 2026-08-31, against Kani 0.67.0 and the CBMC 6.8.0 it ships:

| looked at | what it gives | why it is not an IVC |
|---|---|---|
| `cargo kani --coverage` | source-based **reachability** coverage: which regions symbolic execution touched | catches some vacuity (an unreachable region), no tautology at all. Kani's own RFC-0011 calls it reachability analysis and not proof-theoretic core extraction |
| `cbmc --slice-formula` | drops assignments unrelated to the property | a solver optimization that reports nothing to the user |
| `cbmc --cover`, `--symex-coverage-report` | test-suite generation against a coverage criterion | again reachability, and about generating tests rather than explaining a proof |
| anything unsat-core shaped in either tool | nothing. `cbmc --help` has no such flag; `cargo kani --help` has no such flag | |

So §134's option C is the affordable option that exists, and this is it.

## The convention

A `Falsification:` block in the comment run immediately above `#[kani::proof]`, past any other
attributes. At the thing, never in a registry, which is milestone 115's argument reused rather than
a new one: two lanes touching two crates cannot collide, and a computed report cannot drift from the
tree.

```rust
/// **Userspace cannot forge a right.** `from_bits` takes an attacker-controlled syscall
/// register (any u32) and the result holds only defined rights, for every possible input.
/// Falsification: replayable 
#[kani::proof]
fn from_bits_cannot_forge_a_right() {
    let raw: u32 = kani::any();
    assert!(Rights::from_bits(raw).is_subset_of(Rights::ALL));
}
```

Four states, and the unknown one is first-class, for exactly the reason `script/names` insists
`unrecorded` stays one: inventing evidence to fill a row puts a false claim in the record whose only
job is saying what is known.

| state | means | what the sweep does |
|---|---|---|
| `replayable <path>` | a patch exists that turns this harness red (backtick the path in a `///` comment; `clippy::doc_markdown` rejects a bare one) | applies it, runs that one harness, **requires red**, reverts |
| `attested <date>` | a person broke the code and watched it fail; nothing can re-check it | counts it, and it is a worklist entry |
| `unfalsifiable <why>` | examined; no defect in the code turns it red | counts it apart; reason required |
| `unfalsified` | nobody has | counts it, and this is the claim's honest denominator |

A patch lives at `<package>/falsifications/<module.path>.<harness_fn_name>.patch`
(`crates/capability/falsifications/verification.subset_is_reflexive.patch`,
`kernel/falsifications/syscall.proofs.the_run_end_is_exact_and_refuses_exactly_what_does_not_fit.patch`).
Backtick the path in the record. These harnesses live in `#[cfg(kani)]` modules, which `script/lint`'s kani-shim pass
compiles with `-D warnings`, and `clippy::doc_markdown` rejects a bare dotted path in a `///`
comment as an unmarked item; that pass is what found it. The reporter strips backticks rather than
requiring them, so a `//` comment, which has no such rule, can carry the same form. Each patch
opens with prose above its first `diff --git` line, which `git apply` ignores and a reader does not. The prose says
which property the patch expects to break, because a falsification that makes a harness fail for the
wrong reason proves nothing. `script/falsifications --check` requires that header to be non-empty;
it cannot check that it is true, which is the same limit `script/names` records for its own
citations.

## Running it

```
script/falsifications                       the table, by package, and the ratio
script/falsifications --unfalsified         the worklist, biggest package first
script/falsifications --check               form only; script/lint runs this
script/falsifications --sweep [package...]  apply every recorded patch, require red, revert
script/falsifications --affected-since <base>   sweep only what a diff can reach
```

**The gate is presence, never `replayable`.** `script/lint` fails a build only when a harness does
not say which of the three states it is in, for the reason `script/names --check` gates on presence
rather than on `ratified`: a lint demanding the queue be drained would hold every unrelated merge
behind proof work nobody can hurry.

The sweep is a report, weekly (`.github/workflows/falsifications.yml`, 06:00 UTC Monday), with
`script/mutation`'s posture deliberately copied. A **survivor** is a harness that stayed green with
its own falsification applied, which is exactly the signal nothing in this tree could previously
detect. A **stale** patch is one that no longer applies, which §134 argues is the mechanism working:
the covered code moved, so the falsification must be redone rather than trusted.

The per-pull-request half is a gate, in `verify.yml`, and the split is `script/lint`'s own: a
patch that a commit staled is a defect in that commit, where a weekly survivor is a fact about the
tree that morning. It costs seconds because a falsification runs **one** harness.

### The cost, measured

`script/verify` is ~650 seconds and this milestone adds **nothing to it**: the sweep is a separate
script and a separate workflow, and `script/verify` was not touched.

| what | cost |
|---|---|
| `script/falsifications --check` (in `script/lint`) | 0.19 s, no build |
| `script/falsifications --sweep`, six records, warm | 4.4 s end to end, rebuilds included |
| one harness alone, `capability`, warm | 0.35 s |

The sweep's cost is dominated by the rebuild each patch forces, so it scales with the number of
distinct crates carrying records rather than with the record count. That is why the weekly workflow
is unsharded, and it is a measurement to re-take rather than a rule.

### A record that never reaches a verdict: the per-record limit

2026-09-24. #1156 landed a `nifefs` record, the length guard weakened to one block, whose replay
never finished. Once the guard admitted the harness's one concrete length, CBMC entered `parse`'s
entry loop, whose bound is a `count` read from symbolic bytes, and unrolled it without end. Nothing
stopped it but the job: the per-pull-request `falsify` job was cancelled at its 45 minutes, and the
weekly sweep (falsifications.yml run 36031723556) at its 120, after that one record had run for
1 h 44 min and every record after it had gone unswept.

So each replay now runs under a wall-clock limit, and one that runs out is `ERROR ... no
verdict`, never red and never a survivor. The sweep goes on to the next record and exits non-zero
at the end.

| record kind | limit | where the number comes from |
|---|---|---|
| Kani harness | 15 min | 1.8 times the slowest record measured |
| kernel test | 30 min | `ci.yml`'s bound on building the kernel and running the whole suite |

The Kani timings are the one full-tree sweep whose timestamps survived, the cancelled run above: 73
records before the hang, median 1.6 s, 90th percentile 8.2 s, and a tail entirely in
`machine_discovery` (116 s, 136 s, 497 s), each including the rebuild its patch forced on a hosted
runner. One record was slower and is not in that sample because the hang came first:
`package_archive::a_short_file_is_refused`, measured locally at 20 to 31 minutes, the same
unbounded-loop shape. It got the harness bound rather than a limit raised to fit it (#1243, red in
1.3 s since), because a record that takes half an hour is the problem, not the limit. Every verdict
line now prints its own time, so the next sweep is a better sample than this one was. The first
such sample, the tail package swept on patagonia under the new limit (2026-09-24): 16 records, all
red, slowest 287.5 s, then 82.6 s and 71.3 s, the other thirteen under 10 s.

#### Example

```
$ FALSIFICATIONS_RECORD_LIMIT=60 script/falsifications --sweep nifefs
==> nifefs::a_short_image_is_refused_not_indexed
    ERROR    no verdict within the 60 s limit; `cargo kani` and its CBMC was killed. ... (60.4 s)
==> nifefs::the_validation_implies_reads_slice_is_in_bounds
    red      the harness caught it (1.9 s)

2 swept, 0 survivors, 0 stale or broken, 1 without a verdict
  no verdict nifefs::a_short_image_is_refused_not_indexed (limit 60 s)
```

That is the limit's own falsification, run on patagonia against #1156's original record (exit 1).
`FALSIFICATIONS_RECORD_LIMIT` (seconds) overrides both limits.

The whole process group is killed, not the child. `cargo` is what the sweep starts; `cbmc` and
QEMU are grandchildren, and killing only `cargo` would leave a solver holding a core and gigabytes
after the sweep had moved on.

#### BUGS

- A wall clock on a shared machine can turn a slow record into a false `no verdict`. The
  headroom is 1.8 times one measured maximum, not a proven margin. A timeout on a busy laptop that
  does not reproduce in CI is load.
- **"Would have finished at minute 16" and "would never finish" are reported alike**, because
  neither is evidence. The fix for either is a faster falsification or a bound on the harness, which
  is what `nifefs::a_short_image_is_refused_not_indexed` got in #1239.

## Milestone 212: the ratio was a fraction of one directory

The walk was `os.walk("crates")`, so the number this script prints as a statement about the tree's
proofs described `crates/` and said nothing about saying so. That is not an undercount, it is a
mislabelled fraction, and §134's whole argument is that a claim about proofs must not rest on an
unstated scope.

Three things were invisible to it by the time it was noticed, all within two days of the mechanism
landing. Milestone 197 (`user/` and `xtask` are out of reach of the prover) put a replayable record
under `user/`. Milestone 193 (put `kernel/src` within reach of the prover) wrote the first two
proofs over the kernel's own source and neither carried a `Falsification:` block, which nothing
reported. Milestone 202 (every confinement test is a ritual until somebody breaks the confinement)
created `kernel/falsifications/` and nothing swept it.

The scope now comes from `cargo metadata`, which is where `script/lint`'s verify-table check
already gets the same fact. A hand-kept list is the same defect one iteration later, and
`script/verify` has recorded that failure twice: `multicast_dns_protocol` and then `jh7110_entropy`, each carrying
harnesses nothing ran, each invisible because a suite whose scope is too small goes green *faster*.

Three things fell out of following packages rather than a directory.

The module path now comes from the Cargo target rather than from counting path components. The
old derivation read `crates/<crate>/src/<...>` and would have called `components/src/printenv.rs`'s
harness `printenv::proofs::push_never_writes_past_the_buffer_it_was_given`. It is a `[[bin]]` root,
so it contributes no module segment at all, and the qualified name Kani answers to is
`proofs::push_never_writes_past_the_buffer_it_was_given`. The path and the sweep's `--exact` filter
are one fact with the separators changed, so getting this wrong would have produced a patch path
that could not be swept.

The sweep learned two package-shaped facts, both derived rather than listed. A package with more
than one binary needs `--bin`, because `cargo kani -p user` compiles all 68 programs and fails on
the first `#![no_std]` root that does not mention Kani; `script/verify` solves the same problem by
deriving a `--bin` list from the tree, and a sweep wants exactly one of them. A package containing
`global_asm!` needs `-Z unstable-options --ignore-global-asm`, or Kani refuses to compile it; that is
the kernel, and it is found by grepping for the macro rather than by naming the package.

**A record can exist for something no sweep can run, and that is not rot.** Milestone 202's patch
falsifies a kernel `#[test_case]` that boots under QEMU. §134 checks both directions precisely so a
patch cannot rot into a file nobody runs, so a patch with no harness is normally a failure. This one
is neither, and hardcoding an exemption in the script whose milestone was removing a hardcoded scope
would have been the joke it sounds like. The distinction is drawn mechanically instead: a patch may
open with ``Falsifies `<fully::qualified::name>` ``, and it is a **known gap** when that name agrees
with the filename and resolves to a function that exists in the package and carries no
`#[kani::proof]`. Everything else is rot, and the rot check stays sharp in the direction that
matters, because a renamed or deleted harness no longer resolves to a function at all. Known gaps
are printed after the ratio and counted in neither half of it.

### What the correction cost, measured

| | before | after |
|---|---|---|
| harnesses seen | 141 | 145 |
| packages seen | 24 | 26 |
| replayable | 25 (18%) | 27 (19%) |
| records nothing can sweep | 0, and one existed | 1, printed |

The ratio was expected to get worse and barely moved, which is worth stating rather than
enjoying. The four harnesses the old walk could not see are two in `user` (one already replayable,
from milestone 197) and two in `kernel`. One kernel record was written here to measure what a kernel
falsification costs, which milestone 212's block asked for before promising one: it is an ordinary
sweep entry, 3.1 seconds, restoring milestone 142's MAJOR 4 itself (`run_end_va` checking only
the addition while the multiply wraps). The `user` sweep, which exercises the `--bin` path, is 4.4
seconds. So the honest reading is not that the denominator was fine. It is that `crates/` held 97%
of the harnesses, and the number was still a claim about a scope nobody had stated.

`script/lint`'s `kani-harnesses` and `harness-crates` counts had the same defect and are fixed in
the same breath. Their derivation walked `crates/` too, with a docstring giving the reason as "the
kernel, the user programs and xtask are not packages it compiles", which stopped being true one
milestone later. Left alone, the tree would have carried two derived numbers for one question, 141
and 145, which is exactly what `<!--count:-->` exists to prevent one level up. The marker name
`harness-crates` now counts packages; it is a name and therefore calef's, and renaming it is
tracked in milestone 212's report rather than done here.

## What six real records found

The six were chosen against the cases that would embarrass the mechanism rather than a toy: milestone
191 named `capability::subset_is_reflexive` as a tautology and the `paging` harnesses as
per-ISA duplicates, so those are where a falsification convention has to earn its keep.

### The capability crate's central theorem is blind to the predicate underneath it

`derive_never_widens_rights` is the crate's flagship: *"the central theorem, on the real
`CapabilityTable::derive`."* Applying the single most plausible defect in `Rights::is_subset_of`, its
two operands swapped (`other.0 & !self.0` for `self.0 & !other.0`), and running the whole crate:

```
Complete - 10 successfully verified harnesses, 2 failures, 12 total.
Verification failed for - verification::subset_matches_allows
Verification failed for - verification::from_bits_cannot_forge_a_right
```

Ten of twelve stayed green, including the central theorem. The reason is worth stating plainly
because it generalizes: `derive_never_widens_rights` states its property *through the predicate it is
testing*. It asserts `derived.rights.is_subset_of(src_rights)` while `derive` guards on
`rights.is_subset_of(src.rights)`, so a consistently wrong `is_subset_of` satisfies both sides and
the harness cannot see the break. It is load-bearing against the **guard** being dropped, which is
what its recorded falsification does, and blind to the comparison the guard calls.

That is a real coverage gap in the crate this whole project rests on, and it was invisible to
everything: the harness passes, `kani::cover!` would pass, the tests pass, mutation testing does not
reach `cfg(kani)` code. It took applying a defect and looking at which harnesses noticed.

**Not fixed here, deliberately.** Closing it means rewriting the harness to state the property in
terms that do not call the predicate (comparing raw bits, or asserting against `allows`), and this
lane was measuring the proofs rather than rewriting them. It wants a lane, and the proposal is in
milestone 194's report.

### `subset_is_reflexive` is falsifiable and still adds nothing

It can be turned red: drop the `!` from `is_subset_of` and `a.is_subset_of(a)` fails. So it is not
decorative in the strict sense, and §134's "no plausible implementation error breaks it" is slightly
too strong.

The sharper statement is the one the sweep can support: its falsification set is a strict subset of
another harness's. Every mutation of `is_subset_of` that reaches it also reaches
`subset_matches_allows`, and the swapped-operand case above reaches `subset_matches_allows` and not
it. A harness whose red cases are a subset of another's adds no discrimination to the suite. Nothing
here proposes deleting it; the record is what lets somebody make that argument with evidence instead
of taste.

### The same shape, one crate along

`filesystem_protocol::a_grandchild_is_bounded_by_the_root` is `attenuate_never_widens` at depth two.
`attenuate` is a single `&`, so the only claim the second harness adds is that AND is associative,
and there is no plausible defect that breaks it while leaving the depth-one harness green. Both go
red on the same one-character patch (`&` becoming `|`). Recorded in the patch prose rather than acted
on.

### The patch path was wrong, and `paging` broke it on first contact

§134's first spelling was `crates/<crate>/falsifications/<harness_fn_name>.patch`, which assumes a
harness function name is unique inside its crate. In `paging` it is not. Six properties are stated
once per ISA, in `aarch64.rs`, `sv39.rs` and `x86_64.rs`, under six shared function names:

```
paging::{aarch64,sv39,x86_64}::verification::distinct_pages_take_distinct_paths
paging::{aarch64,sv39,x86_64}::verification::index_is_always_in_bounds
paging::{aarch64,sv39,x86_64}::verification::the_indices_and_offset_tile_the_address
paging::{aarch64,sv39,x86_64}::verification::the_leaf_keeps_address_and_permissions_apart
paging::{aarch64,sv39,x86_64}::verification::the_two_halves_are_disjoint
paging::{aarch64,sv39,x86_64}::verification::the_user_va_gate_admits_only_the_aligned_low_half
```

Eighteen of that crate's twenty-six harnesses, three to a filename. `cargo kani --harness
index_is_always_in_bounds` cannot separate them either, so the sweep could not have run one of them
if the file had existed.

calef amended §134 on 2026-08-31: the module path is always included, with no branch. Refused,
and it is the refusal worth keeping, was *unqualified when unique and module-qualified when not*,
which is a branch keyed on an **unstable** property, since a harness added elsewhere would
retroactively invalidate an existing path. That is the same shape as the two-tier program-naming rule
calef rejected on 2026-08-01, one domain over.

The amendment buys more than uniqueness, and this is why it was the right answer rather than the
adequate one. The qualified path is Kani's own fully qualified harness name with the separators
changed, so the sweep now filters with `--harness <qualified> --exact` instead of a substring match
on a bare function name. Under the old filter, `paging`'s three `index_is_always_in_bounds` harnesses
would all have run, and "one of them went red" would have proved nothing about the one being
falsified. The path and the filter are one fact written twice, which is the shape of a convention
that cannot drift.

`script/falsifications --check` keeps a collision check, but it now guards this script's own
module-path tracking rather than the tree: two harnesses cannot share a qualified name in Rust, so
if two ever compute the same patch path, the brace counting that derives module paths is wrong and
the sweep would prove the wrong harness.

## The sweep for self-referential harnesses

Milestone 211, 2026-09-01. This note's own line about `derive_never_widens_rights` (a harness
that states its property through the predicate it is testing cannot see that predicate break)
was written as a description of one harness. Milestone 202 then found the same shape in
`component_plan`, independently, two days later. Two instances in two crates is a class, so
every harness in the tree was asked the question.

146 harnesses swept. That is the 141 `script/falsifications` walks in `crates/`, plus the
two in `kernel/src/syscall.rs`, the two in `components/src/printenv.rs` and the one in
`vendor/redoxfs/src/node.rs`, which the walk does not reach (milestone 212 is fixing that).

11 were blind, and blind is a measurement here rather than a reading. For each one there is
a patch in the tree that applies a defect to the function the harness stated its property
through; the pre-211 phrasing was run against that patch and stayed **green**, and the rewritten
harness goes **red**. Both directions were checked for every one, because the whole point of
§134 is that a claim about a proof is not evidence about a proof.

| harness | stated through | the defect it could not see |
|---|---|---|
| `capability::derive_never_widens_rights` | `Rights::is_subset_of`, which `derive` guards on | the predicate's two operands swapped |
| `component_plan::a_plan_never_grants_a_right_the_declaration_did_not_ask_for` | `Direction::rights`, which `plan` fills the word from | `Use` routed `READ`, so a client can receive on its server's rendezvous |
| `component_plan::the_device_split_partitions_the_mappings` | `PageKind::mode`, which `plan` fills the word from | device registers mapped `MAP_RW` |
| `component_plan::dependents_finds_exactly_the_non_target_instances_that_declared_it` | `str_eq`, which `dependents` decides membership with | `str_eq` true for strings of different lengths |
| `credential_protocol::no_request_word_makes_the_parse_read_outside_the_page` | `id_len`, which `read` slices with | the identity length read from the wrong four bits of the request word |
| `direct_memory_access_validator::an_accepted_descriptor_is_confined` | `Desc::is_indirect`, which `check_descriptor` guards on | the indirect flag tested against the wrong bit, so an indirect table reaches the device |
| `jh7110_entropy::ready_requires_rand_rdy_and_carries_the_words_untouched` | `assemble`, which `interpret` calls | entropy laid out big-endian |
| `network_time_protocol::accepting_is_total_and_a_sample_is_coherent` | `Interval::is_negative`, which `accept` guards on | the predicate never true, so a negative delay is accepted |
| `paging::sv39::the_leaf_keeps_address_and_permissions_apart` | `entry_pa`, the decoder for the encoder under test | `PPN_SHIFT` moved, so both agree on the wrong bits |
| `paging::aarch64::the_leaf_keeps_address_and_permissions_apart` | the same round trip | the descriptor-type field dropped, so every leaf faults |
| `paging::x86_64::the_leaf_keeps_address_and_permissions_apart` | the same round trip | the present bit dropped |

All eleven now carry a replayable record of exactly that defect, which is the point: the
evidence that a rewrite closed a hole is a patch a machine replays, not a paragraph.
`script/falsifications` counted 25 of 141 replayable before this sweep and 33 after, measured on
this lane's own base. The tree's number is 35 of 145, because milestone 212 (`script/falsifications`
walks `crates/` only, so the ratio it prints is not the tree's) landed between the two and widened the denominator; take the merged
figure rather than either branch's.

**Three more were rewritten and are recorded as *not* findings**, which is the half worth being
strict about. `filesystem_protocol`'s two attenuation harnesses state their property through
`allows`, and `capability::a_deleted_capability_stays_deleted` states it through `get`. Each
looks like the class and is not: the sweep tried to exhibit a defect their original phrasing
misses and could not. `attenuate` does not call `allows`, and a widening `attenuate` shows up on
some input under every wrong `allows` tried; `delete` does not call `get`. They were made more
direct anyway, and their comments say plainly that no blind spot was demonstrated, because
recording a fix for a hole nobody found is the same one-step-from-evidence failure §134 exists
to end.

### What the mechanical narrowing did and did not do

The block asked for mechanical narrowing before reading 146 harnesses by hand, and it is worth
recording what that bought, because the answer is "a third of the way".

A script parsed every harness body, resolved each call against the functions defined in that
harness's crate, and flagged a harness where one function it calls appears inside another
function it calls. 23 candidates out of 146, and all 11 findings are among them, which
sounds like a win and is worth being suspicious of.

Three of the eleven, the `paging` leaf trio, were flagged by an **artifact**. The extractor
takes a function body by counting braces over a bounded window, so it sometimes swallows a
neighboring function, and that is why `entry_pa` appeared to call `leaf_entry`. It does not.
An encoder and its own decoder never call each other, so a *correct* call-graph test would have
missed all three, and the round trip between an encoder and its decoder is the same failure in
different clothes. The narrowing is therefore weaker than its hit rate suggests.

It is also noisy in the other direction: several candidates it flagged are legitimate agreement
claims, and `globally_unique_identifier_partition_table::crc32_matches_its_bitwise_definition`, which is the model of a *good* harness,
is one of them.

So the narrowing is a worklist, not a verdict, exactly as the block predicted. Every harness was
read.

### The patterns that make a harness fine, since most of them are

Grouped, because the reasons repeat and a reader deciding whether their own new harness is at
risk wants the pattern rather than 135 rows:

- **The claim is stated in arithmetic the implementation does not own.** `kernel`'s two run
  harnesses do it in `u128`, `device_tree_blob::be32_reads_big_endian_when_in_bounds` writes the four shifts
  out, `paging`'s `the_indices_and_offset_tile_the_address` reconstructs the address by hand.
  This is the shape every rewrite above converged on.
- Two independent implementations are compared on purpose.
  `globally_unique_identifier_partition_table::crc32_matches_its_bitwise_definition` is the clearest: a table-driven CRC against the
  bitwise definition, where a self-consistent wrong table is the exact defect it exists to catch.
  `capability::subset_matches_allows` and `calendar::the_calendar_algorithms_are_mutual_inverses`
  are the same move.
- The harness carries its own model. `intrusive_fifo` records push order in a local array and
  compares the queue against it; `generational_table` counts against a counter the walk does not
  use.
- The property is totality or termination, where there is no predicate to be blind to:
  `glob::matching_is_total`, `pci::the_capability_walk_terminates_on_any_device`, `elf`'s four.
- The assertion is a constant. `non_volatile_memory_express::prp_pair_is_total_and_page_disciplined`,
  `paging::x86_64::no_vtd_entry_ever_sets_a_reserved_bit`.

### A second shape, found along the way and not this milestone's

`nifefs::the_validation_implies_reads_slice_is_in_bounds` calls neither `parse` nor `read`. It
re-implements both functions' arithmetic inside the harness and proves a property of the copy.
That is not self-reference, it is a proof about code that is not the code, and it goes green
however `parse` is rewritten. Its doc comment is honest that it states the arithmetic
("Exactly parse's acceptance condition", "Exactly read's slice arithmetic"), so this is a
recorded design rather than a mistake, and closing it means restructuring `nifefs` so the
condition is a function both the parser and the harness can call. Raised as **proposed milestone
213 (provisional number)**: a harness that duplicates the implementation instead of calling it.

## The sweep for harnesses that duplicate the implementation

Milestone 213, 2026-09-02. Milestone 211's sweep found this shape beside the one it was looking
for and could not fix it in passing, because the repair is a restructuring of the code rather
than a rewrite of a harness. 211 asks whether a harness states its property *through* the
function under test. This asks a blunter question: is the function under test in the harness at
all?

148 harnesses read, one by one. That is `script/falsifications`' 146, which now follows
`cargo metadata` rather than a directory walk, plus the two in `vendor/redoxfs/src/node.rs`,
which is vendored and in no workspace package. No mechanical narrowing was attempted and none is
offered: 211 measured its own extractor flagging three findings for the wrong reason and missing
an entire family, and the cheapest honest thing here was to read them.

### The discriminator, which is the hard part

Some duplication is the point. 211's own eleven rewrites all converged on restating a property in
raw bits, in the wire format's own shifts, or in architecture bit positions as literals,
*precisely so* the harness cannot repeat the implementation back at itself. A sweep that treated
recomputation as the defect would undo that work. So the question is not whether a harness
recomputes something. It is:

> Which side of the assertion did the crate produce?

An assertion compares a **subject** with an **expectation**. The crate under test must produce the
subject, because nothing else in the harness can be broken by breaking the crate. The expectation
should come from somewhere the implementation does not get to choose, which is exactly why it is
so often a recomputation, and that recomputation is the virtue. The defect is the case where the
crate produced **neither** side: no rewrite of the code can then change whether the assertion
holds.

`device_tree_blob::be32_reads_big_endian_when_in_bounds` writes the four shifts out and compares them against
`be32`'s answer: subject from the crate, expectation from the format. `nifefs`'s harness wrote
both sides itself. Same shape at a glance, opposite in what they prove.

A second question, for duplication on the assumption side, where a harness restates a guard in
order to reach the state it wants: which way does drift fail? If the implementation moves and
the harness's assumed set becomes *wider* than the code's accepted set, the harness asserts on
inputs the code now refuses and goes red, which is the safe direction.
`direct_memory_access_validator::an_oversized_batch_is_refused` restates the batch guard's condition
and fails this way. If drift makes the assumed set narrower, or if the assertion is the harness's
own arithmetic too, it goes green and says nothing. That is the `nifefs` case, and it is why the
implication in it was the whole harness rather than a detail of it.

### The finding: one, and it is the one the block named

`nifefs::the_validation_implies_reads_slice_is_in_bounds` called neither `Fs::parse` nor
`Fs::read`. It recomputed `parse`'s acceptance condition as its antecedent and `read`'s slice
arithmetic as its consequent, under two comments claiming each was "exactly" what the code did,
and proved that a copy of `read`'s arithmetic stays inside a copy of `parse`'s bound.

Measured in both directions, as 211 did. `read` was changed to slice from
`(start_block + DIR_BLOCKS) * BLOCK`, a whole directory span past what `parse` accepted, which is
a defect the crate's own module comment invites (`start_block` is absolute, and a reader who
"helpfully" added the directory offset would write exactly this). The old phrasing verified in
0.04 seconds. The rewritten harness goes **red** on the same defect, and the patch that restores
it is `crates/nifefs/falsifications/verification.the_validation_implies_reads_slice_is_in_bounds.patch`.

**Honest caveat, because this one does not read the way the other eleven did:** the crate's host
tests *do* catch that defect, seven of them. So it is a blind spot in the proof rather than a hole
in the crate, and the falsification record is evidence about the harness, which is all §134 ever
claimed for one. What the harness was for is the part the tests cannot reach: every image, at
every length, including one a kernel is handed off a disk rather than one `write_image` produced.
Its own doc comment called that "the kernel-facing guarantee, free of any bound on the image
size", and that is the sentence the old phrasing did not deliver.

### The repair earned its keep, and the block was right to make that the first question

213's BUGS warned that factoring a condition out to make it provable adds a function whose only
caller-visible reason is the proof, which §46 refuses. That is not what happened here, and the
reason is worth stating because it is the test to apply next time.

The duplication was in the implementation before it was in the harness. `parse` validated with
three lines and `read` sliced with a copy of the same three lines, and `read`'s unchecked
`image[start..start + len]` was sound only because a reader could see the two matched. So
`Fs::entry_bounds` does not exist for the prover. It has two ordinary callers, it turns an
invariant that lived in a reader's head into one expression, and `read`'s `expect` now names why
the failure cannot fire. The prover benefit is a consequence, which is exactly the standing
`elf::check_segment_bounds` has one crate over.

The test the next lane should apply: would this function be worth extracting if there were no
harness? If the answer is no, §46's refusal stands and the honest outcome is a recorded
limitation rather than a split.

### The negative half: two that have the shape and are not findings

Recording these is not modesty. 211 found its own first draft wrong about one of these once, and
a sweep that reports only its hits is unfalsifiable prose.

`credential_protocol::a_request_word_round_trips_every_field` is blind on its own, and covered by
its neighbor. It is an encoder round-tripped through its own decoder, which is 211's third
family. Swapping the identity and secret length fields in `req` and in `id_len`/`secret_len`
together leaves it verifying: both sides move and the round trip is perfect, while a client using
the real wire format has its identity length read as its secret length. It was left alone, with
the measurement written at the harness. The defect is caught next door by
`no_request_word_makes_the_parse_read_outside_the_page`, which 211 rewrote to state the two shifts
as literals, and which goes red on exactly that patch. Restating the shifts twice would be a
second copy of a claim one harness already pins. What that costs is a fact about the suite
rather than the harness, so weakening the sibling silently un-covers the wire format, and the
comment now says so where a reader meets it.

`kernel::every_page_between_the_checked_ends_is_itself_a_user_page` models two call sites, and
the model is faithful today. It restates the guard `page_frame_map` and `MAP_INTO` apply before
their map loops. Both were read on 2026-09-02 and both apply exactly it. This is the shape and
cannot be repaired the way `nifefs` was, because what is duplicated is a *caller's* control flow
rather than a function: there is nothing to extract and call. It stays, with the check and its
expiry recorded at the harness.

Two more were looked at and dismissed on sight, and are named so nobody re-derives them.
`direct_memory_access_validator`'s `walk` and `inter_process_communication`'s `seed` look like
harness-side reimplementations and are not: `walk` calls the real `shadow_one_head` and `seed`
builds its symbolic state through the real `push_back`. `component_plan::declares_by_core_eq` is the
good version in its purest form, an independent implementation of `str_eq` standing on the
expectation side on purpose.

`script/falsifications` reads 36 of 146 after this lane, from 35.

## `machine_discovery` swept clean, and the gap was a harness that did not exist

2026-09-23. The crate parses ACPI tables and device trees, which is the bytes a machine hands us,
so it was taken as a falsification target on the reading that `script/falsifications --unfalsified`
would name several of its harnesses. It named none. All fifteen already carried `replayable`
records, written by milestone 319 (the crate that parses firmware), and
`script/falsifications --sweep machine_discovery` replayed all fifteen on patagonia: 15 swept, 0
survivors, 0 stale or broken. Nothing here needed repair.

Two of those fifteen were false when they were written, which is the strongest evidence in this
note against fatal risk 2's claim that the proofs prove trivia.
`acpi::the_dmar_fixed_part_decodes_without_arithmetic_overflow` found `host_address_width` computing
`body[0] + 1` in a `u8`, which panicked the x86 boot path on a firmware byte of `0xff`, and
`an_ecam_windows_size_is_total_and_counts_one_mebibyte_per_bus` found the window size wrong beside
it. Both are on a boot path, both were found by the proof rather than by a test, and both are
recorded at the harness.

### What was actually missing

Milestone 524 (the three `x86_64` boot gates) added 342 lines to `x86_64.rs` on 2026-09-21 and no
harness with them, so nothing was `unfalsified`: the code was *unproved*, which no ratio in this
note can see. That is worth stating as its own shape, because it is the cheaper failure to have and
the harder one to notice. The worklist counts harnesses, so code with no harness is invisible to
it, and a crate can go to 100% falsified by adding nothing and proving nothing.

The gap closed here is `a_feature_is_never_reported_from_a_leaf_the_part_does_not_answer` (name
provisional). `CPUID` has no fault for a leaf a part does not implement: a read above the maximum
answers with some other leaf's data, so absence and a clear bit are the same bits on the wire, and
the four maximum-leaf gates in `Isa::decode` are the only thing telling them apart. Its falsification
gates an extended leaf on the standard maximum, one word of difference that every machine in this
building agrees with, which is why a proof and not a test is what catches it.

## BUGS

- The weekly sweep in CI has swept nothing since it was written, and reported success every
  time. Found 2026-09-23 by dispatching `falsifications.yml` on a lane branch and reading the log:
  `script/falsifications --sweep refuses to run on a dirty working tree`, because the step is
  `script/falsifications --sweep 2>&1 | tee sweep.txt` and the redirection creates `sweep.txt` in the
  checkout **before** the script reads `git status`, which counts an untracked file as dirty. The
  refusal returns 2, `tee` is the last command in the pipe, so the step exits 0 and the job is green.
  Every scheduled run since 2026-08-31, when the workflow was written, has this in its log. The
  refusal is the script working exactly as its own BUGS section says it should; what failed is a
  report that says nothing and looks like a report that says everything, which is the shape this
  whole note exists to refuse, one level out. The fix is to write the transcript outside the
  checkout (`$RUNNER_TEMP`), and it is a workflow edit rather than a script one, so it is recorded
  here rather than taken by the lane that found it (which was confined to `crates/machine_discovery`).
  Until it is fixed, a `replayable` record older than the last hand-run sweep is a claim nothing
  has re-checked.

- The self-referential sweep cannot be a gate and cannot be repeated cheaply. No check
  distinguishes "asserts through the function under test" from "legitimately asserts agreement",
  which is why milestone 211 is a sweep with a worklist rather than a lint. Its per-finding
  evidence is a falsification patch, so the weekly sweep does re-run it, but a harness found
  *fine* today carries no artifact at all and will go quiet if the code under it is refactored
  into the shape it was cleared of.
- **Eleven findings is a floor, not a count.** Each one is a defect somebody thought of. A
  harness with no demonstrated blind spot is a harness nobody has broken the right way yet, and
  the three recorded above as "not findings" are exactly that claim made honestly rather than a
  clean bill of health.

- Nothing forces the ratio upward. A harness may sit at `unfalsified` for ever with `script/lint`
  green; the backlog reached zero on 2026-10-06 by work, not by gate. The ratio the reporter prints
  is the number that matters.
- **A recorded falsification proves the harness catches *that* defect, not the class.** It is a
  floor, and a low one. `derive_never_widens_rights` above is the worked example of exactly this: it
  now carries a green record and a documented blind spot at the same time, both true.
- A patch rots against refactors. §134 argues that is correct rather than harmless, and the
  sweep reports a stale patch as a failure rather than a skip. It is still churn, and a heavily
  refactored crate re-falsifies often.
- `--sweep` refuses to run on a dirty working tree. It applies and reverts with `git apply`, and
  a failure mid-run would otherwise leave a deliberate defect in somebody's source. So you cannot
  sweep while you work, which is a real limit and not a preference.
- **`--affected-since` is crate-granular, not harness-granular.** A change anywhere in a harness
  crate's dependency closure re-falsifies every replayable harness in that crate, because nothing in
  this tree knows which lines a harness covers. That is what an IVC would have bought. It fails
  toward doing too much, which for a run measured in seconds is the right direction.
- **Module paths come from brace counting, not from a Rust parser.** A brace inside a string or a
  comment at module scope would miscount and produce a wrong patch path. Nothing in this tree does
  that, and the failure surfaces as a path `--check` reports rather than as a silently wrong sweep,
  but it is a real limit of a 30-line derivation standing in for `syn`.
- `kernel/src/arch/` carries no harnesses and gains nothing here. The architecture layer is
  outside this record entirely, the same scope gap §134 (a harness carries a machine-replayable
  falsification record, or it is not evidence) names. (This entry used to say that is where "the
  VisionFive 2's undelivered-wake defect actually lived." That reading is **retracted**,
  `notes/visionfive2.md`'s fifth bench stop, 2026-08-15: it was a completed tour's terminal state,
  not a stranded receiver, and never happened. Found still repeating it here 2026-09-23; §134 lives
  in `design/decisions/` and was left for its own lane to correct, see that section's own text.)
- A file's module path comes from the Cargo target it belongs to, and one shape defeats that.
  `components/src` holds 49 `[[bin]]` roots and two single-consumer `#[path]` modules that rule 7
  permits; `fixtures/src` holds 23.
  A root contributes no module segment; a `#[path]` module contributes whatever the including file
  calls it, which need not be its filename. Neither carries a harness today, and one that arrived in
  a `#[path]` module would get a patch path naming the file, which `--check` reports as a mismatch
  rather than accepting.
- A kernel `#[test_case]` falsification is recorded and unswept, and the section above says how
  it is told apart from rot. This bullet said it stays that way "until milestone 210 (no kernel test
  can be run by name) lands"; 210 landed on 2026-08-31, and `cargo xtask test --test <substring>`
  now filters the kernel suite, so the sweep this was waiting on is affordable and nobody has run
  it. That is a lane rather than a limitation now.
- Vacuity is still unguarded. §134 recommended option A (a lint requiring `kani::cover!`)
  alongside option C, and this milestone built only C. 23 `cover!` sites across 4 of 24 harness
  crates is the current state, from milestone 191. That wants a lane.
- One finding is a floor too, and a lower one than eleven was. 213 asked a narrower question
  than 211 and got a narrower answer, and the honest reading is not "the tree has one such
  harness". It is that one harness had the shape in its pure form, where the crate is on neither
  side of the assertion. The mixed cases are the ones a sweep is bad at: a harness whose subject
  comes from the crate can still rest on a recomputed assumption, and telling a safe restatement
  from an unsafe one took reading the code both times rather than applying the rule.
- The sweep leaves no artifact on the 147 harnesses it cleared, the same limit 211 records.
  A harness that is fine today becomes a duplicate the moment somebody inlines the function it
  calls, and nothing says so. The two recorded above are the exception only because their reasons
  are written at the harness rather than here.
- A model of a caller has no repair and no expiry date. The kernel harness above is faithful
  because somebody read two call sites on one day. Nothing re-reads them, and the failure is
  silent in the worst direction: the harness stays green while proving a claim about a guard the
  kernel no longer applies.
- A hand dispatch of the weekly sweep cancels any other branch's. `falsifications.yml`'s
  `concurrency` group is the bare string `falsifications` with no ref in the key, and it carries
  `cancel-in-progress: true`. That is right for the cron, where a queued weekly run has nothing to
  say that the newer one will not say better, and wrong for `workflow_dispatch`: on 2026-09-23 three
  falsification lanes dispatched the workflow within twenty minutes of each other and the second
  killed the first. The run it killed was the only one in the workflow's life that would have
  replayed a patch. Check `gh run list --workflow falsifications.yml` for a run in progress before
  dispatching, and re-dispatch if yours is cancelled. See
  `notes/corrections/2026-09-23-the-sweep-that-swept-nothing.md`.
