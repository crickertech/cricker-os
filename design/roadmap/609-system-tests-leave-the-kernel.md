---
status: PARTIAL
raised: 2026-09-27
milestone_dependencies: none
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: yes
---
# 609. The system tests leave the kernel crate

Raised 2026-09-27 (UTC), when calef took proposal P2 of pull request #1389 (the packages design).
The kernel should be buildable and releasable on its own. A base-service change should not need a
kernel change. The measurements, the options and the reasoning are in
[notes/system-tests-and-the-kernel-crate.md](../../notes/system-tests-and-the-kernel-crate.md).
Number and slug provisional.

## Index row

The kernel crate was the kernel, the integrator and the whole-system test harness at once, so every
kernel build compiled service and fixture code. The test-only and fixture edges are cut and gated;
where the 20,584 lines of system tests go is proposed and waits on calef.

## Built

- The kernel's test-only and fixture dependencies are cut from its normal graph (2026-09-27).
  `ps`, `pgrep`, `pmap`, `network_time_protocol` and `package_archive` are dev-dependencies;
  `coremark`, `job_mix` and `soak_page` are optional behind the features that use them (and
  dev-dependencies where the suite uses them too). `cargo build -p kernel` compiles eight fewer
  crates on aarch64, riscv64 and x86_64. Every build that uses them checks clean on all three:
  plain, `--tests`, `bench`, `icount`, `job_mix`, `soak_test` and `ipc_stack_depth`.
- `script/lint` check 13 holds it (number provisional): a non-optional kernel dependency named
  only by test files, named by nothing, or a fixture crate fails the lint. Proven by planting `ps`
  back into `[dependencies]`.

## Remaining

- **PROPOSED: where the system tests go.** The recommendation is a system-test image crate
  (provisional name `system_tests`) that links the kernel as a library and takes the 61
  `cfg(test)`-only files in `kernel/src/user/` unchanged. It needs a crate name that sticks. It
  changes what `script/test` runs. So it waits for calef. The note has the four options and why each
  lost or won.
- The five service crates the kernel still links for real (`calendar`, `jh7110_entropy`,
  `video_terminal`, `block_roster`, `non_volatile_memory_express`) wait on #1389's division of them.

## Follow-on

- **Outstanding.** The trust root leaves the kernel binary (calef, 2026-09-27): a separate milestone,
  not built here, for the maintainer to mint. It is what still makes a kernel build pack the archive
  first.
- **Outstanding.** The five service crates the kernel links for real wait on pull request #1389's
  division of them; each stays, moves, or has its shared part split into a contract.
