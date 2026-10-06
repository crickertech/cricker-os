# Kani proof failures in CI, 2026-10-06 (provisional name)

Measured 2026-10-06 (UTC), read-only, for fatal risk 2 (the proofs prove trivia).

Result. Across 9,794 `verify.yml` runs (2026-08-03 to 2026-10-06, all events), plus `ci.yml`
runs from 2026-07-29 (Kani lived there from 2026-07-30 until the split), no standing Kani harness
failed verification (`VERIFICATION:- FAILED`) on a pull request or merge group. No harness was
found weakened. Real catches 0, planted 0, harness-weakened 0, harness-bug 0.

Method. Failed runs (64), cancelled runs (1,139, for a proof job that finished red before the
cancel), earlier attempts of re-run runs (18) and `ci.yml` failed or cancelled runs (290) were
checked. Logs were grepped for the failure line. Every red proof job was infrastructure or configuration: runner
shutdowns, a rename that broke the shard list, a model-mismatch check, and runs with zero jobs.

re-falsify jobs. 27 failed: 21 from #1532 dropping timetable's `mod proofs;`, 3 stale patches
where the covered code moved, 2 Kani bundle download failures, 1 runner shutdown. Survivors: 0.

Adjacent, not a standing-harness catch. Milestone 319 (The crate that parses firmware had no proofs) on 2026-09-17 found two
`machine_discovery` harnesses false when written (a `u8` overflow and a size underflow) and fixed
the code. These are catches at authoring time.

Limits. GitHub only: a lane that ran `script/verify` locally and fixed before pushing leaves
no trace. Before 2026-07-30 Kani was not in CI. Re-falsify catches a weakening only when the
harness has a replayable record and the mutation lies inside the excluded region.

Zero is the measured count of catches, not evidence the harnesses are strong. See
[the falsification note](falsification.md) and risk 2.
