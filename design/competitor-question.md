# One decision this roadmap still forces

§14 resolved the verification-endgame fork (verification *is* the goal) and converted the old "POSIX
posture" question into milestone 19's real-workload sub-decision (reach binds now that "real
workloads" is committed). What remains open, and what was decided about it once its trigger fired:

- **When the demonstrator becomes a competitor, if ever.** §14 keeps a general-purpose competitor as
  an explicit *later optionality*, parked until the demonstrator earns it. The trigger to reopen it is
  concrete: a verified core that actually runs a real workload (milestone 19, BUILT), plus a reason the
  world needs another OS that the demonstrator has by then proved. Until both hold, competitor-shaped
  work (broad driver coverage, a full Linux ABI, a package ecosystem) is out of scope, and saying so
  keeps the demonstrator from sliding into a second, unfinished Linux.

  A candidate answer to the second half now exists, and it is the first one ever proposed.
  [DECISIONS §145](decisions/0145-compartmentalization-at-process-cost.md), raised by calef
  2026-09-05: compartmentalization at process cost, which is Qubes' stated mission delivered without
  the hypervisor Qubes needs because Linux processes are not a security boundary. It is `PROPOSED`
  and recommends taking Qubes as a benchmark rather than a product target, so nothing in the
  paragraph below is loosened by it. Worth naming here because a parked question that never
  acquires a candidate has quietly become a no.

  The first half fired 2026-08-26 (the display ladder's rung two, milestone 33, landed), and
  [DECISIONS §131](decisions/0131-hold-at-rung-two.md) is that call, made: hold at rung two, prove
  something useful on text mode first, rather than proceed to rungs three and four (real
  applications, GPU acceleration) on the strength of the technical trigger alone. The second half of
  this question is still open, on purpose, until that proof exists.
