# 1. Only software written for nife runs on nife

*Risk 1 of [the nine](README.md). The status vocabulary, the rule an entry meets and the running order are there.*

The claim, stated so it can fail: the platform runs hand-written Rust and nothing else, so every
piece of software anyone wants has to be rewritten. It is the most dangerous entry because it is
structural. Optimization cannot fix "nothing runs here". A system in this state is a research
demonstrator forever, which is not what DECISIONS §14 (a verified-Rust capability microkernel that
runs real workloads) claims.

**The experiment:** milestone 121 (`ripgrep`: enumeration as a capability), for its real dependency
tree, its filesystem walk and its threads.

**Experiment status: RUN, 2026-08-31.** GREEN on all three architectures since 2026-09-16, and the
blocker is not what anyone predicted. Unmodified `ripgrep` 14.1.1, forty transitive crates, zero
patches, and three byte-identical transcripts from three separately built binaries
([`notes/ripgrep-on-nife.md`](../../notes/ripgrep-on-nife.md)). What stopped it was the missing
argument vector, milestone 205 (how a foreign program is told what to do). Correction, 2026-09-27:
205 is BUILT, and what keeps `ripgrep` from the prompt now is the 256 KiB image ceiling (#1399).

DECISIONS §105 (`std::thread::spawn` stays declined, until a customer needs it) was never reached,
and that reverses the premise. `ripgrep` asks `available_parallelism()` rather than assuming it, and
nife answers `Ok(1)` honestly. A platform answering `Unsupported` there would have failed this
program.

The caveat. The structural fear is retired. The one published argument that speaks to this says it
goes badly: clean-slate kernels have *"significantly fewer features than Linux ... impeding
adoption"*, risk 8's paper ([`notes/incremental-path.md`](../../notes/incremental-path.md)).
[Appendix](somebody-elses-software.md).
