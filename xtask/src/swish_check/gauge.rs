//! **The gauges the kernel prints into a transcript, taken out as it arrives** (`script/swish-check`).
//! The progenitor-stack and capability-slot gauges speak from the kernel's idle loop, between a
//! prompt and the next line's echo, and every reader in [`super`] assumes those two are adjacent.
//! Moved out of `swish_check.rs` unchanged by milestone 809 (the package client becomes a program),
//! when that file outgrew §266 (a Rust source file stays under 2,000 lines)'s ratchet.

#[allow(unused_imports)]
use super::*;

/// **Takes the kernel's progenitor stack gauge out of the transcript as it arrives**, and keeps it.
///
/// The gauge (`kernel::progenitor_stack`) speaks from the idle loop once the stack's mark has been
/// still for a while, and on a healthy boot that is shortly after a command's prompt has come back.
/// So it lands between `$ ` and the next line's echo, and every reader in this file assumes those
/// two are adjacent: the first version with the gauge in it waited thirty seconds for a prompt that
/// had already been printed, then read `echo hello world | wc` as having answered nothing. Rather
/// than teach each reader about a third writer, the gauge never reaches them.
///
/// **Streaming, and prefix-stable on purpose.** The reader thread sees the UART in arbitrary
/// chunks, so a gauge line can arrive in pieces, and the waits below take `seen.len()` as a cursor
/// into text that is still growing. So this never emits text it might later want back: a tail that
/// could be the start of a gauge line is held until it either is one (and is dropped, newline and
/// all) or is not (and is emitted). What is emitted is only ever appended to.
///
/// It removes the kernel's line only when the kernel's line is whole. A gauge a userspace writer
/// shuffled into is left in place, which fails the run in the way any shuffle does.
///
/// **The capability-slot gauge is taken out too** (2026-10-02 (UTC), milestone 152's lane). It
/// speaks from the same idle loop once its own mark settles, and until then it always had settled
/// before the first prompt. Milestone 152 moved the peak into the login block's tail, so the line
/// landed after the first `$ ` and the gate waited thirty seconds for a prompt that had already
/// been printed, reporting `the prompt never came back` for a shell that was waiting to be typed
/// at. Its check reads the raw transcript, which still carries it.
pub(super) struct GaugeFilter {
    pending: String,
    needles: &'static [&'static str],
}

impl Default for GaugeFilter {
    fn default() -> Self {
        Self::with_needles(&Self::NEEDLES)
    }
}

impl GaugeFilter {
    /// What `kernel::progenitor_stack::announce` prints first, its leading indent included: the
    /// kernel's own prefix for the line. Name provisional.
    pub(super) const NEEDLE: &'static str = "  progenitor stack:";
    /// What `kernel::cap::report_peak` prints first, its leading indent included.
    pub(super) const SLOT_NEEDLE: &'static str = "  capability slots:";
    /// Every kernel line this filter takes out. Both start with the kernel's two-space indent,
    /// which the bare-prompt exception in [`GaugeFilter::feed`] relies on.
    pub(super) const NEEDLES: [&'static str; 2] = [Self::NEEDLE, Self::SLOT_NEEDLE];

    /// A filter for other kernel lines with the same shape (milestone 342's flood probe).
    pub(super) fn with_needles(needles: &'static [&'static str]) -> Self {
        GaugeFilter {
            pending: String::new(),
            needles,
        }
    }

    /// Feed the next chunk. Text that is certainly not a gauge is appended to `out`; each whole
    /// gauge line is pushed to `gauges` with the length `out` had when it was removed.
    pub(super) fn feed(
        &mut self,
        chunk: &str,
        out: &mut String,
        gauges: &mut Vec<(usize, String)>,
    ) {
        self.pending.push_str(chunk);
        loop {
            if let Some(at) = self
                .needles
                .iter()
                .filter_map(|n| self.pending.find(n))
                .min()
            {
                out.push_str(&self.pending[..at]);
                match self.pending[at..].find('\n') {
                    Some(nl) => {
                        let line = self.pending[at..at + nl].trim().to_string();
                        gauges.push((out.len(), line));
                        self.pending.drain(..at + nl + 1);
                    }
                    None => {
                        self.pending.drain(..at);
                        return;
                    }
                }
            } else {
                // Hold back the longest tail that is a proper prefix of the needle, except the
                // space of a bare `$ `: the needle starts with the kernel's indent, so without
                // this the prompt every wait below looks for would never be emitted whole.
                let mut keep = self
                    .needles
                    .iter()
                    .filter_map(|needle| {
                        (1..needle.len())
                            .rev()
                            .find(|&n| self.pending.ends_with(&needle[..n]))
                    })
                    .max()
                    .unwrap_or(0);
                let before = &self.pending[..self.pending.len() - keep];
                let after_dollar = if before.is_empty() {
                    out.ends_with('$')
                } else {
                    before.ends_with('$')
                };
                if keep > 0 && after_dollar {
                    keep -= 1;
                }
                let cut = self.pending.len() - keep;
                out.push_str(&self.pending[..cut]);
                self.pending.drain(..cut);
                return;
            }
        }
    }
}

/// One segment of a gauge sentence: literal text, or one of its numbers. A number is a wildcard
/// (matched as "one or more digits") rather than a fixed value, because the value is not known
/// ahead of time: [`degauge`] is asking "is a gauge here at all", not "is this exact gauge here".
#[derive(Clone, Copy)]
pub(super) enum GaugeSeg {
    Lit(&'static str),
    Num,
}

/// Every sentence `kernel::progenitor_stack::announce` and `kernel::cap::announce_peak` can print,
/// grepped from those two functions verbatim (2026-09-27). Longer variants first, so a `BELOW` or
/// `ABOVE` sentence is matched whole rather than leaving its tail as unmatched noise once the
/// shorter, common prefix has already been consumed. See [`degauge`].
pub(super) const GAUGE_TEMPLATES: &[&[GaugeSeg]] = &[
    &[
        GaugeSeg::Lit("  progenitor stack: "),
        GaugeSeg::Num,
        GaugeSeg::Lit(" of "),
        GaugeSeg::Num,
        GaugeSeg::Lit(" bytes at peak, "),
        GaugeSeg::Num,
        GaugeSeg::Lit(" spare, BELOW the "),
        GaugeSeg::Num,
        GaugeSeg::Lit("-byte floor in kernel/src/progenitor_stack.rs"),
    ],
    &[
        GaugeSeg::Lit("  progenitor stack: "),
        GaugeSeg::Num,
        GaugeSeg::Lit(" of "),
        GaugeSeg::Num,
        GaugeSeg::Lit(" bytes at peak, "),
        GaugeSeg::Num,
        GaugeSeg::Lit(" spare"),
    ],
    &[
        GaugeSeg::Lit("  capability slots: "),
        GaugeSeg::Num,
        GaugeSeg::Lit(" of "),
        GaugeSeg::Num,
        GaugeSeg::Lit(" at peak, ABOVE the "),
        GaugeSeg::Num,
        GaugeSeg::Lit(" recorded in kernel/src/cap.rs"),
    ],
    &[
        GaugeSeg::Lit("  capability slots: "),
        GaugeSeg::Num,
        GaugeSeg::Lit(" of "),
        GaugeSeg::Num,
        GaugeSeg::Lit(" at peak"),
    ],
];

/// Try `template` starting at `chars[start]`, tolerating intruder characters wedged between its
/// own the way [`find_marker`] tolerates them in a flat needle, up to the same
/// [`SWISH_CHECK_MARKER_SLACK`] budget. `None` if the template does not fit in the budget or runs
/// off the end of `chars`. On success, returns where the match ended and, for every position from
/// `start` to that end, whether it belongs to the gauge (`true`) or is an intruder byte that must
/// be left alone (`false`).
pub(super) fn gauge_template_match(
    chars: &[char],
    start: usize,
    template: &[GaugeSeg],
) -> Option<(usize, Vec<bool>)> {
    let mut mask = Vec::new();
    let mut j = start;
    let mut skipped = 0usize;
    for seg in template {
        match *seg {
            GaugeSeg::Lit(word) => {
                for want in word.chars() {
                    loop {
                        if j >= chars.len() {
                            return None;
                        }
                        if chars[j] == want {
                            mask.push(true);
                            j += 1;
                            break;
                        }
                        if skipped >= SWISH_CHECK_MARKER_SLACK {
                            return None;
                        }
                        mask.push(false);
                        skipped += 1;
                        j += 1;
                    }
                }
            }
            GaugeSeg::Num => {
                let mut got_digit = false;
                loop {
                    if j >= chars.len() {
                        if got_digit {
                            break;
                        }
                        return None;
                    }
                    if chars[j].is_ascii_digit() {
                        mask.push(true);
                        got_digit = true;
                        j += 1;
                    } else if got_digit {
                        // The number ended: this character belongs to whatever comes next, not to
                        // the digit run, so it is left for the following segment to see.
                        break;
                    } else if skipped >= SWISH_CHECK_MARKER_SLACK {
                        return None;
                    } else {
                        mask.push(false);
                        skipped += 1;
                        j += 1;
                    }
                }
            }
        }
    }
    Some((j, mask))
}

/// Delete the best (fewest intruder characters) occurrence of any [`GAUGE_TEMPLATES`] template
/// from `text`. `None` if no template appears at all within budget.
pub(super) fn strip_one_gauge(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut best: Option<(usize, usize, Vec<bool>, usize)> = None;
    for template in GAUGE_TEMPLATES {
        let GaugeSeg::Lit(first_word) = template[0] else {
            unreachable!("every gauge template starts with a literal");
        };
        let first_char = first_word.chars().next().expect("non-empty literal");
        for start in 0..chars.len() {
            if chars[start] != first_char {
                continue;
            }
            if let Some((end, mask)) = gauge_template_match(&chars, start, template) {
                let skipped = mask.iter().filter(|kept| !**kept).count();
                if best.as_ref().is_none_or(|(_, _, _, s)| skipped < *s) {
                    best = Some((start, end, mask, skipped));
                }
            }
        }
    }
    best.map(|(start, end, mask, _)| {
        let mut out = String::with_capacity(text.len());
        out.extend(&chars[..start]);
        for (offset, keep) in mask.iter().enumerate() {
            if !keep {
                out.push(chars[start + offset]);
            }
        }
        // The kernel ends its line with a newline, and when the gauge was spliced into an echo that
        // newline lands after the gauge's last word, splitting the echo in two (`echo h` and
        // `ello world | wc` in #1377's riscv64 run of 2026-10-03). A newline right where the gauge
        // ended is the kernel's, so it goes with the gauge.
        let rest = if chars.get(end) == Some(&'\n') {
            end + 1
        } else {
            end
        };
        out.extend(&chars[rest..]);
        out
    })
}

/// **Interim measure for §175 (where the kernel's own output goes once userspace owns the
/// console), ruled 2026-09-27 to go through a ring a log service drains, and not yet built.**
/// Takes a gauge's own characters out of a transcript
/// even when a second writer spliced them in one at a time, rather than as the whole line
/// [`GaugeFilter`] above assumes. That assumption held until #1371's CI run, where the
/// progenitor-stack gauge landed character-by-character inside the shell's own echo of
/// `package install`, producing `package   proinstgenitor sall tack: 31528 of 49152 bytes at
/// peak, 17624 spare`: no contiguous `"progenitor stack:"` was ever there for `GaugeFilter` to
/// find, `swish_check_leg`'s exact search for `"package install\n"` never matched either, and the
/// run failed with "the prompt never echoed `package install`", a false report of a hung shell.
/// The same race hit #1420 and is not particular to one command; anything typed while a gauge
/// happens to print can be shuffled the same way.
///
/// Same asymmetry [`find_marker`] relies on: interleaving can destroy a known string, never
/// manufacture one, so matching the gauge's own words (numbers as wildcards, since their values
/// are not known ahead of time) has no false positives worth the name. Unlike `find_marker`, which
/// only answers "is it there", this deletes just the matched characters and hands back everything
/// else exactly where it was, because the caller needs the *rest* of the stream back in a shape its
/// own exact-match waits can still recognize.
///
/// Remove once §175 is built: a kernel that no longer writes the UART directly once userspace owns
/// it has nothing left here to splice.
pub(super) fn degauge(text: &str) -> String {
    let mut out = text.to_string();
    // Bounded rather than "until none found": a gate must not hang on a text that somehow keeps
    // offering a match. A boot does not print more than a handful of gauge lines.
    for _ in 0..64 {
        match strip_one_gauge(&out) {
            Some(next) => out = next,
            None => break,
        }
    }
    out
}

/// **Takes out the prompt lines the console drew a second time beneath a gauge** (2026-10-04 UTC,
/// the noteless flake, `notes/swish-check-flake.md`). `filtered` is [`GaugeFilter`]'s output and
/// `gauges` the offsets it removed lines at.
///
/// A kernel line that reaches the console mid-line waits for the line to end. If it is still
/// waiting when the system log service's flush timer fires (`components/src/system_log.rs`'s
/// `FLUSH_NANOS`), the console writes a line end, the kernel line, and the partial line again
/// (`system_log_protocol::console::Inserter::flush`), which is milestone 342's design and what a
/// person at the terminal should see. With the gauge taken out, a flush that fell after the echo
/// of a typed line's last character and before the echo of its Enter leaves `$ line\n$ line\n`,
/// and [`swish_check_answer`] read the first copy as a command that printed nothing.
///
/// Only an exact copy is removed: the line ending at a gauge's offset, starting `$ `, followed at
/// that offset by the same line and its line end. A flush that fell mid-typing leaves a prefix
/// (`$ pack\n$ packages/...\n`), which every reader already handles, and is left as it is. The one
/// shape this could mistake for a redraw is a line typed twice in a row whose first run printed
/// nothing, and `no_script_types_a_silent_line_twice_in_a_row` keeps every script free of it.
///
/// **Only gauges are taken out of the redraw's queue.** Another kernel line flushed above a redraw
/// stays in the transcript and fails the line it lands in, loudly, which is the right default for a
/// line nothing here expects.
pub(super) fn without_redraws(filtered: &str, gauges: &[(usize, String)]) -> String {
    let mut cuts: Vec<(usize, usize)> = Vec::new();
    let mut offsets: Vec<usize> = gauges.iter().map(|(at, _)| *at).collect();
    offsets.dedup();
    for at in offsets {
        let Some(before) = filtered.get(..at).and_then(|b| b.strip_suffix('\n')) else {
            continue;
        };
        let start = before.rfind('\n').map_or(0, |i| i + 1);
        let partial = &before[start..];
        if !partial.starts_with("$ ") {
            continue;
        }
        let copy = &filtered[at..];
        if copy.starts_with(partial) && copy[partial.len()..].starts_with('\n') {
            cuts.push((start, at));
        }
    }
    let mut out = String::with_capacity(filtered.len());
    let mut from = 0;
    for (start, end) in cuts {
        if start >= from {
            out.push_str(&filtered[from..start]);
            from = end;
        }
    }
    out.push_str(&filtered[from..]);
    out
}

/// Which typed command a gauge removed at `at` belongs to: the last `$ ` line before it, skipping
/// the bare prompt the gauge usually follows, since that prompt is the *next* command's.
pub(super) fn gauge_follows(transcript: &str, at: usize) -> &str {
    let before = transcript[..at].trim_end_matches("$ ");
    before
        .lines()
        .rev()
        .find_map(|l| l.strip_prefix("$ ").filter(|c| !c.trim().is_empty()))
        .unwrap_or("(boot)")
}
