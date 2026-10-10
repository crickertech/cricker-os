# Which pull requests pay down debt

The `debt-paydown` label marks a pull request whose purpose is to repay something the tree already
owes, rather than to give it something new. It exists so the share of work spent repaying can be
counted (calef, 2026-10-10 UTC: "label every lane that's debt pay down, technical or otherwise. I
include all work to make the tree comply with our standards").

Name: provisional, minted 2026-10-10 (UTC) by lane `maintainer/debt-paydown-label` (#1894). A label
name is a name, so calef ratifies it ([naming authority](skills/naming-authority/SKILL.md)). The
`script/claim` flag below carries the same name and is provisional with it.

## The definition

Label a pull request when its title and opening paragraph give one of these as its main purpose.

1. Repair. Something already merged does not behave as its record says, and this makes it: a bug
   or security fix in existing code, a flaky, racy or load-sensitive test, a leak, a gate that
   fires wrongly or never fires, a regression back inside its band, lost work restored.
2. Cleanup with no new capability. A refactor, removing dead code, unused dependencies or a
   workaround, moving code to where a codebase rule says it lives (`arch/`, a crate, out of the
   kernel crate), lock contention or false sharing removed, divergence from a vendored upstream
   reduced.
3. Record repair. A record that is wrong or missing what it owes: a correction of error (a COE), a
   fact correction, a stale status, date, path or claim brought current, an owed record or
   falsification backfilled, a citation or provenance added where it was missing, knowledge landed
   from a branch that was holding it.
4. Compliance. Bringing the tree up to a standard it has already written down: prose budget (§212),
   writing standards (§213), bold, heading, spelling and comment-block sweeps, a ratified rename
   applied, survivor triage, coverage and lint ratchets, an audit and its findings closed, an
   architectural parity gap closed (rule 5), and a gate whose purpose is enforcing a rule decided
   before the pull request that adds it.

Do not label:

- A new capability, a milestone's first build, a new driver, program or protocol.
- A proposal, a provisional milestone block, or a decision recorded in the normal course. The pull
  request that does the work gets the label; the one that plans it does not.
- Benchmarks, measurements and experiments, unless they measure a named defect so it can be fixed
  (a flake, a regression).
- Routine upkeep on a schedule: the weekly metrics, the nightly toolchain bump, Dependabot.
- A new rule, or a gate added in the same pull request as the rule it enforces.

When a pull request mixes the two, judge by what its title leads with. A fix found while building a
feature, and landed inside the feature's pull request, is not labeled; the same fix in its own pull
request is.

## Calls that set precedent

From the first pass, 2026-10-10 (UTC), so the next labeler can match them:

| pull request | call | why |
|---|---|---|
| #1630, #1686 | labeled | lock contention and false sharing in existing paths: cleanup |
| #1534 | labeled | an existing claim proved on the two ISAs that lacked it: parity |
| #1409 | labeled | an unmeasured stack risk given headroom and a gate; repair of a recorded risk |
| #1447, #1486 | labeled | measurements, but of a named flake and a named drift |
| #1868 | labeled | ratchet for §266, which #1867 decided first |
| #1735, #1791 | not labeled | each records a new rule and ratchets it in the same pull request |
| #1589 | not labeled | a new check for a rule not written before it, and it found nothing |
| #1444, #1458 | not labeled | existing CI checks copied into pre-push: throughput, not compliance |
| #1299, #1473 | not labeled | hardware bring-up and a performance gain, not a repair of a record |
| #1744 | not labeled | a new protocol that closes a confinement claim; new capability first |
| #1309 and other rulings | not labeled | recorded promptly; #1411 and #1324 say "owed" and are |

## Applying it

A lane that knows at the claim says so: `script/claim <branch> --debt-paydown` opens the draft with
the label on. Otherwise, whoever briefs or reviews adds it:

    gh pr edit <N> --add-label debt-paydown

Counting it:

    gh pr list --state merged --label debt-paydown --search "merged:>=2026-09-26" --limit 1000

## The first pass

On 2026-10-10 (UTC), 188 of the 560 pull requests merged from 2026-09-26 were labeled, and none of
the 4 open. The 30-day window held 1,058, more than one careful pass could read, so the window was
cut to 14 days and anything earlier is unlabeled, not judged.

## BUGS

- The flag needs whoever runs `script/claim` to know the work is debt, which is memory. The pull
  request that added this note (#1894) proposes stronger rungs.
- Judged from titles and opening paragraphs. A pull request whose title undersells its purpose is
  missed.
- Adding any label to a pull request, open or merged, runs `architect-hold.yml` once, since it
  listens for `labeled`. The run is one API call; the first pass cost 188 of them.
