# The x86_64 target spec, field by field

*An appendix to [`notes/std.md`](../std.md), which is the page to read. This file holds milestone
184's reasoning for each field of `targets/x86_64-unknown-nife.json`, the SSE probe and the
entry-point stub. It was moved here verbatim from the main page on 2026-09-25 (UTC), under [§212 (a
prose budget)](../../design/decisions/0212-a-prose-budget-for-every-document.md). The directory
`notes/std/` and this file's stem are provisional names, minted that day by the lane that split the
file. Naming is calef's.*

The records this file cites by number:

- milestone 184 (extend the `std` port)


## The x86_64 spec, field by field (milestone 184)

It is the built-in `x86_64-unknown-none` spec with four deliberate differences, because that spec
is a kernel's and this one is a ring-3 program's:

- `"code-model": "small"`, not `"kernel"`. The kernel model assumes every symbol lives in the
  top 2 GiB of the address space, which is where a higher-half kernel is and where a program linked
  at `0x40_0000` is not. It happens to link anyway (a low address also fits a sign-extended 32-bit
  displacement), which is why the `no_std` programs built on `x86_64-unknown-none` work, but the
  small model is the one whose assumption is true here.
- `"relocation-model": "static"`, and no `position-independent-executables`. The kernel's ELF
  loader maps segments at their link addresses and applies no relocations. The built-in spec
  defaults to static-PIE, which `.cargo/config.toml` overrides for the `no_std` programs by flag;
  here it is in the spec itself, the same as the other two nife targets.
- `"os": "nife"`, `"singlethread": true`, `"executables": true`, `metadata.std: true`, as on
  the other two.
- The feature list, `rustc-abi`, `disable-redzone`, `data-layout`, `llvm-target` and
  `stack-probes` are copied unchanged. `rustc-abi: softfloat` is required: current nightlies refuse
  a soft-float x86 custom target without it.

How you would know if SSE leaked in. Absence of `xmm` in a program that does no float work
proves nothing, so the check is a probe that does real `f64` and `f32` arithmetic. Built for
`x86_64-unknown-nife` it disassembles to zero instructions naming an `xmm`, `mm` or x87 register
and nine calls into compiler-builtins' `__adddf3`, `__muldf3`, `__divdf3`, `__subdf3`,
`__mulsf3` and `__floatundidf`. The identical probe against a copy of the spec with `+sse,+sse2` and
no `rustc-abi` has 92 vector-register instructions and no helper calls, so the grep sees SSE when
it is there:

```text
$ llvm-objdump -d --no-show-raw-insn <elf> | grep -Eci 'xmm|ymm|zmm|%mm[0-7]|%st|fld|fstp|fild'
```

The entry point differs on x86_64, and only there. The kernel enters ring 3 with `rsp` on a page
boundary (`rsp % 16 == 0`), while the x86-64 psABI promises a function `rsp % 16 == 8`, as if a
`call` had just pushed a return address. aarch64 and RISC-V push nothing on a call, so they have no
such offset. `sys/pal/nife/mod.rs` therefore makes `_start` a two-instruction `global_asm!` stub
(`call` the Rust entry, then `ud2`), which makes the psABI's assumption true rather than hoping the
soft-float code never depends on it. Nothing faults on a misaligned stack without SSE, which is
exactly why it would have stayed latent.

The build also passes `-Zbuild-std-features=compiler-builtins-mem` to supply `memcpy`/`memset` for
the bare target.
