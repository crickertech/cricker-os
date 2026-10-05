// throwaway: a code change, to prove the full run (never merges)
//! What the renderer must get right, and the two properties that make it usable at all.
//!
//! The interesting tests here are the last two. `framing_does_not_matter` is what lets `doc` render
//! straight off sixteen-byte sink messages with no reassembly buffer, and `every_word_survives`
//! is the closed-corpus argument in executable form: this renderer was written instead of taking
//! `pulldown-cmark`, and the justification for that is that the input set is *this repository*, so
//! the conformance claim is checkable directly against it rather than against a spec suite.

use documentation::{Renderer, Sink, Style};

struct Buf(Vec<u8>);

impl Sink for Buf {
    fn put(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }
}

fn plain(src: &str, width: u16) -> String {
    render(
        src,
        Style {
            width,
            color: false,
        },
    )
}

fn render(src: &str, style: Style) -> String {
    let mut out = Buf(Vec::new());
    let mut r = Renderer::new(style);
    r.feed(src.as_bytes(), &mut out);
    r.finish(&mut out);
    String::from_utf8(out.0).unwrap()
}

#[test]
fn headings_shout_once() {
    // Level one is the page's name and is uppercased; level two keeps its case, because a document
    // whose every heading shouts has told the reader nothing about which one is the title.
    assert_eq!(
        plain("# Title\n\n## Section\n\nBody.\n", 80),
        "TITLE\n\nSection\n\n  Body.\n"
    );
}

#[test]
fn a_hash_without_a_space_is_not_a_heading() {
    // Half the fenced code in this repository is shell, and shell comments start with `#`. This is
    // the rule that keeps `#!/bin/sh` and `# a comment` out of the heading path.
    assert_eq!(plain("#nope\n", 80), "  #nope\n");
}

#[test]
fn paragraphs_rewrap() {
    // Source line breaks are not output line breaks: a paragraph is re-flowed to the terminal's
    // width, which is the whole reason to render markdown rather than print it.
    let out = plain("one two\nthree four five\n", 12);
    assert_eq!(out, "  one two\n  three four\n  five\n");
}

#[test]
fn code_blocks_are_verbatim() {
    let out = plain("```sh\n  ls   -l\n```\n", 80);
    assert_eq!(out, "      ls   -l\n");
}

#[test]
fn a_code_span_protects_its_contents() {
    // 11,281 code spans in this corpus, many of them full of the characters the other inline rules
    // look for. If this ordering were wrong, `*ptr` inside backticks would open emphasis and eat
    // the rest of the line.
    let out = render(
        "a `*ptr [x](y)` b\n",
        Style {
            width: 80,
            color: true,
        },
    );
    assert!(out.contains("\x1b[36m*ptr [x](y)"));
    assert!(!out.contains("\x1b[4m"));
}

#[test]
fn underscores_are_never_emphasis() {
    // The corpus writes `snake_case` identifiers in running prose constantly. CommonMark would read
    // `__rust_alloc` as an opened strong span; here it is text, and that is a deliberate narrowing
    // recorded in the renderer's BUGS.
    let out = render(
        "the __rust_alloc symbol and filesystem_protocol\n",
        Style {
            width: 80,
            color: true,
        },
    );
    assert!(!out.contains('\x1b'));
}

#[test]
fn emphasis_needs_a_closer_that_is_not_a_space() {
    let styled = render(
        "a *b* c\n",
        Style {
            width: 80,
            color: true,
        },
    );
    assert!(styled.contains("\x1b[4mb"));
    // A lone asterisk mid-sentence has no closer and must not swallow the rest of the line.
    let stray = render(
        "2 * 3 = 6\n",
        Style {
            width: 80,
            color: true,
        },
    );
    assert!(!stray.contains('\x1b'));
}

#[test]
fn list_markers_are_normalised_and_hang() {
    // Three spellings of a bullet on one screen read as three different things, so they are one
    // spelling on output. The continuation aligns under the text, not under the bullet.
    let out = plain("* one two three\n+ four\n- five\n", 14);
    assert_eq!(out, "  - one two\n    three\n  - four\n  - five\n");
}

#[test]
fn ordered_lists_keep_their_numbers() {
    assert_eq!(plain("1. one\n2. two\n", 80), "  1. one\n  2. two\n");
}

#[test]
fn block_quotes_get_a_rule() {
    assert_eq!(plain("> quoted\n", 80), "  | quoted\n");
}

#[test]
fn tables_align_to_their_widest_cell() {
    let out = plain("| a | bbbb |\n|---|---|\n| cc | d |\n", 80);
    assert_eq!(out, "  a  | bbbb\n  ---+-----\n  cc | d   \n");
}

#[test]
fn a_pipe_line_without_a_delimiter_is_still_printed() {
    // A paragraph that happens to start with a pipe must not be silently eaten by the table path.
    let out = plain("| not a table\n", 80);
    assert!(out.contains("not a table"));
}

#[test]
fn links_show_where_they_point() {
    // A terminal cannot follow a link, so a reader who cannot see the destination has been handed a
    // worse document than the source.
    let out = plain("see [the note](notes/glob.md) for more\n", 80);
    assert_eq!(out, "  see the note notes/glob.md for more\n");
}

#[test]
fn an_attribute_never_survives_a_newline() {
    // A terminal left bold at end of line paints the next row and the prompt with it. Every line
    // this renderer emits ends reset.
    let out = render(
        "# Title\n\n**strong text that wraps past the edge**\n",
        Style {
            width: 20,
            color: true,
        },
    );
    for line in out.split('\n') {
        if line.contains('\x1b') {
            assert!(
                line.ends_with("\x1b[0m") || !line.contains("\x1b["),
                "line: {line:?}"
            );
        }
    }
}

#[test]
fn a_fence_inside_a_block_quote_closes() {
    // The defect this replaces: `is_closing` was matched against the raw line, so a quoted closing
    // fence never matched its own opener and the renderer stayed in code mode to the end of the
    // document. A page with one quoted transcript in it rendered its next three hundred lines as
    // quoted code, and the corpus test could not see it, because verbatim output loses no
    // characters. What that test *did* see was the opening fence's info string, which is why the
    // filter below strips quote markers now: the renderer is right about that line, and it was the
    // filter that was wrong to keep it as evidence.
    let src = "> intro\n>\n> ```text\n>   code here\n> ```\n\nAfter.\n\n## A heading\n";
    let out = plain(src, 80);
    assert_eq!(
        out, "  | intro\n\n    |   code here\n\n  After.\n\nA heading\n",
        "{out:?}"
    );
}

#[test]
fn a_code_line_that_starts_with_a_quote_marker_keeps_it() {
    // The other side of the same rule. Outside a block quote the depth is zero, so nothing is
    // stripped and a mail quote, a diff or a shell transcript inside a fence survives intact.
    assert_eq!(plain("```text\n> quoted\n```\n", 80), "    > quoted\n");
}

#[test]
fn framing_does_not_matter() {
    // The property `doc` depends on: input arrives as sixteen-byte sink messages, and a renderer
    // that produced different output for different chunkings would need a reassembly buffer, which
    // is a memory grant the program can otherwise do without.
    let src = "# T\n\ntext with **bold** and `code`\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n> q\n\n- item\n";
    let whole = plain(src, 40);
    for chunk in [1usize, 3, 16, 17, 4096] {
        let mut out = Buf(Vec::new());
        let mut r = Renderer::new(Style {
            width: 40,
            color: false,
        });
        for piece in src.as_bytes().chunks(chunk) {
            r.feed(piece, &mut out);
        }
        r.finish(&mut out);
        assert_eq!(
            String::from_utf8(out.0).unwrap(),
            whole,
            "chunk size {chunk}"
        );
    }
}

#[test]
fn a_document_with_no_trailing_newline_still_ends() {
    assert_eq!(plain("# T", 80), "T\n");
}

#[test]
fn an_overlong_line_is_reported_rather_than_hidden() {
    let long = "x".repeat(documentation::LINE_MAX + 10);
    let mut out = Buf(Vec::new());
    let mut r = Renderer::new(Style {
        width: 80,
        color: false,
    });
    r.feed(long.as_bytes(), &mut out);
    r.finish(&mut out);
    assert!(r.is_truncated());
}

/// Is this source line a fence opener or closer, quoted or not?
///
/// A quoted fence is a fence: the renderer strips the markers before it classifies the line, so
/// `> ` followed by three backticks opens a code block exactly as three backticks alone do.
fn is_fence(line: &str) -> bool {
    let mut s = line.trim_start();
    while let Some(rest) = s.strip_prefix('>') {
        s = rest.trim_start();
    }
    s.starts_with("```")
}

/// Every letter and digit in the repository's own markdown reaches the rendered output, in order.
///
/// This is the claim that replaces a `CommonMark` conformance suite. A renderer that silently ate a
/// construct (an HTML block, a reference link, an indented continuation, a table row) would drop
/// the characters inside it, and a subsequence check finds that on real files rather than on
/// invented ones. Subsequence rather than equality because the renderer legitimately *adds*
/// characters (an image's `image:` marker) and legitimately *joins* what markup split
/// (`**L**inkable` is two runs in the source and one word on screen).
///
/// It runs at a terminal four thousand columns wide, because a real one truncates table cells to
/// their column width, and that is a formatting choice the BUGS section records rather than a
/// parsing failure. The number was found rather than picked: at 1000 columns this repository's
/// widest table still shrinks, which is what a documentation service's honest caveat looks like
/// from the inside.
/// **Not run under Miri, and the reason is that Miri has nothing to say about it** (milestone 238).
/// This test's subject is a corpus on disk: it opens 547 markdown files, 7.2 MB, and renders every
/// one. Under the interpreter that needs two concessions, and neither is cheap. It reads
/// `CARGO_MANIFEST_DIR` at run time, which Miri does not forward, so it wants
/// `-Zmiri-env-forward=CARGO_MANIFEST_DIR`; and then it calls `read_dir`, which Miri's isolation
/// refuses, so it wants `-Zmiri-disable-isolation` as well, for the whole workspace run rather than
/// for this one test. The measurement that settled it: **0.74 seconds natively, and still running
/// after 12 minutes under Miri when it was killed** (2026-09-03).
///
/// What that would buy is nothing. `crates/documentation` has **no dependencies and no `unsafe`**, so the
/// rules Miri enforces (aliasing, provenance, uninitialized reads) cannot be broken by any line in
/// its call graph. The nineteen tests above it in this file exercise the same renderer on in-memory
/// input, cover every construct it parses, and do run under Miri; what this one adds is corpus
/// breadth, which is a documentation claim rather than a soundness one.
///
/// So this is the `cfg(miri)` sampling convention the other suites use
/// (`globally_unique_identifier_partition_table`'s corruption sweeps,
/// `glob`'s strides, `network_time_protocol`'s 10^9-value sweep), taken to its limit: the sampled paths here are
/// the nineteen unit tests, and the corpus stays native-only. See notes/undefined-behavior.md.
///
/// This is also the whole content of the three weeks the weekly `undefined-behavior check` spent
/// red. It was never undefined behaviour; it was an environment variable, and behind that a
/// directory read.
#[test]
#[cfg_attr(
    miri,
    ignore = "547 markdown files off disk; the nineteen unit tests cover the renderer"
)]
fn every_character_survives() {
    // Runtime, not env!: the compile-time form bakes the absolute path into the binary, and a
    // cached artifact built before a checkout moves (the 2026-08-15 cricker-os -> nife directory
    // rename) then reads a directory that no longer exists and checks zero files. Cargo sets the
    // variable at run time too, and that one is always the live path.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets this for tests");
    let root = std::path::Path::new(&manifest)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let mut checked = 0;
    for dir in ["notes", "design/decisions", "design/roadmap"] {
        let Ok(entries) = std::fs::read_dir(root.join(dir)) else {
            continue;
        };
        // One level down as well: §212 (a prose budget) moves a long page's depth into appendices
        // under `notes/<stem>/`, and a corpus that stopped at the parent would stop checking every
        // word the split moved.
        let mut paths = Vec::new();
        for e in entries.flatten() {
            let path = e.path();
            if path.is_dir() {
                if let Ok(sub) = std::fs::read_dir(&path) {
                    paths.extend(sub.flatten().map(|e| e.path()));
                }
            } else {
                paths.push(path);
            }
        }
        for path in paths {
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }
            let Ok(src) = std::fs::read_to_string(&path) else {
                continue;
            };
            // A fence's info string (the `bash` in a ```bash line) is a language tag rather than
            // content, so it is not offered as evidence. Nothing else is excluded: a table longer
            // than the buffer spills into another chunk rather than losing rows, which is exactly
            // the behaviour this test exists to hold in place.
            //
            // **Block-quote markers come off first**, and the order that happened in matters. This
            // filter used to miss a quoted fence, so a page containing one failed a test it had
            // rendered fine for; the entry that recorded that refused to widen the filter until
            // somebody answered whether the renderer keeps its quote state across a nested fence.
            // It did not: the fence never closed, and the rest of the page rendered as quoted code
            // (2026-08-18, `crates/documentation`'s BUGS). Widening the filter first would have hidden
            // that, which is exactly what the refusal was protecting.
            let want: Vec<char> = src
                .lines()
                .filter(|l| !is_fence(l))
                .flat_map(|l| l.chars().chain(core::iter::once('\n')))
                .filter(char::is_ascii_alphanumeric)
                .map(|c| c.to_ascii_lowercase())
                .collect();
            // **No page ends inside a fence.** Every page in this repository closes the fences it
            // opens, so a renderer that thinks otherwise at the end of one is wrong about the
            // renderer rather than about the page.
            //
            // It is a **partial** guard and the limit is worth knowing, because it was measured
            // rather than assumed. A renderer stuck in code mode still emits every character
            // verbatim, so the subsequence check below cannot see it at all; this one sees it only
            // if nothing later in the page happens to close the stuck fence. Reverting the
            // quoted-fence fix leaves notes/documentation.md ruined from its own worked example onward and
            // **this assertion still passes**, because a bare closing fence three sections later
            // matches. `a_fence_inside_a_block_quote_closes` is the guard; this is a cheap
            // invariant that catches the case where the stuck fence is the last one.
            let mut out = Buf(Vec::new());
            let mut r = Renderer::new(Style {
                width: 4000,
                color: false,
            });
            r.feed(src.as_bytes(), &mut out);
            r.finish(&mut out);
            assert!(
                !r.has_unclosed_fence(),
                "{}: the renderer ended inside a code fence",
                path.display()
            );

            let have: Vec<char> = String::from_utf8(out.0)
                .unwrap()
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .map(|c| c.to_ascii_lowercase())
                .collect();
            let mut i = 0;
            for c in &have {
                if i < want.len() && want[i] == *c {
                    i += 1;
                }
            }
            assert!(
                i == want.len(),
                "{}: rendering dropped text near {:?}",
                path.display(),
                want[i.saturating_sub(30)..(i + 30).min(want.len())]
                    .iter()
                    .collect::<String>()
            );
            checked += 1;
        }
    }
    assert!(checked > 100, "only checked {checked} files");
}

// ---- milestone 280: what the mutation sweep found nothing asserting -------------------------
//
// Every test below was written against a surviving mutant. The sweep scored this crate at 52% in
// the first published report, and the first half of that was a measurement bug (see
// notes/mutation-testing.md); this is the other half, which was a real gap. Two of them are not
// tests for behaviour that already worked: `read_align` filled a per-column alignment nothing read,
// and `indent_of` returned a column count nothing read, so the mutants there could not be killed
// until the values had a consumer.

#[test]
fn table_columns_align_the_way_the_delimiter_row_says() {
    // `|:---|---:|:-:|` is left, right and centre, and until milestone 280 all three rendered
    // identically because `flush_table` padded every cell on the right. Sixteen mutants in
    // `read_align` survived the sweep for that reason, including replacing the whole function with
    // `()`: a value nothing reads cannot be computed wrongly in a way anything notices.
    let out = plain("| aaa | bbb | ccc |\n|:---|---:|:-:|\n| x | y | z |\n", 80);
    assert_eq!(
        out, "  aaa | bbb | ccc\n  ----+-----+----\n  x   |   y |  z \n",
        "{out:?}"
    );
}

#[test]
fn alignment_does_not_outlive_its_table() {
    // Alignment is a property of one delimiter row, so it is reset with `delimited` rather than
    // carried. A paragraph that happens to begin with a pipe follows a right-aligned table here,
    // and takes the default: if the array were not reset its cells would sit on the right.
    let out = plain(
        "| a | b |\n|---:|---|\n| x | y |\n\n| pp | q |\n| r | s |\n",
        80,
    );
    assert!(out.ends_with("  pp | q\n  r  | s\n"), "{out:?}");
}

#[test]
fn an_escaped_pipe_is_cell_text_and_not_a_column_boundary() {
    // `notes/scripts/build-test-and-gate.md` has one (`--arch aarch64\| riscv64`; the row lived in
    // `notes/scripts.md` until 2026-09-25), and reading it as a separator gave
    // that table a third column nobody wrote and squeezed the other two to pay for it. The
    // backslash is the escape and not the text, so it does not reach the output either.
    let out = plain("| cmd | note |\n|---|---|\n| a \\| b | two |\n", 80);
    assert_eq!(
        out, "  cmd   | note\n  ------+-----\n  a | b | two \n",
        "{out:?}"
    );
}

#[test]
fn a_trailing_pipe_does_not_add_a_column() {
    // `| a | b |` and `| a | b` are the same two-column row: the cell after the last pipe is one
    // nobody typed, and keeping it would widen every table in the repository by a blank column.
    assert_eq!(
        plain("| a | b |\n|---|---|\n| x | y |\n", 80),
        plain("| a | b\n|---|---\n| x | y\n", 80)
    );
}

#[test]
fn a_table_wider_than_the_terminal_shrinks_its_widest_column() {
    // Shrinking the widest column rather than all of them is what keeps a table of one long prose
    // column and three short labels readable: the prose loses characters and the labels do not.
    let out = plain(
        "| aaaaaaaaaa | bb |\n|---|---|\n| cccccccccccccccccccc | d |\n",
        20,
    );
    assert_eq!(
        out, "  aaaaaaaaaa    | bb\n  --------------+---\n  ccccccccccccc | d \n",
        "{out:?}"
    );
    // Every line fits the terminal, which is the property the shrink exists for.
    for line in out.lines() {
        assert!(line.chars().count() <= 20, "line over width: {line:?}");
    }
}

#[test]
fn a_table_longer_than_the_buffer_spills_rather_than_losing_rows() {
    // Losing text is the one failure mode a documentation service cannot have, so a table past
    // `TABLE_ROWS` is emitted in chunks, each aligned to its own widths, with no blank line between
    // them because they are one table. `design/decisions/README.md` is over 200 rows, so this is
    // real.
    let mut src = String::from("| n | v |\n|---|---|\n");
    let rows = documentation::TABLE_ROWS + 10;
    for i in 0..rows {
        src.push_str(&format!("| r{i} | v{i} |\n"));
    }
    let out = plain(&src, 80);
    for i in 0..rows {
        assert!(
            out.contains(&format!("r{i} ")) || out.contains(&format!("r{i}\n")),
            "row {i} lost"
        );
    }
    assert!(
        !out.contains("\n\n"),
        "a spilled chunk is not a new block: {out:?}"
    );
}

#[test]
fn a_header_row_is_emphasised_only_when_a_delimiter_row_says_it_is_one() {
    // The delimiter row is what promotes a run of pipe-led lines to a table; without one the first
    // row is ordinary text that happens to contain pipes, and emphasising it would claim a
    // structure the author did not write.
    let table = render(
        "| h |\n|---|\n| c |\n",
        Style {
            width: 40,
            color: true,
        },
    );
    assert!(table.contains("\x1b[1mh"), "{table:?}");
    let not_a_table = render(
        "| h |\n| c |\n",
        Style {
            width: 40,
            color: true,
        },
    );
    assert!(!not_a_table.contains("\x1b[1m"), "{not_a_table:?}");
}

#[test]
fn thematic_breaks_are_three_or_more_of_one_mark() {
    // Three of `-`, `*` or `_` with only spaces between them, and nothing else. The corpus writes
    // `---` far more often as a break than as a setext underline, which is why this renderer has no
    // setext headings at all; that trade is only honest if the break itself is right.
    let rule = "  ------------------\n";
    for src in ["---", "***", "___", "- - -", "----------"] {
        let out = plain(&format!("a\n\n{src}\n\nb\n"), 20);
        assert_eq!(
            out,
            format!("  a\n\n{rule}\n  b\n"),
            "{src} is a thematic break"
        );
    }
    // Two marks is not three, a different mark is not a rule at all, and a mark with text beside it
    // is a paragraph. Each of these renders as its own source text.
    for src in ["--", "+++", "-a-", "** *x"] {
        let out = plain(&format!("a\n\n{src}\n\nb\n"), 20);
        assert!(!out.contains("-----"), "{src} rendered as a rule: {out:?}");
    }
}

#[test]
fn a_document_that_lost_nothing_says_so() {
    // The other side of `an_overlong_line_is_reported_rather_than_hidden`, and it is the side a
    // caller acts on: `doc` prints a warning when this is true, so a renderer that always answered
    // true would warn about every page in the store. Asserting only the positive left a mutant that
    // hard-codes it alive.
    let mut out = Buf(Vec::new());
    let mut r = Renderer::new(Style {
        width: 80,
        color: false,
    });
    r.feed(b"# Title\n\nA line well inside the limit.\n", &mut out);
    r.finish(&mut out);
    assert!(!r.is_truncated());
}

#[test]
fn a_page_that_never_closes_its_fence_says_so() {
    // `every_character_survives` asserts this is false for all 547 pages, which a renderer that
    // always answered false would pass. The guard is only worth having if it can answer true.
    let mut out = Buf(Vec::new());
    let mut r = Renderer::new(Style {
        width: 80,
        color: false,
    });
    r.feed(b"```text\nstill inside\n", &mut out);
    r.finish(&mut out);
    assert!(r.has_unclosed_fence());
}

#[test]
fn a_seventh_hash_is_not_a_heading() {
    // Markdown stops at six, and past that the line is text. The rule sits beside the one that
    // keeps `#!/bin/sh` out of the heading path, and neither had a test on its upper edge.
    assert_eq!(plain("###### six\n", 80), "        six\n");
    assert_eq!(plain("####### seven\n", 80), "  ####### seven\n");
}

#[test]
fn a_tab_indents_as_four_columns() {
    // `indent_of` counts a tab as four columns and a space as one, and until milestone 280 every
    // caller took its byte offset instead, so a tab indented by one. Nothing in this repository
    // indents with a tab outside a fence, where this does not run, so the sweep was the only thing
    // that could have found it.
    assert_eq!(
        plain("para\n\n\tmore text\n", 40),
        plain("para\n\n    more text\n", 40)
    );
}

#[test]
fn an_ordered_marker_may_be_a_parenthesis_or_two_digits() {
    // `1.`, `1)` and `10.` are all list markers and keep the number the author wrote, because a
    // renumbered list is a different document.
    assert_eq!(plain("1) one\n10. ten\n", 80), "  1) one\n  10. ten\n");
}

#[test]
fn a_destination_too_long_for_the_line_is_broken_rather_than_overrun() {
    // A word is never broken and a URL has to be: it is one unbreakable run that no terminal is
    // wide enough for, and the alternative to breaking it is a line that runs off the screen.
    let out = plain(
        "see [x](aaaaaaaaaabbbbbbbbbbccccccccccdddddddddd) end\n",
        20,
    );
    assert_eq!(
        out, "  see x\n  aaaaaaaaaabbb\n  bbbbbbbccccccccccd\n  ddddddddd end\n",
        "{out:?}"
    );
}

#[test]
fn a_quoted_code_line_keeps_its_own_indentation() {
    // Inside a fence the quote markers come off and nothing else does: a code line's own leading
    // spaces are its meaning. The classifier's stripping loop eats indentation after a marker,
    // which is right for a quoted paragraph and would be wrong here.
    assert_eq!(
        plain("> ```text\n>     indented\n> ```\n", 40),
        "    |     indented\n"
    );
}

// ---- milestone 280, second pass ------------------------------------------------------------
//
// The first pass closed the block constructs. These are the inline scanner and the four helpers
// under it, which is where the survivors concentrated once the tables and the rules had tests:
// three inline forms with no test at all, and a set of bounds that only an input at the edge tells
// apart from the bound next to it.

#[test]
fn strikethrough_is_rendered_and_needs_both_of_its_tildes() {
    // `~~` had no test of any kind, so the whole arm was unasserted: nine mutants lived in it,
    // including one that made every character *not* a tilde open a strike run.
    let out = render(
        "a ~~gone~~ b\n",
        Style {
            width: 40,
            color: true,
        },
    );
    assert!(out.contains("\x1b[9mgone"), "{out:?}");
    assert_eq!(plain("a ~~gone~~ b\n", 40), "  a gone b\n");
    // One tilde is a tilde. The corpus writes `~~` struck-through prose and `~` in paths and
    // regular expressions, and the difference has to be the count.
    assert_eq!(plain("a ~gone~ b\n", 40), "  a ~gone~ b\n");
}

#[test]
fn an_image_shows_what_it_is_and_where_it_points() {
    // A terminal cannot draw a picture, so the destination is the only thing a reader can act on:
    // it is the file they would open on the host. This was found in 2026-09-02 by the corpus
    // subsequence check, on the first page under `notes/` to carry an image, and then had no test
    // of its own; twelve mutants sat in the arm.
    assert_eq!(
        plain("![a picture](notes/x.png) after\n", 60),
        "  [image: a picture] notes/x.png after\n"
    );
    // The `!` only means an image when a `[` follows it immediately, and a `]` only opens a
    // destination when a `(` follows it immediately. Neither is an image, and both are ordinary
    // enough in prose that reading them as one would misrender.
    assert_eq!(plain("!x[a](y.png)\n", 40), "  !xa y.png\n");
    assert_eq!(plain("![a] (x.png)\n", 40), "  ![a] (x.png)\n");
}

#[test]
fn inline_markup_stops_descending_at_the_depth_bound() {
    // `MAX_DEPTH` is a bound rather than a guess: this renders in a process with one 4 KiB stack
    // page, so unbounded recursion on adversarial input is a fault and not a slow render. Past the
    // bound the markup is emitted literally, which is the visible half of that promise and had no
    // test. Four levels here: strong, emphasis, strike, and one more that is not taken.
    assert_eq!(
        plain("**a *b ~~c *d* e~~ f* g**\n", 60),
        "  a b ~~c *d e~~ f* g\n"
    );
}

#[test]
fn a_closer_preceded_by_a_space_is_not_one() {
    // The rule that stops a stray `*` mid-sentence swallowing the rest of a paragraph, tested on
    // both delimiter lengths because they are separate code paths, and on the case where a run
    // begins with its own delimiter and has no closer at all.
    assert_eq!(plain("*a * b*\n", 40), "  a * b\n");
    assert_eq!(plain("a **b ** c**\n", 40), "  a b ** c\n");
    assert_eq!(plain("a ****b\n", 40), "  a ****b\n");
}

#[test]
fn a_line_of_marks_alone_is_not_a_heading_or_a_list() {
    // Three classifiers index one byte past a run they have just counted, and each is guarded by a
    // length test that only a line consisting of nothing but that run can tell from the test beside
    // it. All three are ordinary lines in this repository's fenced blocks.
    assert_eq!(plain("###\n", 40), "  ###\n");
    assert_eq!(plain("#\n", 40), "  #\n");
    assert_eq!(plain("123\n", 40), "  123\n");
}

#[test]
fn an_ordered_marker_is_digits_then_a_dot_or_a_bracket_then_a_space() {
    // Each clause on its own: no digits at all, digits followed by something else, and the marker
    // length, which only shows where a wrapped item's continuation lines align.
    assert_eq!(plain(". text\n", 40), "  . text\n");
    assert_eq!(plain("1a text\n", 40), "  1a text\n");
    assert_eq!(
        plain("1. a very long ordered item that wraps around\n", 24),
        "  1. a very long ordered\n     item that wraps\n     around\n"
    );
}

#[test]
fn an_indent_is_kept_outside_a_block_quote_and_dropped_inside_one() {
    // Two facts in one `if`, and the earlier tab test could not separate them because it compared
    // two indented renderings against each other: both sides move together when the indent is lost
    // altogether. These are absolute.
    assert_eq!(
        plain("para\n\n    more text\n", 40),
        "  para\n\n      more text\n"
    );
    // A quoted paragraph's own indentation means nothing: the rule is its structure, and indenting
    // past it would claim a nesting the author did not write.
    assert_eq!(
        plain(">     quoted indented\n", 40),
        "  | quoted indented\n"
    );
}

#[test]
fn a_quoted_fence_takes_the_markers_it_was_opened_with_and_no_more() {
    // `past_quote` steps over at most the depth the fence opened at. A line inside the fence with
    // FEWER markers is a lazy continuation and is taken as it stands, marker and all, which is this
    // crate's recorded limitation rather than a guess at which reading the author meant.
    assert_eq!(
        plain(">> ```text\n>>   deep\n>> ```\n", 40),
        "    | |   deep\n"
    );
    assert_eq!(plain(">> ```text\n> one\n>> ```\n", 40), "    | | one\n");
    assert_eq!(
        plain("> ```text\n>   two\nlazy\n> ```\n", 40),
        "    |   two\n    | lazy\n"
    );
}

#[test]
fn a_wide_character_is_one_column_of_the_table_it_sits_in() {
    // Width is counted in characters and not bytes, so a two-byte character occupies one column and
    // a column sized in bytes would be one too wide. The BUGS section is honest that a CJK
    // character then occupies two columns and is counted as one; this is the Latin case, which is
    // the one the corpus has.
    assert_eq!(
        plain("| é | bb |\n|---|---|\n| x | y |\n", 80),
        "  é | bb\n  --+---\n  x | y \n"
    );
}

#[test]
fn a_backslash_that_is_not_escaping_a_pipe_is_ordinary_cell_text() {
    // The escape scan reads the byte after the backslash, and the byte before nothing: a cell that
    // opens with a backslash and a cell that ends with one are the two edges of it. Both are real,
    // because this repository's tables carry paths and regular expressions.
    assert_eq!(
        plain("| \\x | b |\n|---|---|\n| p | q |\n", 40),
        "  \\x | b\n  ---+--\n  p  | q\n"
    );
    assert_eq!(
        plain("| a\\ | b |\n|---|---|\n| x | y |\n", 40),
        "  a\\ | b\n  ---+--\n  x  | y\n"
    );
}

#[test]
fn a_table_inside_a_block_quote_fits_inside_the_rule() {
    // The room a table has is the terminal less the margin AND less the quote rules drawn down the
    // left of every row, which is two columns per level. A table sized against the bare terminal
    // would overrun by exactly that, and only a quoted table wide enough to be shrunk shows it.
    let quoted = plain(
        "> | aaaaaaaaaa | bb |\n> |---|---|\n> | cccccccccccccccccccc | d |\n",
        24,
    );
    let bare = plain(
        "| aaaaaaaaaa | bb |\n|---|---|\n| cccccccccccccccccccc | d |\n",
        24,
    );
    for line in quoted.lines() {
        assert!(line.chars().count() <= 24, "a quoted row overran: {line:?}");
    }
    assert!(
        quoted.contains("ccccccccccccccc |") && !quoted.contains("cccccccccccccccc |"),
        "{quoted:?}"
    );
    // And the bare table gets the two columns back, which is the other side of the same sum.
    assert!(bare.contains("ccccccccccccccccc |"), "{bare:?}");
}

#[test]
fn a_table_is_one_chunk_until_its_text_will_not_fit() {
    // The spill test above fills the ROW counter; this fills the TEXT arena, which is the other
    // half of the same `if` and the half that decides widths. Two rows, well inside `TABLE_ROWS`,
    // whose cells are long: they are one chunk, so the short row is padded to the long one's width
    // and every output line is the same length. A flush between them would align each to itself.
    let src = format!(
        "| {} | b |\n|---|---|\n| {} | c |\n",
        "w".repeat(100),
        "W".repeat(150)
    );
    let out = plain(&src, 4000);
    let widths: Vec<usize> = out.lines().map(str::len).collect();
    assert_eq!(widths.len(), 3);
    assert!(
        widths.iter().all(|&w| w == widths[0]),
        "not one chunk: {widths:?}"
    );
}

// ---- milestone 280, third pass: the output cursor ------------------------------------------
//
// `col` is the renderer's only piece of arithmetic with two consumers, and almost every survivor
// left in it is a mutant of one accumulator. Where the cursor feeds nothing but `close_line`'s
// "is a line open" test, a wrong value is genuinely invisible and the mutant is equivalent; where
// it feeds a wrap decision, or where it can reach zero, the output moves. These are the inputs that
// tell those two cases apart, and each one is an ordinary page rather than a contrivance.

#[test]
fn a_blank_line_inside_a_fence_is_a_blank_line() {
    // The one code line whose visible width is zero, which is what separates "add nothing to the
    // cursor" from "multiply the cursor by nothing": the second closes the line without a newline
    // and joins the code to whatever follows. Most fenced blocks in this repository have one.
    assert_eq!(
        plain("```text\none\n\ntwo\n```\n", 40),
        "    one\n    \n    two\n"
    );
}

#[test]
fn an_image_leaves_the_cursor_where_its_text_ended() {
    // `[image:` and `]` are the only runs this renderer emits that are not in the line buffer, and
    // the cursor has to move by their width like any other word, or the wrap that follows lands in
    // the wrong place. Nothing after an image had ever wrapped in a test.
    assert_eq!(
        plain("![pic](d.png) then some more words here\n", 24),
        "  [image: pic] d.png\n  then some more words\n  here\n"
    );
}

#[test]
fn a_quoted_paragraph_wraps_inside_its_rule() {
    // Each level of block quote costs two columns of every line, and the cursor has to start past
    // them: a paragraph wrapped against the bare margin overruns by exactly two per level. Two
    // levels, because one level cannot tell an addition from the constant it adds.
    assert_eq!(
        plain("> one two three four five six seven\n", 20),
        "  | one two three\n  | four five six\n  | seven\n"
    );
    assert_eq!(
        plain(">> one two three four five six seven\n", 20),
        "  | | one two three\n  | | four five six\n  | | seven\n"
    );
}

#[test]
fn a_carriage_return_at_end_of_line_is_not_content() {
    // A markdown file that has been through an editor on another system arrives with CRLF, and the
    // `\r` is framing rather than text. Nothing in this repository has one, so only an explicit
    // test can hold it: a renderer that kept it would print a stray control character, and one that
    // trimmed the wrong byte would eat the last character of every line.
    assert_eq!(plain("a\r\nb\r\n", 40), "  a b\n");
    assert_eq!(plain("# Title\r\n", 40), "TITLE\n");
}

#[test]
fn an_indented_block_quote_indents_by_its_rule_and_not_by_its_spaces() {
    // A quoted line's own indentation is not structure, so the rule replaces it. The earlier
    // indent tests both start at column zero, where the indent being dropped and the indent being
    // zero look the same.
    assert_eq!(
        plain("  >   quoted indented\n", 40),
        "  | quoted indented\n"
    );
}

#[test]
fn a_table_of_twelve_columns_keeps_every_character() {
    // The minimal reproduction of the 2026-09-23 red `main`, reduced from `notes/rented-metal.md`:
    // twelve columns against a `TABLE_COLS` of 8. The last four used to be dropped in silence and
    // `every_character_survives` found it an hour after the page landed. They are folded into the
    // last column now, which is ugly and lossless, and lossless is the property that matters.
    let src = "| a | b | c | d | e | f | g | h | i | j | k | l |\n\
               |---|---|---|---|---|---|---|---|---|---|---|---|\n\
               | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 0 | y | z |\n";
    let out = plain(src, 200);
    for want in ["h | i | j | k | l", "8 | 9 | 0 | y | z"] {
        assert!(out.contains(want), "{want:?} was dropped: {out:?}");
    }
}

#[test]
fn the_fold_leaves_no_trailing_pipe_of_its_own() {
    // The fold takes the rest of the line verbatim, so the row's own closing pipe would otherwise
    // become part of the last cell's text and every folded table would end in a stray bar.
    let src = "| a | b | c | d | e | f | g | h | i |\n\
               |---|---|---|---|---|---|---|---|---|\n\
               | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 |\n";
    let out = plain(src, 200);
    for line in out.lines() {
        assert!(
            !line.trim_end().ends_with('|'),
            "the row's closing pipe survived the fold: {line:?}"
        );
    }
}

#[test]
fn a_row_with_fewer_cells_than_the_table_is_padded_rather_than_ragged() {
    // Markdown in this repository is written by hand and a short row is ordinary. The missing cells
    // are blanks of the column's width, so the rows below still line up under their headings.
    assert_eq!(
        plain("| a | b | c |\n|---|---|---|\n| x |\n| p | q | r |\n", 40),
        "  a | b | c\n  --+---+--\n  x |   |  \n  p | q | r\n"
    );
}

// ---- milestone 280, fourth pass: the cases where a wrong answer looks right on one line --------
//
// Three classifiers can be wrong about what a line IS and still emit the same bytes for it, because
// the marker they invent is made of the same characters they would otherwise have printed as text.
// The difference is the margin a continuation aligns to, so it takes two lines to see.

#[test]
fn a_line_that_is_not_a_list_does_not_hang_like_one() {
    // `. text` has no digits before its dot and `1a text` has no dot after its digits: neither is an
    // ordered marker, and a classifier that accepted either would print the same first line and then
    // indent everything after it under a bullet nobody wrote.
    assert_eq!(
        plain(". one two three four five six\n", 20),
        "  . one two three\n  four five six\n"
    );
    assert_eq!(
        plain("1a one two three four five six\n", 20),
        "  1a one two three\n  four five six\n"
    );
}

#[test]
fn a_blank_line_inside_a_quoted_fence_is_not_read_past_its_marker() {
    // The commonest line in a quoted transcript: a `>` with nothing after it. The marker walk has
    // three length tests and each one indexes the byte after what it just consumed, so this is the
    // line that separates them from the line buffer behind them. Both depths, because one level
    // cannot tell a bound from the loop that stops at it.
    assert_eq!(
        plain("> ```text\n>\n> after\n> ```\n", 40),
        "    | \n    | after\n"
    );
    assert_eq!(
        plain(">> ```text\n>>\n>> after\n>> ```\n", 40),
        "    | | \n    | | after\n"
    );
}

#[test]
fn a_code_span_hands_back_the_line_past_its_closing_backtick() {
    // The existing code-span test asks what is inside the span; this asks where the scanner resumes.
    // One byte early and the closing backtick is emitted as the first character of the next word,
    // which a `contains` check cannot see.
    assert_eq!(plain("a `code` b\n", 40), "  a code b\n");
}

// ---- survivor triage of the 2026-10-03 census: the last column's fold, and the table arena's edges -----------------------
//
// The fold that keeps a ninth cell from being dropped (see `a_table_of_twelve_columns_keeps_every_
// character`) strips trailing blanks, then one closing pipe unless a backslash escapes it. Each of
// those three steps was tested only on a clean row.

const NINE: &str = "|-|-|-|-|-|-|-|-|-|\n";

#[test]
fn a_folded_row_with_trailing_blanks_still_loses_its_closing_pipe() {
    // Blanks after the closing pipe are part of the line, so the pipe is not the last byte. A fold
    // that stops looking at the end of the line prints it, and a tab is blank too.
    let src = format!(
        "| a | b | c | d | e | f | g | h | i |\n{NINE}| 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | \t \n"
    );
    let out = plain(&src, 200);
    assert_eq!(
        out.lines().last().unwrap(),
        "  1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9"
    );
}

#[test]
fn a_folded_row_keeps_an_escaped_closing_pipe_as_text() {
    // `\|` is a pipe in the cell, not the row's end, and the backslash is not text. The last cell
    // here is `i |`, so the pipe stays and the row has no closing pipe of its own.
    let src = format!(
        "| a | b | c | d | e | f | g | h | i |\n{NINE}| 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 \\|\n"
    );
    let out = plain(&src, 200);
    assert_eq!(
        out.lines().last().unwrap(),
        "  1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 |"
    );
}

#[test]
fn a_cell_that_ends_in_a_backslash_is_not_read_past_its_end() {
    // The escape check looks one byte ahead of the backslash. At the end of a cell there is nothing
    // there, and the lookahead has to say so rather than read the next cell's pipe or the buffer.
    assert_eq!(
        plain("| a\\ | b |\n|---|---|\n| x\\ | y\\ |\n", 40),
        "  a\\ | b \n  ---+---\n  x\\ | y\\\n"
    );
}

#[test]
fn a_row_that_exactly_fills_the_text_arena_is_still_one_chunk() {
    // The arena holds `TABLE_TEXT` bytes of cell text and a row is admitted while `used + row <=
    // TABLE_TEXT`; the row that lands on the limit exactly belongs to the chunk, and the one after
    // it does not. Four rows of 2,046 bytes (a 2,048-byte line is the longest the renderer keeps)
    // leave 8 bytes, which is what the indented last row measures to. One byte more and it is a
    // chunk of its own, so the two cases differ by whether every line is one width.
    let long = format!("|{}|\n", "a".repeat(2046));
    let delim = "|-|\n";
    let head = format!("{long}{delim}{long}{long}{long}");
    let exact = plain(&format!("{head}  |bbbbbb|\n"), 10_000);
    let widths: Vec<usize> = exact.lines().map(str::len).collect();
    assert_eq!(widths.len(), 6, "four rows, a rule and the last row");
    assert!(
        widths.iter().all(|&w| w == widths[0]),
        "the exact fit was split: {widths:?}"
    );
    let over = plain(&format!("{head}  |bbbbbbb|\n"), 10_000);
    let widths: Vec<usize> = over.lines().map(str::len).collect();
    assert_ne!(
        widths.last(),
        widths.first(),
        "a row one byte over the arena stayed in the chunk"
    );
}

// ---- survivor triage of the 2026-10-03 census: the inline scanner's depth bound and its last-byte reads --------------------
//
// The scanner reads one byte ahead of a marker, and the line buffer behind the live line holds the
// previous, longer line. A read one byte too far therefore does not fail on its own; it fails when
// the leftover byte happens to be the one being looked for. These put the right leftovers there.

fn stale(first: &str, second: &str, width: u16) -> String {
    plain(&format!("{first}\n\n{second}\n"), width)
}

#[test]
fn nesting_stops_at_the_depth_bound_whichever_marker_opens_each_level() {
    // Three levels deep, each opened by a different marker (strong, strike, link), so every site
    // that deepens the scan is on the path. The fourth level is literal: markup there is text.
    let emph = plain("**a ~~b [c *z* d](u) b~~ a**\n", 80);
    assert_eq!(emph, "  a b c *z* d u b a\n");
    let strike = plain("**a *b [c ~~z~~ d](u) b* a**\n", 80);
    assert_eq!(strike, "  a b c ~~z~~ d u b a\n");
    let link = plain("**a ~~b *c [x](u) c* b~~ a**\n", 80);
    assert_eq!(link, "  a b c [x](u) c b a\n");
}

#[test]
fn a_single_tilde_does_not_open_a_strike() {
    // Two tildes open one. The second is read at `i + 1`, and a scanner that read it at `i` would
    // see the first tilde twice and strike through everything up to the next pair.
    assert_eq!(plain("~a ~~b~~\n", 40), "  ~a b\n");
}

#[test]
fn a_strike_closer_is_looked_for_after_the_opener() {
    // The search starts two bytes past the opening pair. Started anywhere else it misses the
    // closer, or finds one that belongs to a later span, so the opener is placed away from byte 0
    // and byte 2 where the wrong starting points coincide with it.
    assert_eq!(plain("xxxxx~~ab~~ y\n", 40), "  xxxxxab y\n");
}

#[test]
fn a_link_label_at_the_end_of_a_line_does_not_borrow_a_paren_from_the_last_one() {
    // `[x]` ends the line, and the byte after it is the `(` the previous line left behind. Taking
    // it as the start of a destination looks for a `)` before the line's start and cannot find
    // the slice it asks for.
    assert_eq!(stale("[x](yy)", "[x]", 40), "  x yy\n\n  [x]\n");
}

#[test]
fn a_heading_with_two_spaces_after_its_hashes_has_no_leading_blank() {
    // The blank is skipped as whitespace and leaves the "a space is owed" flag set. A margin of zero
    // is where that flag and the cursor agree, so the first word must not pay it.
    assert_eq!(plain("#  Title\n", 40), "TITLE\n");
}

#[test]
fn an_image_marker_moves_the_cursor_by_its_own_width() {
    // `[image:` is written from outside the line buffer, so the cursor has to be advanced by hand.
    // `[image: pic]` is 12 columns from a margin of 2 and `d.png` brings the line to 20. Any other
    // width puts that word on one side of the wrap or the other, so a width of 21 is the edge.
    assert_eq!(
        plain("![pic](d.png) x\n", 21),
        "  [image: pic] d.png\n  x\n"
    );
}

#[test]
fn a_long_destination_is_wrapped_on_character_boundaries() {
    // The wrap steps one character at a time by asking how long the character at the cursor is. A
    // two-byte character asked about the wrong byte is split in half and the output stops being text.
    let out = plain("[a](éééééééééééééé)\n", 12);
    assert_eq!(out, "  a\n  ééééééééé\n  ééééé\n");
}

#[test]
fn a_quoted_fence_keeps_a_marker_of_its_own() {
    // A fence opened one quote deep strips one marker from each line. A second `>` on a line is
    // the code's, as in a quoted mail inside a quoted block.
    assert_eq!(plain("> ```text\n> > keep\n> ```\n", 40), "    | > keep\n");
}

#[test]
fn a_line_with_fewer_markers_than_the_fence_was_opened_at_is_not_over_read() {
    // The fence opened two deep and this line has one marker. The walk past the markers has to stop
    // at the end of the line, not index it.
    let out = plain(">> ```text\n>\n>> x\n>> ```\n", 40);
    assert!(out.contains('x'), "{out:?}");
}
