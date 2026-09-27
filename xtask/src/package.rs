//! **`cargo xtask package`**: build a package from a reviewed recipe, and say what its digest is.
//!
//! Rung 3a of milestone 198 (a package manager, and the trivial install that makes a second
//! customer possible) has two halves, and this is the producer.
//! DECISIONS §195 (a reviewed recipe vouches for a package), in
//! [its own file](../../design/decisions/195-a-recipe-vouches-and-the-owner-may-overrule.md),
//! puts a package's digest in a version-controlled recipe changed by human review, which is
//! Homebrew's arrangement; this is the tool that turns such a recipe into the one archive file
//! DECISIONS §197 (a package is one archive file), in
//! [its own file](../../design/decisions/197-a-package-is-one-archive-file.md), rules a package
//! is.
//!
//! **nife cannot build software**, so a package is produced here, on a host with a cross-toolchain,
//! and consumed by a target with no compiler. That is why this lives in `xtask` and why a recipe
//! names an architecture: three triples, three packages, §197's "per-architecture output is the
//! normal case".
//!
//! # EXAMPLES
//!
//! ```text
//! $ cargo xtask package packages/uptime.recipe.toml
//! uptime 0.1.0 aarch64, 2 members, 4600 bytes
//!   uptime          4528 bytes  7d8e...c1
//!   uptime.licence    68 bytes  a3f0...9e
//! wrote target/packages/uptime-0.1.0-aarch64.nifepkg
//! digest 9f2c...4b  (recorded in the recipe, and it matches)
//! package: PASS
//! ```
//!
//! A recipe with no `digest` key builds and prints the digest to paste in; one whose `digest`
//! disagrees with the bytes fails, which is the whole mechanism: the reviewed line is what decides
//! whether the bytes may run, so a rebuild that does not reproduce it is a fact somebody must see.
//!
//! # The recipe format
//!
//! TOML (calef, 2026-09-27: package declarations and recipes both move to TOML; JSON was refused
//! for having no comments, and YAML too). The file extension `.recipe.toml` and every key name
//! are provisional, named by the lane of milestone 611 (every program and crate belongs to a
//! package).
//!
//! ```toml
//! name = "uptime"              # required, at most package_archive::NAME_LEN bytes
//! version = "0.1.0"            # required
//! architecture = "aarch64"     # required: aarch64, riscv64 or x86_64
//! digest = "9f2c..."           # optional: 64 hex characters, checked against what was built
//!
//! [[member]]                   # members in order: the package's bytes depend on it
//! program = "uptime"           # a built ELF for that architecture, resolved from target/
//!
//! [[member]]
//! name = "uptime.licence"      # any file, by a path relative to the repository root
//! file = "LICENSE-MIT"
//! ```
//!
//! `program` exists so a recipe does not have to spell a target triple and a cargo profile, which
//! are facts about this checkout rather than about the package. A key the parser does not know is
//! refused, so a misspelt `digest` cannot pass review as a recipe that records none.
//!
//! The parser is the `toml` crate, host-only: `xtask` never reaches the shipping graph, and no
//! target reads a recipe. The target reads the image's catalogue (`name digest` lines, below).
//!
//! # BUGS
//!
//! - **It builds nothing.** A `program` whose ELF is not in `target/` is an error naming the file,
//!   not a cargo invocation. Packaging and building are separate acts here for the reason milestone
//!   150 gives about hand-maintained lists: a tool that quietly rebuilt would hide which binary it
//!   had packed.
//! - **Nothing installs the result.** A target can fetch a package and check it against the image's
//!   catalogue (`image_catalogue` below, and the kernel's package tests), and nothing after that:
//!   installing it waits on how an installed program reaches the spawner, which is calef's
//!   (notes/packages.md, "Where this stops").
//! - **The catalogue is a file in `target/` and an archive entry**, not a repository index. §195's
//!   per-source trust needs a catalogue per source; the image's own is the only source there is.
//! - **A recipe cannot say where its source came from.** Homebrew's formula carries a URL and a
//!   digest of the upstream tarball; this carries neither, because the only packages that exist are
//!   built from this repository.

use std::path::PathBuf;

use package_archive::{Attributes, Package, package_size, sha256, write_package};

use crate::host::workspace_root;
use crate::{RISCV_TARGET, TARGET, X86_TARGET, profile_dir};

/// Where built packages land. Under `target/` because a package is an artifact, and because
/// nothing in this tree is ready to publish one.
const OUTPUT: &str = "target/packages";

/// A recipe, parsed. The parser is a pure function over the text, so a test can call it with a
/// string literal, which is the shape every parser in this tree that is worth testing has.
#[derive(Debug, PartialEq, Eq)]
struct Recipe {
    name: String,
    version: String,
    architecture: String,
    /// `(member name, where its bytes come from)`, in the order the recipe lists them, because the
    /// package's bytes are a function of that order and a reviewed digest depends on it.
    members: Vec<(String, Source)>,
    digest: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum Source {
    /// A built ELF for the recipe's architecture, resolved from `target/`.
    Program(String),
    /// A path relative to the repository root.
    File(String),
}

/// `cargo xtask package <recipe>`: build it, check it against the recipe, and report.
pub(crate) fn package(recipe_path: Option<String>) -> bool {
    let Some(recipe_path) = recipe_path else {
        eprintln!("usage: cargo xtask package <recipe>");
        return false;
    };
    let root = workspace_root();
    let text = match std::fs::read_to_string(root.join(&recipe_path)) {
        Ok(text) => text,
        Err(error) => {
            eprintln!("package: could not read {recipe_path}: {error}");
            return false;
        }
    };
    let built = match build(&root, &text) {
        Ok(built) => built,
        Err(complaint) => {
            eprintln!("package: {recipe_path}: {complaint}");
            return false;
        }
    };
    let parsed = Package::parse(&built.file).expect("build() read it back already");
    println!(
        "{} {} {}, {} members, {} bytes",
        parsed.name(),
        parsed.version(),
        parsed.architecture(),
        parsed.len(),
        built.file.len()
    );
    for index in 0..parsed.len() {
        let name = parsed.member_name(index).unwrap_or("");
        let len = parsed.member(index).map(<[u8]>::len).unwrap_or(0);
        let digest = parsed.member_digest(index).unwrap_or_default();
        println!("  {name:<24} {len:>9} bytes  {}", hex(&digest));
    }
    if built.recorded {
        println!(
            "digest {}  (recorded in the recipe, and it matches)",
            built.digest
        );
    } else {
        println!(
            "digest {}  (the recipe records none; review it and add it)",
            built.digest
        );
    }
    match write_out(&root, &built) {
        Ok(written) => {
            println!("wrote {}", relative(&root, &written));
            println!("package: PASS");
            true
        }
        Err(complaint) => {
            eprintln!("package: {complaint}");
            false
        }
    }
}

/// A package built from a recipe and checked, not yet written anywhere.
pub(crate) struct Built {
    /// `name-version-architecture`, the file's stem and its catalogue name.
    pub(crate) stem: String,
    pub(crate) file: Vec<u8>,
    /// SHA-256 over the whole file, as 64 lowercase hex characters.
    pub(crate) digest: String,
    /// Whether the recipe recorded a digest (which, if this returned at all, matched).
    recorded: bool,
}

/// Build the package a recipe describes, read it back with the target's parser, and check it
/// against the recipe's recorded digest. Writes nothing: a refusal here leaves the disk as it was.
pub(crate) fn build(root: &std::path::Path, text: &str) -> Result<Built, String> {
    let recipe = parse_recipe(text)?;

    // Read every member first, so a missing file is reported before anything is written.
    let mut bytes = Vec::new();
    for (name, source) in &recipe.members {
        let path =
            resolve(root, &recipe.architecture, source).map_err(|c| format!("{name}: {c}"))?;
        // **A program is packed stripped**, the same bytes the image packs (`read_stripped`).
        // Unstripped, `uptime` was 881,152 bytes against a job region of 40 pages (160 KiB): an
        // installed program is bytes something on the target must read into memory to build a
        // process from, and debug sections nobody on the target reads would have cost more than the
        // program. Found by rung 3a's consumer lane, 2026-09-24.
        let content = match source {
            Source::Program(_) => crate::inspect::read_stripped(&path.display().to_string()),
            Source::File(_) => std::fs::read(&path),
        }
        .map_err(|e| format!("{name}: could not read {}: {e}", path.display()))?;
        bytes.push(content);
    }
    let members: Vec<(&str, &[u8])> = recipe
        .members
        .iter()
        .zip(&bytes)
        .map(|((name, _), content)| (name.as_str(), content.as_slice()))
        .collect();

    let attributes = Attributes {
        name: &recipe.name,
        version: &recipe.version,
        architecture: &recipe.architecture,
    };
    let mut file = vec![0u8; package_size(&members)];
    write_package(&attributes, &members, &mut file).map_err(|e| format!("refused: {e:?}"))?;

    // Read the package back with the target's own parser before anything is written. The producer
    // and the consumer share one definition of the format, and this is where that stops being a
    // claim: a writer that could emit a file its reader refuses would ship one.
    let parsed =
        Package::parse(&file).map_err(|e| format!("wrote a file its own reader refuses: {e:?}"))?;
    if let Err(error) = parsed.verify() {
        return Err(format!("member {} does not match its digest", error.index));
    }

    let digest = hex(&sha256(&file));
    // **The recorded digest is checked before anything is written**, which is the order §195 asks
    // for even though it costs a rebuild to find out. A package whose bytes do not reproduce the
    // reviewed line is one nothing accepts, so leaving it on disk beside a catalogue entry
    // vouching for it would be the tool disagreeing with itself.
    if let Some(recorded) = &recipe.digest
        && *recorded != digest
    {
        return Err(format!(
            "the recipe records {recorded}, these bytes are {digest}: a rebuild that does not \
             reproduce the reviewed digest is the failure DECISIONS §195 exists to make visible; \
             nothing was written"
        ));
    }
    Ok(Built {
        stem: format!("{}-{}-{}", recipe.name, recipe.version, recipe.architecture),
        file,
        digest,
        recorded: recipe.digest.is_some(),
    })
}

/// Write a built package under `target/packages/` and its line into the host's catalogue there.
fn write_out(root: &std::path::Path, built: &Built) -> Result<PathBuf, String> {
    let output = root.join(OUTPUT);
    std::fs::create_dir_all(&output)
        .map_err(|e| format!("could not create {}: {e}", output.display()))?;
    let written = output.join(format!("{}.nifepkg", built.stem));
    std::fs::write(&written, &built.file)
        .map_err(|e| format!("could not write {}: {e}", written.display()))?;
    // The catalogue line is `measured_boot`'s manifest shape (a name, a space, 64 hex characters),
    // which is the format the progenitor already reads to decide whether a program may run. §195
    // makes the image's measurement table the first source of trust, so a package's entry looking
    // like an entry in that table is the point rather than a coincidence.
    let catalogue = output.join("catalogue");
    append(&catalogue, &format!("{} {}\n", built.stem, built.digest))
        .map_err(|e| format!("could not write {}: {e}", catalogue.display()))?;
    Ok(written)
}

/// **The image's own package source** (rung 3a of milestone 198): build every recipe under
/// `packages/` whose architecture is `architecture`, write each package where the test's HTTP peer
/// serves it from, and return the catalogue lines for the image to carry.
///
/// This is DECISIONS §195's "the image's measured table becomes the first source" made literal: the
/// catalogue goes into the initrd archive *before* the measurement table is computed, so the kernel's
/// trust root vouches for the catalogue and the catalogue vouches for the package. A package fetched
/// over plain HTTP is then checked against a digest that never crossed the network.
///
/// It also means every archive build runs the producer end to end, which is the gate
/// notes/packages.md's BUGS said nothing ran.
pub(crate) fn image_catalogue(architecture: &str) -> Result<String, String> {
    let root = workspace_root();
    let dir = root.join("packages");
    let mut recipes: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(|e| format!("could not read {}: {e}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".recipe.toml"))
        })
        .collect();
    recipes.sort();
    let mut catalogue = String::new();
    for path in recipes {
        let text = std::fs::read_to_string(&path)
            .map_err(|e| format!("could not read {}: {e}", path.display()))?;
        if parse_recipe(&text)?.architecture != architecture {
            continue;
        }
        let built = build(&root, &text).map_err(|c| format!("{}: {c}", relative(&root, &path)))?;
        write_out(&root, &built)?;
        catalogue.push_str(&format!("{} {}\n", built.stem, built.digest));
    }
    Ok(catalogue)
}

/// Append a catalogue line, replacing any earlier line for the same name so a rebuild does not
/// leave two answers to one question.
fn append(path: &std::path::Path, line: &str) -> std::io::Result<()> {
    let name = line.split(' ').next().unwrap_or_default();
    let existing = std::fs::read_to_string(path).unwrap_or_default();
    let mut text: String = existing
        .lines()
        .filter(|kept| kept.split(' ').next() != Some(name))
        .map(|kept| format!("{kept}\n"))
        .collect();
    text.push_str(line);
    std::fs::write(path, text)
}

fn relative(root: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn hex(digest: &[u8]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Where a member's bytes live on this host.
fn resolve(root: &std::path::Path, architecture: &str, source: &Source) -> Result<PathBuf, String> {
    match source {
        Source::File(path) => Ok(root.join(path)),
        Source::Program(name) => {
            let triple = match architecture {
                "aarch64" => TARGET,
                "riscv64" => RISCV_TARGET,
                "x86_64" => X86_TARGET,
                other => return Err(format!("unknown architecture {other}")),
            };
            let path = root.join(format!("target/{triple}/{}/{name}", profile_dir()));
            if path.exists() {
                Ok(path)
            } else {
                Err(format!(
                    "{} is not built; build it first (cargo xtask initrd-{})",
                    relative(root, &path),
                    if architecture == "aarch64" {
                        "aarch64"
                    } else if architecture == "riscv64" {
                        "riscv"
                    } else {
                        "x86"
                    }
                ))
            }
        }
    }
}

/// Parse a recipe, or say which key could not be read and why.
///
/// A pure function over the text, so the interesting half is host-testable in milliseconds, which
/// is what `AGENTS.md` asks of every parser in this tree.
fn parse_recipe(text: &str) -> Result<Recipe, String> {
    let table: toml::Table = text
        .parse()
        .map_err(|e: toml::de::Error| e.message().to_string())?;
    let string = |key: &str| -> Result<Option<String>, String> {
        match table.get(key) {
            None => Ok(None),
            Some(toml::Value::String(value)) if !value.is_empty() => Ok(Some(value.clone())),
            Some(_) => Err(format!("{key} is a non-empty string")),
        }
    };
    for key in table.keys() {
        if !["name", "version", "architecture", "digest", "member"].contains(&key.as_str()) {
            return Err(format!("unknown key {key}"));
        }
    }
    let name = string("name")?.ok_or("no name")?;
    let version = string("version")?.ok_or("no version")?;
    let architecture = string("architecture")?.ok_or("no architecture")?;
    let digest = string("digest")?;
    if let Some(digest) = &digest
        && (digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()))
    {
        return Err("a digest is 64 hex characters".to_string());
    }

    let mut members = Vec::new();
    let entries = match table.get("member") {
        None => &Vec::new(),
        Some(toml::Value::Array(entries)) => entries,
        Some(_) => return Err("member is an array of tables, [[member]]".to_string()),
    };
    for (index, entry) in entries.iter().enumerate() {
        let number = index + 1;
        let Some(entry) = entry.as_table() else {
            return Err(format!("member {number} is not a table"));
        };
        let field = |key: &str| entry.get(key).and_then(toml::Value::as_str);
        for key in entry.keys() {
            if !["program", "name", "file"].contains(&key.as_str()) {
                return Err(format!("member {number}: unknown key {key}"));
            }
        }
        match (field("program"), field("name"), field("file")) {
            (Some(program), None, None) => {
                members.push((program.to_string(), Source::Program(program.to_string())));
            }
            (None, Some(member), Some(path)) => {
                members.push((member.to_string(), Source::File(path.to_string())));
            }
            _ => {
                return Err(format!(
                    "member {number} is either `program` alone or `name` with `file`"
                ));
            }
        }
    }

    Ok(Recipe {
        name,
        version,
        architecture,
        members,
        digest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const RECIPE: &str = r#"
# a comment
name = "uptime"
version = "0.1.0"
architecture = "aarch64"

[[member]]
program = "uptime"

[[member]]
name = "uptime.licence"
file = "LICENSE-MIT"
"#;

    #[test]
    fn a_recipe_reads_as_what_it_says() {
        let recipe = parse_recipe(RECIPE).unwrap();
        assert_eq!(recipe.name, "uptime");
        assert_eq!(recipe.version, "0.1.0");
        assert_eq!(recipe.architecture, "aarch64");
        assert_eq!(recipe.digest, None);
        assert_eq!(
            recipe.members,
            vec![
                ("uptime".to_string(), Source::Program("uptime".to_string())),
                (
                    "uptime.licence".to_string(),
                    Source::File("LICENSE-MIT".to_string())
                ),
            ]
        );
    }

    #[test]
    fn a_missing_field_is_named() {
        for (text, missing) in [
            ("name = \"uptime\"", "no version"),
            ("version = \"0.1.0\"", "no name"),
            ("name = \"uptime\"\nversion = \"0.1.0\"", "no architecture"),
        ] {
            assert_eq!(parse_recipe(text).unwrap_err(), missing);
        }
    }

    #[test]
    fn a_digest_that_is_not_a_digest_is_refused() {
        // The one field where a typo would otherwise pass review and then fail a build on another
        // host, which is the failure §195's whole arrangement is trying not to have. The digest
        // goes before the first [[member]], where TOML puts a top-level key.
        let with = |digest: &str| {
            RECIPE.replacen(
                "[[member]]",
                &format!("digest = \"{digest}\"\n\n[[member]]"),
                1,
            )
        };
        assert_eq!(
            parse_recipe(&with("abc123")).unwrap_err(),
            "a digest is 64 hex characters"
        );
        assert_eq!(
            parse_recipe(&with(&"a".repeat(64))).unwrap().digest,
            Some("a".repeat(64))
        );
    }

    #[test]
    fn a_key_nobody_can_read_names_itself() {
        // A misspelt key must not read as an absent one: a misspelt `digest` would otherwise be a
        // recipe that records no digest, which review would pass.
        assert_eq!(
            parse_recipe("name = \"uptime\"\nfetch = \"https://example.invalid\"\n").unwrap_err(),
            "unknown key fetch"
        );
        assert_eq!(
            parse_recipe(&RECIPE.replace("file = \"LICENSE-MIT\"", "")).unwrap_err(),
            "member 2 is either `program` alone or `name` with `file`"
        );
        assert_eq!(
            parse_recipe("name = \"\"\n").unwrap_err(),
            "name is a non-empty string"
        );
    }

    #[test]
    fn every_recipe_in_the_tree_parses() {
        // The recipes are read by every archive build, so a malformed one breaks the image rather
        // than this command; catching it here costs milliseconds.
        let dir = workspace_root().join("packages");
        let mut seen = 0;
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.to_string_lossy().ends_with(".recipe.toml") {
                parse_recipe(&std::fs::read_to_string(&path).unwrap())
                    .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
                seen += 1;
            }
        }
        assert!(seen > 0, "no recipes found");
    }
}
