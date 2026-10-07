# Options B and C, in full

This appendix belongs to [milestone 705 (the graphical terminal runs full-screen programs)](../705-the-graphical-terminal-runs-full-screen-programs.md); it holds the full seven-question entries for options B (libghostty-vt as a second engine) and C (the `vte` crate).

## B. libghostty-vt as a second engine behind the C seam

Everything below was read from `ghostty-org/ghostty` `main` on 2026-10-02, except where marked.

- *Maturity.* `include/ghostty/vt.h` opens "WARNING: This is an incomplete, work-in-progress API. It is
  not yet stable and is definitely going to change." `lib_vt.zig` says the behavior is stable
  (extracted from a shipped terminal) and the API is not. The tree's own note already says to pin.
- *Toolchain.* `build.zig.zon` requires Zig 0.16.0 or later. Zig is not installed here (Homebrew has
  0.16.0), so the build-time cost is unmeasured. The source it compiles is large:
  `src/terminal` holds 164 Zig files totaling 6.2 MiB, tests included, line count not taken.
- *Freestanding.* Not "no allocation" (section above). It also needs a `std.Io` (`TinyIo` is the
  small one), and an entropy callback on targets with no `getrandom`
  (`GHOSTTY_SYS_OPT_RANDOM_SECURE`, `vt_sys.h`; whether the terminal path exercises it is not
  checked). Whether the Zig 0.16 freestanding build links without libc symbols is the first thing
  a lane would have to find out; I did not try.
- *License.* MIT (GitHub license field, and the repository `LICENSE`).
- *What it buys.* Alternate screen (modes 47, 1047, 1049), device attributes, cursor report, size
  report, mouse and key encoders, reflow and scrollback, per `vt.h`, `vt_modes.h` and
  `vt_terminal.h`.
- *Cells versus pixels.* Its C ABI gives cells through a render-state API (`render.h`, 1,104 lines of
  header). `Vt::pixel(x, y)` is a pure function of our own grid. The rebuild is the expected-picture
  definition, as `notes/glyphs.md:694` says: either it moves to the Zig side or it is rewritten over
  the cell layout and checked against `Vt` on the same scripts. That check, two engines graded against
  each other, is the milestone 23 (a capability-routed component OS with live replacement)
  demonstration the existing note argues for.

1. Considered instead: A, which avoids the dependency and loses on completeness and on the milestone 23
   demonstration; and C, which is a parser only (below).
2. See the shared answer.
3. Prior art read: the headers above. Ghostty's conformance history against `vttest` is claimed in
   `notes/glyphs.md:681`; I did not verify it.
4. See the shared answer, including the stale "no allocation" line.
5. Cost: a Zig 0.16 toolchain in the build (unmeasured), a pinned vendored tree under §18's policy, an
   allocator for the component, a cell-based expected-picture check (not estimated), and shim code
   (not estimated).
6. Hard to reverse once programs rely on its behavior; easy to reverse before. A dependency in the
   shipping graph is the irreversible category in `CLAUDE.md`.
7. Mostly yes. It is the option this tree already prefers for its milestone 23 claim, and the
   answer is less about effort than about whether the claim is worth a toolchain.

## C. `vte` (the Rust crate)

Read from the 0.15.0 source (crates.io, `Apache-2.0 OR MIT`, 57 KB).

- *What it is.* A parser: "The state machine doesn't assign meaning to the parsed data and is thus not
  itself sufficient for writing a terminal emulator" (its `README.md`). You implement `Perform` and
  receive `print`, `execute`, `csi_dispatch`, `esc_dispatch`, `osc_dispatch` and `hook`. The parser is
  `src/lib.rs`, 832 lines before its tests, plus `params.rs` (144).
- *What 0.15.0 adds.* An optional `ansi` feature (2,016 lines before tests) with a `Handler` trait
  that already names `set_scrolling_region`, `device_status`, `identify_terminal` and a
  `PrivateMode` enum including `SwapScreenAndSetRestoreCursor` (1049). It is still stateless: the
  grid, the regions, the second screen and the replies stay ours. It also pulls `alloc`, `log`,
  `bitflags` and `cursor-icon`, and `display_terminal` has no allocator.
- *What it would save.* The sequence-recognition half of `csi` (`lib.rs:1393-1442`): the parameter and
  intermediate-byte handling that this engine already has and tests. It saves none of the state the
  Part 2 list needs. The roadmap's phrase "much less complete" (milestone 29) understates this: it
  is not an engine at all.
- *Dependencies.* The parser needs `arrayvec` (not in `Cargo.lock`) and `memchr` (already there).

1. Considered instead: A. It is A with a different front half.
2. See the shared answer.
3. Prior art read: `ansi.rs` says it "was originally part of the `alacritty_terminal` crate", so the
   state half lives in a different crate.
4. See the shared answer.
5. Cost: the lines above, two dependencies, and the engine work of A on top.
6. Reversible before use; a dependency in the shipping graph once used.
7. No. At equal cost A wins over C, since C adds a dependency to save a parser that is already written
   and tested here. C is listed so that a refusal, if it comes, has its reason on record.
