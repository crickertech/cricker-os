---
status: NOT-STARTED
raised: 2026-09-14
promoted_from: one-home-for-the-trap-on-false-helper
milestone_dependencies: none
decision_dependencies: 188
machine_requirements: none
specific_machine: none
needs_person: no
---
# 408. One home for `fn check(ok: bool)`, which nine programs now write out by hand

Promoted from the proposal `one-home-for-the-trap-on-false-helper`, filed
2026-09-14 by milestone 291's lane, which added seven of the nine copies and said so rather than
leaving the count to be re-derived. *(Number provisional until the merge queue lands it.)*

The decision is
§188 (what the lifted `fn check(ok: bool)` is called, now that nine programs write it out by hand), written up
2026-09-19 by milestone 435's slice-c lane because this gate named no section. The obvious home is
`crates/user_rt`, and a public function name there is an architect's (AGENTS.md, "calef names the
crates, the programs, and the shared modules", extended to public function and method names on
2026-08-23).

**Only the name is open, and that is a narrowing this block did not have.**
[§94](../decisions/94-what-may-live-in-a-library.md) already decided the lift: it asks what
the language forces to be per-binary and lifts everything else, and its own tell is this case
exactly, *"a per-binary item whose body is copied verbatim into every binary. If the body is
identical everywhere, it is not per-binary; only its declaration is."* It was written about 58
copied panic handlers; nine copied `check`s is the same shape at a seventh the scale.

**Premise re-checked 2026-09-19, still true, with two corrections.**
`grep -rn 'fn check(ok: bool)'` finds nine copies, the same count, and not the same nine:
`fixtures/src/memory_region_depleter.rs` no longer carries one and `fixtures/src/hello.rs` does.
`components/src/block_driver.rs` still reaches the trap through `panic!()` rather than
`user_mode_runtime::trap()`, so the two spellings of "say no" this file names have not converged.
And the crate called `crates/user_rt` below is `crates/user_mode_runtime` since milestone 285, which
changes where the function would go and nothing about the decision the gate names.

## The duplicate

Every one of these is the same three lines, and they all mean *this program's only way to say no is
to die where the mistake was, because a failed check must be indistinguishable from a broken
program*:

```rust
fn check(ok: bool) {
    if !ok {
        user_rt::trap()
    }
}
```

Nine copies as of milestone 291: `components/src/block_driver.rs` (which panics instead, reaching
the same trap through `panic_handler!`), `fixtures/src/fs_test_client.rs`, and the seven fixtures
291 created (`console_test_client`, `memory_region_depleter`, `page_frame_producer`, `call_server`,
`frame_revoker`, `rendezvous_minter`, `rendezvous_peer`). `crates/loaded_image_check` has a tenth,
private, for its own use.

**This is not a `#[path]` module and `script/lint` check 5 will never see it**, because nobody
shared a file: each copy was typed again. That is the cheaper failure of the two and it is still a
failure, since the nine can drift on what "say no" means (two of them already do: a panic and a
trap are not the same path, though they arrive at the same instruction).

## Why it is a proposal and not a change

291 chose the duplicate deliberately, because the alternative was adding a public function to
`user_rt` in a lane already touching two archive tables and the filesystem's directory geometry,
and because `user_rt` is the crate every program in the tree links. The existing two copies were
the tree's established pattern; following it kept the change reviewable. The argument for fixing it
is that "fewer places to be wrong" is AGENTS.md's own definition of elegance, and nine is past the
point where a pattern is a convention.

The name is the whole decision. `check` is one of the generic words AGENTS.md names as a failure
mode: half the tree checks something. `require` is the kernel's own word for the same shape
(`trust::require` halts on a mismatch). `insist` and `must` were considered and are worse for
opposite reasons: the first is unusual enough to need explaining, the second reads as a modal verb
at the call site. A public name on `user_rt` is also the most-read function name this tree could
add, which is a reason to spend a decision on it rather than to take one.

## Index row

Nine programs each write out the same three-line `fn check(ok: bool)` over
`user_mode_runtime::trap()`, typed again rather than shared, so `script/lint` check 5 never sees
them: seven arrived with milestone 291's split of `hello` into programs, and two of the nine already
disagree about what saying no means, since `components/src/block_driver.rs` panics where the rest
trap. The obvious home is `crates/user_mode_runtime`, which every program in the tree links, and
that makes this the most-read function name the tree could add. `check` is one of the generic words
design/naming.md calls a failure mode, `require` is the kernel's own word for the same shape, and
the name is the whole decision, which is why this is gated on calef rather than on a lane.
