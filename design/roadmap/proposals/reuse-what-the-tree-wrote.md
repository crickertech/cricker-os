---
status: PROPOSED
raised: 2026-10-04
milestone_dependencies: none
decision_dependencies: 46
machine_requirements: none
specific_machine: none
needs_person: no
---
# Reuse what the tree wrote: which first-party code an existing library should replace

Written by the research lane `lane/reuse-audit` on 2026-10-04 (UTC) at base `7da1a9f29`, answering
calef's question of the same day: under the §46 (thin primitives or whole subsystems) amendment,
*what that we already have in the tree would be a better candidate for reuse?* The ruling: outside the
kernel and the crates Kani proves, the default is to take or adapt existing code, and writing our own
needs a recorded reason. The amendment's text was still in flight as pull request #1637 (a claim
commit only), so the `Reuse:` lines below use the ruling's terms and should be conformed to the
landed wording. Title and slug are drafts.

This is a proposal, not a decision. Every replacement below adds a dependency, which §46 makes an
architect's call, so each proposed milestone waits on its own ruling.

## Method, and what the numbers mean

- **Inventory.** All 91 crates under `crates/`, the 58 programs in `components/src/`, `uefi_loader`,
  `redoxfs_server`, `tools/redoxfs_host` and the host tools under `xtask`. Line counts are code lines
  (blank and comment lines removed, **test modules included**), measured by a script, not `tokei`,
  which is not installed. Kani harnesses are counted as `#[kani::proof]` occurrences.
- **Candidates.** For each external crate, the `.crate` file was downloaded from crates.io and its
  `Cargo.toml` and `lib.rs` read; downloads, licence and last release are from the crates.io API on
  2026-10-04; advisories are from a fresh clone of `rustsec/advisory-db`. Anything not read that way
  is marked *from memory*.
- **Licence.** Base (needed to boot, install or fetch a package) is permissive only, and so is
  anything linked into one of our programs, per §135 (running GPL software is aggregation). A whole
  GPL program running as its own process is allowed as an `optional` package. `deny.toml` today
  allows MIT, Apache-2.0, BSD-3-Clause, 0BSD and ISC, so an MPL-2.0 crate (`fdt`, `noline`,
  `serialport`) would need a licence ruling first.

## The inventory, grouped by why it stays or goes

**Kernel, or used by the kernel for more than shared constants, keep by default (rule 3 and the
amendment's exemption):** `paging` (2,866 lines, 36 Kani), `capability` (746, 14),
`inter_process_communication` (1,008, 14), `machine_discovery` (5,714, 16, including ACPI at 1,954
and 8), `pci` (1,156, 8), `non_volatile_memory_express` (711, 8), `direct_memory_access_validator`
(667, 7), `page_frames` (444, 5), `generational_table` (297, 4), `address_space_identifier` (107, 3),
`intrusive_fifo` (178, 1), `memory_regions`, `work_steal_slot`, `thread_wake_handshake`,
`memory_corruption_canary_gate` and `clock_protocol` (loom), `jh7110_entropy` (516, 4),
`jh7110_clock_and_reset` (659), `firmware_configuration`, `screen_console`, `compositor`,
`measured_boot` (440), and the kernel's own 35,186 lines.

**Kani-proved and outside the kernel, keep unless a proof is no longer worth having:** `calendar`
(1,089, 11), `timetable` (2,884, 10), `globally_unique_identifier_partition_table` (2,219, 9),
`elf` (1,173, 8, also kernel), `network_time_protocol` (684, 7), `glob` (650, 6), `component_plan`
(1,248, 5), `subtree_scope` (384, 5), `device_tree_blob` (1,814, 4, also kernel),
`credential_protocol` (322, 3), `filesystem_protocol` (2,636, 3), `package_archive` (677, 2),
`manifest_note` (607, 2), `nifefs` (354, 2).

**The ABI, a capability, or one of the 19 `*_protocol` crates (rule 3):** every `*_protocol` crate,
`abi`, `grant_plan` (6,405), `swish` (3,404 plus the 3,690-line program), `system_initializer`
(2,972), `supervision_protocol`, `swap_protocol`, `user_mode_runtime`, `system_log` (1,214), the
`procps` tools (`ps`, `pgrep`, `pmap`, `top`, `free`, `vmstat`, `slabtop`, `uptime`, about 1,700
together, all reading nife's own statistics ABI), `login`, `credentialer` (already takes `argon2`).

**Not kernel, not proved, not protocol: the candidates.** Note one surprise here: the kernel lists
`video_terminal` and `line_editor` as dependencies, but reads only their wire constants
(`video_terminal::status`, `line_editor::proto`). Their engines are userspace code.

| Ours | Lines | Untrusted input | Tier | Candidate | Licence, `no_std` | Downloads, last release | Advisories |
|---|---|---|---|---|---|---|---|
| `http_response` head parser | 294 (about 70 non-test in the parser) | network | base | `httparse` 1.10.1 | MIT/Apache, `no_std`, no deps | 762M, 2025-03 | none |
| `video_terminal` escape parser | 2,523 crate | every program's output | base | `vte` 0.15.0 | MIT/Apache, `no_std` (`arrayvec`, `memchr`) | 80M, 2025-02 | none |
| `virtio` + `gpu_driver` + `keyboard_driver` + `net_transport` | 715 + 303 + 215 + 251 | device | base | `virtio-drivers` 0.13.0 | MIT, `no_std` | 387k, 2026-03 | none |
| `user_mode_heap` | 272 | every allocation | base | `talc` 5.1.1 | MIT, `no_std` | 2.1M, 2026-09 | none (`linked_list_allocator` has RUSTSEC-2022-0063) |
| `uefi_loader/src/efi.rs` | 307 | firmware | base | `r-efi` 7.1.0 | MIT/Apache/LGPL, take MIT; `no_std`, no deps | 382M, 2026-08 | none |
| `stick_maker` `plist.rs` | 335 | host OS output | conveyed host program | `plist` 1.10.1 | MIT, std | 74M, 2026-09 | none |
| `portable_executable` | 778 | our own ELF | host (`xtask`) | `object` 0.40.0 `write::pe::Writer` | MIT/Apache | 640M, 2026-08 | none |
| `board_console` `port.rs` | part of 3,418 | none | host | `rustix` or `libc` termios | MIT/Apache | *(from memory)* very wide | none checked |
| `coreutils` programs `wc`, `rm`, `printenv`, `date` | 87 + 199 + 110 + 145 | user | base | uutils `uu_*` 0.12.0 | MIT, std | 130k each, 2026-09 | none |
| `rmle` editor | 460 | user files | optional | `kibi` 0.3.3, or GNU `nano` as GPL-as-package, milestone 170 (`nano`: a real, full-featured screen editor) | MIT/Apache, std plus `libc` termios | 12k, 2026-02 | none |
| `file_allocation_table` (writer) | 512 | none (writes our own) | base | `fatfs` 0.3.6 | MIT; `no_std` only through `core_io` | 1.6M, **2023-01** | none |
| `documentation` markdown renderer | 802 of 2,587 | shipped docs | base | `pulldown-cmark` 0.13.4 | MIT, **std only** (`lib.rs` uses `std::fmt`) | 166M, 2026-05 | none |
| xHCI, unbuilt (milestone 242 (USB host and HID)) | 0 | device | base | rust-osdev `xhci` 0.9.2 | MIT/Apache | 107k, **2023-07**, last push 2024-09 | none |

## Ranked: trust gained per cost

**1. Adapt `http_response` to `httparse`.** The package fetch path, rung 3a of milestone 198 (a package manager, and the trivial install that
makes a second customer possible), parses a
network peer's response head with about 70 lines of our own, fuzzed (46 million inputs) but not
proved: the crate's own BUGS section records that a Kani harness did not finish. `httparse` is under
`hyper`, has no dependencies, builds `no_std`, and has had no advisory. Keep our request writer, the
2 KiB head cap, and our `Content-Length` policy (no close-delimited body); hand the head to
`httparse::Response::parse` and let its `Partial` status replace our `ends_with` scan. The existing
fuzz target keeps working unchanged against the adapted crate. Cost: small, one lane-day *(estimate)*.
Loss: none proved; one zero-dependency crate in the base graph.

**2. Adapt `video_terminal` to `vte` for parsing, keeping our grid.** Every program that writes to a
terminal feeds this parser, so it is the widest untrusted-input surface in userspace that nothing
proves. Our parser has four states and its comment says so on purpose: "a VT's full state machine
has a dozen states". `vte` is that full machine (Paul Williams' DEC parser, *from memory* the one
Alacritty ships) behind a `Perform` trait our grid can implement. Cost: medium; the engine is 2,523
lines and the lane must measure how much of it is parser rather than grid before claiming a number.
Loss: none proved; two widely used crates (`arrayvec`, `memchr`) in the base graph.

**3. Adapt the virtio drivers to `virtio-drivers`, one device per milestone.** About 1,500 lines of
device-facing code, with no proofs, driven by bytes a device or a hostile hypervisor writes.
`virtio-drivers` covers block, net, GPU and input over MMIO and PCI ECAM, is active (last push
2026-09-25), and its leading contributor by commits is `qwandor` (488 commits) *(from memory: of
Google's Android virtualization work)*. **Read, and the reason this is third rather than first:**
its README and source do not negotiate `VIRTIO_F_ACCESS_PLATFORM`, which `net_transport.rs` and
`keyboard_driver.rs` accept today because they sit behind the IOMMU. So the first milestone is an
upstream patch, and the plan fails if upstream refuses it, leaving a fork. Its `Hal` trait is static
methods, so the DMA capability has to sit in a global. The kernel's DMA validation is untouched. New
base-graph crates: `zerocopy`, `safe-mmio`, `thiserror`, `enumn`, `log`, `bitflags`. Throughput must
be measured with `script/bench` before and after each device.

**4. Measure `user_mode_heap` against `talc`, then decide.** 272 lines of first-fit free list in every
process. `talc` is maintained (2026-09) and `no_std`. The prior art cuts both ways:
`linked_list_allocator`, the same algorithm as ours, had out-of-bounds writes (RUSTSEC-2022-0063),
which is the class of bug a hand-written heap carries. Measure first, per the tenet: allocation
latency and fragmentation on the `job_mix` workload, then rule. Retires little code; the case is
exposure.

**5. Replace `uefi_loader/src/efi.rs` with `r-efi`.** 307 lines of hand-transcribed UEFI tables, whose
own header names the foot gun: "**`BootServices`'s field order is the ABI** ... getting the count of
unused ones wrong silently re-points every call after it." `r-efi` has no dependencies and is the
binding Rust's own UEFI targets use *(from memory; consistent with its 382M downloads and its
`rustc-dep-of-std` feature, which was read)*. The header's recorded reason was that `uefi` is "in
between"; `r-efi` is a thin primitive, which the old §46 already allowed. Cost: small.

**6. Replace `stick_maker`'s `plist.rs` with `plist`.** A conveyed host program, run by people on
macOS. 335 lines retire. Input is `diskutil` output, so the trust gain is modest; the case is the
amendment's default and a format we do not otherwise own. Adds `quick_xml`, `time`, `base64`,
`indexmap`, `serde` to a host program's graph, none of them in a nife image.

**7. Survey `portable_executable` against `object`'s PE writer.** `object` 0.40.0 has
`write::pe::Writer` (read). Host-only, so no base-graph cost; no untrusted input, so the gain is code
retired (up to 778 lines) and a writer that more PE readers have checked. A survey lane says how much
of the ELF-to-PE logic is ours by necessity.

**8. Survey uutils for the `coreutils` package.** MIT, so base-eligible. `ripgrep` already runs
unmodified (milestone 121 (`ripgrep` on nife)), which shows the `std` path works. Open: how `rm` maps onto a directory
capability through `std::fs`, and what `uucore`'s dependency weight does to the base image. The
lines retired are few; the value is the demonstration milestone 121 started.

**9. Editor: `rmle` stays until one of two takes it.** `kibi` (MIT/Apache, a Rust editor inspired by
the same `kilo` `rmle` is modelled on) needs `libc` termios, so it waits on raw mode through `std`. GNU `nano`
is the GPL-as-package path and already has milestone 170. `rmle` is `optional`, so either can replace
it without touching base.

**10. xHCI, when milestone 242 starts: adapt rust-osdev `xhci` for register and TRB definitions.**
Write the driver logic; take the definitions. The crate is dormant (release 2023-07), so expect to
carry it as a fork or vendor it under §34 (RedoxFS is the primary filesystem, on three conditions)'s "we must patch it" trigger.

Smaller, recorded rather than ranked: `board_console`'s `port.rs` could take `rustix` termios
(host-only, small); DNS is being surveyed under milestone 384 (in a capability system the resolver is a grant), pull request #1634, where
`hickory-proto` carries three advisories (RUSTSEC-2025-0006, 2026-0118, 2026-0119) and
`domain_name_system` has 3 Kani harnesses.

## Running another OS's drivers unmodified

calef asked that this path be noted. Read for this proposal:

- **NetBSD rump kernels** run NetBSD drivers unmodified in userspace, including on L4 through Genode
  (Wikipedia, "Rump kernel"). The `rumpkernel/rumprun` repository was last pushed 2020-05-11 and lists
  x86 and ARM, not aarch64 or riscv64; a platform supplies the `rumpuser` hypercall interface. NetBSD
  source is BSD-licensed *(from memory)*, so rump-hosted drivers could be base.
- **DDE / DDEKit** (TU Dresden) maps the in-kernel Linux driver interface onto a host system so Linux
  drivers run unmodified in userspace, originally on L4. Genode's release notes show `dde_linux` kept
  current, to Linux 6.18 in 26.02. Those drivers are GPL, so under §135 they can be packages only:
  acceptable for, say, a Wi-Fi card an installed system adds, never for a boot or install driver.

Either is a C toolchain and a large host-interface shim in a tree that is Rust end to end. Survey
further, and only when a driver nobody has in Rust is on the customer path (for example, for e1000e,
milestone 494 (a driver for the network card a PC actually has), *from memory* FreeBSD's `em(4)` is the permissive reference).

## The seven questions, for the top five

1. **Considered and lost.** `uefi` lost to `r-efi` (`uefi` is 23k lines, ours needs the bindings
   only). `fatfs` lost because its only `no_std` route is `core_io` and it has not released since
   2023-01. `pulldown-cmark` lost because it is std-only. `noline`, `fdt` and `serialport` lost on
   licence (MPL-2.0, not in `deny.toml`). `goblin` and `xmas-elf` lost to our Kani-proved `elf`
   (`xmas-elf` has RUSTSEC-2025-0018, an out-of-bounds read on a malformed ELF).
2. **What the tree already does.** `smoltcp` is taken whole and unpatched, and argon2 and RustCrypto
   are ordinary dependencies; `httparse`, `vte` and `r-efi` fit that shape. `virtio-drivers` needs a
   patch, which is RedoxFS's shape (§34).
3. **Prior art.** In the table, read from crates.io and the advisory database on 2026-10-04.
4. **Is the premise true?** Mostly. The surprise is that the kernel's dependency on `video_terminal`
   and `line_editor` is constants only, so the kernel exemption does not cover their engines.
5. **Cost.** Lines retired are measured above. Lane time is estimated. Throughput and heap numbers are
   owed by candidates 3 and 4 before any ruling.
6. **Reversibility.** Each replacement is reversible code; adding a crate to the base graph is §46's
   expensive category, and nobody has acted on it yet.
7. **Same cost?** Yes for 1, 2, 5 and 6: with equal effort, the exposed library wins on trust. 3 is
   closer, because the fork risk is real. Nothing here is recommended because it is less work.

## Proposed milestones, one replacement each

- **(a)** `http_response` parses its head with `httparse`. Gate: existing host tests and fuzz target.
- **(b)** `video_terminal` parses with `vte`; the grid stays. Gate: existing host tests, plus a fuzz
  target the crate does not have.
- **(c0)** Upstream `VIRTIO_F_ACCESS_PLATFORM` to `virtio-drivers`. Then **(c1)** input, **(c2)** GPU,
  **(c3)** block, **(c4)** net, each with a `script/bench` number before and after, on all three
  architectures.
- **(d)** Measure `user_mode_heap` against `talc`; the ruling follows the data.
- **(e)** `uefi_loader` binds UEFI through `r-efi`.
- **(f)** `stick_maker` reads `diskutil` through `plist`.
- Surveys: (g) `portable_executable` and `object`; (h) uutils for `coreutils`.

## Recorded reasons for what stays

Each line is ready to paste as the component's `Reuse:` line, to be conformed to the amendment's
landed wording.

- `calendar`: Reuse: write. Proved (11 Kani over every day in range); the §46 worked example, and
  `time` and `chrono` cannot be restructured for CBMC. The proof argument has not changed.
- `elf`: Reuse: write. Kernel, and proved (8 Kani); `xmas-elf` shows the bug class (RUSTSEC-2025-0018).
- `device_tree_blob`: Reuse: write. Kernel, proved (4 Kani); `fdt` is MPL-2.0.
- `globally_unique_identifier_partition_table`: Reuse: write. Proved (9 Kani, CRC restructured for
  CBMC per §46); `gpt` and `gptman` are std crates *(from memory)*; installer and loader must agree
  byte for byte.
- `network_time_protocol`: Reuse: write. Proved (7 Kani, fixed-point multiply restructured per §46);
  `sntpc` exists but would lose the proof.
- `glob`, `timetable`, `component_plan`, `subtree_scope`, `manifest_note`: Reuse: write. Proved, and
  each encodes a capability rule no upstream has.
- `package_archive`: Reuse: write. Decided by §197 (a package is one archive file); `tar` refused.
- `nifefs`: Reuse: write. Kernel; a deliberate minimal format.
- `bitmap_font`: Reuse: taken. The glyphs were chosen by §100 (the terminal font).
- `line_editor`: Reuse: write. No permissive `no_std` candidate with field exposure (`noline` is
  MPL-2.0 with 37k downloads); revisit when a std terminal program can use `rustyline`.
- `documentation`: Reuse: write, for now. `pulldown-cmark` is std-only; revisit when `mdr` runs on std.
- `file_allocation_table`: Reuse: write, for now. Writes only our own ESP; `fatfs` has no `no_std`
  release without `core_io`. Revisit if `fatfs` 0.4 releases.
- `swish`, `grant_plan`, `system_initializer`, the `*_protocol` crates, `system_log`, the `procps`
  tools: Reuse: write. The ABI or a capability (rule 3); nothing upstream reads nife's ABI.
- `redoxfs_server`, `tools/redoxfs_host`: Reuse: adapted. The adapter around vendored RedoxFS (§34).
- `coremark`: Reuse: taken. A port of the upstream benchmark.

## One question for an architect, outside the default

`measured_boot` hand-writes SHA-256 (about a hundred lines, by its own header) while `sha2` is already
an ordinary dependency of `cryptography_provider`. Rule 3 (kernel) says write; rule 4 (crypto) says
take. The header's reason, a small trusted computing base checked against FIPS 180-4 vectors, is a
real one, and the kernel is exempt from the new default, so this proposal does not recommend a change.
It does record that two rules of §46 disagree here and that only an architect can say which wins.
