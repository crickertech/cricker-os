# Can GLM-5.3 Flash do lane work?

*Name: provisional (2026-10-10 UTC). Asked by calef on 2026-10-10 (UTC): "Can you launch a lane to
assess GLM-5.3 Flash for lane work?" The method below was committed before the first trial run;
anything that changed after it is recorded as a deviation.*

## Method

Two halves. A retrospective over the window when lanes ran on GLM under opencode, which costs
nothing, and a controlled trial of three tasks, each run once on `glm-5.3-flash` and once on
`glm-5.3`, six runs in all.

### The trial

Each task replays lane work that has already merged, from its base commit, so the merged commit is
the known-good answer and the Claude baseline. Claude is not rerun.

| task | kind | base | merged answer |
|---|---|---|---|
| (a) | debt paydown, comments only: stale `#[path]` claims | `ea7a58d60` | #1863 (`ce3ef6c8b`) |
| (b) | host crates: tests that kill nine census survivor keys (milestone 637 (triage the crates the 2026-09-21 mutation census measured for the first time), batch 7) | `ad4ca79eb` | #1838's test commit (`1f37b3cb4`) |
| (c) | kernel: milestone 95 (an unmap primitive, and the mappings init never lets go): `AddressSpace::UNMAP` on three ISAs, system tests, aarch64 QEMU | `65ee81831` | #1678 (`c9afcb837`, `4a327062c`) |

How each run is made:

- Harness: opencode 1.18.35, headless: `opencode run --auto -m <model> --dir <clone> --format json "<brief>"`.
  The JSON event stream records every tool call, so rule-following is graded from what was run.
- Provider: OpenRouter, through a LiteLLM proxy the maintainers run, on routes pinned to
  `z-ai/glm-5.3-flash` and `z-ai/glm-5.3`. Cost per run is the proxy's own per-request cost log,
  summed over the run's model group; opencode reports no cost for a custom provider.
- Isolation: Each run gets its own clone made by `git init` and a fetch of the base commit alone,
  so the answer is absent from its history. No remote, `GIT_SSH_COMMAND=false`, an unauthenticated
  `gh`, and a throwaway `HOME` so opencode loads no user-level configuration or instructions. The
  only instruction file it loads is the clone's `AGENTS.md`. Web fetch is denied.
- Brief: A common preamble (no remote, skip the claim, read `AGENTS.md` and the skills it
  names, run `script/lint` and the tests, report at the end), then the task text a lane brief would
  carry. Tasks (a) and (b) are host-only. Task (c) may boot aarch64 only.
- Bounds: 60 minutes of wall time per run, enforced by the runner. The two runs of a task go at
  once, so both see the same machine load.
- Grading: Blind first: the run's diff against the merged answer before its narration is read.
  Then `script/lint` (Homebrew Python first on `PATH`) and the task's tests, pass or fail per gate,
  rerun by the grader. Then correctness against the merged answer, rule-following (`AGENTS.md`, the
  skills, no em-dashes, cited sections glossed, names marked provisional, no invented decisions),
  and the number of corrections a reviewer would have had to make.
