---
status: DECIDED
raised: 2026-08-04
decided: 2026-08-25
ratified_by: calef
---

# 78. Signed commits: worth doing, and not as a side effect

calef, 2026-08-25, in conversation: *"Let's decide not to do signing with a note
on those conditions that would cause us to reconsider."* (raised 2026-08-04 when §73 closed, so the
deferral does not die with the section that carried it.)

What. Require signed commits on `main`. `notes/repo-hardening.md:82` files this under **Do NOT
enable** with a reason that is about sequencing rather than merit: "Nothing here is signed today;
turning this on would block every merge until signing is set up. Worth doing eventually, as its own
decision, not as a side effect of this one."

That reasoning is right and it left the eventual decision homeless. Milestone 44 is BUILT without it,
so this is the one piece of that milestone deliberately carried forward rather than dropped.

Three things it needs, and the second has the real cost.

- A key and a method. SSH signing is the cheap path: git supports it, GitHub verifies it, and the
  key already exists on the machine that pushes. GPG is the older path with more tooling around it.
- Every automated committer signs too, or the rule blocks them. This tree merges lane work
  constantly and takes Dependabot pull requests, and a required-signature rule applies to both. This
  is the part to check before turning it on, because the failure mode is the one this repository
  already lived through on 2026-08-04: a requirement nothing can satisfy blocks every merge. Measured
  today, `git log --format=%G?` over recent `main` returns a mix of `E` and `N`, so nothing is
  uniformly signed and Dependabot's commits are GitHub's to sign, not ours.
- A statement of what it buys here. For a public repository with a security thesis, signatures say
  the commits are from who they claim to be, which is a supply-chain property adjacent to milestone
  42's `cargo-audit`/`cargo-deny` work rather than a code-quality one. Say that plainly, or it reads
  as ceremony.

The recommendation. Not yet, and for a reason that got sharper today: the repository just adopted
"require branches to be up to date before merging", which already serialized a ten-pull-request
backlog. Adding a second requirement that can block every merge, while the first one's cost is still
being measured, stacks two novel failure modes. Do it when the merge pipeline is quiet, verify
Dependabot's commits are accepted **before** making the rule blocking, and start with the rule in a
non-enforcing state if the ruleset supports it.

Re-measured 2026-08-25, three weeks after the original data. `git log --pretty='format:%G?' -100`
on current `main` still returns a mix, 32 `E` / 68 `N`, nothing has changed. The `E` commits are
almost entirely GitHub's own merge-queue merge commits, signed on GitHub's own infrastructure rather
than by anything this repository does; the `N` commits are the real work, every squashed lane commit
and direct push, still entirely unsigned. Dependabot sampled separately: 7 `E` / 2 `N`, the same shape
the original measurement found. The operational preconditions above are all still unmet: the pipeline
is not quiet (seven pull requests were in flight through the merge queue at the moment of this
re-measurement), and Dependabot's acceptance under a blocking rule is still untested.

## What would change this

The three items above (quiet pipeline, Dependabot verified, non-enforcing rollout) are about *when*
it would be safe to turn this on, not *why* it would be worth turning on. Absent one of the four
conditions below, signing today would buy little beyond the ceremony this file's own text already
warns against: a real cost (every merge blocked until every committer can satisfy the rule), for a
property nothing here currently needs.

1. Milestone 128 lands (real per-agent identity). Every commit on this repository pushes under
   calef's account through the shared `gh` token today (`AGENTS.md` says this plainly: "every pull
   request here is authored under calef's account by the `gh` token"). A signature right now would
   only re-assert "the token holder," the same fact push access already establishes. Once agents and
   sessions have distinct identities, a signature starts meaning the thing a signature is supposed to
   mean.
2. The trust boundary widens past one operator. This project is architect-plus-lanes today,
   functionally one actor holding one credential. Real outside contributions -- milestone 117's
   stranger-test thesis extended from "can a stranger read this" to "can a stranger commit to this"
   -- would make "is this genuinely from a known, trusted party" a live question that only a
   signature answers.
3. A customer requires it. `AGENTS.md`'s own ranking-function principle: "no customer runs a
   backup server they do not trust with the only copy." A real customer evaluating whether to trust
   this kernel with real data is exactly the party who might reasonably want supply-chain provenance
   on what is running. No such customer exists yet.
4. An actual incident or credential concern. Evidence that the shared push credential was
   compromised, leaked, or reused somewhere it should not have been would make this urgent rather
   than optional, regardless of the operational cost.

The point of naming these is to keep this decision from being revisited on a calendar or out of
habit: it stays `DECIDED` until one of the four actually happens, not until enough time has passed
that it feels overdue.

**Not blocked.** Nothing waits on this. It is recorded so that a deliberate deferral stays a decision
rather than becoming an omission.
