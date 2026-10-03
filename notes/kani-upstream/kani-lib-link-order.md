# Draft: the `kani_lib.c` link-order follow-up to model-checking/kani#4913

*Name: provisional, minted 2026-10-03 (UTC) by milestone 635 (riscv64 proofs check against the
riscv64 model). The parent note is [kani-upstream.md](../kani-upstream.md). **Not posted.** calef
reads every upstream word before anything goes to model-checking/kani, and nothing here has been
pushed to calef/kani.*

This is the pull request promised in calef's reply to tautschnig on #4913
([discussion_r4172069063](https://github.com/model-checking/kani/pull/4913#discussion_r4172069063)):
"I'd rather do the precompiled-object change as a follow-up, because it changes the linked model
for every run, host runs included." tautschnig's comment it answers traced the cause and proposed
the fix; this draft follows his proposal except where the section "Call-outs" says otherwise.

## Branch plan

- Fork: `calef/kani` (exists; default branch `main`).
- Branch: `kani-lib-link-order` (provisional), cut from upstream `main`, not from #4913's
  `riscv64-target`. The fix stands on its own: it changes host runs on macOS today, so it should not
  wait for #4913 or be reverted with it. Upstream `main` was `1640445` when this was drafted, and
  `kani-driver/src/call_goto_cc.rs` there is unchanged from the `kani-0.67.0` tag nife builds.
- One commit, the diff below plus the test below. Kani squash-merges, so the pull request title is
  what lands; it is written in the imperative like Kani's recent history ("Only rewrite the jumps
  that leave a `prev` loop's body (#4937)").
- Before opening: `cargo build-dev`, `./scripts/kani-fmt.sh --check`, clippy with `-D warnings` on
  `kani-driver`, the new test, and the `kani`, `expected`, `ui` and `script-based-pre` suites on an
  Apple Silicon Mac (the host where the model changes) and on x86_64 Linux (where it should not).
  None of this has been run on upstream `main` yet; the evidence below is from nife's patched 0.67.0.

## The change

`kani-driver/src/call_goto_cc.rs`, `link_goto_binary`. This is the hunk nife carries as the second
commit of `patches/kani-0.67.0-riscv64-target.patch`, and it applies to upstream `main` as it stands:

```rust
        // Compile `kani_lib.c` on its own and link the object after the Rust inputs. When goto-cc
        // links goto binaries, the first input's `__CPROVER_architecture_*` symbols win, so the
        // machine model Kani wrote survives. A C source in the link command instead is compiled
        // with goto-cc's own (host) configuration, which then overwrites that model, and CBMC
        // loads its C library for the host rather than for the model Kani wrote.
        let kani_lib = output.with_extension("kani_lib.o");
        let mut compile = Command::new("goto-cc");
        compile.arg("-c").arg(&self.kani_lib_c).arg("-o").arg(&kani_lib);
        self.run_suppress(compile)?;
        self.record_temporary_file(&kani_lib);

        let mut args: Vec<OsString> = Vec::new();
        args.extend(inputs.iter().map(|x| x.clone().into_os_string()));
        // A `--c-lib` given as C source still overwrites the model, for the reason above.
        args.extend(self.args.c_lib.iter().map(|x| x.clone().into_os_string()));
        args.push(kani_lib.into_os_string());
```

The removed `TODO` ("kani_lib_c is just an empty c file") was already stale: the file holds
`__rust_alloc` and friends.

## The test (proposed, not yet written upstream)

A `tests/script-based-pre/linked_machine_model/` test builds a one-harness crate with
`--keep-temps`. It reads every `__CPROVER_architecture_*` symbol from the harness's `.symtab.out`
and its linked `.out` with `goto-instrument --show-symbol-table`, strips the `(__CPROVER_integer)`
cast the link adds, and fails if any field in the `.symtab.out` is missing or different in the
`.out`. It hard-codes no host's values, so it runs on every CI runner;
it can only fail on a host whose goto-cc configuration differs from Kani's model, which today is
macOS (both architectures, `char_is_unsigned`; arm64 also `long_double_width`). nife's
`script/verify-riscv64` is this check, and was red on 7 of 7 harnesses before the change and green
after it.

## Commit message

```
Link kani_lib.c as a precompiled goto object after the Rust inputs

link_goto_binary passed kani_lib.c to goto-cc as C source. goto-cc
compiles a C source with its own host configuration, and the resulting
__CPROVER_architecture_* symbols replaced the machine model Kani wrote
into the symbol table. CBMC then loaded its C library for the host, and
interpreted C (kani_lib.c, the CPROVER library, any --c-lib) with the
host's char signedness and long double width.

When goto-cc links goto binaries the first input's symbols win, so
compiling kani_lib.c on its own and linking the object last keeps
Kani's model. The compile is the one that already ran inside the link,
so the cost is one more goto-cc process per harness.

A --c-lib given as C source still overwrites the model; that is left
as it is and commented.
```

## Pull request body

Title: Link `kani_lib.c` as a precompiled goto object after the Rust inputs

> `link_goto_binary` passes `kani_lib.c` to `goto-cc` as C source. `goto-cc` compiles it with its
> own host configuration, and the `__CPROVER_architecture_*` symbols that produces replace the
> machine model Kani wrote into the symbol table. @tautschnig traced this in review of #4913
> ([comment](https://github.com/model-checking/kani/pull/4913#discussion_r4143913613)), and this is
> the follow-up I offered there.
>
> This PR compiles `kani_lib.c` on its own and passes the object after the Rust inputs. When
> `goto-cc` links goto binaries the first input's symbols win, so the model Kani wrote survives.
>
> **What changes for host runs.** On Linux, nothing that I could measure: on x86_64 and aarch64
> Linux, `goto-cc`'s configuration already matches Kani's model field for field. On macOS it does
> not. On an Apple Silicon Mac the linked model goes from `char_is_unsigned = 0` and
> `long_double_width = 64` (goto-cc's) to `1` and `128` (Kani's aarch64 model), and CBMC adds its
> library for `arm64` either way. That is why this is separate from #4913: it changes what host
> runs check against, and I think it is a fix, but it deserves its own review. If the 128-bit
> `long double` in Kani's aarch64 model is wrong for `aarch64-apple-darwin`, that is a model bug
> this PR would expose rather than cause; happy to open an issue. Apple's arm64 ABI has a 64-bit
> `long double` and a signed `char`, unlike aarch64 Linux. With Apple clang 21.0.0, `clang -arch
> arm64 -dM -E -x c /dev/null` gives `__SIZEOF_LONG_DOUBLE__ 8` and `__LDBL_MANT_DIG__ 53` (the
> same as `double`) and leaves `__CHAR_UNSIGNED__` undefined. With `--target=aarch64-linux-gnu` the
> same command gives `16`, `113` and `__CHAR_UNSIGNED__ 1`.
>
> **What it does not change.** `os` is not part of Kani's model, so the linked binary still names
> the host's OS. A `--c-lib` given as C source still overwrites the model the same way; I left that
> alone and added a comment, since precompiling user C is a larger change.
>
> **Testing.** A new script-based test reads every `__CPROVER_architecture_*` field from the
> harness's `.symtab.out` and its linked `.out` and fails if any field Kani wrote changed. It hard
> codes no values. Before this change it fails on macOS and passes on Linux; after it, it passes on
> both. Downstream, the same check runs in a project that proves seven riscv64 harnesses with
> #4913's `--target`. It was red on all seven before this change and green after. CBMC gave
> identical property statuses and SAT variable and clause counts under both models.
>
> Resolves #ISSUE-NUMBER *(none filed; drop the line, or file the issue offered in #4913 first)*
>
> By submitting this pull request, I confirm that my contribution is made under the terms of the
> Apache 2.0 and MIT licenses.

## Call-outs for calef before posting

The `long double` claim in the body was checked on patagonia on 2026-10-03 (UTC), with Apple clang
21.0.0 (clang-2100.3.34.2): a compiled `sizeof(long double)` is 8 and `LDBL_MANT_DIG` is 53 for
`-arch arm64`. It is no longer on this list.

1. Per link, not once. tautschnig suggested compiling `kani_lib.c` once, "for example in
   build-kani". This draft compiles it in `link_goto_binary`, once per harness, which is the same
   compile the link already did. It changes one function and no install layout. Compiling in
   build-kani instead means shipping a goto object in the release bundle and teaching
   `InstallType` to find it; fewer processes, more moving parts. Would we choose per-link if both
   cost the same? Yes for the first PR, because the bundle layout is a maintainer's call; the body
   could offer the build-kani shape.
2. The Linux "field for field" claim is half measured. aarch64 Linux was observed in nife's CI
   (`verify` run 37108539047: only `arch` differed, and only because the model was riscv64's). x86_64
   Linux is tautschnig's measurement in his #4913 comment, not ours. Rerun before posting.
3. The test is not written. The body describes it in the past tense; it must exist and have run
   on both hosts before the body is true.
