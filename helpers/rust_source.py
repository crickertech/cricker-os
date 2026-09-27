"""Reading this tree's Rust source, for the `script/` entry points that count things in it.

**Why this file exists.** Three derivations were written twice, once in a gate and once in the
dashboard, because each lived inside a `#!/bin/sh` script's inline `python3` heredoc: the `unsafe`
census (`script/lint` and `script/metrics`), the comment-and-literal strip the code-line and
comment-line split is built on (the same pair), and the proof-harness count (`script/falsifications`,
`script/metrics`, and a third derivation inside `script/lint`). A gate that changed its definition
would leave the dashboard quietly asserting the old one, with both looking authoritative and nothing
firing. See milestone 236 (three derivations are copied between scripts, and nothing notices when
they drift).

**The premise that made the copies look unavoidable is false, and checking it is what shrank this
milestone.** `script/lint` and `script/metrics` both `cd` to the repository root before they
`exec python3`, so their heredocs have a stable working directory, and a heredoc with a stable
working directory can `sys.path.insert(0, 'helpers')` and import like anything else. Nothing about
being a shell entry point prevented it. That matters because the alternative on the table was a host
crate, which would have made three `script/` commands depend on a `cargo build` and changed what a
`script/` command is; this changes nothing a caller can see.

**What is deliberately NOT here, and it is the part worth reading.** The file *sourcing* stays in
each caller, and only the derivation over text is shared. `script/lint` walks `git ls-files` and
reads the working tree, because a gate is asked about the tree in front of it. `script/metrics`
streams blobs out of `git cat-file --batch` at eight historical revisions and never checks anything
out, because a report is asked about the past and AGENTS.md forbids the checkout that would be
needed. That difference is exactly the tolerance milestone 236's block warned a shared
implementation might dissolve, so it is kept where it belongs: in the input, not in the count.

**And the harness count is still derived twice, on purpose.** `script/lint` and
`script/falsifications` attribute each harness to a workspace package from `cargo metadata`, which
needs a checked-out workspace and a cargo that can parse it. `script/metrics` cannot have either: it
reads blobs from July, nothing is on disk, and its own header records that nothing there is allowed
to build. `harness_count` below is the pure-text derivation the report uses; `script/lint` keeps its
own and **compares the two**, which is the honest answer for a question two callers genuinely cannot
ask the same way.

Every function here takes `files`, an iterable of `(path, text)` pairs, with `path` relative to the
repository root and slash-separated.
"""

import re

# The comment and literal stripper. Rust source is not a regular language, so this is lexer-shaped
# rather than a parse, and every count built on it is honest rather than close: a naive grep for
# `unsafe {` over this tree scores 1071 against the census's 1057, and the fourteen it invents are
# all `unsafe {}` written inside a `//!` doc example in `intrusive`, `inter_process_communication`,
# `paging` and `user_mode_heap`. Those are documentation ABOUT unsafe, and a ceiling that counted
# them would fire when somebody improved a doc comment.
#
# Block comments are matched non-greedily and do NOT nest, which Rust's do. The tree has none
# nested; a nested one would end the strip early and could only ADD to a count, which fails loud
# rather than quiet.
_NON_CODE = re.compile(
    r'/\*.*?\*/'                        # block comment
    r'|//[^\n]*'                        # line comment, doc comment included
    r'|r(#*)"(?:.|\n)*?"\1'             # raw string, any hash count
    r'|"(?:[^"\\\n]|\\.)*"'             # ordinary string
    r"|'(?:[^'\\]|\\.)'",               # char literal (a lifetime has no closing quote, so it is
    re.S)                               # left alone, which is what we want)


def strip_non_code(text):
    """Blank every comment and literal, keeping the line structure so line numbers still line up.

    Replacing with spaces rather than deleting is what makes `non_blank` on the result a **code
    line** count: a comment block and a multi-line string literal both become blank lines and
    contribute nothing, so it is a quantity nobody can move by writing prose.
    """
    return _NON_CODE.sub(lambda m: re.sub(r'[^\n]', ' ', m.group(0)), text)


def non_blank(text):
    """Lines with something other than whitespace on them."""
    return sum(1 for line in text.split('\n') if line.strip())


# --- the unsafe census (milestone 134) -----------------------------------------------------------
#
# Raised by calef on 2026-08-18 as one question: "how much unsafe code is there in a code base, and
# is that something we should be monitoring and driving in a particular direction over time?" See
# notes/register-of-measures.md and notes/unsafe-obligations.md.
#
# Shapes assumed, each checked against the real tree rather than against expectation:
#   - a block is `unsafe` then optional whitespace then `{`, MINUS the `unsafe extern "C" {` form,
#     which is a linkage declaration and not a block anybody reasons about (zero in the tree today,
#     subtracted anyway so the day one lands it does not read as unsafe code);
#   - a thread-safety claim is an `unsafe impl` of Send or Sync SPECIFICALLY, which is a different
#     animal from an `unsafe impl` of an unsafe trait (`GlobalAlloc`, `intrusive_fifo::Node`): the
#     trait is unsafe there because its own contract is, while `unsafe impl Send for T` is a
#     hand-written assertion that the compiler is wrong about T, and it is the most consequential
#     unsafe in this tree. The `[^{;]*?` tail keeps it inside one item header so a body cannot
#     donate a match.
#
# There is deliberately NO `unsafe fn` count here. `script/lint`'s `==> unsafe fn contracts` check
# derives one and prints it, and a second count of the same thing taken from a slightly different
# scope is the drift milestone 134 exists to stop.
UNSAFE_BLOCK = re.compile(r'\bunsafe\s*\{')
UNSAFE_EXTERN = re.compile(r'\bunsafe\s+extern\s+"[^"]*"\s*\{')
THREAD_SAFETY = re.compile(r'\bunsafe\s+impl\b[^{;]*?\b(?:Send|Sync)\s+for\b')

# What is out of the census, and every exclusion has a reason rather than a convenience.
#
# `bench/host/` holds the Linux and macOS programs the cross-OS comparison runs: ~100 `unsafe`
# blocks of libc FFI that those operating systems require and that say nothing about nife's
# soundness. Counting them would move the census every time somebody added a comparison. `xtask/`,
# `tools/`, `fuzz/` and `helpers/` are build and image tooling that runs on the developer's Mac and
# cannot fault the kernel.
#
# `patches/` is excluded for a different reason and it is the one worth reading: it holds our
# platform layer for Rust's `std`, which DOES run on the machine, so this is a real hole rather than
# a tidy boundary. It is out because a ceiling asserts a direction, and that code's shape is
# upstream's rather than ours: it implements `std`'s internal interfaces and cannot be restructured
# to hold fewer unsafe blocks without diverging further from the crate we are trying to track. It
# also matches the `unsafe fn contracts` check's scope, so there is one answer to "which Rust is
# ours" instead of two. The 37 blocks it leaves uncounted are recorded in
# notes/register-of-measures.md's BUGS, where a reader meets them.
#
# The list is exclusions rather than inclusions on purpose: a new subsystem directory is counted by
# default, and being left out has to be somebody's decision.
HOST_ONLY = ('bench/host/', 'xtask/', 'tools/', 'fuzz/', 'helpers/', 'patches/')


def unsafe_census(files):
    """Every unsafe number this tree tracks, in one pass over the Rust that runs on nife.

    Returns `outside_arch`, `inside_arch`, `thread_safety`, `code_lines` and `density`.

    **Density rather than a raw count, measured before it was chosen.** Outside
    `kernel/src/arch/` the block count went 171 to 747 between 2026-07-15 and 2026-08-18, almost all
    of it the system being BUILT rather than anything drifting, so a raw ceiling would have fired on
    nearly every lane. Per 10,000 code lines the same period reads 22.8, then 11.8, then 9.3,
    falling at every sample: the tree is getting proportionally safer, and holding that claim is
    what the ceiling is for. Per 10,000 rather than per 1,000 because the counted-claim markers read
    integers, and 93 keeps a digit that 9 would throw away.

    The denominator is the non-arch code only, so the density and its numerator describe the same
    code. Mixing arch lines in would let assembly-heavy months dilute a number that is deliberately
    not about assembly.
    """
    out = {'outside_arch': 0, 'inside_arch': 0, 'thread_safety': 0, 'code_lines': 0}
    for path, text in files:
        if path.startswith(HOST_ONLY):
            continue
        code = strip_non_code(text)
        blocks = len(UNSAFE_BLOCK.findall(code)) - len(UNSAFE_EXTERN.findall(code))
        out['thread_safety'] += len(THREAD_SAFETY.findall(code))
        if path.startswith('kernel/src/arch/'):
            out['inside_arch'] += blocks
        else:
            out['outside_arch'] += blocks
            out['code_lines'] += non_blank(code)
    # Truncated rather than rounded: a ceiling must never fail a tree that sits exactly on it.
    out['density'] = (10000 * out['outside_arch'] // out['code_lines']
                      if out['code_lines'] else 0)
    return out


# --- the unsafe census, split by trust boundary (milestone: unsafe census by trust boundary,
# provisional) -------------------------------------------------------------------------------------
#
# `unsafe_census` above answers "how much unsafe code is there in the tree", and one population
# mixes two things that mean opposite things in a capability microkernel: an `unsafe` block inside
# `kernel/src` runs with no confinement over it at all, and an `unsafe` block in a userspace program
# is confined by the same MMU-plus-capability-table mechanism that confines every other program.
# `notes/trusted-base.md` (the `maintainer/redleaf-comparison` lane, 2026-09-20, landing alongside
# this one) hand-computed a first split -- kernel/src/** total 577, "everything else that runs on
# nife" 561 -- and flagged in its own BUGS that nothing keeps it computed. This is that mechanism.
#
# **The split that hand computation did NOT make, and the one this file exists to make**: crates/
# is not one population. `crates/paging` and `crates/direct_memory_access_validator` were lifted out
# of `kernel/src` on purpose so a model checker could reach them (see the BUGS in
# notes/trusted-base.md, which names exactly this risk: "a tree can shrink [the kernel line count]
# by moving code out of `kernel/src` without reducing what anyone has to trust"). A crate that ships
# ONLY in the kernel binary is the trusted base wherever it lives on disk; a crate that ships ONLY
# in a userspace program is confined wherever it lives; and roughly half of `crates/` ships in BOTH,
# which this split reports as a bucket of its own rather than guessing.
#
# **How the tables below were derived, 2026-09-20, on this worktree.** `cargo metadata
# --format-version 1`, once, at the repository root: every workspace member's `resolve.nodes[*].
# deps[*].dep_kinds` was read, and a crate is `KERNEL_ONLY`/`USERSPACE_ONLY` by whether it is
# reachable, over edges that are NOT exclusively `dev`, from the `kernel` package versus from
# `components` or `fixtures` (the two packages holding every EL0 program, milestone 175 (split
# `user/`: `components/` for services, `fixtures/` for test and benchmark programs)). A crate
# reachable from both sides is `SHARED`. This is mechanical and checkable
# (`cargo metadata | helpers/<this file's own derivation>`, not reproduced as a script here because
# `script/metrics` can never run cargo -- see this file's own module docstring -- so the result is
# baked in below, the same trade `MILESTONE_STATUSES`/`NAME_STATUSES` already make in
# `script/metrics`: today's definition, applied uniformly across history).
#
# **A `SHARED` crate is not a hedge.** `environment_protocol` and `clock_protocol` are the clearest
# cases: the kernel builds the shared page (`PageBuilder`) and a userspace `std` program reads it
# back through the identical `unsafe fn new`/`from_raw_parts` accessor, so the SAME unsafe source
# genuinely executes with kernel privilege in one binary and under confinement in another. There is
# no single number that is not either an overcount (attribute it to both) or an undercount
# (attribute it to neither), so it is reported as its own line rather than folded into either side.
#
# **What is not attempted here.** Some of a `SHARED` crate's unsafe may in fact be reached only from
# the kernel's own `#[cfg(test)]` modules (several of the Cargo.toml comments say exactly this: "the
# kernel does no date arithmetic of its own; its tests predict what the command printed"), which
# would mean it never ships in the production kernel binary at all. Telling that apart from unsafe
# the kernel's real logic calls needs a source-level, per-call-site read of each of the (currently
# five) `SHARED` crates that carry any `unsafe`, which this pass did not do; it is recorded as
# future work rather than guessed at.
KERNEL_ONLY_CRATES = frozenset({
    'address_space_identifier', 'capability', 'cpu_set', 'direct_memory_access_validator',
    'firmware_configuration', 'generational_table', 'inter_process_communication',
    'intrusive_fifo', 'jh7110_clock_and_reset', 'memory_corruption_canary_gate',
    'memory_regions', 'page_frames', 'paging', 'pci', 'thread_wake_handshake',
    'work_steal_slot',
})

# The 16 `crates/` members reachable only from `components` or `fixtures`. The five directories
# below are NOT under `crates/`: each is its OWN cargo workspace (an empty `[workspace]` in its own
# `Cargo.toml`), deliberately outside the main one (`std_exerciser`'s and `entropy_backend`'s own
# headers give the reason: build-std flags and a `getrandom_backend` cfg the workspace crates must
# never inherit), so `cargo metadata` at the repository root never sees them and they are matched by
# path instead. Every one of them is, by its own header, a program or a library that runs on nife at
# EL0 and never as kernel code: `redoxfs_server`'s `el0` feature is the real filesystem server
# (`hosttest`, its other mode, never ships); `std_exerciser` and `cryptography_exerciser` are std
# workloads built for the `*-unknown-nife` targets; `entropy_backend` is the `getrandom` backend
# linked into every userspace `std` program; `cryptography_provider` is the `rustls` provider glue
# those programs use.
USERSPACE_ONLY_CRATES = frozenset({
    'c_seam', 'component_plan', 'credentialer', 'documentation',
    'globally_unique_identifier_partition_table', 'loaded_image_check', 'schedule_store',
    'supervision_protocol', 'swap_protocol', 'swish', 'system_initializer', 'timetable',
    'uptime', 'user_mode_heap', 'user_mode_runtime', 'virtio',
})
USERSPACE_ONLY_DIRS = ('redoxfs_server/', 'std_exerciser/', 'entropy_backend/',
                       'cryptography_exerciser/', 'cryptography_provider/',
                       # `fs_server/` (2026-08-01 to 2026-08-25) and `fs-server/` (before that) are
                       # `redoxfs_server`'s own past names (its Cargo.toml header has the history);
                       # a historical week's tree carries whichever name was current, never both.
                       'fs_server/', 'fs-server/')

# The 36 `crates/` members reachable from BOTH sides, over real (non-`dev`) edges.
SHARED_CRATES = frozenset({
    'abi', 'bitmap_font', 'block_roster', 'boot_ladder', 'byte_sink_protocol', 'calendar',
    'capability_witness_protocol', 'clock_protocol', 'compositor', 'coremark',
    'counter_frequency_protocol', 'credential_protocol', 'device_tree_blob', 'elf',
    'entropy_protocol', 'environment_protocol', 'filesystem_protocol', 'glob', 'grant_plan',
    'graphics_protocol', 'jh7110_entropy', 'job_mix', 'line_editor', 'login_protocol',
    'machine_discovery', 'measured_boot', 'network_time_protocol', 'nifefs',
    'non_volatile_memory_express', 'pgrep', 'pmap', 'ps', 'screen_console', 'soak_page',
    'socket_protocol', 'video_terminal',
})

# `uefi_loader` and the one crate only it reaches (`sealed_pair`) are neither: they run once, before
# the kernel starts, with the full privilege of the pre-OS environment, to decide WHICH kernel image
# gets control. `notes/trusted-base.md` draws the trusted base at "the kernel plus the hardware" and
# means the isolation boundary the kernel enforces at runtime; `uefi_loader` has come and gone
# (its memory reclaimed) before that boundary exists, so it is a boot-chain-of-trust question rather
# than a runtime-isolation one, and folding it into either side would misstate which claim it backs.
# Reported as its own bucket; which claim it belongs to is an architect's to decide.
BOOT_CHAIN_DIRS = ('uefi_loader/',)
BOOT_CHAIN_CRATES = frozenset({'sealed_pair'})

# Three more `crates/` members that are host tooling and never run on nife, exactly like
# `HOST_ONLY` above, just not under one of ITS path prefixes: each says so in its own header.
# `board_console` and `portable_executable` are read by `xtask` only (`cargo xtask board-image`'s PE
# conversion and the board's serial-log matcher); `stick_maker` is a USB-stick writer that "runs on
# macOS, Linux and Windows and never on nife" (its own words) and is invoked directly rather than
# depended on by anything. `unsafe_census` above does not know about these three (its `HOST_ONLY` is
# unchanged, so `script/lint`'s ceiling keeps meaning exactly what it meant); this split excludes
# them because the question this split answers -- kernel privilege or userspace confinement -- has
# no answer for code that runs on neither.
HOST_TOOL_CRATES = frozenset({'board_console', 'portable_executable', 'stick_maker'})


def trust_bucket(path):
    """Which trust-boundary population a source file belongs to, or `None` for one this split
    has no opinion about (already out of every census, e.g. `vendor/`, or excluded above as host
    tooling).

    `user/src/` is milestone 175's predecessor to `components/`/`fixtures/` (renamed 2026-09-13);
    a historical week's tree carries whichever name was current then, never both, so both are
    handled here rather than only in the caller.
    """
    if path.startswith(HOST_ONLY):
        return None
    if path.startswith(BOOT_CHAIN_DIRS):
        return 'boot_chain'
    if path.startswith(USERSPACE_ONLY_DIRS):
        return 'userspace'
    if path.startswith(('components/', 'fixtures/', 'user/src/')):
        return 'userspace'
    if path.startswith('kernel/src/'):
        return 'kernel'
    if path.startswith('crates/'):
        crate = path.split('/')[1]
        if crate in HOST_TOOL_CRATES:
            return None
        if crate in KERNEL_ONLY_CRATES:
            return 'kernel'
        if crate in USERSPACE_ONLY_CRATES:
            return 'userspace'
        if crate in SHARED_CRATES:
            return 'shared'
        if crate in BOOT_CHAIN_CRATES:
            return 'boot_chain'
        # A `crates/` directory this table has never seen: a crate added, renamed or removed
        # since this table was last written by hand, over a week this split cannot re-derive
        # (`script/metrics` can never run cargo against a historical revision to check). Counted
        # rather than dropped, so the gap is visible instead of a silent undercount -- the same
        # choice `names_total` already makes for the three weeks before milestone 115 (the names
        # that were ratified, and the ones that were refused).
        return 'unclassified'
    return None


def trust_boundary_census(files):
    """The unsafe census, split into kernel/userspace/shared/boot_chain/unclassified.

    Same input as `unsafe_census` (path, text pairs over the Rust that runs on nife, or once did),
    same stripper, same block pattern. Returns `<bucket>` and `<bucket>_code_lines` for each of
    `kernel`, `userspace`, `shared`, `boot_chain` and `unclassified`, plus `kernel_density` and
    `userspace_density` (blocks per 10,000 code lines, truncated like `unsafe_census`'s own): the
    two populations large enough and unambiguous enough to hold a ceiling, if one is ever wanted.
    `shared` and `boot_chain` get no density; see this file's own comment above for why a single
    number for either would be a guess.
    """
    buckets = ('kernel', 'userspace', 'shared', 'boot_chain', 'unclassified')
    out = {b: 0 for b in buckets}
    out.update({b + '_code_lines': 0 for b in buckets})
    for path, text in files:
        bucket = trust_bucket(path)
        if bucket is None:
            continue
        code = strip_non_code(text)
        blocks = len(UNSAFE_BLOCK.findall(code)) - len(UNSAFE_EXTERN.findall(code))
        out[bucket] += blocks
        out[bucket + '_code_lines'] += non_blank(code)
    for b in ('kernel', 'userspace'):
        cl = out[b + '_code_lines']
        out[b + '_density'] = (10000 * out[b] // cl) if cl else 0
    return out


# --- the proof-harness count, the text-only derivation -------------------------------------------
#
# The falsification record milestone 194 built and DECISIONS §134 ratified: what evidence each Kani
# harness carries that it can fail. `replayable` has a patch a script applies to turn the harness
# red; `attested` is a person who broke it and watched; `unfalsified` is the claim's honest
# denominator. A harness with no record at all counts as unfalsified, which is what it is.
PROOF = re.compile(r'#\[kani::proof\b')
FALSIFICATION = re.compile(r'\bFalsification:\s+(replayable|attested|unfalsified)\b')

# Directories holding `#[kani::proof]` that belong to no workspace package, expressed as path
# prefixes because that is all a text-only derivation has. `helpers/` is the `kani-lint-shim` source
# `script/lint` compiles by hand; `patches/` is our `std` platform layer. `vendor/redoxfs` is the
# third such file and is already outside every caller's file set.
NOT_A_PACKAGE = ('helpers/', 'patches/')


def falsification_records(text):
    """The state of each `Falsification:` record that annotates a `#[kani::proof]`.

    Text-only, like everything else here, and the approximation is stated rather than hidden: a
    record is attributed to the **first attribute** that follows it, skipping the rest of its own
    comment run. That is what `script/falsifications` does with a real module walk, and it is exact
    for every shape this tree uses, because the block is defined as the comment run immediately
    above the attribute. A record separated from its attribute by a blank line would be missed here
    and by `script/falsifications` both, which is the same answer rather than a drift.
    """
    lines = text.split('\n')
    out = []
    for i, line in enumerate(lines):
        hit = FALSIFICATION.search(line)
        if hit is None:
            continue
        for j in range(i + 1, min(i + 40, len(lines))):
            s = lines[j].strip()
            if s.startswith('//'):
                continue
            if PROOF.search(s):
                out.append(hit.group(1))
            elif s.startswith('#['):
                continue  # another attribute between the comment and the one that decides
            break
    return out


def harness_count(files):
    """Proof harnesses and how many carry a falsification record, from the source text alone.

    `script/lint` and `script/falsifications` answer the same question by attributing each harness
    to a workspace package out of `cargo metadata`, which is the better derivation where it can run
    and is why they keep it. This one exists for `script/metrics`, which reads blobs from revisions
    that are not checked out and could not run cargo against them if it wanted to. `script/lint`
    runs both and fails on a disagreement, which is what keeps the two honest.
    """
    total = falsified = 0
    for path, text in files:
        if path.startswith(NOT_A_PACKAGE):
            continue
        # Stripped, so the sentence in `kernel/src/syscall.rs` explaining what a `#[kani::proof]`
        # is does not count as one. A raw grep counts 151 where the tree has 146.
        total += len(PROOF.findall(strip_non_code(text)))
        # Raw, because a falsification record lives in a doc comment by construction, and
        # attributed to what it annotates, because since milestone 305 a record may sit above a
        # kernel `#[test_case]` instead of a `#[kani::proof]`. `total` counts harnesses, so a
        # record counted here that belongs to a test would put two different populations in one
        # fraction: it inflates `falsified` and deflates `unfalsified` by the same amount, and
        # `script/lint`'s drift check against `script/falsifications --count` reports the
        # disagreement rather than either being wrong. Nine such records arrived at once and it
        # fired, which is the check working.
        falsified += sum(1 for kind in falsification_records(text)
                         if kind in ('replayable', 'attested'))
    return {'total': total, 'falsified': falsified, 'unfalsified': max(total - falsified, 0)}
