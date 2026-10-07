# Rust filesystem crates on nife's own targets, 2026-10-07 (provisional name)

Measured 2026-10-07 (UTC), host and cross-compile only, at base `c70832af7`, by
lane/filesystem-crate-probe. It answers question 1 of DECISIONS §34 (RedoxFS is the primary filesystem, on three conditions):
does a usable Rust implementation exist, and at what completeness? calef approved measuring it
when he ruled #1797 fork 3 (redoxfs becomes the only writable filesystem-contract provider in base): *"Yes begrudingly.
I suspect btrfs is likely a better FS choice than redoxfs, but it is what we have today."* The
scripts are in [filesystem-crates-2026-10-07/](filesystem-crates-2026-10-07/); the forks near the end
are calef's. §34 carries only a pointer here, because it is over the word cap of §212 (a prose budget: 3,000 words of main body) and may not grow.

The question, so the data has a reader, covers each maintained Rust btrfs implementation and the
writable FAT crates that would give base a second provider. Does it build for
`targets/{aarch64,riscv64,x86_64}-unknown-nife.json`? Does it need threads (our `std` is
`singlethread`) or unwinding (`panic-strategy = abort`)? Does it work on an image some other
system made?

## Result

- **Every Rust btrfs reader builds for all three targets**, three of them with no change at all
  (lambutter, btrfs-core, ferrosys). rust-fs-btrfs needs one line, and that line is upstream's
  own platform gate applied to its C ABI module. On the host, lambutter, rust-fs-btrfs and
  btrfs-fs return the right bytes from mkfs-made and kernel-made volumes.
- **No Rust btrfs is a writable filesystem.** rust-fs-btrfs refuses every ordinary write: its
  copy-on-write path does not yet write checksum items, and it refuses to mount read-write any
  volume with the block-group tree, which current `mkfs.btrfs` turns on by default. The only crate
  that creates a file is btrfs-transaction, a tree-mutation library. A caller makes a file from
  five calls and has to pick the inode number and directory index itself, because there is no
  allocator for either. As shipped it does not build here (zstd's C sources need `string.h`). With
  its single zstd call removed it builds on all three, and the file it created on three volumes
  read back correctly through four other implementations. Neither `btrfs check` nor a Linux kernel
  has judged that file (see BUGS).
- **Writable FAT is ready.** Four read-write FAT crates build on all three targets with no change.
  Three of them (lamfat, hadris-fat, embedded-sdmmc) round-trip cleanly on a macOS-formatted stick
  image, judged by macOS's own `msdos` driver and `fsck_msdos`. The fourth is crates.io's current
  `fatfs` (0.3.6, January 2023), which writes subdirectories that `fsck_msdos` rejects. Upstream
  fixed that on 2024-10-23 and never released the fix; `lamfat` 0.4.2, a republish of upstream's
  main line, is clean.
- Threads and unwinding are not what stops anything. Only btrfs-fs needs a thread, because it
  runs every operation through tokio's `spawn_blocking`. rust-fs-btrfs's `catch_unwind` around its
  C ABI becomes an abort under our panic strategy, which loses a defensive layer and breaks nothing.
  What actually stops builds is the two classes milestone 442 (a crypto provider `rustls` can use on all three bare-metal targets) already met: `getrandom` with no
  backend, and SIMD on `x86_64-unknown-nife`. The third class is new to this note: a C dependency
  with no libc headers.

## The table

"Shipped" is the crate built as published; "tree" adds what this tree already does for other
crates (`entropy_backend`, `script/crypto-probes`' soft recipe) and the named one-line patch. a, r
and x are aarch64, riscv64 and x86_64. Host results use the images in the method section.

| crate | version, released | license | what it does | shipped a/r/x | tree a/r/x | host round trip |
|---|---|---|---|---|---|---|
| lambutter | 0.3.1, 2026-06-04 | MIT or Apache-2.0 | btrfs, read-only, `no_std` | pass | pass | reads both mkfs volumes and the kernel one; does not cross into a subvolume |
| rust-fs-btrfs | 0.10.2, 2026-10-07 | MIT | btrfs read; in-place overwrite of `nodatacow` files; mkfs | fail: `capi` imports `FileDevice` (a, r); `sha2` SIMD (x) | pass, with `capi` gated | reads everything, subvolumes included; every write refused (above) |
| btrfs-core | 0.1.5, 2026-08-26 | Apache-2.0 | forensic btrfs reader over a whole image in memory | pass | pass | reads the two small volumes; nothing on the kernel one, whose filesystem tree is more than one leaf |
| btrfs-fs | 0.13.0, 2026-05-14 | MIT or Apache-2.0 | btrfs read API (btrfsutils) | fail: `getrandom` | fail: `zstd-sys` C, no `string.h` | reads everything; every call needs tokio's blocking pool, so a thread |
| btrfs-transaction | 0.13.0, 2026-05-14 | MIT or Apache-2.0 | offline btrfs read-write tree library (btrfsutils) | fail: `getrandom` | pass, with zstd removed | created `/nife.txt` on all three volumes; four readers return its bytes |
| ferrosys | 0.6.0, 2026-10-05 | MIT or Apache-2.0 | readers for btrfs, FAT, exFAT, ext; whole-image writers | pass | pass | reads btrfs and FAT; refuses zstd-compressed btrfs and the ext4 fixture's `encrypt` feature |
| fatfs | 0.3.6, 2023-01-17 | MIT | FAT12/16/32 read-write | pass | pass | reads; writes; **`fsck_msdos` rejects the new subdirectory** |
| fatfs, no `chrono` | 0.3.6 | MIT | the same without the clock | pass | pass | the same |
| lamfat | 0.4.2, 2026-06-08 | MIT | rust-fatfs main line, republished | pass | pass | reads; writes; `fsck_msdos` clean; macOS reads the file |
| hadris-fat | 3.0.0-rc.1, 2026-10-06 | MIT | FAT12/16/32 and exFAT read-write | pass | pass | reads; writes; `fsck_msdos` clean; macOS reads the file |
| embedded-sdmmc | 0.10.0, 2026-08-10 | MIT or Apache-2.0 | FAT16/32 read-write, 8.3 names, MBR required | pass | pass | reads; writes; `fsck_msdos` clean; macOS reads the file |
| ext4-view | 1.0.0, 2026-09-14 | MIT or Apache-2.0 | ext2/3/4, read-only | pass | pass | reads (776 root entries) |
| ext4_rs | 1.3.3, 2026-01-13 | MIT | ext4 read-write, 4 KiB blocks | pass | pass | lists 3 of 776 root entries; its new file is invisible to ext4-view |

Upkeep, looked up rather than measured: rust-fatfs has 363 stars over nine years and its last push
was 2026-08-22. embedded-sdmmc has 486 and hadris 21. Every btrfs crate here is under seven
months old, and the largest of them, btrfsutils, has 14.

## Method, and how to re-take it

From the repository root:

```sh
cargo xtask std-src                                          # the patched farm, target/nife-farm
notes/filesystem-crates-2026-10-07/probe.sh nife             # "shipped" columns
notes/filesystem-crates-2026-10-07/probe.sh nife-tree        # "tree" columns
notes/filesystem-crates-2026-10-07/probe.sh host
notes/filesystem-crates-2026-10-07/make-images.sh /tmp/fsimg # macOS
notes/filesystem-crates-2026-10-07/run-host.sh /tmp/fsimg
```

`probe.sh` is `script/crypto-probes`' recipe: probes generated outside the repository so its
`rust-toolchain.toml` cannot shadow the farm, `-Zbuild-std`, release, `panic = "abort"`, versions
pinned with `=`. Each probe is a `[[bin]]` whose `main` reads, and where the crate can, writes, so
the linker has to resolve the code that matters. The probe sources are in `probes/`.

No image was made by a crate under test. Two are `mkfs.btrfs` volumes from lambutter's fixtures,
one uncompressed and one zstd. One is btrfsutils' fixture, with subvolumes and a read-only
snapshot, which only a kernel mount creates. One is ext4-view's journaled 4 KiB fixture. Two are
FAT32 images that macOS formatted and wrote, one unpartitioned and one inside an MBR the way a
stick arrives. Commits are
pinned in `make-images.sh`. Every extracted file is checked against `expected.sha256`, whose
hashes come from the fixture's own record or from the bytes written, never from the probe being
judged.

Threads and unwinding were checked twice. The build alone cannot answer it, because a crate calling
`std::thread::spawn` links here and fails only at run time. So every `thread::spawn`,
`catch_unwind` and `tokio::` in each crate's own source was also read, setting aside tests.

## What was measured and what was looked up

Measured: every build cell, every host cell. Looked up and not measured: licenses, release dates
and repository activity (crates.io and GitHub APIs, 2026-10-07); the derivation claims below.

Licenses, and the GPL question the brief raised. btrfs in Linux is GPL-2.0. rust-fs-btrfs, lambutter and
btrfsutils' library crates each state in their own README that they were written from the on-disk
format and not from kernel or btrfs-progs source. btrfsutils is a mixed workspace: the repository
license is GPL-2.0, `btrfs-mkfs` and `btrfs-cli` are GPL-2.0-only, and the four library crates
(`btrfs-disk`, `btrfs-transaction`, `btrfs-fs`, `btrfs-uapi`) carry their own MIT and Apache
files. That claim is the authors' and was not audited here. Taking any of these would want that
read by a person before it ships.

## Candidates not probed, and why

- frankenfs (GitHub only): a FUSE reimplementation of ext4 and btrfs whose README says it
  re-implements behavior extracted from the Linux kernel's C. Its license is MIT with a rider that
  excludes OpenAI, Anthropic and anyone acting for them, so this lane, which is an Anthropic model,
  did not build it. It is not an OSI license, and it would be refused here regardless.
- btrfs-peek, btrfs-diskformat and btrfs-no-std: a read-only CLI, and two crates of on-disk struct
  definitions, none of them a filesystem.
- `btrfs`, `libbtrfs`, `btrd` and the snapshot managers: ioctl wrappers for a Linux host's mounted
  btrfs.
- starry-fatfs and axfatfs: forks of rust-fatfs kept by Starry OS and ArceOS, two Rust kernels
  that took rust-fatfs. That is prior art for fork 2. lamfat stands for the main line instead.
- `fatfs-embedded` wraps the C FatFs library, and simple-fatfs, rimfs-fat, vfat-rs and fat32 are
  young or unmaintained. Of the exFAT crates only hadris-fat, which carries both, was probed.

## Forks for calef

Each is reversible, so each carries a recommendation.

### Fork 1. Does milestone 140's FAT32 stratum write?

Milestone 140 (mount a drive this system did not create) lists FAT32 first and implies reading.
Writing is what gives base a second filesystem-contract provider, and the measurement says it
costs little more: every candidate crate does both, and three passed.

- (a) FAT32 read-write as 140's first stratum, one `fat32_server` per volume as 140 decided.
- (b) Read-only first, writing later.
- (c) A milestone of its own.

**Recommendation: (a).** The caveat travels with it: FAT has no crash consistency, which 140
already calls tolerable on a stick and says must be written where a user meets it. Nothing outside
the tree depends on a FAT server, so this is cheap to undo.

### Fork 2. Which FAT crate

- (a) rust-fatfs's main line at a pinned commit. `lamfat` 0.4.2 shows that line works. It is nine
  years old with 363 stars, and two other Rust kernels ship forks of it (ArceOS's `axfatfs`,
  Starry OS's `starry-fatfs`).
- (b) hadris-fat, which also carries exFAT (140 lists exFAT), but is a release candidate with 21
  stars.
- (c) embedded-sdmmc, refused: 8.3 names only, so a stick's `Long File Name.txt` arrives as
  `LONGFI~3.TXT`.
- (d) crates.io's fatfs 0.3.6, refused: the subdirectory bug above.

**Recommendation: (a)**, by git pin rather than vendored, since §46 (thin primitives or whole
subsystems; we write everything in between) vendors only what needs a patch, and this needs none.
It would still be the choice if (b) cost the same: age is the reason, and exFAT can come from a
second crate.

### Fork 3. btrfs, and what to do now

calef's suspicion is about the format, and the format may well be better. This measured the
implementations, and none is yet a writable filesystem.

- (a) Add btrfs read-only to 140's table after FAT, on rust-fs-btrfs or lambutter.
- (b) A writable-btrfs milestone on btrfs-transaction. That means writing the file layer
  (allocators, create, unlink, rename, truncate, fsync ordering), then milestone 37 (prove
  RedoxFS's crash consistency)'s injector and a real `btrfs check`. It is writing a filesystem on
  someone else's B-tree, which §34's original reasoning refused for this role.
- (c) Re-take this probe in three months. rust-fs-btrfs published three versions in the 14 hours
  before this ran, so the answer is moving.

**Recommendation: (a) only when a btrfs drive needs reading, and (c) for writing.** calef's own
drives are ext4, per milestone 190 (ext4, read and write). Base keeps RedoxFS as its writable
provider, as #1797 ruled.

### Fork 4. Three upstream submissions

§253 (concurrent login sessions) records that each upstream submission is an outward-facing act
calef approves, so each is asked here.

- (a) rust-fs-btrfs: gate its `capi` module on the platforms `fs_core` gates `FileDevice` to.
- (b) btrfsutils: make zstd optional in btrfs-transaction, whose only use is compress-on-write.
- (c) rust-fatfs: ask for a release carrying the 2024-10-23 fix to a first-level directory's `..`
  entry.

**Recommendation: yes to all three.** Each is one line or one issue, and (a) and (b) are what
would let those crates build here unpatched.

## BUGS

- No btrfs write was judged by `btrfs check` or by a kernel mount. The Linux VM this host has
  (podman's) refused connections all session and was not restarted, because it is not this lane's.
  The judges were four independent readers and rust-fs-btrfs's own checker (a subset of `btrfs
  check`, by its own description), which found nothing on the two small volumes. On the kernel
  volume it reports the same four findings before the write as after, all of them its own
  misreading of the block-group tree. Re-take with `btrfs check --readonly` before a btrfs row is
  trusted for writing.
- Build and link only on nife. Nothing ran under nife. Milestone 442 found a crate that builds for
  `x86_64-unknown-nife` and then executes AVX2 in ring 3. `blake2b_simd` (rust-fs-btrfs, for
  blake2b-checksummed volumes) picks its backend the same way, so the same failure is possible there
  and was not tested.
- hadris-fat was driven over an in-memory image, as its own examples are; it has a `BlockDevice`
  trait a server would implement instead.
- Versions are pinned, but `[patch]` copies come from the local registry cache, so a probe run on
  a machine without that version cached fetches it first.
