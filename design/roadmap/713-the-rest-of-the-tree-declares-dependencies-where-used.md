---
status: NOT-STARTED
raised: 2026-10-03
promoted_from: the-rest-of-the-tree-declares-dependencies-where-used
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 713. The rest of the tree declares its dependencies where they are used

Promoted from `design/roadmap/proposals/the-rest-of-the-tree-declares-dependencies-where-used.md` on 2026-10-03 (UTC). The number 713 was minted by the maintainer in a batch promotion of the proposal pile and is provisional until the queue lands it. *(Title and slug are drafts.)*

<!-- writing-standards: exception. Granted 2026-10-03 (UTC) by the maintainer minting this milestone, not ratified by an architect. Reason: this block was promoted unedited from design/roadmap/proposals/, which the prose scope excludes, so it meets the sentence and bold limits only after an edit that promotion does not make. Trimming it is a separate pass, and the exception goes when it is done. -->

Raised 2026-10-03 (UTC) by the lane that built milestone 631 (the kernel declares each dependency
where it is used), from its own `script/lint` transcript. That milestone covered the kernel only.
The same `cargo::unused_dependencies` lint (new in `nightly-2026-10-02`, warning-only) fires on two
other manifests during `script/lint`'s clippy passes:

| Manifest | Dependency | Pass that warns |
|---|---|---|
| `crates/user_mode_runtime/Cargo.toml` | `counter_frequency_protocol` | `clippy: kernel + user + user_mode_runtime (aarch64)`. Its own comment says only `cntfrq`'s x86_64 arm reads it and that it is unconditional on purpose. |
| `redoxfs_server/Cargo.toml` | `manifest_note`, `address_space_map`, `globally_unique_identifier_partition_table`, `entropy_protocol` | `clippy: redoxfs_server (its own workspace)`, which is a host build. Whether these are unused everywhere or only on the host was not checked. |

Nothing fails: the lint is warning-only and `-D warnings` does not promote it. The work is to
decide each one the way milestone 631 did: a target table where the split is by architecture, a
removal where the dependency is dead, or a `BUGS` entry beside it where the warning is the cheaper
truth. `user_mode_runtime`'s comment already argues for keeping it unconditional, so that one may
end as a recorded exception rather than a move.

## Index row

`cargo::unused_dependencies` warns on `user_mode_runtime` and `redoxfs_server`, outside the kernel that milestone 631 covered. Proposed: declare those dependencies where they are used.
