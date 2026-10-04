# helpers/lint-no-cargo.awk: split script/lint into its sections and keep the ones that never run cargo.
#
# `script/lint --no-cargo` (the pre-push hook's mode) runs this over script/lint itself, so the
# partition is derived from the sections and not from a second list that can lag them. A section
# starts at a column-0 `echo "==> ` line and runs to the next one; whatever precedes the first is
# the preamble and is always kept.
#
# A section is a cargo section when a line that is not a comment invokes cargo: a shell command
# whose first word is `cargo` (after optional VAR="..." assignments and `if !`), a Python argv list
# opening `["cargo"`, or `command -v cargo`. Prose that merely mentions cargo does not match. A
# cargo section is dropped from `--no-cargo`. Because the rule reads the section, a new check lands
# in exactly one bucket by construction, and one that runs cargo is dropped unless it opts in.
#
# **Opting in** is for a section that is mostly text and has one cargo-backed part: put a comment
# line `# no-cargo-ok: <why>` in it, and guard the cargo part on `$LINT_NO_CARGO`, which the mode
# exports. The counted-claims section does this for its harness-count cross-check. Two failures keep
# the marker honest: a marker on a section that runs no cargo, and a marker whose section never
# mentions LINT_NO_CARGO. Cargo in the preamble fails too, since the preamble cannot be dropped.
#
# Usage: awk -v mode=emit|list -f helpers/lint-no-cargo.awk script/lint
#   emit  prints the script to run (preamble plus the kept sections).
#   list  prints one `run`/`skip` line per section, for a person asking what the mode covers.

function is_cargo(line) {
    if (line ~ /^[ \t]*#/) return 0
    if (line ~ /^[ \t]*([A-Za-z_]+="[^"]*" )*(if ! |! )?cargo[ \t]/) return 1
    if (line ~ /\[[\x27"]cargo[\x27"]/) return 1
    if (line ~ /command -v cargo/) return 1
    return 0
}

function close_block(   keep) {
    if (name == "") {
        if (cargo) { print "lint --no-cargo: the preamble of script/lint invokes cargo, so no section split can drop it" > "/dev/stderr"; bad = 1 }
        out = out body
        return
    }
    if (optin && !cargo) { print "lint --no-cargo: \"" name "\" carries a no-cargo-ok marker but runs no cargo; delete the marker" > "/dev/stderr"; bad = 1 }
    if (optin && !guarded) { print "lint --no-cargo: \"" name "\" carries a no-cargo-ok marker but never reads LINT_NO_CARGO, so it would run cargo in the mode that forbids it" > "/dev/stderr"; bad = 1 }
    keep = (!cargo || optin)
    if (mode == "list") print (keep ? "run   " : "skip  ") name
    else if (keep) out = out body
}

/^echo "==> / {
    close_block()
    name = substr($0, 7, length($0) - 7); body = $0 "\n"
    cargo = 0; optin = 0; guarded = 0
    next
}
{
    body = body $0 "\n"
    if (is_cargo($0)) cargo = 1
    if ($0 ~ /^[ \t]*#[ \t]*no-cargo-ok:/) optin = 1
    if ($0 !~ /^[ \t]*#/ && $0 ~ /LINT_NO_CARGO/) guarded = 1
}
END {
    close_block()
    if (bad) exit 1
    if (mode != "list") printf "%s", out
}
