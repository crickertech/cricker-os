# Pipes BUGS history

*An appendix to [`notes/pipes.md`](../pipes.md), which is the page to read. This file holds every
BUGS entry as it stood on 2026-09-25 (UTC), with the history of each; the main page holds the
current state. It was moved here verbatim from the main page on 2026-09-25 (UTC), under [§212 (a
prose budget)](../../design/decisions/0212-a-prose-budget-for-every-document.md). The directory
`notes/pipes/` and this file's stem are provisional names, minted that day by the lane that split
the file. Naming is calef's.*

The records this file cites by number:

- §106 (take the `terminal_sink_caretaker` narrowing)
- milestone 151 (notification objects)
- milestone 51 (wall-clock time)
- milestone 31 (a capability shell)
- §55 (the file behind a `>` is the shell itself)
- §67 (a program's second stream is a declaration)


## BUGS, named where the reader meets them

- The terminal's sink adapter has a second consumer now: an unredirected tail stage's primary output
  (DECISIONS §106, 2026-08-22). This entry used to ask the question and leave it open. The decision
  answers it: the shell hands the terminal's sink over only when the line named neither `>` nor `|`
  for that stage, decided from the plan before anything spawns. So a stage never loses the ability
  to be redirected, it only loses the shell as a reader when nothing asked for one. See
  notes/documentation.md's `BUGS` for the worked case (`doc gate.txt`) and the cost that trade
  carries (the caretaker-hop display race, tracked at milestone 151).
- `OPERATION_PRINT` carries eight bytes, so a sixteen-byte sink message is two calls to the terminal.
  That is the terminal contract's request shape rather than a choice (see notes/sink-protocol.md),
  and it doubles the round trips on a path that is a person reading text.
- `script/swish-check` is not in `script/test` or in CI. It is the only gate on the real progenitor
  (both of them) and nothing runs it automatically, which is a weaker version of the gap it closed.
  It has now caught two boots that printed nothing, which is two more than any automatic gate did.
  Wiring it into the CI test job is a one-line change and is deliberately still not taken here.
- `fixtures/src/file_sink.rs` and `fixtures/src/file_source.rs` are no longer on the shell's path.
  They are still the right shape for an adapter whose client is not the shell, and `sink_tests`
  still proves them against a real image, but nothing at the prompt builds one.
  (`components/src/terminal_sink_caretaker.rs` is that shape with a client the prompt does build,
  which is the closest this has come to being used.) `file_source` also still opens the one name in
  `byte_sink_protocol::fixture` and cannot be told another; the shell would have had to hand it a
  name the way `fs_file_caretaker` is handed one, and it turned out not to need to.
- The interactive prompt holds the image root, unnarrowed. A `fs_subtree_caretaker` between it
  and the FS server would cost one process and would make the prompt's own authority as legible as
  the authority it hands out. It is the machine's own shell, so this is a defensible default rather
  than an oversight, but it is a default and not a decision anybody made on the record.
- `rm` is still not reachable from the prompt, and neither is a per-file capability. The shell holds
  a directory, so the refusal is no longer "you hold no such capability"; what is missing is the
  caretaker the progenitor would have to build per invocation, and `spawn` says so rather than
  spawning `rm` with nothing. The progenitor deletes its copy of the FS endpoint after building the
  shell, so that is the line that changes first. The same gap is why `FileSpec::Required` has no
  consumer: `wc gate.txt` grants a *stream* of one file (the shell opens it), which is narrower than
  the per-file capability `fs_file_caretaker` serves and is not the same claim. See
  notes/grant-expression.md.
- Slot 1 is the clock, the input source, or the `--mem` untyped, whichever applies. Three things in
  one ordered position now (milestone 51's wiring added the clock, which the progenitor endows from
  the program's manifest rather than from the request). It is unambiguous only because no manifest
  declares two of them, and `grant_plan` is where that stops being true. A program endowed a budget
  *and* an input, or a clock *and* an input, needs a numbered slot convention rather than an ordered
  one.
- And slot 0 is the output except behind a directory grant (milestone 31 phase 3, 2026-08-17), where
  the caretaker's narrowed endpoint takes it and the output moves to slot 1. That is not a second
  convention invented at the spawn service: it is the contract `components/src/rm.rs` documents and
  the kernel's `start_granted_dir` already wired, so one program means one thing in a guest test and
  at the real prompt. It is still an *exception* to an ordered convention, which is one more reason
  the numbered one above is owed. `grant_plan::PROG_COUNT`'s manifests are what keep it safe today:
  the one program with `DirSpec::Required` declares no input, no clock and no budget.
- A pipeline is full lockstep. There is no buffer: every sixteen bytes is a rendezvous. Unix's 64 KB
  pipe buffer lets a producer run ahead and this does not. *(This entry used to end "and nothing
  here has been benchmarked against a Unix pipeline", which stopped being true on 2026-08-03 and was
  left standing for a day. It has been: [the buffering section](buffering.md) has the numbers, and
  the finding is that the sixteen-byte message rather than the lockstep is what costs.)*
- A line whose bytes all come from this shell needs a stage that reads to the end, unless its tail
  is screen-narrowed (DECISIONS §106 updated this, 2026-08-22). One wait point per process, no
  select, and [the wait-point section](one-wait-point.md) has the whole of why; that has not
  changed. What changed is that a filter which renders as it reads is no longer refused when it runs
  on its own. Its output goes to `terminal_sink_caretaker` rather than back to this shell, so there
  is no longer a second reader for the shell to wait behind. `wc` remains the only stream-absorbing
  barrier in this tree, and a filter piped into one (`doc page.md | wc`) still runs the same way it
  always did; what is new is that the filter no longer needs one to run unredirected. A filter that
  is also redirected (`doc page.md > out.txt`) is still refused: `>` still comes back through this
  shell (DECISIONS §55), so that shape still needs a barrier or nothing to wait on.
- `doc` is `writes_while_reading`'s first declarer, reachable from the prompt (DECISIONS §106).
  This entry used to record the declaration as ahead of its first user, provable only by host tests;
  `doc <page>` at a real prompt is that proof now. See notes/documentation.md's `BUGS`.
- The refusal is named after a program and is a fact about the line, and it still fires for the
  redirected shape. `doc page.md > out.txt` prints `doc: writes while it reads...`, which reads as
  a complaint about `doc`. Nothing is wrong with `doc`, and the same program renders straight to the
  screen one operator earlier. The name is there because it is the stage a person would put the
  `| wc` after; the wording gap is the same shape as `<<`'s below.
- `2>` works only on a program that declares a second stream, and one does (`date`). That is
  DECISIONS §67's cost, stated where a person meets it: `wc gate.txt 2> err.txt` is refused, and the
  fix is `wc` declaring a second output rather than anything about the operator.
- Under a `2>`, a declaring program must say everything before it produces anything. The shell
  drains the diagnostic stream to end-of-stream *before* the output, because there is no
  receive-on-a-set and any other order deadlocks a pipeline ([the `2>`
  section](second-stream.md#the-one-rule-a-2-costs-and-why-only-a-2-costs-it) has the chain). At the
  default destination the shell is not in the path at all and the rule does not apply, so the same
  program is correct without the operator and would deadlock with it. That asymmetry is real and is
  the price of the shell backing files itself (DECISIONS §55). `date` is unaffected: it closes its
  second stream before its first, always.
- A `2>` on a builtin is refused as "declares no second output". True (a builtin has no manifest
  and no second stream) and a slightly odd sentence about `ls`, which is not a program at all. The
  wording gap is the same shape as `<<`'s below.
- The shell's diagnostic endpoint is never destroyed, so a writer parked on it after the shell
  stopped reading would stay parked rather than getting `Gone`. It cannot happen today (the shell
  always drains to end-of-stream before it reads anything else) and it is a real asymmetry with the
  pipeline region, which is split and destroyed per line precisely to produce that `Gone`.
- No here-document, and `<<` says the wrong thing about why. It is refused as "a redirection
  needs a name", because the second `<` is read as the operator it is. The refusal is right and the
  sentence is about the wrong thing.
- ~~No quoting anywhere in this shell, so a file whose name contains `>` cannot be named.~~
  Closed by milestone 67 (notes/swish-language.md). `date > "my out.txt"` writes to a name with a
  space in it and `echo "a > b"` has no redirection on it, because a quoted operator is an ordinary
  byte. The residual is that quoting delimits and never rewrites, so there is no backslash escape
  and `a"b"` is refused rather than joined; a shell whose tokens are slices of the line has nothing
  to join pieces into.
- `wc` has no `-l`, `-w` or `-c`. It prints all three, because selecting among them is
  formatting and formatting belongs downstream.
- A `date` whose reader stopped early stays parked. `date`'s end-of-stream message is a
  rendezvous send like any other, so a reader that took its line and stopped leaves that process
  blocked until its region is reclaimed. Inside a pipeline the region is destroyed and it ends; in
  `kernel::user::date_tests`, which read a line and stop, it does not. Blocked, not spinning, and
  the suite's leaked-thread gate is about runnable threads.
