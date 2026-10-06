---
status: PROPOSED
raised: 2026-10-06
milestone_dependencies: 501
decision_dependencies: unwritten
machine_requirements: none
specific_machine: none
needs_person: no
---
# The TLS graph enters the gated build

Raised 2026-10-06 (UTC) by the lane for milestone 501 (a TLS client that speaks to one pinned
peer). It built the client and found that nothing checks it keeps working.

## The gap

`cryptography_provider`, `pinned_tls_client` and the two programs over them are each their own
workspace. Their crates are fetched only when somebody runs `helpers/build-cryptography-exerciser.sh`
or `helpers/build-pinned-tls-exerciser.sh`. Their kernel tests skip in CI. Their host tests run in no
gate at all.

That was milestone 442 (a crypto provider `rustls` can use on all three bare-metal targets)'s
posture, chosen while the primitives were still calef's to rule on. They are ruled now. §196 (nife
carries TLS) took `rustls`, and §198 (the glue is ours, the primitives are not) took the primitives,
`rsa` included. Milestone 801 (packages over the internet) cannot ship a client no gate builds. A
toolchain bump or a `std` overlay change can break this graph today, and no check would notice.

## What it would cost, measured 2026-10-06 on patagonia

- Crates: 70 in `pinned_tls_client`'s normal graph, 73 in `pinned_tls_exerciser`'s
  (`cargo tree -e normal`). `deny.toml` and `script/supply-chain` already scan the provider's.
- Build time: about 28 s per architecture for `pinned_tls_exerciser` from clean. The `std` farm
  was already built. Under 90 s for all three.
- Run time: The kernel test is under 5 s per architecture. The host tests are under 2 s.
- Image size: 1.27 MB (x86_64) to 1.89 MB (riscv64) per program before the archive strips it.

**Reuse:** the two build helpers as they are and CI's existing crates.io fetch; nothing here is
written, and the crates are the ones §196 and §198 already took.

## The options

1. Build both programs in CI's kernel legs and the host tests in `script/test`'s host phase,
   fetching from crates.io as the rest of the workspace does.
2. The same, vendored: §46 (thin primitives or whole subsystems) says to vendor only what needs a
   patch, and nothing here does.
3. A scheduled, non-gating job: the graph is built nightly and a break is a notice rather than a
   red pull request.
4. Leave it out until milestone 801 needs it.

The lane recommends the first. The second buys nothing §46 asks for. The third finds a break
days late, on nobody's pull request. The fourth leaves the client unchecked through the toolchain
bumps most likely to break it.
