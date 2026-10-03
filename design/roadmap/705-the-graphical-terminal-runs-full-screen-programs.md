---
status: PROPOSED
raised: 2026-10-02
milestone_dependencies: 29, 142
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# The graphical terminal runs full-screen programs

*(Title and slug are drafts. Number minted at promotion.)*

Raised by a maintainer-assigned lane, from calef's question of whether the graphical terminal can run
`vim`, `less` and `top`, and what the engine choice that §29 (framebuffer grant) deferred has to do
with it. Every claim below was checked in the tree or upstream on 2026-10-02 (UTC); a claim from memory
is marked as such.

The engine choice is a dependency decision (§46 (thin primitives or whole subsystems; we write everything in between)), so section 4 gives options and
no recommendation (A to E; E was added at calef's request on 2026-10-03 UTC).

## 1. The gap, and the goal

`crates/video_terminal` (milestone 29, the display terminal) implements printable text with UTF-8, CR,
LF, BS, TAB, CSI `A B C D H f J K m` (SGR with 256-colour and 24-bit) and RIS
(`crates/video_terminal/src/lib.rs:1373-1478`). Its own header lists what it leaves out: no scrolling
regions, no alternate screen, no origin mode, fixed tab stops, no mouse, no reporting sequences, no
reflow (`lib.rs:117-123`, `notes/glyphs.md` "Honest limits"). Anything else is swallowed whole
(`lib.rs:1373-1390`, `1475-1478`).

Milestone 142 (a text display good enough that people use it instead of a GUI) is PARTIAL and is about
glyph quality, scrollback, UTF-8 and grid size. None of its increments is about DEC-mode
compatibility, so this is not already owned.

**Goal.** `vim`-class editors, `less` and a repainting `top` draw correctly on the graphical terminal.
Today none of the three exists on nife: the only full-screen program in the tree is `rmle`
(milestone 169 (the smallest real text editor, as the forcing function for raw terminal input)), there is no pager (milestone 334 (colour and the pager: the spawn protocol's other two thirds)
is NOT-STARTED), and `watch` was cut (milestone 281 (`watch` holds exactly what `ps` holds, so it is nothing)). So the terminal half and the program half are
separate work. This proposal is the terminal half; the programs arrive through whatever ports them
(milestone 198 (a package manager, and the trivial install that makes a second customer possible) is the road).

**Measure.** Two numbers, both taken before and after.

1. *Screens matching a reference* on the host, driving the real programs. A host harness runs the
   program in a pty, feeds its output to `video_terminal`, and compares the 80x24 grid with the same
   session captured from a reference terminal. I built a throwaway version of this for the inventory
   below, using `tmux capture-pane` as the oracle. It is the measure that exists today.
2. *`vttest` menus passed*, by menu. Menus 1 (cursor movement), 2 (screen features, which holds
   scrolling regions, origin mode and tab stops), 6 (terminal reports) and 8 (VT102 insert and delete)
   are the ones full-screen programs need; 3, 4, 5 and 7 (character sets, double-size, keyboard,
   VT52) are out of scope. The menu names are from memory; `vttest` is not installed here (Homebrew
   has 20251205).

**Can `vttest` run on nife today? No.** It is a C program over `termios`, `ioctl(TIOCGWINSZ)`, `select`
and signals. The tree has no libc and no `termios` (`grep termios` finds only a note about a serial
tool and a rejected crate in milestone 40 (documentation as a system service)); the C seam is freestanding and makes no syscalls (§31 (the foreign-language seam: C holds no capabilities and makes no syscalls)).
`notes/c-seam.md` tier 3, "Full POSIX", is "later, if ever". `vttest` appears in the tree once,
as a reason in `notes/glyphs.md:681`. The closest measure is therefore the host harness: `vttest` in a
pty, its output into `Vt`, a screen dump at each "Push <RETURN>" pause, graded against goldens
recorded once from a reference terminal and reviewed by a person. `vttest` has no machine-readable
verdict, so the goldens are the verdict. Menu 6 has nothing to grade without the reply path in section 3.
I did not run `vttest`.

## 2. What the three programs need that is missing

Escape-sequence census, taken on the host (macOS, `TERM=xterm`, 80x24, vim 9.1, less from the OS). The
harness answered `ESC[6n` and `ESC[>c` like an xterm would.

| program | session | sequences it sent that `Vt` does not act on |
|---|---|---|
| vim | open, `:split`, scroll, `:q!` | `?1049h/l` (x1 each), `r` DECSTBM (x9, `1;24` and `1;11`), `>c` (x1), `6n` (x2), `?1h` + `ESC =`, `t` (x4), `?2004h`, `?1004h`, `>4;2m` |
| less | open, page, `b`, `k`, `q` | `ESC M` reverse index (x24), `?1049h/l` (x1 each), `?1h` + `ESC =` |

The inventory, each with how a program uses it.

- **DECSTBM scrolling region (`CSI r`).** vim sets `scroll_region = TRUE` whenever terminfo has `csr`
  (`src/term.c:3499-3503`) and emits `CSI 1;11 r` to scroll one window of a split (captured). `xterm`
  and `vt100` terminfo both define `csr=\E[%i%p1%d;%p2%dr`. Today a line feed always scrolls the whole
  screen (`line_feed`, `lib.rs:1328`).
- **Reverse index (`ESC M`, terminfo `ri`).** `less` scrolls backward with `ri` or `il1`, whichever is
  cheaper (`screen.c:1730-1745`, `sc_addline`), and only falls back to a repaint when both are absent.
  `ESC M` falls into the swallow-everything arm (`lib.rs:1386-1390`). Measured: after `space space b`
  and after `G k`, 22 of 24 rows differ from the oracle, and both repeat. This is the one gap I saw
  break a real program deterministically.
- **Alternate screen (`?1049`, `?47`, `?1047`).** Both programs enter with `smcup=\E[?1049h` and leave
  with `rmcup`. Without it the program works and leaves its last screen in the user's scrollback and
  the prompt under it. It needs a second grid, a saved cursor and a clear on entry.
- **Cursor position report (DSR 6, `CSI 6 n`).** vim sends `u7=\E[6n` in `check_terminal_behavior`
  (`term.c:4238-4245`) to learn ambiguous-width behaviour. Whether it waits for a reply, and for how long, I did not
  measure (the harness always answered). `rmle` sends no DSR (checked); real `kilo` sends it only when
  the window size is otherwise unavailable (from memory).
- **Device attributes (`CSI c`, `CSI > c`).** vim sends `ESC[>c` (`term.c:528`, `4226-4230`) to
  identify the terminal and pick feature sets; `xterm` terminfo carries `u8`/`u9` for the same query.
  Without a reply it assumes less.
- **Insert and delete, and cursor addressing.** `il dl ich dch ech` (`CSI L M @ P X`), `hpa`/`vpa`
  (`CSI G`, `CSI d`), scroll by n (`CSI S`, `CSI T`), and `sc`/`rc` (`ESC 7`, `ESC 8`) are all in the
  `xterm` entry and all swallowed today.
- **Small mode switches.** DECTCEM (`?25`, cursor hide), DECAWM (`?7`, procps `top` toggles it with
  `rmam`/`smam`, `src/top/top.c:173-174`), DECCKM and `ESC =` (`smkx`, application cursor keys).
  The last matters on the input side: with `?1h` set a real terminal sends `ESC O A` for Up, and
  `video_terminal::keymap` always sends `CSI A`. vim accepts both; whether the others do I did not check.

**`top` is the nearest to working.** procps `top` uses `clear`, `ed`, `el`, `cup`, `home`, reverse,
cursor hide and show, and autowrap off (`top.c:160-179`, `801-817`); it enters no alternate screen. Its
gap is DECAWM, which only matters when a line is wider than the grid.

**What the replay measured.** Each session was replayed through `Vt` and its final 24 rows compared
with `tmux capture-pane`. vim, eight scenarios (open, `G`, `ggdd`, Ctrl-E, `5dd`, and split-window
variants): every run matched except the split-window `Ctrl-E x3, Ctrl-Y` case, which differed in 10 of 24
rows in two of three runs. less: opening and paging forward matched; backward scroll (`b`, and `G`
then `k`) differed in every run, 22 of 24 rows. The harness is timing-sensitive (keys land 0.8 s apart
against the oracle's 0.6 s), and I did not find out why vim agreed as often as it did, so read it as
"the failures are real, the agreement is not a guarantee". The throwaway harness lives in the lane's scratch directory and is not committed.

## 3. Keeping the engine a value when it has to reply

Reports need the engine to produce bytes the program reads. Today it cannot: it "never writes to its
input", which is what sans-IO means here (`lib.rs:119-121`), and `csi_final` says so in a comment
(`lib.rs:1475-1478`). The property worth keeping is that the engine holds no endpoint and makes no
call, not that it never has output.

Proposal, the shape `take_damage` already has. `Vt::feed` keeps its signature. A new `Vt::take_reply`
hands back a bounded buffer of reply bytes and clears it, "taken, not read", exactly as damage is
(`lib.rs:1176` and the crate-level example). The component forwards those bytes where a keystroke
goes: into `display_terminal`'s `OP_BYTES` path (`components/src/display_terminal.rs:513-523`), so the
line discipline in front of it and then `OP_READRAW` deliver them to the program. The buffer has a
fixed bound; a flood of queries drops the newest reply and says so in `BUGS`, which fails towards a
program that times out rather than a terminal that blocks. The three-witness expected-picture check is
untouched, because replies do not change the grid. The wiring and the bound are a lane's to build and
an architect does not need to choose them.

Under option B the library's own shape is a callback (`GHOSTTY_TERMINAL_OPT_WRITE_PTY`, read in
`vt_terminal.h`); the shim would collect it into the same `take_reply` buffer.

## 4. The engine question, as options

Answers to questions 2 and 4 of the seven are the same for every option, so they are here once.

- *What does this tree do in the analogous case?* It built `line_editor` against the prior-art rule's
  default (`notes/line-discipline.md`, "The build-vs-reuse call"), built `video_terminal` before any
  foreign engine existed, and recorded the libghostty-vt cost in `notes/glyphs.md:669-715` and §37
  (text as a value), which says "Architect's call, not taken here". Vendoring policy for a foreign
  library is §18 (the PCIe transport), the RedoxFS precedent.
- *Is the premise true?* Partly. The gap is real and measured above, but it needs a program to hit it,
  and no such program ships (section 1). Separately, a premise in the record is stale. `notes/glyphs.md`
  and `notes/c-seam.md:95` list libghostty-vt as freestanding with "no allocation". Upstream's
  `allocator.h` says it "does require memory allocation for various operations", that on a native
  freestanding target the default allocator always fails, and that the consumer must supply one. So
  tier 1 of the C seam holds, but "fixed buffers, no allocation" does not. `display_terminal` has no
  allocator today.

### A. Extend `video_terminal`

The list in section 2. Estimate: 500 to 900 lines of engine code plus about as many of tests. This
is an estimate by analogy, not a measurement: the engine today is 1,711 lines before its tests, plus
`keymap.rs` (527) and `script.rs` (274), and `display_terminal.rs` is 536. A second grid and a
region-aware scroll touch `line_feed`, the damage record and the scrollback ring, which is where the
risk is, not the sequence count.

1. Considered instead: B and C, each under its own entry, and D for the ordering.
2. See the shared answer.
3. Prior art: xterm's control-sequence document and `vttest` define the behaviour (not read here; named
   from memory). Every real engine in section C or B is an existence proof.
4. See the shared answer.
5. Cost: lines above, no new dependency, no new toolchain, host-testable in milliseconds. Build and
   binary size: not measured; the debug `display_terminal` ELF is 1.77 MB on aarch64, which says
   nothing about the engine's share.
6. Fully reversible. No wire format changes, and the engine is behind `display_terminal`.
7. Yes, at equal cost: it keeps the three-witness proof (`pixel(x, y)`) with no rebuild, and it is the
   only option with no dependency in the shipping graph.

### B. libghostty-vt as a second engine behind the C seam

Everything below was read from `ghostty-org/ghostty` `main` on 2026-10-02, except where marked.

- *Maturity.* `include/ghostty/vt.h` opens "WARNING: This is an incomplete, work-in-progress API. It is
  not yet stable and is definitely going to change." `lib_vt.zig` says the behaviour is stable
  (extracted from a shipped terminal) and the API is not. The tree's own note already says to pin.
- *Toolchain.* `build.zig.zon` requires Zig 0.16.0 or later. Zig is not installed here (Homebrew has
  0.16.0), so the build-time cost is unmeasured. The source it compiles is large:
  `src/terminal` holds 164 Zig files totalling 6.2 MiB, tests included, line count not taken.
- *Freestanding.* Not "no allocation" (section above). It also needs a `std.Io` (`TinyIo` is the
  small one), and an entropy callback on targets with no `getrandom`
  (`GHOSTTY_SYS_OPT_RANDOM_SECURE`, `vt_sys.h`; whether the terminal path exercises it is not
  checked). Whether the Zig 0.16 freestanding build links without libc symbols is the first thing
  a lane would have to find out; I did not try.
- *Licence.* MIT (GitHub licence field, and the repository `LICENSE`).
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
6. Hard to reverse once programs rely on its behaviour; easy to reverse before. A dependency in the
   shipping graph is the irreversible category in `CLAUDE.md`.
7. Mostly yes. It is the option this tree already prefers for its milestone 23 claim, and the
   answer is less about effort than about whether the claim is worth a toolchain.

### C. `vte` (the Rust crate)

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
  section 2 list needs. The roadmap's phrase "much less complete" (milestone 29) understates this: it
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

### D. A then B, or B then A

The two are not exclusive: B behind the seam does not remove A, and `notes/glyphs.md` already
recommends keeping `video_terminal` as the reference the foreign engine is checked against.

- **A then B.** The full-screen goal is met without a toolchain, and B arrives later for its own
  reason (milestone 23). The cost is that the expected-picture check is not rebuilt until B, and A's
  region and alternate-screen code becomes the reference B is graded against, which is a useful
  property and not an accident.
- **B then A.** Full-screen programs come from B's completeness first. The cost is that the goal waits
  on the Zig toolchain and the cells-based check, and A is then written afterwards as a second engine
  for no customer reason. I see no argument for this order other than the milestone 23 claim.

1. Considered instead: A alone, B alone. Already above.
2. to 4. See the shared answers.
5. Cost: the sum, spread over two lanes; the first delivers the goal.
6. As reversible as each half.
7. For A then B, yes at equal cost, because the first step delivers the measure. For B then A, the
   ordering is effort-neutral and I would not choose it.

### E. Replace `video_terminal` with libghostty-vt

B without keeping A's engine: libghostty-vt becomes the only VT engine behind `display_terminal`, and
`video_terminal` is retired (or kept only as a host-side test fixture). Facts about the library are
those in B; this entry is about what losing the second engine costs and buys.

- *Against.*
  - `video_terminal`'s `pixel(x, y)` carries the three-witness check: the component draws, the kernel
    test predicts, and xtask grades the scanout. libghostty-vt gives cells, so the check has to be
    rebuilt on cells with a small renderer we own.
  - Losing the second engine loses differential testing. §37 (text as a value) and `notes/glyphs.md`
    call that comparison the stronger milestone 23 (a capability-routed component OS with live
    replacement) demonstration.
  - As the only engine, every upstream API break ("definitely going to change") breaks the display
    path. That path then needs the Zig toolchain and a supplied allocator just to work.
- *For.*
  - No 500 to 900 lines of our own engine code to keep up.
  - No double maintenance, and no triage of disagreements between two engines.
  - Immediate completeness: DEC modes, resize, grapheme clustering, mouse, and a `vttest` history
    (claimed in `notes/glyphs.md`, not verified here).
- *What would make E more attractive.* An upstream API declared stable.

1. Considered instead: A (keeps the second engine, costs the lines), B and D (keep both engines).
2. to 4. See the shared answers.
5. Cost: B's toolchain, vendoring and allocator costs, plus the cell-based check, minus the engine
   lines of A. Not measured beyond what B records.
6. Harder to reverse than B: once `video_terminal` is gone, bringing it back is a rewrite, and the
   display path depends on a library whose API is declared unstable.
7. Not decided here. At equal cost I would still keep two engines, because the differential check is
   the claim; E wins only if that claim is not worth the second engine's upkeep.

## Ruling

calef, 2026-10-03 UTC: "not yet". The display paths sit largely unused (the 2026-09-30 ruling) and
nothing is blocked, so the engine choice waits. The proposal stays `PROPOSED` and nothing is built.

Revisit when either of these happens:

- someone needs a full-screen program at a display path, or
- libghostty-vt stabilises its C API.

## 5. Priority, and whether the serial console has the same gap

calef ruled on 2026-09-30 that graphics is not at boot and "will sit there largely unused for some
time" (PR #1493, graphics on demand, number provisional). That makes the question of which terminal a customer meets first a real one.

**The serial path has no such gap, because nife does not interpret escapes there.**
`components/src/console.rs` copies the bytes of an `OP_WRITE` to the UART verbatim, and the terminal
contract says "ANSI from the application reaches the screen intact"
(`notes/terminal-contract.md:177-179`). The user's host terminal (`xterm`, `Terminal.app`, `tmux`)
is the VT engine, so `vim`-class programs get a complete one for free. `rmle` already does this: it
emits `ESC[?25l`, `ESC[H`, `ESC[K`, `ESC[7m` and a cursor-position `CSI H` (`components/src/rmle.rs:526-560`) and
depends on nothing the display terminal lacks. Replies from the host terminal arrive on the UART
as input bytes and reach the program through `OP_READRAW` while raw mode is on
(`notes/terminal-contract.md:105-115`), so DSR and DA already work in principle on the serial path;
I did not test it.

The serial path has a different gap. The contract carries no window size, so `rmle` fixes 80x24
(`rmle.rs` "No terminal size negotiation") and a differently sized host window redraws wrongly
(`notes/terminal-contract.md:185-188`). That is a separate piece of work and is not this proposal.

**Where this proposal matters.** A screen the user is looking at, with no host terminal in the
loop: the `screen` session of PR #1493 on a machine with a display, and the firmware-screen console, where
`console.rs` hands "the same stream" to `display_terminal` beside the UART (`MODE_SCREEN`, `console.rs:19-30`).
A machine with a monitor and no serial cable meets this gap with the first
full-screen program it runs.

So the order of customer relevance is: the serial path works now and needs a size fix; the display
paths need this milestone, and there is no program to measure against until one is ported. I
could not verify that a program launched from `screen` reaches `display_terminal` through
`OP_RAWMODE` end to end; #1493 is unmerged and I did not read its guest tests.

## 6. Dependencies, and what is blocked

- The graphics-on-demand work (#1493, provisional number 632, `screen` launched from the swish prompt) is unaffected by
  the engine choice: the engine sits behind `display_terminal`, and the launch builds the same
  endpoints whichever engine answers them. It is not listed in the frontmatter because it is not on
  `main` and its number is provisional.
- Milestone 29 (the display terminal) and milestone 142 (a text display good enough that people use
  it instead of a GUI) are the blocks this touches. 142's remaining increments are about glyph
  quality and do not collide with the engine's state work.
- Blocked until the choice is made again: options B, D and E cannot start. Option A needs no ruling and could start
  now, but A is also step one of D, so starting it quietly would pick D's first half.
- A program to measure against (a port of `less`, `vim` or procps `top`) is milestone 198's road and
  not this one's. The host-harness measure does not need one.

## What would be built, if the answer is A

1. A host harness in `xtask` (name provisional): a program in a pty, its output into `Vt`, a screen
   dump compared with a committed golden.
2. `Vt::take_reply`, DSR 6, DA, then `ESC M`, `ESC 7`/`8`, `CSI L M @ P X G d S T`, DECSTBM, DECAWM,
   and the alternate screen, each with a test that fails first.
3. `display_terminal` forwarding the reply bytes into its keystroke path.
4. `notes/glyphs.md` and `notes/terminal-contract.md` updated; the stale "no allocation" claim
   corrected wherever it is repeated.
