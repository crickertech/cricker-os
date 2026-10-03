# Draft: the `kani_lib.c` link-order follow-up to model-checking/kani#4913

*Name: provisional, minted 2026-10-03 (UTC) by milestone 635 (riscv64 proofs check against the
riscv64 model). The parent note is [kani-upstream.md](../kani-upstream.md). **Not posted.** calef
reads every upstream word before anything goes to model-checking/kani, and nothing here has been
pushed to calef/kani. Measured and rewritten 2026-10-03 (UTC) by the lane
`lane/kani-lib-link-order`.*

This is the pull request promised in calef's reply to tautschnig on #4913
([discussion_r4172069063](https://github.com/model-checking/kani/pull/4913#discussion_r4172069063)):
"I'd rather do the precompiled-object change as a follow-up, because it changes the linked model
for every run, host runs included (on macOS `char_is_unsigned` would go from 0 back to the aarch64
model's value)." tautschnig's comment it answers
([discussion_r4143913613](https://github.com/model-checking/kani/pull/4913#discussion_r4143913613))
traced the cause and proposed the fix.

**One decision is calef's before posting**: what to do about Kani's aarch64 model saying `char` is
unsigned on Apple Silicon, which this change makes CBMC honour. See "The design question".

## What the measurements changed in the earlier draft

The first draft (same file, merged earlier on 2026-10-03) made three claims the machine overruled:

1. *"On an Apple Silicon Mac the linked model goes from `char_is_unsigned = 0` and
   `long_double_width = 64` to `1` and `128`."* Wrong about `long double`. Kani's aarch64 model has
   split on the OS since #2757 (2023): `long_double_width` is 128 on Linux and 64 elsewhere. A host
   run here writes 64 and links 64. The 128 the draft saw was nife's riscv64 model. On this Mac only
   `char_is_unsigned` changes, 0 to 1, which is what calef's #4913 comment already said.
2. *"It can only fail on macOS (both architectures)."* Wrong both ways. x86_64 macOS matches
   Kani's model on every field Kani writes (measured by proxy, below). aarch64 Linux does not:
   `goto-cc` there writes `wchar_t_is_unsigned = 0`, Kani writes 1.
3. *"On Linux, nothing that I could measure."* True for x86_64 Linux, false for aarch64 Linux, for
   the reason in 2. There the change is a correction: gcc on aarch64 Linux says `wchar_t` is
   `unsigned int`, so Kani's 1 is right and `goto-cc`'s 0 is wrong.

## Branch plan

- Base: upstream `main` at `1640445da3fa0d2cb19739652057dc992fef3f18` (2026-10-02, "Stop a SIMD
  comparison with float mask lanes from aborting the crate (#4951)"). Unchanged since the first
  draft.
- Fork `calef/kani`, branch `kani-lib-link-order` (provisional). Cut from `main`, not from #4913,
  because it changes host runs and should not wait for or be reverted with #4913.
- One commit: [kani-lib-link-order.patch](kani-lib-link-order.patch), `git format-patch` form. To
  use it: `git checkout -b kani-lib-link-order 1640445da && git am
  notes/kani-upstream/kani-lib-link-order.patch`. 5 files, 96 lines added, 4 removed.

## The change

`kani-driver/src/call_goto_cc.rs`, `link_goto_binary`: the hunk nife carries as the second commit
of `patches/kani-0.67.0-riscv64-target.patch`, applied unchanged. It compiles `kani_lib.c` with
`goto-cc -c` to `<output>.kani_lib.o`, records that as a temporary, and passes it after the Rust
inputs and any `--c-lib`. The stale `TODO` ("kani_lib_c is just an empty c file") goes.

The mechanism was re-measured here with CBMC 6.11.0, using a C file compiled with `-funsigned-char`
as the stand-in for Kani's symbol table. Object then C source gives the source's model. Object then
object keeps the first; reversed, the second object's model wins.

## The test

`tests/script-based-pre/linked_machine_model/`: a one-harness file, run with `--keep-temps` in a
`mktemp -d` directory. For every `*.symtab.out` it reads each `__CPROVER_architecture_*` symbol
from that file and from the linked `.out` with `goto-instrument --show-symbol-table`, strips the
`(__CPROVER_integer)` cast, and requires every line of the first to appear unchanged in the second
(`comm -23`). Fields the link adds and Kani does not write (`os`, `argument_evaluation_order`) are
allowed. It fails loudly if no `.symtab.out` exists or one has no such symbols, so it cannot pass
vacuously. No host values are written into it.

It proves it can fail, on this Mac:

| build | result |
|---|---|
| upstream `main`, no hunk | **red**, compiletest exit 1: `__CPROVER_architecture_char_is_unsigned 1` written, `0` linked |
| with the hunk | **green**, compiletest exit 0, 1 passed |

Kani's CI runs its regression on `macos-15-intel`, `ubuntu-22.04`, `ubuntu-24.04`, `macos-14` and
`ubuntu-24.04-arm` (`.github/workflows/kani.yml`). By the table under "What changes for each host",
the test without the hunk would be red on `macos-14` and `ubuntu-24.04-arm` and green on the other
three. That is inferred, not run: I have no Linux Kani build.

## Runs on patagonia, 2026-10-03 (UTC)

Apple Silicon, macOS 26 (Darwin 25.6.0), toolchain `nightly-2026-09-25` (Kani's pin). CBMC 6.11.0
and kissat 4.0.4 unpacked from the cached Homebrew bottles into the session scratchpad, cvc5 1.3.0
(Kani's CI pin) from its GitHub release zip, nothing installed. Every job niced, one at a time;
load average was 7 to 36 from other lanes throughout.

| run | exit | result |
|---|---|---|
| `cargo build-dev -- -j 4`, upstream `main` | 0 | 186 s wall, 1.73 GB peak resident |
| `cargo build-dev -- -j 4`, with the hunk | 0 | incremental, kani-driver only |
| `./scripts/kani-fmt.sh --check` | 0 | clean |
| `cargo clippy -p kani-driver -- -D warnings` | 0 | clean |
| new test, no hunk | 1 | red, as above |
| new test, with hunk | 0 | green |
| `kani` suite | 1 | 618 passed, 2 failed, 23 ignored. Both failures are `#[kani::solver(cvc5)]` harnesses (`Quantifiers/arbitrary_range`, `Quantifiers/typed_variables`); cvc5 was absent |
| those two, rerun with cvc5 1.3.0 on `PATH` | 0 | 2 passed |
| `expected` suite | 0 | 504 passed, 15 ignored |
| `ui` suite | 0 | 153 passed |
| `script-based-pre` suite | 0 | 91 passed, 1 ignored. The ignored one is the new test: compiletest marks a test whose stamp is current as ignored, and it had just passed. It ran in the row below and in the model-fix run |
| `cargo-kani` suite | 0 | 71 passed |
| `git status` after all suites | | only the commit's own files; no `.kani_lib.o` left behind |

Not run: x86_64 Linux and aarch64 Linux suites (no such Kani host; cordoba belongs to another
agent), `coverage`, `cargo-coverage`, `std-checks`, firecracker.

## What changes for each host

Fields Kani writes that differ between Kani's model and `goto-cc`'s host configuration, which is
what the linked `.out` carried before this change. Every other field Kani writes was equal.

| host | before (goto-cc) | after (Kani's model) | right answer | how measured |
|---|---|---|---|---|
| aarch64 macOS | `char_is_unsigned = 0` | 1 | 0: Apple clang leaves `__CHAR_UNSIGNED__` undefined, Rust's `c_char` is `i8`, CBMC's own `config.cpp` says signed for `macos`/`arm64` | Kani run, `.symtab.out` and `.out` |
| x86_64 macOS | none | none | | proxy: `goto-cc -arch x86_64` on this Mac against Kani's x86_64 model in source |
| aarch64 Linux | `wchar_t_is_unsigned = 0` | 1 | 1: gcc 13.3 on Ubuntu 24.04 arm64 gives `__WCHAR_TYPE__ unsigned int` | CBMC 6.11.0 `.deb` in an arm64 Ubuntu container; Kani's side from source |
| x86_64 Linux | none | none | | CBMC 6.11.0 `.deb` in an amd64 Ubuntu container (emulated); agrees with tautschnig's #4913 measurement |

So this change corrects aarch64 Linux and, on Apple Silicon, swaps a right value for a wrong one
because Kani's model is wrong there.

## The design question: Kani's aarch64 model on Apple Silicon

### (a) What Kani's model does per target today

`new_machine_model` in `kani-compiler/src/codegen_cprover_gotoc/compiler_interface.rs` matches on
architecture, then on OS for two fields of aarch64 only: `long_double_width` (128 on Linux, else
64) and `wchar_t_is_unsigned` (Linux only). `char_is_unsigned` is `true` for every aarch64 target.
x86_64 has no OS split and needs none. `check_target` accepts exactly four hosts:
`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-*`, `arm64-apple-*`.

Prior discussion, read: in #1167 (Apple M1 support,
https://github.com/model-checking/kani/issues/1167) diagprov measured aarch64 Linux on
2022-07-20, AGSaidi cited AAPCS64 for unsigned `char`, and diagprov found `wchar_t` signed on an M1.
Nobody measured `char` on the M1, and the `wchar_t` split is what landed. #2757
(https://github.com/model-checking/kani/pull/2757, aarch64 Linux support) added the `long double`
split. Searches of issues and pull requests for `char_is_unsigned`, `long_double_width`,
`kani_lib.c`, "machine model macos" and "signed char apple" found nothing else on this.

### (b) Is there a harness whose verdict changes?

None found. Kani's own `char` type (`CInteger(Char)`) is lowered with Kani's model at codegen, so
the link never touches Rust code. What the linked symbols decide is how CBMC parses its C library
(`goto-instrument --add-library`, "Adding CPROVER library (arm64)") and nothing in `kani_lib.c`
uses `char`. So the question is which CBMC 6.11.0 library functions depend on the signedness of
plain `char`, and whether a harness reaches them.

- Without `-Z c-ffi`, a harness reaches only the builtins Kani declares in
  `cprover_bindings/src/goto_program/builtin.rs` (`memcmp`, `memcpy`, `memset`, the math
  functions, and so on). `memcmp` and `memset` cast to `unsigned char` explicitly.
- With `-Z c-ffi`, more of the library is reachable. I grepped all 34 files of the CBMC 6.11.0 C
  library. The plain-`char` variables are in `strcpy`-style copies and `getline` (compared only with `'\0'`), `strtol` (compared with
  ASCII ranges, where a high-bit byte fails both ways), and `intrin.c` (MSVC interlocked
  intrinsics, Windows only). `strcmp`, `strcasecmp` and `strncmp` copy into `unsigned char` first.
- `--c-lib` given as C source still overwrites the model, so it is unaffected. Given as a `goto-cc`
  object, its code was typed when compiled, so only the library is reinterpreted.

I ran five harnesses: `[u8]` ordering over `memcmp`; `strcmp`, `strchr` and `strtol` on `0xff` under
`-Z c-ffi`; and Rust's `c_char` sign. Each gave the same verdict and the same SAT variable and clause
counts before the change, after it, and after it with the model fix in (c). So the Apple `char`
bug is wrong in the model and harmless in every case found. That is a property of CBMC's library
being written carefully, not a guarantee.

### (c) What a per-target model costs

One line in `new_machine_model`, the same shape as the `wchar_t` split already there:

```rust
            // Apple's arm64 ABI makes `char` signed, unlike AAPCS64 and Linux.
            // https://developer.apple.com/documentation/xcode/writing-arm64-code-for-apple-platforms
            let char_is_unsigned = matches!(os, Os::Linux);
```

Measured on top of the link-order commit, the incremental `cargo build-dev` exited 0. The new test
was green, now with `char_is_unsigned = 0` in both files. Suites: `kani` 620 passed (cvc5 present), `expected`
504, `ui` 153, `script-based-pre` 92 (stamps invalidated, so every test ran) and `cargo-kani` 71, each
exit 0 with no failures. No test in the tree hard-codes
`char` signedness on macOS. x86_64-apple-darwin needs nothing. A test for the model itself would
have to name host values, which is what the link test avoids; a unit test of `new_machine_model`
would need a `Session` and is more than the line it guards.

### Options

1. **Model fix first, as its own small pull request, then this one.** Each does one thing. With
   the model fixed, this pull request changes nothing on macOS and corrects aarch64 Linux, and its
   description becomes simple. Cost: a second review, and this one waits for the first or names it
   as a dependency. Reversible until posted; after posting, as reversible as any upstream PR.
2. **Both in one pull request.** One review, and no window where macOS checks against the wrong
   `char`. Cost: two purposes under one squash-merged title, and the Apple `char` question (a model
   change any maintainer may want to weigh separately) rides on a link fix already promised.
3. **Open as drafted, and file an issue for the Apple `char`.** Keeps the promise exactly as made.
   Cost: until the issue is fixed, Apple Silicon runs check CBMC's library against the wrong
   `char`, which (b) found harmless in every case tried. And calef's #4913 comment already flagged
   the flip, so a reviewer may ask why it was left.
4. **Also precompile `--c-lib` sources** (in any option). Removes the last way C source overwrites
   the model. Cost: a loop over `c_lib` and a temp object each; user C would still be compiled
   with the host's configuration, so it fixes the symbols, not the typing. Considered and left
   out: it widens a promised fix into new behaviour for an experimental flag.
5. **Compile `kani_lib.c` once in build-kani**, tautschnig's other suggestion. Fewer processes, but
   a goto object in the release bundle and `InstallType` taught to find it. The bundle layout is a
   maintainer's call; the body offers it.

**Recommendation, for calef to decide: option 1, both posted together.** Open the one-line model
fix first, then this pull request saying it is on top of that one, so a reviewer sees the order
and neither waits on a reply from us. The question that decides it is whether the same review
should judge an ABI fact and a link-order fix; they share no code and have different evidence.
Would we choose this if both cost the same? Yes: option 2 is less work, and the recommendation is
against it. The draft PR body for the model fix is at the end of this note.

## Commit message

In the patch. Its claims match the measurements above.

## Pull request body, ready to paste

Title: Link `kani_lib.c` as a precompiled goto object after the Rust inputs

> `link_goto_binary` passes `kani_lib.c` to `goto-cc` as C source. `goto-cc` compiles it with its
> own host configuration, and the `__CPROVER_architecture_*` symbols that produces replace the
> machine model Kani wrote into the symbol table. CBMC reads those symbols back when it adds its C
> library. @tautschnig traced this in review of #4913
> ([comment](https://github.com/model-checking/kani/pull/4913#discussion_r4143913613)), and this is
> the follow-up I offered there.
>
> This PR compiles `kani_lib.c` on its own and passes the object after the Rust inputs. When
> `goto-cc` links goto binaries the first input's symbols win, so the model Kani wrote survives.
>
> **What changes for host runs.** Comparing each field Kani writes with what the link produced
> before, with CBMC 6.11.0:
>
> - x86_64 Linux and x86_64 macOS: nothing differs.
> - aarch64 Linux: `wchar_t_is_unsigned` was 0 (`goto-cc`'s) and is now 1 (Kani's). gcc on aarch64
>   Linux has `wchar_t` as `unsigned int`, so this is a correction.
> - Apple Silicon: `char_is_unsigned` was 0 and is now 1, because Kani's aarch64 model says
>   unsigned for every OS. Apple's arm64 ABI has a signed `char`, so that is a model bug this PR
>   exposes; #MODEL-PR fixes it, and this PR is meant to land after it.
>
> I could not find a harness whose verdict changes. The only C these symbols affect is CBMC's
> library, and the functions I checked (`memcmp`, `strcmp`, `strchr`, `strtol`) gave the same
> results and formula sizes either way.
>
> **What it does not change.** `os` is not part of Kani's model, so the linked binary still names
> the host's OS. A `--c-lib` given as C source still overwrites the model; I left that alone and
> added a comment. Compiling `kani_lib.c` once in build-kani would save a process per harness but
> changes the bundle layout; happy to do that instead if you prefer.
>
> **Testing.** A new script-based test, `linked_machine_model`, compares every
> `__CPROVER_architecture_*` field in each harness's `.symtab.out` with the linked `.out`. It hard
> codes no values. On an Apple Silicon Mac it fails without this change and passes with it; by the
> list above it should also fail without it on aarch64 Linux, which I could not run. On that Mac,
> `kani`, `expected`, `ui`, `script-based-pre` and `cargo-kani` pass, plus `kani-fmt.sh --check`
> and clippy with `-D warnings` on `kani-driver`.
>
> By submitting this pull request, I confirm that my contribution is made under the terms of the
> Apache 2.0 and MIT licenses.

Drop the "#MODEL-PR" sentence and the last clause of that bullet under option 3; under option 2,
replace it with "This PR also fixes that model".

### The model-fix pull request, for options 1 and 2

Title: Make `char` signed in the machine model for Apple arm64

> Kani's aarch64 machine model sets `char_is_unsigned = true` for every OS. That is AAPCS64 and
> Linux, but Apple's arm64 ABI makes `char` signed
> ([Apple](https://developer.apple.com/documentation/xcode/writing-arm64-code-for-apple-platforms)),
> Rust's `c_char` is `i8` on `aarch64-apple-darwin`, and CBMC's own configuration for `macos`/`arm64`
> says signed. The model already splits `long_double_width` and `wchar_t_is_unsigned` on the OS for
> the same reason (#2757, #1167); this does the same for `char`. On an Apple Silicon Mac, `kani`,
> `expected`, `ui`, `script-based-pre` and `cargo-kani` pass with it.
>
> By submitting this pull request, I confirm that my contribution is made under the terms of the
> Apache 2.0 and MIT licenses.

## calef's checklist before posting

- [ ] Pick an option. Everything above is written for option 1.
- [ ] `git am` the patch on `1640445da` (or rebase if `main` moved) and rerun the new test once; it
      should be red without the hunk and green with it.
- [ ] Apple's page is cited from CBMC's `config.cpp` comment and Kani's own source, not read here.
      Open it once before linking it.
- [ ] "x86_64 macOS: nothing differs" rests on `goto-cc -arch x86_64` on an arm64 Mac, not on an
      Intel Mac. Kani's `macos-15-intel` CI job is the real check.
- [ ] aarch64 Linux is measured for `goto-cc` and read from source for Kani; no Linux Kani run.
- [ ] The body says "I". Every run in it was made by an agent on patagonia, as recorded here.
