#!/usr/bin/env python3
"""The diff-reading half of `needs-architect` (§88): a predicate over a unified diff.

§88 (`needs-architect` as a required check, rather than as a script's restraint) built the
enforcement half -- `architect-hold.yml` fails while the label is present -- and named plainly what
it did not build: *"a check that reads the diff and demands the label when the syscall surface or
the dependency graph moved... the second is the real answer."* `.github/workflows/coe-architect-
label.yml` built that answer for one shape, a correction of error. This is the general one, wired
by `.github/workflows/architect-label.yml`.

This module is a pure predicate: it reads a unified diff (as `git diff --unified=1000000 base head`
produces, so every file's diff is one hunk holding the WHOLE file, changed lines marked) and reports
which of five rules fired, and where. It never runs `git` and never talks to GitHub; the workflow
does both and pipes the diff text in on stdin. That split is what makes `--selftest` below run
against literal fixture strings, no repository and no subprocess required, cheap enough for
`script/lint` to run on every invocation.

    python3 helpers/architect-label-rules.py --selftest        # fixtures; script/lint runs this
    git diff --unified=1000000 BASE HEAD | python3 helpers/architect-label-rules.py
        # prints one "rule: file: line" per match on stdout, exits 0 if anything fired, 1 if
        # nothing did, 2 on a usage error. Nothing here reads BASE/HEAD; a caller diffs first.

# The five rules (the brief's wording; this file is the one place that has to agree with it)

1. **abi-surface**: an object type, method number or syscall number moved under `crates/abi/`.
   Detected as any changed `pub const NAME: <int type> = ...` line under that path. Broad on
   purpose: `crates/abi/src/lib.rs` also declares rights bitflags and the register convention's
   constants the same way, so this also fires on those (see BUGS). A false positive there costs a
   human one look; a syscall number landing unlabelled costs more.
2. **dependency**: an external dependency entered a `Cargo.toml`'s dependency graph. Computed as a
   set difference (external deps at head minus external deps at base) per file, not a line match,
   so a bare version bump of an already-external dependency does NOT fire and a `path = ` entry
   never counts (see BUGS for what "external" cannot see: a multi-line inline table).
3. **format-crate**: a version constant, magic value, or documented on-disk layout moved in a crate
   that is classified, from its own head content, as a format crate: it declares a `pub const`
   ending in `VERSION` or `MAGIC`, or its module doc carries a markdown table with a `version`
   column (the two tells the brief names). Firing then also covers any changed `//!` line in that
   same file, which is where a layout diagram lives (`crates/nifefs`'s "# The layout" is the model).
4. **spawnproto**: a `pub const` changed in a file named `spawnproto.rs` (the wire layout
   `crates/grant_plan/src/spawnproto.rs` documents; matched by filename rather than the one path in
   the tree today, so a second one elsewhere is still caught).
5. **decisions**: `CLAUDE.md` changed at all, or a `design/decisions/*.md` file (excluding the
   generated `README.md`) changed OUTSIDE its YAML frontmatter block (the first `---`-delimited
   section). A frontmatter-only edit (a status flip, a date) is provenance, not the decision's
   substance, and does not fire; see the frontmatter shape in any decision file's first lines.

# BUGS

- **Rule 2 assumes a dependency's TOML value is one line.** Every `Cargo.toml` in this tree writes
  dependencies as `name = "..."`, `name = { version = "...", path = "..." }`, or similar, entirely
  on one line (checked directly: no continuation line exists in this tree's own manifests today).
  A hand-wrapped multi-line inline table would be misread: the first line (no `path =` on it yet)
  reads as external, and the continuation's own `path = "..."`  line would itself parse as a bogus
  dependency literally named `path`. Both failures are false positives, never a missed real one,
  which is the direction this tool is allowed to be wrong in.
- **Rule 1 does not distinguish an object type from a rights bitflag or a register-convention
  constant.** All three are `pub const NAME: uNN = ...` in the same file. Narrowing this to the
  `objtype`/method-number submodules specifically would need to track which `pub mod` block a line
  falls in, which the diff's context lines make possible but this pass does not do; the cost today
  is a wider true-positive rate on `crates/abi/`, not a missed one.
- **A renamed file (`git diff -M`) is read as delete+add or as a content-only rename diff depending
  on git's own detection**, and either shape is parsed as an ordinary file if it carries a
  `diff --git a/... b/...` header with hunks; a pure rename with no content change carries no hunk
  at all and this tool has nothing to look at, which is correct (nothing moved that these rules
  care about).

Name: provisional, minted by this lane, 2026-09-27. Hyphenated per design/naming.md's `helpers/`
entry-point rule even though it ends in `.py`, matching `helpers/handoff-check.py`'s precedent: it
is a standalone CLI, not a module another script imports.
"""
import re
import sys

# ---- parsing a `git diff --unified=<huge>` into one whole-file view per path -------------------
#
# With a big enough -U, git merges every hunk in a file into one, from the first changed line back
# to the top of the file and forward to the bottom, PROVIDED no unrelated content sits between two
# hunks past the context window; -U1000000 exceeds every file's line count in this tree, so in
# practice each file's diff is exactly one hunk holding the whole file, in order, each line tagged
# ' ' (context, i.e. unchanged), '+' (added) or '-' (removed). That is what lets `head_lines` /
# `base_lines` below reconstruct each side's full content by keeping/dropping only the other side's
# lines, with no separate `git show` call.

FILE_RE = re.compile(r'^diff --git a/(.+?) b/(.+)$')


def parse_diff(text):
    """A unified diff -> [{'path': str, 'lines': [(prefix, content), ...]}, ...].

    `prefix` is ' ', '+' or '-'. Extended headers (`index ...`, `new file mode ...`, `rename from
    ...`, `Binary files ... differ`) start with none of ' +-' and are silently dropped; so are the
    `--- a/x` / `+++ b/x` file markers and `@@ ... @@` hunk markers, since the file path already
    came from the `diff --git` line and this tool never needs a hunk's line numbers.
    """
    files = []
    cur = None
    for raw in text.splitlines():
        m = FILE_RE.match(raw)
        if m:
            if cur is not None:
                files.append(cur)
            # The b/ path names where the file ends up; a pure deletion still has SOME b/ path in
            # git's own header (historically `/dev/null` is only the +++ line, not this one), and
            # nothing here acts on a file with no b/ path anyway (no rule fires on a deleted file).
            cur = {'path': m.group(2), 'lines': []}
            continue
        if cur is None:
            continue
        if raw.startswith('--- ') or raw.startswith('+++ ') or raw.startswith('@@'):
            continue
        if raw[:1] in ('+', '-', ' '):
            cur['lines'].append((raw[0], raw[1:]))
        # else: extended header or "\ No newline at end of file"; not a content line, drop it.
    if cur is not None:
        files.append(cur)
    return files


def head_lines(fd):
    """This file's content at the diff's post-image (context + added, removed dropped)."""
    return [line for prefix, line in fd['lines'] if prefix != '-']


def base_lines(fd):
    """This file's content at the diff's pre-image (context + removed, added dropped)."""
    return [line for prefix, line in fd['lines'] if prefix != '+']


def changed_lines(fd):
    """(prefix, line) pairs for every added or removed line, in file order."""
    return [(p, line) for p, line in fd['lines'] if p in ('+', '-')]


# ---- rule 1: crates/abi -------------------------------------------------------------------------

ABI_CONST_RE = re.compile(
    r'^\s*pub const [A-Za-z_][A-Za-z0-9_]*\s*:\s*(u8|u16|u32|u64|usize|i8|i16|i32|i64|isize)\b')


def rule_abi_surface(fd, out):
    if not (fd['path'].startswith('crates/abi/') and fd['path'].endswith('.rs')):
        return
    for _prefix, line in changed_lines(fd):
        if ABI_CONST_RE.match(line):
            out.append(('abi-surface', fd['path'], line.strip()))
            return  # one citation per file is enough for a label; the comment names the file


# ---- rule 2: an external dependency entered a Cargo.toml -----------------------------------------

SECTION_RE = re.compile(r'^\[(.+)\]$')
DEP_KEY_RE = re.compile(r'^([A-Za-z0-9_.-]+)\s*=\s*(.*)$')


def toml_external_deps(lines):
    """{dep_name} present in a dependency section, on one line, with no `path = ` on that line.

    See the module docstring's BUGS entry for what a multi-line inline table does to this.
    """
    external = set()
    in_deps = False
    for raw in lines:
        stripped = raw.strip()
        m = SECTION_RE.match(stripped)
        if m:
            in_deps = m.group(1).endswith('dependencies')
            continue
        if not in_deps or not stripped or stripped.startswith('#'):
            continue
        km = DEP_KEY_RE.match(stripped)
        if not km:
            continue
        name, rest = km.group(1), km.group(2)
        if 'path' in rest:
            continue  # `path = "../crates/x"` somewhere in the value: an in-tree dependency
        external.add(name)
    return external


def rule_dependency(fd, out):
    if not fd['path'].endswith('Cargo.toml'):
        return
    added = toml_external_deps(head_lines(fd)) - toml_external_deps(base_lines(fd))
    for name in sorted(added):
        out.append(('dependency', fd['path'], name))


# ---- rule 3: a format crate's version, magic or documented layout --------------------------------

VERSION_OR_MAGIC_CONST_RE = re.compile(r'^\s*pub const \w*(VERSION|MAGIC)\w*\s*:')
VERSION_TABLE_RE = re.compile(r'^\s*//!.*\|\s*version\s*\|', re.IGNORECASE)


def is_format_crate_source(lines):
    for line in lines:
        if VERSION_OR_MAGIC_CONST_RE.match(line) or VERSION_TABLE_RE.match(line):
            return True
    return False


def rule_format_crate(fd, out):
    if not (fd['path'].startswith('crates/') and '/src/' in fd['path']
            and fd['path'].endswith('.rs')):
        return
    if not is_format_crate_source(head_lines(fd)):
        return
    for _prefix, line in changed_lines(fd):
        if VERSION_OR_MAGIC_CONST_RE.match(line) or line.strip().startswith('//!'):
            out.append(('format-crate', fd['path'], line.strip()))
            return


# ---- rule 4: spawnproto ---------------------------------------------------------------------------

SPAWNPROTO_CONST_RE = re.compile(r'^\s*pub const \w+\s*:')


def rule_spawnproto(fd, out):
    if not fd['path'].endswith('spawnproto.rs'):
        return
    for _prefix, line in changed_lines(fd):
        if SPAWNPROTO_CONST_RE.match(line):
            out.append(('spawnproto', fd['path'], line.strip()))
            return


# ---- rule 5: CLAUDE.md and design/decisions/*.md (outside frontmatter) ---------------------------

def rule_decisions(fd, out):
    path = fd['path']
    if path == 'CLAUDE.md':
        if changed_lines(fd):
            out.append(('decisions', path, '(CLAUDE.md edited)'))
        return
    if not (path.startswith('design/decisions/') and path.endswith('.md')):
        return
    if path == 'design/decisions/README.md':
        return  # the generated index; see the brief's exclusion
    dash_count = 0
    in_frontmatter = False
    for prefix, line in fd['lines']:
        if line.strip() == '---':
            dash_count += 1
            in_frontmatter = (dash_count == 1)
            continue
        if in_frontmatter:
            continue
        if prefix in ('+', '-') and line.strip():
            out.append(('decisions', path, line.strip()))
            return


RULES = (rule_abi_surface, rule_dependency, rule_format_crate, rule_spawnproto, rule_decisions)


def evaluate(diff_text):
    """The diff text -> [(rule, path, detail), ...], one entry per file per rule that fired."""
    out = []
    for fd in parse_diff(diff_text):
        for rule in RULES:
            rule(fd, out)
    return out


# ---- the selftest ---------------------------------------------------------------------------------
#
# Every fixture is a literal `diff --git` block, the shape `git diff --unified=1000000` produces
# (each fixture is deliberately small, well under any real file's line count, so the "one hunk holds
# the whole file" property the docstring argues for holds trivially here too). Each names the rule
# it is meant to prove, or `None` for a negative case that must stay quiet.

FIXTURES = [
    ("abi: a new object type constant fires", """\
diff --git a/crates/abi/src/lib.rs b/crates/abi/src/lib.rs
--- a/crates/abi/src/lib.rs
+++ b/crates/abi/src/lib.rs
@@ -1,4 +1,5 @@
 pub mod objtype {
     pub const RENDEZVOUS: u64 = 1;
+    pub const NOTIFICATION: u64 = 8;
     pub const ADDRESS_SPACE: u64 = 2;
 }
""", 'abi-surface'),

    ("abi: a doc comment change alone stays quiet", """\
diff --git a/crates/abi/src/lib.rs b/crates/abi/src/lib.rs
--- a/crates/abi/src/lib.rs
+++ b/crates/abi/src/lib.rs
@@ -1,3 +1,3 @@
-/// old wording
+/// new wording, no number moved
 pub const SYS_EXIT: u64 = 0;
""", None),

    ("abi: outside crates/abi, the same const shape stays quiet", """\
diff --git a/crates/other/src/lib.rs b/crates/other/src/lib.rs
--- a/crates/other/src/lib.rs
+++ b/crates/other/src/lib.rs
@@ -1,1 +1,2 @@
 pub const EXISTING: u64 = 1;
+pub const NEW_THING: u64 = 2;
""", None),

    ("dependency: a brand-new external crate fires", """\
diff --git a/entropy_backend/Cargo.toml b/entropy_backend/Cargo.toml
--- a/entropy_backend/Cargo.toml
+++ b/entropy_backend/Cargo.toml
@@ -1,3 +1,4 @@
 [dependencies]
 getrandom = "0.4"
+rustls-rustcrypto = "0.1"
""", 'dependency'),

    ("dependency: an in-tree path dependency stays quiet", """\
diff --git a/kernel/Cargo.toml b/kernel/Cargo.toml
--- a/kernel/Cargo.toml
+++ b/kernel/Cargo.toml
@@ -1,2 +1,3 @@
 [dependencies]
+calendar = { version = "0.1.0", path = "../crates/calendar" }
""", None),

    ("dependency: a version bump of an already-external dep stays quiet", """\
diff --git a/entropy_backend/Cargo.toml b/entropy_backend/Cargo.toml
--- a/entropy_backend/Cargo.toml
+++ b/entropy_backend/Cargo.toml
@@ -1,2 +1,2 @@
 [dependencies]
-getrandom = "0.4"
+getrandom = "0.5"
""", None),

    ("dependency: a target-specific dependency section is still read", """\
diff --git a/kernel/Cargo.toml b/kernel/Cargo.toml
--- a/kernel/Cargo.toml
+++ b/kernel/Cargo.toml
@@ -1,2 +1,3 @@
 [target.'cfg(target_arch = "aarch64")'.dependencies]
+aarch64-cpu = "9.0"
""", 'dependency'),

    ("format-crate: a MAGIC constant change in a classified crate fires", """\
diff --git a/crates/nifefs/src/lib.rs b/crates/nifefs/src/lib.rs
--- a/crates/nifefs/src/lib.rs
+++ b/crates/nifefs/src/lib.rs
@@ -1,2 +1,2 @@
 //! magic   "CRKR0002"   (8 bytes)
-pub const MAGIC: [u8; 8] = *b"CRKR0002";
+pub const MAGIC: [u8; 8] = *b"CRKR0003";
""", 'format-crate'),

    ("format-crate: a layout doc-comment edit in a classified crate fires even untouched const", """\
diff --git a/crates/nifefs/src/lib.rs b/crates/nifefs/src/lib.rs
--- a/crates/nifefs/src/lib.rs
+++ b/crates/nifefs/src/lib.rs
@@ -1,3 +1,4 @@
 //! # The layout
-//! blocks 0..DIR_BLOCKS   the superblock and directory
+//! blocks 0..DIR_BLOCKS+1 the superblock and directory
 pub const MAGIC: [u8; 8] = *b"CRKR0002";
""", 'format-crate'),

    ("format-crate: an ordinary crate with no VERSION/MAGIC/version-table stays quiet", """\
diff --git a/crates/glob/src/lib.rs b/crates/glob/src/lib.rs
--- a/crates/glob/src/lib.rs
+++ b/crates/glob/src/lib.rs
@@ -1,2 +1,2 @@
-//! matches a glob pattern
+//! matches a shell glob pattern
 pub fn matches(pattern: &str, name: &str) -> bool { true }
""", None),

    ("format-crate: detected by a doc version table instead of a VERSION/MAGIC const", """\
diff --git a/crates/manifest_note/src/lib.rs b/crates/manifest_note/src/lib.rs
--- a/crates/manifest_note/src/lib.rs
+++ b/crates/manifest_note/src/lib.rs
@@ -1,3 +1,3 @@
 //! | version | field |
-//! | 1       | a     |
+//! | 1       | a, b  |
 pub fn parse() {}
""", 'format-crate'),

    ("spawnproto: a changed slot constant fires", """\
diff --git a/crates/grant_plan/src/spawnproto.rs b/crates/grant_plan/src/spawnproto.rs
--- a/crates/grant_plan/src/spawnproto.rs
+++ b/crates/grant_plan/src/spawnproto.rs
@@ -1,1 +1,1 @@
-pub const RUN_UNVOUCHED_SLOT: u64 = 22;
+pub const RUN_UNVOUCHED_SLOT: u64 = 23;
""", 'spawnproto'),

    ("spawnproto: prose-only doc change in the same file stays quiet", """\
diff --git a/crates/grant_plan/src/spawnproto.rs b/crates/grant_plan/src/spawnproto.rs
--- a/crates/grant_plan/src/spawnproto.rs
+++ b/crates/grant_plan/src/spawnproto.rs
@@ -1,1 +1,1 @@
-//! old description
+//! clearer description, no constant moved
""", None),

    ("spawnproto: a const change in a same-named file elsewhere still fires", """\
diff --git a/crates/other/src/spawnproto.rs b/crates/other/src/spawnproto.rs
--- a/crates/other/src/spawnproto.rs
+++ b/crates/other/src/spawnproto.rs
@@ -1,1 +1,1 @@
-pub const SLOT: u64 = 1;
+pub const SLOT: u64 = 2;
""", 'spawnproto'),

    ("decisions: any CLAUDE.md edit fires", """\
diff --git a/CLAUDE.md b/CLAUDE.md
--- a/CLAUDE.md
+++ b/CLAUDE.md
@@ -1,1 +1,1 @@
-old sentence
+new sentence
""", 'decisions'),

    ("decisions: a substantive body edit to a decision file fires", """\
diff --git a/design/decisions/88-needs-architect-as-a-check.md b/design/decisions/88-needs-architect-as-a-check.md
--- a/design/decisions/88-needs-architect-as-a-check.md
+++ b/design/decisions/88-needs-architect-as-a-check.md
@@ -1,6 +1,6 @@
 ---
 status: DECIDED
 ---
-**What.** old text.
+**What.** new text, a different rule.
""", 'decisions'),

    ("decisions: a frontmatter-only status flip stays quiet", """\
diff --git a/design/decisions/88-needs-architect-as-a-check.md b/design/decisions/88-needs-architect-as-a-check.md
--- a/design/decisions/88-needs-architect-as-a-check.md
+++ b/design/decisions/88-needs-architect-as-a-check.md
@@ -1,4 +1,4 @@
 ---
-status: PROPOSED
+status: DECIDED
 ---
 **What.** unchanged text.
""", None),

    ("decisions: design/decisions/README.md is excluded even on a substantive edit", """\
diff --git a/design/decisions/README.md b/design/decisions/README.md
--- a/design/decisions/README.md
+++ b/design/decisions/README.md
@@ -1,1 +1,1 @@
-88. old title
+88. a completely different title
""", None),

    ("decisions: an unrelated design/ file stays quiet", """\
diff --git a/design/naming.md b/design/naming.md
--- a/design/naming.md
+++ b/design/naming.md
@@ -1,1 +1,1 @@
-old rule
+new rule
""", None),
]


def selftest():
    bad = []
    for name, diff_text, want_rule in FIXTURES:
        got = evaluate(diff_text)
        got_rules = {r for r, _p, _d in got}
        ok = (want_rule in got_rules) if want_rule else (not got_rules)
        print(f"  {'ok  ' if ok else 'FAIL'}  {name}")
        if not ok:
            bad.append(f"{name}: wanted {want_rule!r}, got {sorted(got_rules)!r}")
    print()
    if bad:
        for b in bad:
            print(f"architect-label-rules: selftest: {b}", file=sys.stderr)
        return 1
    print(f"architect-label-rules: {len(FIXTURES)} fixtures, every rule fires and every "
          "near-miss stays quiet")
    return 0


USAGE = (
    "usage: python3 helpers/architect-label-rules.py [--selftest]\n"
    "  --selftest   run the fixtures above; no stdin, no git, no repository\n"
    "  (no args)    read a unified diff on stdin, print each rule that fired, "
    "exit 0 if any did"
)


def main(argv):
    if argv and argv[0] == '--selftest':
        return selftest()
    if argv:
        print(USAGE, file=sys.stderr)
        return 2
    diff_text = sys.stdin.read()
    matches = evaluate(diff_text)
    for rule, path, detail in matches:
        print(f"{rule}: {path}: {detail}")
    return 0 if matches else 1


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
