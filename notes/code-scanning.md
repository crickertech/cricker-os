# Code scanning: what CodeQL looks at, and what became of its alerts

GitHub's CodeQL runs on every push and pull request through default setup, with no workflow file
in the tree; DECISIONS §36 (the repository is part of the TCB) says why. Every alert gets one of
the three dispositions in DECISIONS §35 (what a scanner is for here, and how its findings get
dispositioned): fixed, dismissed with a written reason, or deferred to a milestone. This note
records the dispositions where a reader of the tree meets them, because GitHub's dismissal
comments are invisible from the source and do not survive a change of tool.

## Which languages are scanned

As of 2026-10-07 (UTC), default setup analyzes **actions, c-cpp and python**. Check it with:

```
gh api repos/nifeos/nife/code-scanning/default-setup --jq '{state, languages}'
```

**Rust is not scanned.** calef removed it from default setup on 2026-10-07 because the
`Analyze (rust)` job failed intermittently while the other three passed: seven failed `CodeQL` runs
between 16:46 and 17:05 UTC that day, for example run
[37655236254](https://github.com/nifeos/nife/actions/runs/37655236254) on `main`. calef saw the
SARIF upload fail. The job log shows extraction and query evaluation completing, and the upload
error's exact text was not captured in this record. The last Rust analysis on record is from commit `f8feff7f3`.

Removing a language does not close its alerts. GitHub closes an alert only when a later analysis
of the same language stops reporting it, so the 43 Rust alerts open on 2026-10-07 could never
auto-close. They were each read and dismissed by hand, below.

Even when Rust was scanned, §36's caveat applied: the extractor runs against the host target with
default features, for a kernel that does not build for the host, and reports macro expansion
failures across the tree. A clean Rust result meant less than it looked.

## The 2026-10-07 triage

44 alerts open, all read at the alert's commit (`f8feff7f3` for the 43 Rust alerts, `9c9e16a49`
for the Python one) and at `main`. No flagged line had moved. One was a real defect; the other 43
were dismissed, each with a comment naming the file and line and why.

| Rule | Count | Disposition | Reasoning class |
|---|---|---|---|
| `py/redos` | 1 | Fixed (#26) | Real. See below. |
| `rust/hard-coded-cryptographic-value` | 23 | Used in tests | Fixture salts and the phc-winner-argon2 reference vector inside `#[cfg(test)] mod tests` in `crates/credentialer/src/lib.rs` |
| `rust/hard-coded-cryptographic-value` | 2 | Used in tests | GCM test case 2's all-zero key and IV in `cryptography_exerciser`, a known-answer program |
| `rust/hard-coded-cryptographic-value` | 2 | False positive | `components/src/credentialer.rs:216` and `:286`: zeroed buffers that `fill()` overwrites from the entropy service before use, or the service refuses |
| `rust/hard-coded-cryptographic-value` | 1 | False positive | `vendor/redoxfs/src/key.rs:71`: a zeroed output buffer Argon2 writes into |
| `rust/access-invalid-pointer` | 9 | False positive | `uefi_loader`: UEFI protocol out-pointers dereferenced only after `SUCCESS` and an `is_null()` check. CodeQL tracks the `ptr::null_mut()` initializer and does not see the firmware write. Firmware is in the trusted computing base, and no untrusted input reaches these pointers |
| `rust/cleartext-logging` | 4 | False positive | `vendor/redoxfs/src/bin/` host tools printing a filesystem UUID, sizes and paths, none derived from the password |
| `rust/cleartext-logging` | 1 | False positive | `kernel/src/arch/x86_64/timer.rs:424`: a TSC rate in Hz in a boot assert |
| `rust/cleartext-logging` | 1 | Used in tests | `crates/credentialer/src/lib.rs:809`: a test assert printing a fixture secret |

Totals: 1 fixed, 26 dismissed as used in tests, 17 as false positives. None of the flagged code had
been removed.

**Alert 26 was real.** `helpers/rust_source.py`'s comment-and-literal stripper matched a raw
string's body with `(?:.|\n)*?` under `re.S`, where `.` already matches a newline. Every newline
had two ways to match, so an unterminated `r"` followed by n newlines cost 2^n steps: 24 newlines
took about a second, 27 took seven. The input is any Rust file a branch adds, and `script/lint`
and `script/metrics` both run it, so one such file would have hung the gate rather than failed it.
The fix is `.*?`, the same language in linear time, and `python3 helpers/rust_source.py
--selftest` (run by `script/lint`) fails the old pattern. Python is still scanned, so the alert
closes on the first analysis of `main` after the fix lands.

The verdicts on the pointer alerts are from reading code, not from trying to break it. An attempt to
reach those dereferences with hostile firmware input was out of scope, and the argument rests on
the firmware being trusted, which is the boot model's assumption rather than something the loader
checks.

## BUGS

- **Rust, nearly all of this tree, is unscanned until a custom workflow exists.** A new Rust alert
  cannot appear, and the 43 dismissed above will not reopen if their code regresses. This is an
  open decision for calef, below.
- §36's stated trigger for leaving default setup has fired: an alert landed in `vendor/**` (five
  of them, all in `vendor/redoxfs`). Default setup cannot exclude a path.
- §35 asks for a dismissal's reason at the code. For the 17 false positives the existing `SAFETY`
  and provenance comments at each site carry the argument, and this note carries the rest; no
  per-line suppression comment was added, because CodeQL reads none.

## Open decision: how Rust gets scanned again

Options, for calef:

1. Advanced setup. A committed `.github/workflows/codeql.yml` covering all four languages
   (GitHub rejects advanced-setup uploads while default setup is on), with `vendor/**` excluded,
   Rust in `build-mode: none`, and an upload retry. Cost: one maintained workflow file, which §36
   avoided while the Rust extractor was moving fast. Recommended, because it answers both the
   vendor trigger and the upload failure and is reversible by deleting the file.
2. Re-add Rust to default setup and accept the intermittent red job. CodeQL is not a required
   check (`notes/repo-hardening.md`, its second section), so a failure does not block a merge, but it is noise on
   every run that fails.
3. Leave Rust unscanned. Kani, the fuzzers and the unsafe census still run. This gives up the
   only tool here that looks for taint flows across the whole tree.

Nothing is blocked on the answer; the cost of waiting is that Rust regressions go unscanned.
