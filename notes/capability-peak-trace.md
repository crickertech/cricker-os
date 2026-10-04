# The progenitor's capability-table peak, slot by slot

*Name provisional. Measured 2026-10-04 (UTC) on aarch64 under QEMU by the lane
`lane/capability-peak-trace`, for milestone 753 (provisional), which was the proposal
`trace-the-progenitors-login-block-peak`.*

The progenitor's table has 32 slots (`kernel::cap::CAPABILITY_TABLE_SLOTS`). A gpu-and-keyboard
boot reaches 31, a gpu boot with no keyboard 28, and a boot with no gpu 24. This note says which
capabilities make up those numbers, how long each is held, and what each option for lowering the
peak would actually save.

## How it was measured

There was no trace path in the tree, so the instrument is a patch kept beside this note rather than
a feature in the build: [instrument.patch](capability-peak-trace/instrument.patch). It hooks the
capability table's two mutation doors (`fill` and `empty` in `crates/capability`). It records every
change to the progenitor's table into a kernel ring, with the syscall's user PC and a frame-pointer
backtrace. The idle loop drains the ring through semihosting to `target/cap-trace/boot-<time>.txt`,
one file per boot, so the console transcript `script/swish-check` reads is untouched. It also
records any other table that reaches 25. None did on any boot, so the peak `report_peak` prints is
the progenitor's.

To reproduce:

```sh
git apply notes/capability-peak-trace/instrument.patch
CARGO_TARGET_AARCH64_UNKNOWN_NONE_SOFTFLOAT_RUSTFLAGS="-C force-frame-pointers=yes" \
  script/swish-check --arch aarch64
LLVM_SYMBOLIZER=/opt/homebrew/opt/llvm/bin/llvm-symbolizer \
  python3 notes/capability-peak-trace/analyze.py target/cap-trace/boot-<time>.txt \
  target/aarch64-unknown-none-softfloat/debug/progenitor
git apply -R notes/capability-peak-trace/instrument.patch
```

[analyze.py](capability-peak-trace/analyze.py) replays a trace and prints the occupancy at the
peak with the `crates/system_initializer/src/lib.rs` line that minted and released each slot.
[counterfactual.py](capability-peak-trace/counterfactual.py) replays the same trace with some
holdings removed and reports the peak that would result. Frame pointers matter: without them the
backtrace has to be guessed from stack contents, and the first attempt at that named stale frames.
With them the gauge read the same 24, 28 and 31, so the flag does not move what is measured.

The three boots are `swish-check`'s own: no gpu (with the NIC), gpu and no keyboard (with the NIC),
and gpu with keyboard. The last one also has a NIC, although `swish_check_boot`'s comment says
it is left off. `xtask/src/host.rs` exports `NIFE_NET=1` into xtask's own environment on every
`cargo` call, and the keyboard boot inherits it. The net stack's endpoint is therefore one of the 31.
Without a NIC the keyboard boot would read 30.

riscv64 was not traced. Its runner has no `-semihosting`, so the drain has nowhere to write, and
adding one was not cheap. `kernel/src/cap.rs` records riscv64 reading the same peaks as aarch64 on
the same code.

## The peak is two plateaus, not one

On the keyboard boot the table sits at 30 or 31 through six phases. The login block is only one of
them.

| Phase | Highest reached | Where |
|---|---|---|
| building `log` | 30 | `boot`, line 1628 |
| building the identity provisioner | 30 | line 2223 |
| `build_child(login)` | 30 | line 2347 |
| placing the durable window in `login` | 31 | lines 2416 to 2423, one event |
| building the session's terminal, at the `graphical_terminal` launch | 31 | line 4369 |
| building the session's supervisor and the keyboard driver | 31 | lines 5854, 4452 |

The serial boot has the same shape at 28: the login block and the launch both reach it. The boot
with no gpu has no launch and reaches 24 once, at the durable-window placement.

Any change that only touches the login block therefore leaves a gpu boot's peak where it is,
because the launch reaches the same number.

## Who holds the 31, at the login block

Taken at the event that first reaches 31 (trace event 1588 of 2,246). "Launch" says whether the
same capability is also held at the launch plateau. Event numbers place each mint and release in
the boot: kernel grants are 0 to 25, the boot servers 25 to 890, the login block 890 to 1,600, and
the launch 1,636 to the end.

| Slot | Capability | Minted (line, event) | Released (line, event) | Launch | Why it is held |
|---|---|---|---|---|---|
| 0 | root untyped | kernel grant, 0 | 2480, 1601 | no | every retype in the block comes from it |
| 1 | `spawn_ep` | 1810, 600 | never | yes | the spawn service's request endpoint |
| 2 | `result_ep` | 1811, 601 | never | yes | the spawn service's answers |
| 3 | clock page | kernel grant, 3 | never | yes | endowed `READ` to children |
| 4 | configuration page | kernel grant, 4 | never | yes | endowed to children declaring it |
| 5 | file service endpoint | kernel grant, 5 | never | yes | §219 (how the shell names an installed program to the spawner) |
| 6 | file window pool | kernel grant, 6 | never | yes | sliced into a window per job |
| 7 | `net_stack` client endpoint | 4035, 70 | never | yes | milestone 590 (the booted system starts its network stack): endowed to `Manifest::network` |
| 8 | `term_out` | 1494, 278 | 2626, 1602 | no | mapped at `INIT_OUT_VA` after the block, then dropped |
| 9 | `term_sink` | 2066, 892 | never | yes | the sink adapter's endpoint |
| 10 | entropy request | 1380, 24 | never | yes | milestone 111 (a shell that can endow a child with entropy) |
| 11 | `term_ep` | 1497, 280 | never | yes | the boot discipline, kept for the next session |
| 12 | gpu surface run | kernel grant, 14 | never | yes | milestone 715 (the spawn service holds the display grants, and the shell holds none): lent to each session |
| 13 | `deaths` | 1816, 602 | never | yes | job supervision |
| 14 | `audit` | 2324, 1053 | 2440, 1597 | no | login input |
| 15 | `verify` | 2138, 928 | 2437, 1594 | no | login input |
| 16 | `login_request` | 2338, 1084 | 2435, 1592 | no | login input |
| 17 | gpu transport | kernel grant, 15 | never | yes | milestone 715 |
| 18 | gpu interrupt | kernel grant, 16 | never | yes | milestone 715 |
| 19 | gpu DMA run | kernel grant, 17 | never | yes | milestone 715 |
| 20 | keyboard transport | kernel grant, 18 | never | yes | milestone 715 |
| 21 | keyboard interrupt | kernel grant, 19 | never | yes | milestone 715 |
| 22 | keyboard DMA page | kernel grant, 20 | never | yes | milestone 715 |
| 23 | `login_result` | 2339, 1085 | 2436, 1593 | no | login input |
| 24 | the shell's TCB | 1965, 886 | 2651, 1635 | no | built before the block, started after it |
| 25 | `verify_page` | 2141, 931 | 2438, 1595 | no | login input |
| 26 | `login_ut` | 5789 via 2340, 1086 | 2439, 1596 | no | login input |
| 27 | window 0 slice | 4841 via 2346, 1087 | 2434, 1591 | no | login input |
| 28 | run-unvouched endpoint (D2) | 2401, 1587 | never | yes | §219 gate D2, placed in `login` and the shell |
| 29 | `login`'s TCB | 2347, 1585 | 2424, 1590 | no | consumed by `start_child` |
| 30 | durable window slice | 4841 via 2416, 1588 | 2423, 1589 | no | placed at `DURABLE_WINDOW_SLOT`, then deleted |

Nineteen are held for the life of the boot, and the seven device grants are seven of them. Twelve
are the login block's transients and the three things that outlive it briefly (the root untyped,
`term_out`, the shell's TCB).

At the launch plateau the same nineteen are held. The other twelve are different: the three
budgets carved after the block (`own_ut`, `jobs_ut`, `images_ut`), the session's region, and eight
session objects. At the keyboard driver's build those are `session_term`, `session_out`,
`session_in`, `term`, `out`, `report`, and the address space and page the loader is laying down.

The serial boot's 28 is the same table without slots 20 to 22. The no-gpu boot's 24 is the same
table without slots 12 and 17 to 22. The device grants account for the whole difference between
boot shapes.

## What each option saves, measured by replay

Each row replays the three traces with the option's holdings removed (`counterfactual.py`). A row
marked derived is arithmetic on the table above, because the option changes the order of events and
a replay cannot model that.

| Option | Keyboard | Serial | No gpu | Source |
|---|---|---|---|---|
| Today | 31 | 28 | 24 | measured |
| A. Raise the table to 64 slots | 31 (of 64) | 28 | 24 | no change to the peak |
| B. A separate process holds the device grants from boot and builds the graphical session | 24 | 24 | 24 | replay |
| C. Release the login inputs when `build_child(login)` returns | 31 | 28 | 23 | replay |
| D. Lend the device grants to a holder; fetch each back for the build that needs it | 28 | 26 | 24 | derived |
| B and C together | 23 | 23 | 23 | replay of C on the no-gpu boot, which is B's shape |

C also moves the shell's build after the block and drops `term_out` earlier, in the fuller form.
Neither changes a gpu boot's peak either, for the plateau reason above. The block that owns the
decision is [milestone 753](../design/roadmap/753-trace-the-progenitors-login-block-peak.md), and
the costs and the seven questions for each option are there.

## What the raise would cost, measured

The free-slot word is a `u32`, so 32 is the ceiling the type allows (`capability::MAX_SLOTS`).
Anything above 32 widens it to a `u64`. Each slot is 32 bytes, so 64 slots add 1,024 bytes to every
thread's table. The table lives inside the thread's TCB page with the FP register file. Measured
2026-10-04 by a compile-time probe of `FP_STATE_OFFSET + size_of::<FpState>()`, that page uses
1,984 of 4,096 bytes on aarch64, 1,968 on x86_64 and 1,712 on riscv64. Sixty-four slots fit on all
three, and cost no new memory, because the page is already allocated whole.

`abi::fault::FAULT_EP_SLOT` is `CAPABILITY_TABLE_SLOTS - 1`, so it moves from 31 to 63. Every
supervisor and every child agrees on it, which makes it an ABI fact. Nothing outside this tree is
built against it today. The IPC fastpath footprint and kernel stack temporaries at 64 were not
measured.

## BUGS

- The instrument is a patch, not a feature. It will stop applying when the files it touches move,
  and nothing checks that. Its doc is this note.
- riscv64 and x86_64 were not traced.
- `swish_check_boot`'s comment says the keyboard boot attaches no NIC, and it does. Recorded here and
  at that comment.
