---
status: BUILT
raised: 2026-08-23
built: 2026-08-25
---
# 158. Build the eleven kernel object and identifier renames DECISIONS §113 decided

Minted 2026-08-23, from calef asking what the milestone was that renamed
the kernel components and finding the answer was "none": DECISIONS §113 (eleven kernel object and
identifier names move from contraction or borrowed jargon to the plain, standard term) was decided
2026-08-23 and never turned into tracked build work. Checked directly before minting this:
`kernel/src/cap.rs`'s `Object` enum still read
`Aspace(u64)`, not `AddressSpace(u64)` -- the decision was real, the rename was not, until each of
the seven names below was renamed in its own lane. `Untyped` -> `MemoryRegion` was the last.

## What was built

`Endpoint`/`EpId`/`EpFail` -> `Rendezvous`/`RendezvousId`/`RendezvousFailure`, the biggest,
most-cited name, done to completion per this block's own instruction to do that one first and
measure before committing to the rest. **The real surface was much larger than this block's own
table**, once re-measured fresh rather than trusted from 2026-08-23's count: not just the 82
occurrences of the type name across 22 `.rs` files, but a whole ABI module
(`abi::endpoint` -> `abi::rendezvous`, the `SEND`/`RECV`/`SEND_CAP`/`RECV_CAP`/`CALL`/`REAP`/`SURVEY`
method-number namespace every user program and the kernel's `syscall.rs` dispatch on), an
`objtype::ENDPOINT` -> `objtype::RENDEZVOUS` object-kind discriminant, `MAX_ENDPOINTS` ->
`MAX_RENDEZVOUS`, and a family of lowercase function/field names spelling the same concept
(`create_endpoint` -> `create_rendezvous`, `endpoint_cap` -> `rendezvous_cap`, `endpoint_of` ->
`rendezvous_of`, the `endpoints` field on `IpcTables` -> `rendezvous_table`, and several test-only
helpers and test names). `EpId` alone was 216 occurrences across 43 files once the type alias's own
call sites were counted, well past the milestone doc's "follows Endpoint's rename" note.

Scoped deliberately to the identifier surface (types, functions, constants, module paths, struct
fields, test names, backtick-quoted doc-comment citations of those identifiers), the same rung
milestone 98's `IpcTables` rename drew: informal lowercase prose that uses "endpoint" as a
descriptive English word without naming a specific identifier (the bulk of it in `kernel/src/user.rs`
and the consumer crates/programs that merely narrate "an endpoint capability") was left alone in
files outside the object's own implementation, matching 98's own recorded scope call rather than
sweeping every prose sentence tree-wide. `crates/glob`'s unrelated "endpoint" (a character-range
boundary) was checked and correctly left untouched -- a real in-tree homonym, not a hit.

Verified: every host-testable crate builds and its full test suite (including doctests) passes; the
kernel builds clean for both `aarch64-unknown-none-softfloat` and `riscv64imac-unknown-none-elf`;
`user`, `fs_server` and `xtask` build clean; the two Kani-proof-bearing crates that reference the
renamed type (`crates/inter_process_communication`, 6 harnesses, and `crates/capability`, 12 harnesses) both verify
successfully post-rename, satisfying milestone 69's proof obligation. `notes/ipc-naming.md` updated
to describe `Rendezvous` as the current name, with a provenance line for a reader who remembers
`Endpoint`, and its "Family resemblance" section left untouched because it never named the old
identifier (it compares the underlying *model* to Mach's port and QNX's channel, not the type name).

`Tcb`/`Tid` -> `ThreadControlBlock`/`ThreadId`, with companions `TcbPtr` -> `ThreadControlBlockPointer`
and `TidSet` -> `ThreadIdSet`, done to completion in a separate lane (milestone/158-tcb-rename). The
strict PascalCase count landed close to this block's own stale table for once (`Tcb` was 12 files/33
occurrences against an estimated 12/28; `Tid` was 13 files/82 against an estimated 12/76), but that
comparison undersells the real surface the same way it did for `Endpoint`: neither of those counts
saw the lowercase identifier family at all. Real surface, re-measured: an ABI module
(`abi::tcb` -> `abi::thread_control_block`, the `CONFIGURE`/`CAP_INSERT`/`START` method-number
namespace every spawn path in the kernel and every child-building user program dispatches through),
an `objtype::TCB` -> `objtype::THREAD_CONTROL_BLOCK` object-kind discriminant, and a family of
lowercase function and field names spelling the same concept (`tcb_cap` -> `thread_control_block_cap`,
`create_tcb` -> `create_thread_control_block`, `configure_tcb` -> `configure_thread_control_block`,
`tcb_insert_cap` -> `thread_control_block_insert_cap`, `start_tcb` -> `start_thread_control_block`,
`tcb_ptr` -> `thread_control_block_ptr`, `tcb_configure`/`tcb_cap_insert` in `syscall.rs`'s dispatch,
`tcb_start` in `supervision_proto` and `system_initializer`, and the `Thread` struct's own
`tcb_kmem`/`tcb_region` fields), plus the `Tid` half's own compound names (`current_tid` ->
`current_thread_id`, `set_current_tid` -> `set_current_thread_id`, `write_tid` -> `write_thread_id`
in `crates/ps`, and five test names built the same way). 47 files changed, 385 insertions, 376
deletions.

Left alone, matching `Endpoint`'s own scope call: bare short local variable and parameter names
that are exactly `tid` or `tcb` with no further word attached (the same restraint the `Endpoint`
rename used, keeping `ep` unchanged throughout `kernel/src/sched.rs`); informal lowercase "tcb"/"TCB" prose describing
the concept in English rather than naming the identifier (`fixtures/src/hello.rs`'s "(endpoint | aspace
| tcb)" list stayed lowercase, matching how "endpoint" stayed lowercase there after that rename);
and one historical citation in `crates/abi/src/lib.rs`'s own crate-naming rationale, which names
`Tcb`/`Aspace`/`Untyped` as the abbreviations a naming review "sank" -- renaming that citation to
`ThreadControlBlock` would have made the sentence describe the winning name as the one that lost,
so it was reverted to the old name it is actually citing. `notes/tcb.md` (the object's own note)
got a provenance line the same way `notes/ipc-naming.md` did; its "acronym collision" section (TCB
as Thread Control Block vs. Trusted Computing Base) is unaffected either way, since both senses are
already informal usage. Verified the same four ways as `Endpoint`: full host test suite green
(including doctests), both kernel targets build clean, `user`/`fs_server`/`xtask` build clean, and
`crates/inter_process_communication` (6 harnesses) plus `crates/capability` (12 harnesses) both re-verify under Kani
post-rename.

`Aspace`/`AddressSpace`, plus its decided companion `FreeVas`/`FreeAddressSpace`
(`kernel/src/thread.rs`), done to completion in its own lane per this block's sequencing rule. This
block's own table estimated 9 files, 17 occurrences for the bare `Aspace` type token; a fresh count
against the same measure (case-sensitive `Aspace` alone, before starting) found 18 occurrences across
9 files, confirming the table's narrow count was accurate for what it measured. The real surface,
once every case form and every compound identifier was counted, was far larger, the same pattern
`Endpoint` set: 391 case-insensitive occurrences of "aspace" across 37 `.rs` files, not 17 across 9,
because the type name was the smallest part of the surface. It touched: a whole `abi::aspace` ->
`abi::address_space` ABI module (`MAP_INTO`/`MAP_RO`/`MAP_RW`/`MAP_CODE`/`LIST`, the method-number
namespace every user program and `kernel/src/syscall.rs`'s dispatch use); `objtype::ASPACE` ->
`objtype::ADDRESS_SPACE`; the `Object::Aspace` capability variant -> `Object::AddressSpace`; a family
of lowercase kernel functions (`aspace_cap` -> `address_space_cap`, `user_aspace_create` ->
`user_address_space_create`, `user_aspace_map` -> `user_address_space_map`, `user_aspace_root` ->
`user_address_space_root`, `take_user_aspace` -> `take_user_address_space`,
`reap_aspaces_in_region` -> `reap_address_spaces_in_region`, `readopt_user_aspace` ->
`readopt_user_address_space`, `aspace_list` -> `address_space_list`); a whole kernel test-infra module
file (`kernel/src/user/aspace_service.rs` -> `kernel/src/user/address_space_service.rs`, with its
`aspace_builder`/`ASPACE_BUILDER` sibling in `fixtures/src/hello.rs` renamed to
`address_space_builder`/`ADDRESS_SPACE_BUILDER` to match); a lock-rank constant
(`kernel/src/sync.rs`'s `ASPACES` -> `ADDRESS_SPACES`); and per-program constants
(`components/src/pmap.rs`'s `ASPACE_SLOT` -> `ADDRESS_SPACE_SLOT`,
`fixtures/src/os_primitives_benchmarker.rs`'s `MAP_ASPACE` -> `MAP_ADDRESS_SPACE`). The `FreeVas`
companion (`kernel/src/thread.rs`'s stack-VA-range free list, an unrelated subsystem reached by the
same abbreviation per DECISIONS §113) was 10 occurrences across 3 files (`thread.rs`, `stack.rs`,
`sched.rs`); renamed `struct FreeVas` -> `struct FreeAddressSpace` and
`static FREE_STACK_VAS` -> `FREE_STACK_ADDRESS_SPACE`, and left the struct's private `vas: [u64; 128]`
field alone (it holds individual freed stack virtual addresses, not address-space objects, and §113
named only the struct and the static, not this field).

**A real near-collision, not previously recorded, checked and resolved rather than a blocker**:
`kernel/src/user.rs` already defines `pub struct AddressSpace` (the kernel-internal page-table object
a process's memory actually is, with its own `Drop`, `map_physical`, etc.), predating this rename.
Renaming `Object::Aspace` to `Object::AddressSpace` gives the capability variant the same name as the
object it names -- but that is not a collision, it is the same pattern this tree already uses for
`Object::Rendezvous(RendezvousId)` naming `struct Rendezvous` in `sched.rs`: a capability variant and
the kernel struct it points at sharing a name, in different modules, is how this tree already reads
"a capability that names one of these." No import collision results (the variant holds a `u64`, not
the struct), and the pairing is arguably more consistent post-rename than `Aspace`/`AddressSpace`
were pre-rename.

Scoped the same way `Endpoint` was: identifier surface (types, functions, constants, module paths,
struct fields, module file names, backtick-quoted doc-comment citations of those identifiers) renamed
throughout; local variable and parameter names that merely *spell* the concept (`aspace`,
`aspace_name`, `aspace_slot`, `b_aspace`, `as_region`) were left alone, matching the precedent's own
treatment of `ep` (never renamed to `rdv` when `Endpoint` became `Rendezvous`) -- a local binding is
not in the milestone's own listed scope, however it happens to be spelled. Prose (comments, panic and
`.expect` message strings) was converted from "aspace" to "address space" throughout, a broader sweep
than `Endpoint`'s (which left much of `kernel/src/user.rs`'s and the test files' prose alone): "aspace"
is not standard English the way "endpoint" is, so leaving it anywhere the rename touched would have
left the exact contraction DECISIONS §113 is retiring sitting in prose next to its own renamed
identifier. `notes/abi.md`, `notes/frames.md`, `notes/object-revocation.md`, `notes/process-view.md`
and `notes/thread-spawn-fork.md` updated for the same reason `notes/ipc-naming.md` was; two genuinely
historical citations were deliberately left alone: `crates/abi/src/lib.rs`'s crate-naming-ratification
note ("...the actual test (does the architect have to ask what this means, which sank `Tcb`/`Aspace`/
`Untyped`)...", 2026-08-23, describing the state of the tree at the moment that test was applied), and
`design/decisions/114-aspace-enumerate.md`, a DECIDED record whose own text already parenthetically
notes the pending rename and was left as the point-in-time record it is, matching how the `Endpoint`
rename left `26-fault-endpoint.md`/`41-endpoint-as-broker.md`/`91-endpoints-before-the-refusal.md`
untouched.

One self-inflicted bug during the build, caught by the crate's own build and fixed before any commit
existed to hide it: a first attempt at the prose sweep used a single blanket word-boundary
substitution that did not distinguish comments and string literals from live code, and briefly turned
every local `aspace` binding (`let aspace = ...`, `aspace_slot` parameters, `aspace.rights` field
access) into the syntactically invalid `let address space = ...`. `cargo check` on every touched crate
caught all of it before anything was committed; recorded here because it is exactly the kind of mistake
the ladder in this file's own conventions expects a gate to catch rather than a person to have avoided,
and it did.

Verified the same way `Endpoint` was: `script/lint` clean; `cargo test --workspace` (excluding the
bare-metal `kernel`/`user`/`fs_server` targets) green, 142 test-suite runs including every doctest;
the kernel builds clean for `aarch64-unknown-none-softfloat`, `riscv64imac-unknown-none-elf` and
`x86_64-unknown-none` (the last is new since `Endpoint`'s rename: milestone 161 item 4 gave x86_64 a
real kernel test leg); `cargo xtask swish-check` green on both aarch64 and riscv64; the two
Kani-proof-bearing crates that touch the renamed `abi::address_space` module
(`crates/capability`, 12 harnesses, and `crates/component_plan`, 5 harnesses) both verify successfully
post-rename; and the full `script/test` suite green on aarch64, riscv64 and x86_64.

`Untyped` -> `MemoryRegion`, done to completion in a separate lane (milestone/158-untyped-rename).
**This block's own table undersold it by roughly an order of magnitude on files touched**: the
strict capitalized-token count (25 files, 60 occurrences) was the pre-rename estimate; the actual
surface, re-measured fresh, was 88 files, 620 insertions/618 deletions, once the predicted-and-real
lowercase identifier family was counted the same way `Endpoint`'s and `Tcb`'s were. The crate this
block already flagged as having "independently converged on 'region' vocabulary" turned out to be
`crates/memory_regions` (not `crates/regions` as this block's own text said -- that crate had
*already been renamed* by an earlier, unrecorded pass, so its own doc comments citing
`kernel/src/untyped.rs` and `crates/regions` were themselves stale and got fixed in the same lane).
Real surface: an ABI module (`abi::untyped` -> `abi::memory_region`, the `MAP`/`RETYPE`/
`RETYPE_OBJ`/`SPLIT`/`DESTROY` method-number namespace nearly every user program's syscall wrappers
dispatch through), a `rank::UNTYPED` -> `rank::MEMORY_REGION` lock-rank constant (`kernel/src/sync.rs`;
no `objtype::UNTYPED` exists, since untyped/region capabilities are not themselves a retype target),
a kernel module file rename (`kernel/src/untyped.rs` -> `kernel/src/memory_region.rs`, and
`kernel/src/user/untyped_service.rs` -> `kernel/src/user/memory_region_service.rs`), and a large
family of lowercase function/field/const names spelling the same concept: `untyped_cap` ->
`memory_region_cap`, `untyped_cap_rights` -> `memory_region_cap_rights`, `untyped_root_cap` ->
`memory_region_root_cap` (`kernel/src/cap.rs`); `untyped_map`/`untyped_retype`/`untyped_retype_obj`/
`untyped_split`/`untyped_destroy` -> their `memory_region_*` equivalents (`kernel/src/syscall.rs`'s
dispatch, and repeated local wrapper fns of the same names in `crates/system_initializer`,
`crates/supervision_proto`, and several `user/src/*.rs` programs that each define their own tiny
syscall wrapper); `UntypedHeap` -> `MemoryRegionHeap` and `untyped_slot`/`UNTYPED_SLOT`/
`NET_UNTYPED_SLOT` -> their `memory_region`-spelled equivalents (`crates/user_rt/src/heap.rs`,
`crates/user_rt/src/lib.rs`, `patches/std-nife`'s std overlay); and two test names
(`a_process_spends_untyped_and_the_kernel_never_allocates`,
`a_process_runs_alloc_collections_on_its_own_untyped`).

Scoped identically to `Endpoint`'s and `Tcb`'s own rung: identifiers, module paths, backtick-quoted
doc-comment citations, and test names were renamed; informal lowercase "untyped" prose describing
the concept in English (hundreds of occurrences, e.g. "spends its own untyped", "an untyped budget")
was left alone throughout, matching how lowercase "endpoint" and "tcb" stayed after those renames.
Three headings that read as prose rather than identifier citation were also left in the old word's
spirit but reworded to match their sibling variants' own `A`/`An` + noun heading pattern rather than
literally kept (`kernel/src/cap.rs`'s `Object::MemoryRegion` doc heading became "A memory
region", matching `Frame`'s "A physical page" and `Aspace`'s "An address space under
construction" rather than reading "**MemoryRegion memory**"). One historical citation in
`crates/abi/src/lib.rs`'s own crate-naming rationale (which names `Tcb`/`Aspace`/`Untyped` as the
abbreviations a naming review "sank") kept `Untyped` for the same reason the `Tcb` lane kept it
there: renaming it would make the sentence describe the winning name as the one that lost.
`notes/untyped.md` -> `notes/memory-regions.md`, rewritten throughout the same way
`notes/ipc-naming.md` was, with a provenance line naming the old identifier.

The compositor collision §113 named (`Untyped`/`Region` colliding with the compositor's own
damaged-screen-rectangle "region") was checked directly: `crates/compositor/src/lib.rs` never
mentions `MemoryRegion`, so there is no adjacency for a reader to misread. Verified the same four
ways as `Endpoint` and `Tcb`: full host test suite green (including doctests), both kernel targets
build clean, `user`/`redoxfs_server`/`xtask` build clean, `crates/inter_process_communication` (6 harnesses) and
`crates/capability` (12 harnesses) both re-verify under Kani post-rename, `script/lint` clean
(including two `clippy::doc_markdown` fixes the rename itself triggered: `MEMORY_REGION` and
`MemoryRegion` need backticks where the old `UNTYPED`/`Untyped` spellings did not, since clippy's
heuristic fires on underscored and CamelCase tokens), and full `script/test` green on both ISAs.

## What is still open

Nothing. `Untyped` -> `MemoryRegion` was the last of the seven names, done to completion in its
own lane (milestone/158-untyped-rename), the same shape as `Endpoint`, `Tcb`, `Aspace` and `Frame`
before it: an ABI module (`abi::untyped` -> `abi::memory_region`), a lock-rank constant
(`kernel/src/sync.rs`'s `rank::UNTYPED` -> `rank::MEMORY_REGION`), a kernel module file rename
(`kernel/src/untyped.rs` -> `kernel/src/memory_region.rs`), and a large family of lowercase
function/field/const names, all confirming the same pattern the other six already showed: the real
surface is well past whatever the capitalized-token estimate said. `Untyped`'s own gap was the
largest of the seven, 88 files against a 25-file pre-rename estimate. See "What was built" above
and the table below for the full account.

## `Frame` -> `PageFrame`: what was built

Built 2026-08-24. The compositor collision named above is real and was checked file by file rather
than trusted from a capitalized-token grep, because "frame" (lowercase) is a common English word with
several genuinely distinct in-tree senses that all had to be told apart before touching anything:

- **`crates/compositor`'s own sense, a rendered screen update** (the collision §113 named). Left
  untouched throughout `crates/compositor/src/lib.rs`, `kernel/src/user/compositor_service.rs`,
  `components/src/compositor.rs` (`serve_frame`, "Serve one frame. Read every client's control page,
  composite whatever changed"), and `crates/watch`/`kernel/src/user/watch_tests.rs`
  (`a_second_frame_erases_the_first_rather_than_leaving_it_on_screen`, a terminal redraw, the same
  sense). Two of these instances sit on lines immediately adjacent to genuine `PageFrame` renames in
  the same file (`kernel/src/user/compositor_service.rs`'s `zeroed_page_frame()` helper beside its
  own "the compositor has processed the frame" prose two functions over), which is exactly the
  fine-grained case the milestone doc's own warning anticipated.
- **A CPU call frame (`TrapFrame`, compiler stack-size accounting), not named or anticipated by
  §113.** `kernel/src/user.rs`'s `enter_frame`/`enter_at` (builds the trap frame a thread's first
  `eret`/`sret` restores from), every architecture's `exceptions.rs`, and `notes/frames.md`'s own
  stack-overflow postmortem section all use "frame" for this unrelated concept and were left alone.
- **A raw network frame (Ethernet/ARP/mDNS), also not anticipated by §113.** `crates/virtio::send_frame`
  (a virtio-net transmit), `components/src/net_transport.rs`'s `VnetRxToken { frame: Vec<u8> }`, and
  `xtask/src/main.rs`'s `arp_request_frame`/`mdns_query_frame` packet builders. None renamed.
- Arbitrary example text, unrelated to any of the above. `crates/manual/src/index.rs`'s search-
  index tokenizer test used the literal string `` `Frame` `` as stand-in content to exercise a
  generic tokenizer, not as a citation of the kernel object; the initial blanket capitalized-token
  pass touched it by accident (it broke a test), and it was reverted to `Frame` once the test failure
  traced it back. Recorded because it is the one miss this lane's own build caught rather than caught
  in review.

What did rename, once the four collisions above were excluded: the type (`Frame` -> `PageFrame`,
`FrameAllocator` -> `PageFrameAllocator` in `crates/page_frames`, the crate that is this object's own
underlying physical-page allocator and was swept at the same identifier-only rung as `crates/page_frames`
itself, not full prose), the `abi::frame` -> `abi::page_frame` ABI module (methods `MAP`/`REVOKE` keep
their own names, unchanged, matching how `Endpoint`'s `SEND`/`RECV` were left alone), and a long tail
of lowercase functions, constants, fields and module paths that spell the concept out: `frame_cap` ->
`page_frame_cap`, `delete_frame_caps` -> `delete_page_frame_caps`, `revoke_frame` ->
`revoke_page_frame`, `frame_map`/`frame_revoke` -> `page_frame_map`/`page_frame_revoke`,
`is_frame_used`/`bring_up_frames`/`free_frames` -> their `page_frame` equivalents,
`map_current_user_frame` (all three architectures) -> `map_current_user_page_frame`, `OutOfFrames` ->
`OutOfPageFrames` (`crates/paging`), `zeroed_frame` -> `zeroed_page_frame` (three IOMMU files plus
`compositor_service.rs`'s own, unrelated to that file's screen-frame prose), the `FRAME_REPORT_MIN`/
`SUITE_FRAME_BUDGET`/`rank::FRAMES` test-ledger and lock-rank constants, `SURFACE_FRAMES`/`SCREEN_FRAMES`/
`DMA_FRAMES` (`graphics_proto`, `compositor_service.rs`, `display_service.rs`) -> their
`_PAGE_FRAMES` forms, `OP_ATTACH_FRAME` (`socket_proto`, a shared-memory attach opcode, not a network
frame) -> `OP_ATTACH_PAGE_FRAME`, the `grant_plan::jobframe` module -> `job_page_frame`, and the
`kernel/src/user/frame_service.rs` module (a kernel-side Frame-producer/consumer test wiring) ->
`page_frame_service.rs`. `FRAME_SIZE` was checked and left alone deliberately: every use names the
size of one physical page frame, which is the correct, unambiguous sense already.

**`DeviceFrame` was left unrenamed**, a judgment call rather than something §113 decided: its own doc
comment already disambiguates it from the compositor ("a device's MMIO page"), it was not among
§113's eleven names or the `CSpace` amendment, and companion-renaming an unlisted sibling is the kind
of scope creep `AGENTS.md`'s naming section asks a lane not to do on its own initiative. `DeviceFrame`'s
own doc citations of bare `` `Frame` `` were updated to `` `PageFrame` ``, since those are citations of
the type that did rename.

Verified: `script/lint` clean; the full host test suite (`script/test`, all three architectures --
aarch64, riscv64, x86_64) green; both Kani-proof-bearing crates that reference the renamed type
(`crates/capability` and `crates/page_frames`) verify successfully post-rename, satisfying milestone
69's proof obligation. `notes/frames.md` rewritten with a provenance note for a reader who remembers
`Frame`, following `notes/ipc-naming.md`'s precedent from the `Endpoint` rename; its stack-frame
postmortem section (a CPU call frame investigation, unrelated to this object) is called out explicitly
as deliberately untouched rather than silently left inconsistent.

## The seven renames, and what each one actually touches

| Was | Becomes | Measured surface (§113's own count where a lane hasn't re-measured; done names carry their real count) |
|---|---|---|
| `Aspace` | `AddressSpace` | **done**: 391 occurrences across 37 `.rs` files, plus `FreeVas`'s 10 across 3 |
| `Untyped` | `MemoryRegion` | **done**: 60 occurrences across 25 `.rs` files, capitalized-token pre-rename estimate; 88 files / 620 insertions / 618 deletions actually changed |
| `Endpoint` | `Rendezvous` | **done**: 82 occurrences across 22 `.rs` files by fresh count; 216 for `EpId` across 43 files |
| `Frame` | `PageFrame` | **done**: 51 files, 172 occurrences of the bare capitalized token alone (stale table undercounted, same pattern as `Endpoint`); real diff 97 `.rs` files plus `notes/frames.md`, 918 insertions / 899 deletions, including two file renames and an `abi::frame` -> `abi::page_frame` module rename. See "`Frame` -> `PageFrame`: what was built" below |
| `Tcb` | `ThreadControlBlock` | **done**: 33 occurrences across 12 `.rs` files by fresh count; the lowercase identifier family well past this |
| `EpId` | `RendezvousId` | **done**, with `Endpoint`: 216 occurrences across 43 files, once the type alias's own call sites were counted |
| `Tid` | `ThreadId` | **done**, with `Tcb`: 82 occurrences across 13 `.rs` files by fresh count |

Plus the four companions §113 also decided in the same entry: `TcbPtr` -> `ThreadControlBlockPointer`
(**done**, with `Tcb`; 11 occurrences, `kernel/src/sched.rs` only), `TidSet` -> `ThreadIdSet`
(**done**, with `Tcb`; 6 occurrences, `kernel/src/user/survey_tests.rs` only), `EpFail` ->
`RendezvousFailure` (**done**, with `Endpoint`), `FreeVas` -> `FreeAddressSpace` (**done**, with
`Aspace`).

**Re-measure before starting, not from this table.** §113's own count is from 2026-08-23 and this
tree changes fast; a lane that trusts a stale number here repeats §76's own recorded mistake.

## What has to be true before this is safe to do in one pass

**Real collisions checked and named by §113 itself, still true and worth restating so a lane does
not have to re-derive them:**

- `Frame` collides with the compositor's own use of "frame" for a rendered screen update
  (`crates/compositor/src/lib.rs`). Renaming the kernel object to `PageFrame` is what resolves this;
  do not rename compositor's own "frame" language to avoid the opposite collision.
- `Untyped`/`Region` collides with the compositor's own use of "region" for a damaged screen
  rectangle, the same file. `MemoryRegion` is deliberately not bare `Region` for this reason.
- `Endpoint` has real prior art (seL4's own term) that some doc comments and notes may still cite
  approvingly; a rename should not silently rewrite a historical citation that correctly described
  what the tree used to be called. Read `notes/ipc-naming.md` before touching it -- §113 cites it
  directly and it may need updating rather than blind search-and-replace.
- `Aspace`/`AddressSpace` had no collision of this shape (checked directly: `crates/compositor`, the
  file that collided with both `Frame` and `Untyped`/`Region`, has no "aspace" or "address space"
  vocabulary of its own), but it did have a real near-collision nobody had recorded:
  `kernel/src/user.rs` already defines `pub struct AddressSpace`, the kernel-internal page-table
  object, predating the rename. Not a blocker in the end -- it is the same pattern
  `Object::Rendezvous(RendezvousId)` already uses naming `struct Rendezvous`, a capability variant
  sharing its name with the object it names, in different modules -- but worth recording so the next
  reader doesn't have to re-derive that it's fine. See "What was built" above.

## Scope and sequencing

**Pure rename, no behaviour change**, the same discipline milestone 98's `IpcTables` rename used
(itself one of the smaller names in this same family -- read that milestone's own completed work,
once merged, as the template for how to gate and verify a rename of this shape here). Milestone
69's proof obligation applies to any renamed type a Kani harness references by name.

Do the biggest, most-cited name first and measure the real diff before committing to doing all
seven in one lane. `Endpoint`/`Rendezvous` is both the largest measured surface and the one most
likely to touch generated or macro-derived code (IPC message types, proof harnesses) in ways a
smaller rename like `Tcb`/`ThreadControlBlock` will not. If the full set turns out too large for one
safe, reviewable commit, split by name rather than doing all seven partially -- an inconsistent tree
where `Aspace` is renamed and `Endpoint` is not is a worse intermediate state than finishing one name
completely and stopping.

**Rename `EpId`/`TcbPtr`/`TidSet`/`EpFail`/`FreeVas` together with the type they name**, not as a
separate pass -- §113 decided them as companions specifically so a type and its own identifier never
disagree mid-rename.

## What this does not decide

Whether `Endpoint`'s rename can land in the same commit as `EpId`'s, or must be sequenced, is an
implementation judgment for whoever builds this, guided by how large the diff actually turns out to
be once measured -- not decided here.

## What it unblocks

Nothing else is gated on this; it closes the gap between a decision calef made and the code
actually reflecting it, which is its own reason to exist per DECISIONS §113's whole argument: a
name only works if a reader meets it. All eleven names are done (`Endpoint`/`EpId`/`EpFail`,
`Tcb`/`TcbPtr`/`Tid`/`TidSet`, `Aspace`/`FreeVas`, `Frame`, and `Untyped`); no reader meets a
contraction or borrowed abbreviation from this list anywhere in the tree anymore.

## Follow-on

- **Refused.** `DeviceFrame` was left unrenamed. It is not one of §113's eleven names or the
  `CSpace` amendment, its own doc comment already disambiguates it from the compositor's frame ("a
  device's MMIO page"), and companion-renaming an unlisted sibling is the scope creep AGENTS.md's
  naming section asks a lane not to take on its own initiative.
- **Refused.** Local bindings and parameters that merely spell a renamed concept (`ep`, `aspace`,
  `tid`, `tcb`) were left alone across all seven names. A local binding names nothing a reader meets
  from outside the function, so it is not on the identifier surface §113 decided.
- **Refused.** Informal lowercase prose that uses "endpoint", "tcb" or "untyped" as English rather
  than as a citation of an identifier stayed as written, outside each object's own implementation.
  The exception was "aspace", swept to "address space" wherever the rename touched, because it is
  not standard English and leaving it would have parked the exact contraction §113 retires next to
  its own renamed identifier.
- **Refused.** Historical citations keep the old spelling on purpose: `crates/abi/src/lib.rs`'s
  naming rationale names `Tcb`/`Aspace`/`Untyped` as the abbreviations a review sank, and renaming
  them would make the sentence describe the winning name as the one that lost.
  `design/decisions/114-aspace-enumerate.md` was left as the point-in-time record it is.

## Index row

All seven names built across five lanes and green on all three ISAs, each measured far larger than
§113's own estimate once its ABI module and lowercase identifier family were counted: `Endpoint`/`EpId`/`EpFail` -> `Rendezvous`/`RendezvousId`/`RendezvousFailure`, `Tcb`/`Tid`/`TcbPtr`/`TidSet` -> `ThreadControlBlock`/`ThreadId`/`ThreadControlBlockPointer`/`ThreadIdSet`, `Aspace`/`FreeVas` -> `AddressSpace`/`FreeAddressSpace`, `Frame` -> `PageFrame`, `Untyped` -> `MemoryRegion`.
