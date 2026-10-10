//! **`activation_set`**: what is installed, as a table that is never edited, only succeeded.
//!
//! DECISIONS §208 (installing a package is granting it, and the activation set is versioned) ruled
//! that installing *records that a package exists*, and that **the table of entries is versioned**
//! so a set can be selected and rolled back as a whole. This crate is that table's logic, pure and
//! host-tested, for whichever process DECISIONS §219 (how the shell names an installed program to
//! the spawner) ends up giving it to. §219 gave it to the progenitor: since milestone 198 rung 3a's
//! image lane (2026-09-26) the progenitor reads the live generation on every image spawn and looks
//! the executable's digest up in it ([`lookup_digest`]).
//!
//! # Where it lives
//!
//! A directory [`DIRECTORY`] at the root of the file service, holding [`CURRENT`] and one file per
//! generation named by its decimal number ([`generation_name`]). Both the progenitor, which reads
//! and writes it, and the shell, which reads it to resolve a bare word and to preview `caps`, read
//! the names from here, which is rule 7: what two programs agree on is a crate. Provisional, like
//! the crate's name.
//!
//! # The shape, and why it is this one
//!
//! **A generation is a text file that is never rewritten.** Installing and removing each produce
//! the *next* generation from the current one ([`with_entry`], [`without_entry`],
//! [`without_version`]); a one-line `current` file names which generation is live
//! ([`parse_current`], [`format_current`]). So a rollback is rewriting one line to name an older
//! generation, and every older set is still on disk, whole, to be named. That is the Nix profile's
//! arrangement (a generation is immutable, the profile is a pointer), recorded as recalled rather
//! than re-read.
//!
//! **A generation carries two kinds of line** (milestone 614 (two installed versions of one
//! program, each runnable, and a caller granted the one it needs), rulings 2 and 3 of 2026-09-29):
//!
//! ```text
//! <digest> <program> <version> <package> <manager>   a row: what is installed, and who installed it
//! default <program> <digest>                         the pointer: what the bare word runs
//! ```
//!
//! A `<digest>` is written `sha256:<64 hex>`, the one text form `measured_boot::digest_text`
//! writes and `measured_boot::parse_digest` reads (DECISIONS §197 (a package is one archive
//! file), its digest ruling of 2026-10-07).
//!
//! **Rows key on the digest; `program`, `version` and `package` are label columns.** The digest is
//! the packager's attestation (the program member's SHA-256, as installing verified it); the
//! version string is the upstream developer's claim. A rebuild claiming a version string already
//! live is a second row, visible, never a silent replacement, because its bytes are a different
//! digest: two versions of one program coexist as two rows, which is what one row per program name
//! could not hold. There is no architecture column: the digest is of target-specific bytes, so
//! builds for two ISAs never collide in one table. There is no install datetime: the generation
//! index is a total order with no clock in it.
//!
//! **The default pointer lives inside the generation file**, so a rollback restores the table and
//! the default together and `current` stays the one commit point. Every install of a program moves
//! its pointer: the bare word means the newest install, which was the implicit rule before and is
//! now a written one. An owner's vouch ([`OWNER`]) claims no name (DECISIONS §229 (how a bare name
//! at the prompt reaches an installed program), B2), so a vouch never writes a pointer and
//! [`lookup`] never answers with one.
//!
//! **An owner's vouch row carries `-` in the version column** ([`NO_VERSION`]): the owner vouches
//! for bytes, not for a version anyone claimed.
//!
//! **Strict, because this table decides what may be spawned.** A malformed line of either kind
//! makes the whole table unreadable ([`Error::Malformed`]) rather than skipped, for `measured_boot`'s
//! reason: a table that half-parses vouches for whatever survived.
//!
//! # EXAMPLES
//!
//! ```
//! use activation_set::{lookup, with_entry, without_entry, Entry, NO_VERSION, OWNER};
//!
//! let digest = [7u8; 32];
//! let uptime = Entry { program: "uptime", version: "0.1.0", package: "uptime", digest, manager: "jig" };
//! let mut first = [0u8; 512];
//! let n = with_entry("", &uptime, false, &mut first).unwrap();
//! let generation_1 = core::str::from_utf8(&first[..n]).unwrap();
//! assert_eq!(lookup(generation_1, "uptime").unwrap().unwrap().version, "0.1.0");
//!
//! let mut second = [0u8; 512];
//! let n = without_entry(generation_1, "uptime", &mut second).unwrap();
//! let generation_2 = core::str::from_utf8(&second[..n]).unwrap();
//! assert!(lookup(generation_2, "uptime").unwrap().is_none());
//! // Generation 1 is untouched, so rolling back is naming it again.
//! assert!(lookup(generation_1, "uptime").unwrap().is_some());
//!
//! // A vouch is a row with no version and no pointer: found by digest, never by name.
//! let vouch = Entry { program: "a.out", version: NO_VERSION, package: OWNER, digest, manager: OWNER };
//! let mut third = [0u8; 512];
//! let n = with_entry(generation_2, &vouch, false, &mut third).unwrap();
//! let generation_3 = core::str::from_utf8(&third[..n]).unwrap();
//! assert!(lookup(generation_3, "a.out").unwrap().is_none());
//! ```
//!
//! # BUGS
//!
//! - **Nothing collects old generations.** Every install leaves one file behind, which is what
//!   makes rollback free and is also unbounded. A retention rule (keep N, keep the last boot's) is
//!   owed before a system installs often.
//! - **A table is read whole into memory**, and its size is whatever the caller's buffer is. With
//!   the version column and the pointer lines a row costs about 140 bytes (the digest's `sha256:` label added seven on 2026-10-07), so thirty-odd entries
//!   fit a page.
//! - **One program per package row.** A package with two programs is two rows naming the same
//!   package, which works and is not tested beyond that.
//! - **The `current` file is the one mutable thing**, so its write is the commit point, and nothing
//!   here makes that write atomic. On RedoxFS it is a one-line overwrite; whether a torn write of it
//!   is possible was not checked.
//!
//! Name: provisional 2026-09-24 (milestone 198 (a package manager)'s rung 3a consumer lane). §208's own phrase is
//! "activation set", which is why; `design/naming.md` is the rule and calef's the call.

#![no_std]

pub use measured_boot::Digest;

/// Why a table or an edit was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// A line is neither a row nor a pointer line, so the table vouches for nothing.
    Malformed,
    /// A name is empty, or holds a space, a newline or a `#`, so it would not read back as itself.
    BadName,
    /// A removal was asked for a program, or a program at a version, the table does not have.
    NotInstalled,
    /// The output buffer is too small for the next generation.
    TooSmall,
    /// **Another package already provides a program of this name** (DECISIONS §229 (how a bare name
    /// at the prompt reaches an installed program), B2). Installing a second package's program
    /// under it would silently take the name from the first. The same package at another version
    /// is not this: both rows live side by side.
    Taken,
    /// **The image carries a program of this name** (DECISIONS §229, calef's ruling of 2026-09-27).
    /// A package cannot take a base program's name. Under §235 (the OS is built and updated from
    /// packages) a base program is updated by writing base packages into the inactive boot slot,
    /// never by `jig install`, so this refusal blocks no update. An owner's vouch claims no
    /// name and is never this.
    ///
    /// Name: provisional, milestone 47 (navigation and naming)'s bare-name lane, 2026-09-27.
    ImageName,
    /// [`without_version`] refused: the version asked for holds the default pointer and more than
    /// one other version remains, so no ordering among live versions exists to pick a new default
    /// with. The caller names the candidates with [`versions_of`]. Nothing is written. The same
    /// word as `package_archive::CatalogMiss::SeveralVersions`, for the same condition.
    ///
    /// Name: ratified 2026-10-07 (calef, §258 (names for two installed versions of one program)).
    /// Refused `Ambiguous` (calef: "ambiguous"), `AmbiguousName` (the name is not ambiguous; the
    /// version is). Minted as `Ambiguous` by milestone 614's build lane, 2026-09-29.
    SeveralVersions,
}

/// One installed program: its name at the prompt, the version its upstream claims, the package it
/// came from, and its digest as it was verified at install. The **digest is the row's key** and the
/// other three are label columns (milestone 614, ruling 2): a rebuild that produces new bytes is a
/// second row even when the version string is one already live.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry<'a> {
    /// The name a person types, and the name the spawner is asked for.
    pub program: &'a str,
    /// **The version the upstream developer claims**: a label, never an identity. An owner's vouch
    /// carries [`NO_VERSION`], because the owner vouches for bytes and claims no version.
    pub version: &'a str,
    /// The package it came from, by name. [`OWNER`] when the vouch is the package's.
    pub package: &'a str,
    /// **The program's own SHA-256**: the digest of the executable member, as the package's table
    /// of contents carries it (`package_archive::Package::member_digest`) and as installing
    /// verified it. Not the package file's digest, which §195 (a reviewed recipe vouches for a
    /// package)'s recipe vouches for and which installing checks first: the spawner is handed the
    /// executable's bytes, never the package's (DECISIONS §219 option D), so the digest it can
    /// compute is the member's. Changed 2026-09-26 by milestone 198 rung 3a's image lane, before
    /// anything wrote a table. Key of the row since milestone 614, ruling 2.
    pub digest: Digest,
    /// **Which package manager installed the row** (milestone 809, calef's 2026-10-06 ruling): the
    /// program that held the installer endpoint, or [`OWNER`] for the owner's console. A label like
    /// the version, never part of the key. [`NO_MANAGER`] for a row written before the column.
    pub manager: &'a str,
}

/// **A default pointer line**: the row the bare word for `program` runs ([`lookup`]). Read with
/// [`defaults`], written by [`with_entry`] and [`without_entry`]. The `default` line kind was
/// ratified 2026-10-07 by calef (§258 (names for two installed versions of one program)); this
/// struct's own name was not ruled on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pointer<'a> {
    /// The program whose bare word this pointer answers.
    pub program: &'a str,
    /// The digest of the row the pointer names. Naming a digest that is not a package row of the
    /// program makes the table unreadable: a pointer to nothing vouches for nothing.
    pub digest: Digest,
}

/// Every row in a generation, or [`Error::Malformed`] at the first line that is not one. Blank
/// lines, `#` comments and well-formed pointer lines are skipped; a malformed pointer line is a
/// malformed table. The pointer lines themselves are read with [`defaults`].
pub fn entries(table: &str) -> impl Iterator<Item = Result<Entry<'_>, Error>> {
    table
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| match classify(line) {
            Line::Row {
                digest,
                program,
                version,
                package,
                manager,
            } => Some(Ok(Entry {
                program,
                version,
                package,
                digest,
                manager,
            })),
            // A well-formed pointer line is not a row; that it is well-formed was the check.
            Line::Pointer { .. } => None,
            Line::Malformed => Some(Err(Error::Malformed)),
        })
}

/// Every default pointer line in a generation, or [`Error::Malformed`] at the first line that is
/// not a row, a comment, a blank, or a well-formed pointer line. The whole table is validated
/// either way, so a caller that reads only pointers still refuses a broken row.
pub fn defaults(table: &str) -> impl Iterator<Item = Result<Pointer<'_>, Error>> {
    table
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| match classify(line) {
            Line::Pointer { program, digest } => Some(Ok(Pointer { program, digest })),
            // A well-formed row is not a pointer; that it is well-formed was the check.
            Line::Row { .. } => None,
            Line::Malformed => Some(Err(Error::Malformed)),
        })
}

/// One parsed line of a generation. Internal; the two iterators above decide which kind they keep.
enum Line<'a> {
    Row {
        digest: Digest,
        program: &'a str,
        version: &'a str,
        package: &'a str,
        manager: &'a str,
    },
    Pointer {
        program: &'a str,
        digest: Digest,
    },
    Malformed,
}

/// Classify one line. A row is `<digest> <program> <version> <package> <manager>`, or the four
/// words before the manager column existed (read as [`NO_MANAGER`]); a pointer line is
/// `default <program> <digest>`; anything else is malformed. A digest is its text form,
/// `sha256:<64 hex>` (`measured_boot::parse_digest`), so an unlabeled or unknown one makes the line
/// malformed and the table unreadable. A row's first word is the key, which is why `default` is a
/// word no digest can be: it has no label.
fn classify(line: &str) -> Line<'_> {
    let mut words = line.split(' ');
    let Some(first) = words.next() else {
        return Line::Malformed;
    };
    if first == "default" {
        let (Some(program), Some(hex), None) = (words.next(), words.next(), words.next()) else {
            return Line::Malformed;
        };
        let Some(digest) = measured_boot::parse_digest(hex)
            .ok()
            .filter(|_| good_name(program))
        else {
            return Line::Malformed;
        };
        return Line::Pointer { program, digest };
    }
    let (Some(program), Some(version), Some(package)) = (words.next(), words.next(), words.next())
    else {
        return Line::Malformed;
    };
    let manager = match (words.next(), words.next()) {
        (None, _) => NO_MANAGER,
        (Some(manager), None) if good_name(manager) => manager,
        _ => return Line::Malformed,
    };
    let Ok(digest) = measured_boot::parse_digest(first) else {
        return Line::Malformed;
    };
    if !good_name(program) || !good_name(version) || !good_name(package) {
        return Line::Malformed;
    }
    Line::Row {
        digest,
        program,
        version,
        package,
        manager,
    }
}

/// **The entry a bare name at the prompt runs** (DECISIONS §229 (how a bare name at the prompt
/// reaches an installed program), B2; milestone 614, rulings 2 and 3): the default pointer for
/// `program`, then the package row it names. An owner's vouch is never the answer, because it
/// claims no name and writes no pointer. Two pointer lines for one program, or a pointer naming a
/// digest that is not a package row of the program, make the table unreadable: the default is one
/// answer, and a pointer to nothing vouches for nothing.
///
/// The whole table is checked first, so a malformed line anywhere is a refusal even when the name
/// asked for is on a good line. Was `lookup_name` until milestone 614 folded the pointer into
/// [`lookup`], which used to answer with any row of the name.
pub fn lookup<'a>(table: &'a str, program: &str) -> Result<Option<Entry<'a>>, Error> {
    let Some(digest) = default_of(table, program)? else {
        return Ok(None);
    };
    for entry in entries(table) {
        let entry = entry?;
        if entry.program == program && entry.package != OWNER && entry.digest == digest {
            return Ok(Some(entry));
        }
    }
    Err(Error::Malformed)
}

/// **The row these bytes belong to**, if the generation has one: what the progenitor asks when it
/// is handed an executable rather than a name (DECISIONS §219 option D). The whole table is
/// checked first, as in [`lookup`], so a malformed line anywhere vouches for nothing.
///
/// Two rows with one digest (one program installed under two names) answer with the **first**,
/// deterministically. That is harmless because the manifest an installed program is endowed with is
/// in its bytes (an ELF note, milestone 597 (a program carries its manifest in an ELF note)), not
/// in its row: one digest is one set of bytes and so one manifest, whichever name was installed.
pub fn lookup_digest<'a>(table: &'a str, digest: &Digest) -> Result<Option<Entry<'a>>, Error> {
    let mut found = None;
    for entry in entries(table) {
        let entry = entry?;
        if found.is_none() && entry.digest == *digest {
            found = Some(entry);
        }
    }
    Ok(found)
}

/// **The package row for this program at this version**, first in file order: what a version set's
/// `<program> <version>` line resolves to at activation, and what `jig remove
/// <program>@<version>` removes. Owner's vouches are not versions and never answer. A rebuild
/// claiming a version already live is a second row, so the first is answered; the digest decides
/// what runs either way. Name: provisional, milestone 614's build lane, 2026-09-29.
pub fn lookup_version<'a>(
    table: &'a str,
    program: &str,
    version: &str,
) -> Result<Option<Entry<'a>>, Error> {
    let mut found = None;
    for entry in entries(table) {
        let entry = entry?;
        if found.is_none()
            && entry.program == program
            && entry.version == version
            && entry.package != OWNER
        {
            found = Some(entry);
        }
    }
    Ok(found)
}

/// **The version labels live for `program`**, in file order, duplicates included: what a refusal
/// that may not pick among versions names instead. Owner's vouches claim no version and are not
/// listed.
///
/// Name: ratified 2026-10-07 (calef, §258 (names for two installed versions of one program)).
pub fn versions_of<'a>(
    table: &'a str,
    program: &str,
) -> impl Iterator<Item = Result<&'a str, Error>> {
    entries(table).filter_map(move |entry| match entry {
        Ok(e) if e.program == program && e.package != OWNER => Some(Ok(e.version)),
        Ok(_) => None,
        Err(e) => Some(Err(e)),
    })
}

/// The next generation: `table` with `entry` installed. Returns the length written to `out`.
///
/// **Upsert by digest, then move the pointer** (milestone 614, rulings 2 and 3):
///
/// - a package row with the same program **and the same digest is replaced in place**, labels and
///   all: the bytes are the identity, and reinstalling them under a new version string relabels
///   the row;
/// - the same package at a **new digest is a second row**: what one row per program name could not
///   hold, two versions of one program, is the point;
/// - another package's claim on the name is still [`Error::Taken`], and nothing is written;
/// - an owner's vouch and a package's row **sit side by side**. A vouch is found by digest and
///   never by name ([`lookup`]), so neither displaces the other. A later vouch of a name replaces
///   the earlier vouch of that name, as it did before rows keyed on digest.
///
/// A package install also **writes the program's default pointer**: the bare word means the newest
/// install. A vouch writes none, because it claims no name.
///
/// `image_carries` is whether the running image has a program named `entry.program`. A package's
/// row under such a name is [`Error::ImageName`] (§229, 2026-09-27), checked before
/// [`Error::Taken`]; a vouch ignores it. It is a required argument rather than a second function
/// so an installer cannot forget to ask.
pub fn with_entry(
    table: &str,
    entry: &Entry<'_>,
    image_carries: bool,
    out: &mut [u8],
) -> Result<usize, Error> {
    if !good_name(entry.program)
        || !good_name(entry.version)
        || !good_name(entry.package)
        || !good_name(entry.manager)
    {
        return Err(Error::BadName);
    }
    let vouch = entry.package == OWNER;
    if !vouch && image_carries {
        return Err(Error::ImageName);
    }
    // Refuse before writing a byte, so a refused install leaves `out` meaning nothing.
    for existing in entries(table) {
        let existing = existing?;
        if !vouch
            && existing.program == entry.program
            && existing.package != OWNER
            && existing.package != entry.package
        {
            return Err(Error::Taken);
        }
    }
    let mut writer = Writer { out, at: 0 };
    let mut replaced = false;
    for existing in entries(table) {
        let existing = existing?;
        let same_kind = (existing.package == OWNER) == vouch;
        let replaced_here = same_kind
            && existing.program == entry.program
            && if vouch {
                // A later vouch of a name replaces the earlier one, whatever its digest.
                true
            } else {
                existing.digest == entry.digest
            };
        if replaced_here {
            writer.entry(entry)?;
            replaced = true;
        } else {
            writer.entry(&existing)?;
        }
    }
    if !replaced {
        writer.entry(entry)?;
    }
    let mut pointed = false;
    for existing in defaults(table) {
        let existing = existing?;
        if !vouch && existing.program == entry.program {
            writer.pointer(entry.program, &entry.digest)?;
            pointed = true;
        } else {
            writer.pointer(existing.program, &existing.digest)?;
        }
    }
    if !vouch && !pointed {
        writer.pointer(entry.program, &entry.digest)?;
    }
    Ok(writer.at)
}

/// The next generation: `table` without `program`. **The verb's object is the program** (milestone
/// 614, ruling 5): every live version's row goes, and the program's pointer with them, so the bare
/// word reaches nothing. An owner's vouch of the same name is not a package and stays; a rollback
/// is what undoes a vouch. The bytes stay where they are, which is what lets a rollback bring every
/// version back. Removing what is not installed is [`Error::NotInstalled`] rather than an identical
/// generation, because an uninstall that silently did nothing is a report the caller should see.
pub fn without_entry(table: &str, program: &str, out: &mut [u8]) -> Result<usize, Error> {
    let mut writer = Writer { out, at: 0 };
    let mut removed = false;
    for existing in entries(table) {
        let existing = existing?;
        if existing.program == program && existing.package != OWNER {
            removed = true;
        } else {
            writer.entry(&existing)?;
        }
    }
    for existing in defaults(table) {
        let existing = existing?;
        if existing.program != program {
            writer.pointer(existing.program, &existing.digest)?;
        }
    }
    if removed {
        Ok(writer.at)
    } else {
        Err(Error::NotInstalled)
    }
}

/// The next generation: `table` without the one row for `program` at `version`, first in file
/// order (milestone 614, ruling 5). Removing a version that is not the default leaves the pointer
/// alone. Removing **the default's version** moves the pointer to the sole remaining version, and
/// is [`Error::SeveralVersions`] when several remain, because no ordering among live versions exists to
/// pick with: the caller names the candidates with [`versions_of`]. Nothing is written on a
/// refusal.
///
/// Name: ratified 2026-10-07 (calef, §258 (names for two installed versions of one program)).
pub fn without_version(
    table: &str,
    program: &str,
    version: &str,
    out: &mut [u8],
) -> Result<usize, Error> {
    // Refuse before writing a byte, as [`with_entry`] does. One pass decides: the row to remove,
    // whether it holds the pointer, and what would survive it. Survivors are counted by *version*,
    // because the pointer's rule and the refusal's candidates are both about versions: two rows at
    // one version string (a rebuild) are one survivor, and the pointer moves to the first of them.
    let mut target = None;
    let mut survivor: Option<Entry<'_>> = None;
    let mut uniform = true;
    for existing in entries(table) {
        let existing = existing?;
        if existing.program != program || existing.package == OWNER {
            continue;
        }
        let is_target = existing.version == version && target.is_none();
        if is_target {
            target = Some(existing);
        } else {
            if let Some(first) = survivor {
                uniform &= first.version == existing.version;
            }
            if survivor.is_none() {
                survivor = Some(existing);
            }
        }
    }
    let Some(target) = target else {
        return Err(Error::NotInstalled);
    };
    let move_pointer = default_of(table, program)?.is_some_and(|d| d == target.digest);
    if move_pointer && !uniform {
        return Err(Error::SeveralVersions);
    }
    let mut writer = Writer { out, at: 0 };
    let mut skipped = false;
    for existing in entries(table) {
        let existing = existing?;
        let same_row = !skipped
            && existing.program == target.program
            && existing.version == target.version
            && existing.digest == target.digest;
        if same_row {
            skipped = true;
        } else {
            writer.entry(&existing)?;
        }
    }
    for existing in defaults(table) {
        let existing = existing?;
        if existing.program == program && move_pointer {
            match &survivor {
                // The last version went with the pointer; a program with no rows keeps none.
                None => continue,
                Some(survivor) => writer.pointer(program, &survivor.digest)?,
            }
        } else {
            writer.pointer(existing.program, &existing.digest)?;
        }
    }
    Ok(writer.at)
}

/// The program's default pointer's digest, as [`lookup`] reads it: `None` when the program has no
/// pointer, [`Error::Malformed`] on two. Internal because [`lookup`] is the ruled reader;
/// [`without_version`] needs the digest to compare against.
fn default_of(table: &str, program: &str) -> Result<Option<Digest>, Error> {
    let mut found = None;
    let mut count = 0usize;
    for pointer in defaults(table) {
        let pointer = pointer?;
        if pointer.program == program {
            found = Some(pointer.digest);
            count += 1;
        }
    }
    if count > 1 {
        return Err(Error::Malformed);
    }
    Ok(found)
}

/// The live generation's number, from the `current` file: decimal digits and an optional newline.
pub fn parse_current(text: &str) -> Option<u32> {
    let digits = text.strip_suffix('\n').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// The directory the activation set lives in, at the root of the file service. Provisional.
pub const DIRECTORY: &str = "activation";

/// The one-line file in [`DIRECTORY`] naming the live generation ([`parse_current`]).
pub const CURRENT: &str = "current";

/// **Where `current` is written before it is renamed over the real one** (milestone 198 rung 3a's
/// installer), so the flip is one `RENAME` and a reader meets the old line or the new one, never a
/// torn write. Provisional; a name no generation can have, because it is not all digits.
pub const CURRENT_STAGED: &str = "current.next";

/// **The directory an installed program's bytes are placed under**, at the root of the file
/// service: `packages/<name>/<version>/<program>`, one directory per package version (milestone
/// 198 rung 3a's installer). A component per field rather than the stem, because a name at the
/// prompt is at most sixteen bytes and `uptime-0.1.0-aarch64` is twenty. The progenitor hashes
/// whatever bytes a person runs (DECISIONS §219 option D), so this is where they are kept, not
/// what vouches for them. Provisional.
pub const PACKAGES: &str = "packages";

/// **What a row's package column says when the owner vouched for the bytes** (DECISIONS §221
/// (the boot prompt is the owner's console), ruling 1; §195 (a reviewed recipe vouches for a
/// package) clause 3). No package name can be this word by construction: the column is the
/// `name` field of a `name-version-architecture` stem, and the one package in this tree whose
/// name could say it does not exist. The row is found by digest like any other and undone by a
/// rollback. It never claims a bare name (§229, B2): [`lookup`] skips it, [`with_entry`] keeps it
/// beside a package's row of the same name rather than replacing either, and only a later vouch of
/// that name replaces it. Provisional, like the column.
pub const OWNER: &str = "owner";

/// **What a row's manager column reads as when the row has none**: written before milestone 809
/// (the package client becomes a program) added the column (calef, 2026-10-06: "each activation set
/// row records which manager installed it"), by the shell's `package` builtin, which the same change
/// removed. Never written. A row's manager is otherwise `jig` or [`OWNER`]. Provisional.
pub const NO_MANAGER: &str = "-";

/// **Whether `requester` may remove or roll back a row `row_manager` installed** (milestone 809,
/// calef's ruling of 2026-10-06: "if the owner ever grants a second manager the endpoint, `jig`
/// refuses to remove or roll back a row it did not install"). A manager may edit its own rows, the
/// owner's ([`OWNER`]: the owner runs every manager it granted the endpoint, and a vouch is undone
/// by a rollback), and rows nothing recorded ([`NO_MANAGER`]). It may not edit another manager's.
/// The owner's console, which holds the spawn endpoint itself, may edit any row.
///
/// Name: provisional.
pub fn may_edit(row_manager: &str, requester: &str) -> bool {
    requester == OWNER
        || row_manager == requester
        || row_manager == OWNER
        || row_manager == NO_MANAGER
}

/// **The first row that differs between two generations whose manager `requester` may not edit**
/// ([`may_edit`]), if any: what a rollback from `newer` to `older` would undo on another manager's
/// behalf. A row differs when no row with the same program and digest is in the other table. Both
/// tables are read whole first, so a malformed line in either is [`Error::Malformed`].
///
/// Name: provisional (milestone 809).
pub fn foreign_change<'a>(
    older: &'a str,
    newer: &'a str,
    requester: &str,
) -> Result<Option<Entry<'a>>, Error> {
    for table in [older, newer] {
        for entry in entries(table) {
            entry?;
        }
    }
    let present = |table: &str, e: &Entry<'_>| {
        entries(table)
            .flatten()
            .any(|o| o.program == e.program && o.digest == e.digest)
    };
    for (from, other) in [(older, newer), (newer, older)] {
        for entry in entries(from).flatten() {
            if !present(other, &entry) && !may_edit(entry.manager, requester) {
                return Ok(Some(entry));
            }
        }
    }
    Ok(None)
}

/// **What a vouch row's version column carries**: the owner vouches for bytes, and claims no
/// version string, so the column holds a word no version set selects and no install writes.
/// Name: ratified 2026-10-07 (calef, §258 (names for two installed versions of one program)).
pub const NO_VERSION: &str = "-";

/// The file name of generation `number` in [`DIRECTORY`]: its decimal digits, no padding.
pub fn generation_name(number: u32, out: &mut [u8; 10]) -> &str {
    let mut n = number;
    let mut at = out.len();
    loop {
        at -= 1;
        out[at] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    // Only ASCII digits were written, so this cannot fail; `unwrap_or` keeps the path panic-free.
    core::str::from_utf8(&out[at..]).unwrap_or("0")
}

/// Write the `current` file naming generation `number`. Returns the length written.
pub fn format_current(number: u32, out: &mut [u8]) -> Result<usize, Error> {
    let mut digits = [0u8; 10];
    let mut n = number;
    let mut len = 0;
    loop {
        digits[len] = b'0' + (n % 10) as u8;
        len += 1;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    let mut writer = Writer { out, at: 0 };
    for i in (0..len).rev() {
        writer.bytes(&[digits[i]])?;
    }
    writer.bytes(b"\n")?;
    Ok(writer.at)
}

fn good_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('#') && name.bytes().all(|b| b > b' ' && b != 0x7f)
}

struct Writer<'o> {
    out: &'o mut [u8],
    at: usize,
}

impl Writer<'_> {
    fn bytes(&mut self, b: &[u8]) -> Result<(), Error> {
        let end = self.at + b.len();
        self.out
            .get_mut(self.at..end)
            .ok_or(Error::TooSmall)?
            .copy_from_slice(b);
        self.at = end;
        Ok(())
    }

    fn entry(&mut self, e: &Entry<'_>) -> Result<(), Error> {
        self.bytes(&measured_boot::digest_text(&e.digest))?;
        self.bytes(b" ")?;
        self.bytes(e.program.as_bytes())?;
        self.bytes(b" ")?;
        self.bytes(e.version.as_bytes())?;
        self.bytes(b" ")?;
        self.bytes(e.package.as_bytes())?;
        self.bytes(b" ")?;
        self.bytes(e.manager.as_bytes())?;
        self.bytes(b"\n")
    }

    fn pointer(&mut self, program: &str, digest: &Digest) -> Result<(), Error> {
        self.bytes(b"default ")?;
        self.bytes(program.as_bytes())?;
        self.bytes(b" ")?;
        self.bytes(&measured_boot::digest_text(digest))?;
        self.bytes(b"\n")
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::collections::BTreeMap;
    use std::string::{String, ToString};
    use std::{format, vec};

    use super::*;

    /// The store as the protocol in the crate documentation uses it: generation files that are
    /// written once, and one `current` line. A `BTreeMap` stands in for the directory.
    struct Store {
        files: BTreeMap<String, String>,
    }

    impl Store {
        fn new() -> Self {
            let mut files = BTreeMap::new();
            files.insert("generation-0".to_string(), String::new());
            files.insert("current".to_string(), "0\n".to_string());
            Store { files }
        }
        fn current(&self) -> u32 {
            parse_current(&self.files["current"]).unwrap()
        }
        fn table(&self) -> &str {
            &self.files[&format!("generation-{}", self.current())]
        }
        fn newest(&self) -> u32 {
            self.files
                .keys()
                .filter_map(|k| k.strip_prefix("generation-"))
                .map(|n| n.parse::<u32>().unwrap())
                .max()
                .unwrap()
        }
        /// Write the next generation (never an existing file), then flip `current`: the commit.
        fn commit(&mut self, next: &[u8]) {
            let number = self.newest() + 1;
            let name = format!("generation-{number}");
            assert!(
                !self.files.contains_key(&name),
                "a generation was rewritten"
            );
            self.files
                .insert(name, String::from_utf8(next.to_vec()).unwrap());
            self.select(number);
        }
        fn select(&mut self, number: u32) {
            let mut out = [0u8; 16];
            let n = format_current(number, &mut out).unwrap();
            self.files.insert(
                "current".to_string(),
                String::from_utf8(out[..n].to_vec()).unwrap(),
            );
        }
        fn install(&mut self, entry: Entry<'_>) {
            let mut out = vec![0u8; 4096];
            let n = with_entry(self.table(), &entry, false, &mut out).unwrap();
            self.commit(&out[..n]);
        }
        fn remove(&mut self, program: &str) -> Result<(), Error> {
            let mut out = vec![0u8; 4096];
            let n = without_entry(self.table(), program, &mut out)?;
            self.commit(&out[..n]);
            Ok(())
        }
        fn remove_version(&mut self, program: &str, version: &str) -> Result<(), Error> {
            let mut out = vec![0u8; 4096];
            let n = without_version(self.table(), program, version, &mut out)?;
            self.commit(&out[..n]);
            Ok(())
        }
        /// The version the bare word for `program` runs, or `None`.
        fn bare_version_of(&self, program: &str) -> Option<String> {
            lookup(self.table(), program)
                .unwrap()
                .map(|e| e.version.to_string())
        }
        fn live_versions_of(&self, program: &str) -> std::vec::Vec<String> {
            versions_of(self.table(), program)
                .map(|v| v.unwrap().to_string())
                .collect()
        }
    }

    /// A package row. One digest per version, so tests can name digests by version.
    fn row<'a>(program: &'a str, package: &'a str, version: &'a str, seed: u8) -> Entry<'a> {
        Entry {
            program,
            version,
            package,
            digest: [seed; 32],
            manager: "jig",
        }
    }

    fn vouch<'a>(program: &'a str, seed: u8) -> Entry<'a> {
        Entry {
            program,
            version: NO_VERSION,
            package: OWNER,
            digest: [seed; 32],
            manager: OWNER,
        }
    }

    /// **The property §208 asked for by name**: a rollback restores the whole set, not one package.
    /// **An owner's vouch is found by digest and claims no name** (DECISIONS §221 (the boot prompt
    /// is the owner's console); §229 (how a bare name at the prompt reaches an installed program),
    /// B2): it reads back, writes no pointer, and an install of the same program name sits beside
    /// it rather than replacing it, and holds the pointer the vouch never did.
    #[test]
    fn an_owner_vouch_is_an_ordinary_row() {
        let built = [9u8; 32];
        let mut g = [0u8; 512];
        let n = with_entry("", &vouch("a.out", 9), false, &mut g).unwrap();
        let table = core::str::from_utf8(&g[..n]).unwrap();
        assert_eq!(
            lookup_digest(table, &built).unwrap().unwrap().package,
            OWNER
        );
        assert!(
            lookup(table, "a.out").unwrap().is_none(),
            "a vouch claims no name"
        );
        assert_eq!(defaults(table).count(), 0, "a vouch writes no pointer");
        let upgrade = Entry {
            program: "a.out",
            version: "0.1.0",
            package: "a.out",
            digest: [1; 32],
            manager: "jig",
        };
        let mut h = [0u8; 512];
        let n = with_entry(table, &upgrade, false, &mut h).unwrap();
        let next = core::str::from_utf8(&h[..n]).unwrap();
        assert_eq!(lookup_digest(next, &built).unwrap().unwrap().package, OWNER);
        assert_eq!(lookup(next, "a.out").unwrap().unwrap().digest, [1; 32]);
    }

    /// **§229 B2, the install half, as milestone 614 reshaped it**: installing a second version
    /// appends a second row and moves the pointer; another package cannot take the name; and a
    /// vouch of the same name neither takes the bare name nor is taken by an install. Removing the
    /// package leaves the vouch and takes the pointer.
    #[test]
    fn a_name_belongs_to_one_package_and_never_to_a_vouch() {
        let mut g = [0u8; 512];
        let n = with_entry("", &row("uptime", "uptime", "0.1.0", 1), false, &mut g).unwrap();
        let t1 = core::str::from_utf8(&g[..n]).unwrap().to_string();

        let mut h = [0u8; 512];
        let n = with_entry(&t1, &row("uptime", "uptime", "0.2.0", 2), false, &mut h).unwrap();
        let t2 = core::str::from_utf8(&h[..n]).unwrap().to_string();
        assert_eq!(lookup(&t2, "uptime").unwrap().unwrap().digest, [2; 32]);
        assert_eq!(entries(&t2).count(), 2, "both versions stay live");
        assert!(lookup_digest(&t2, &[1; 32]).unwrap().is_some());

        let mut k = [0u8; 512];
        assert_eq!(
            with_entry(&t2, &row("uptime", "procps", "4.0.0", 3), false, &mut k),
            Err(Error::Taken)
        );

        let n = with_entry(&t2, &vouch("uptime", 4), false, &mut k).unwrap();
        let t3 = core::str::from_utf8(&k[..n]).unwrap().to_string();
        assert_eq!(lookup(&t3, "uptime").unwrap().unwrap().digest, [2; 32]);
        assert_eq!(
            lookup_digest(&t3, &[4; 32]).unwrap().unwrap().package,
            OWNER
        );

        let mut m = [0u8; 512];
        let n = without_entry(&t3, "uptime", &mut m).unwrap();
        let t4 = core::str::from_utf8(&m[..n]).unwrap();
        assert!(lookup(t4, "uptime").unwrap().is_none());
        assert_eq!(lookup_digest(t4, &[4; 32]).unwrap().unwrap().package, OWNER);
        assert_eq!(defaults(t4).count(), 0, "the pointer went with the program");
    }

    /// **§229, calef's ruling of 2026-09-27: a package cannot take a name the image carries.**
    /// Refused before `Taken` and before a byte is written, whether or not the table already has
    /// the name, and at any version. An owner's vouch of the same name claims no name and is kept.
    #[test]
    fn a_package_cannot_take_an_image_programs_name() {
        let mut out = [0u8; 512];
        let base = row("uptime", "uptime", "0.1.0", 1);
        assert_eq!(with_entry("", &base, true, &mut out), Err(Error::ImageName));

        // Already installed (a later base added the name, which §229's prompt refusal covers):
        // an install of that package at a new version is refused too, and so is another package's.
        let n = with_entry("", &base, false, &mut out).unwrap();
        let t = core::str::from_utf8(&out[..n]).unwrap().to_string();
        let mut next = [0u8; 512];
        let upgrade = row("uptime", "uptime", "0.2.0", 2);
        assert_eq!(
            with_entry(&t, &upgrade, true, &mut next),
            Err(Error::ImageName)
        );
        let other = row("uptime", "procps", "4.0.0", 3);
        assert_eq!(
            with_entry(&t, &other, true, &mut next),
            Err(Error::ImageName)
        );

        let n = with_entry(&t, &vouch("uptime", 4), true, &mut next).unwrap();
        let t2 = core::str::from_utf8(&next[..n]).unwrap();
        assert_eq!(lookup_digest(t2, &[4; 32]).unwrap().unwrap().package, OWNER);
    }

    /// **Milestone 614's first Done-means test**: installing 0.2.0 over 0.1.0 leaves both digests
    /// live, the bare name resolves to the version the ruled default names (ruling 3: the newest
    /// install), and a rebuild claiming a live version string is a second row (ruling 2).
    #[test]
    fn two_versions_of_one_program_stay_live_and_the_bare_word_means_the_newest() {
        let mut store = Store::new();
        store.install(row("uptime", "uptime", "0.1.0", 1));
        assert_eq!(store.bare_version_of("uptime").unwrap(), "0.1.0");
        store.install(row("uptime", "uptime", "0.2.0", 2));

        let table = store.table();
        assert!(lookup_digest(table, &[1; 32]).unwrap().is_some());
        assert!(lookup_digest(table, &[2; 32]).unwrap().is_some());
        assert_eq!(
            store.bare_version_of("uptime").unwrap(),
            "0.2.0",
            "ruling 3: the bare word means the newest install"
        );
        assert_eq!(store.live_versions_of("uptime"), ["0.1.0", "0.2.0"]);

        // **A rebuild claiming a live version string is a second row, never a replacement** (the
        // bytes differ, and the digest is the key). The pointer moves to it, because it is the
        // newest install.
        store.install(row("uptime", "uptime", "0.1.0", 5));
        let table = store.table();
        assert_eq!(entries(table).count(), 3, "three rows, one per digest");
        assert!(lookup_version(table, "uptime", "0.1.0").unwrap().is_some());
        assert_eq!(store.bare_version_of("uptime").unwrap(), "0.1.0");

        // Reinstalling the *same* bytes is a relabel, not a new row: the rebuild's row changes
        // version string under its digest, and the original 0.1.0 row still says what it said.
        store.install(row("uptime", "uptime", "0.1.0-rc1", 5));
        let table = store.table();
        assert_eq!(entries(table).count(), 3, "same digest, same row");
        assert_eq!(
            lookup_version(table, "uptime", "0.1.0")
                .unwrap()
                .unwrap()
                .digest,
            [1; 32],
            "the original row is untouched"
        );
        assert!(
            lookup_version(table, "uptime", "0.1.0-rc1")
                .unwrap()
                .is_some()
        );
    }

    /// **The version-qualified removal, ruling 5**: removing one version leaves the other; removing
    /// the default's version moves the pointer to the sole survivor; and with several remaining the
    /// removal refuses and [`versions_of`] names the candidates. Removing a version that does not
    /// hold the pointer leaves the pointer alone.
    #[test]
    fn a_version_qualified_remove_honours_the_pointer_rule() {
        let mut store = Store::new();
        store.install(row("uptime", "uptime", "0.1.0", 1));
        store.install(row("uptime", "uptime", "0.2.0", 2));

        // A version that does not hold the pointer: the row goes, the pointer stays.
        store.remove_version("uptime", "0.1.0").unwrap();
        assert_eq!(store.bare_version_of("uptime").unwrap(), "0.2.0");
        assert!(
            lookup_version(store.table(), "uptime", "0.1.0")
                .unwrap()
                .is_none()
        );
        store.install(row("uptime", "uptime", "0.1.0", 1));

        // The default's version, one survivor: the pointer moves to it.
        store.remove_version("uptime", "0.2.0").unwrap();
        assert_eq!(store.bare_version_of("uptime").unwrap(), "0.1.0");

        // The default's version, several survivors: refused, candidates named, nothing written.
        store.install(row("uptime", "uptime", "0.3.0", 3));
        store.install(row("uptime", "uptime", "0.4.0", 4));
        assert_eq!(store.bare_version_of("uptime").unwrap(), "0.4.0");
        let newest = store.newest();
        let mut out = [0u8; 4096];
        assert_eq!(
            without_version(store.table(), "uptime", "0.4.0", &mut out),
            Err(Error::SeveralVersions)
        );
        assert_eq!(store.newest(), newest, "a refusal writes no generation");
        assert_eq!(
            store.live_versions_of("uptime"),
            ["0.1.0", "0.3.0", "0.4.0"],
            "the candidates the refusal names"
        );

        // A version nobody installed is a report, not a silence.
        assert_eq!(
            store.remove_version("uptime", "9.9.9"),
            Err(Error::NotInstalled)
        );
    }

    /// **Bare removal at two versions live, milestone 614's second Done-means test**:
    /// `jig remove <program>` takes every live version's row and the pointer, and a rollback
    /// brings both rows back, because bytes are never deleted.
    #[test]
    fn bare_removal_takes_every_version_and_a_rollback_brings_them_back() {
        let mut store = Store::new();
        store.install(row("uptime", "uptime", "0.1.0", 1));
        store.install(row("uptime", "uptime", "0.2.0", 2));
        let before = store.current();

        store.remove("uptime").unwrap();
        assert_eq!(
            store.live_versions_of("uptime"),
            std::vec::Vec::<String>::new()
        );
        assert_eq!(store.bare_version_of("uptime"), None);
        assert_eq!(defaults(store.table()).count(), 0);

        store.select(before);
        assert_eq!(store.live_versions_of("uptime"), ["0.1.0", "0.2.0"]);
        assert_eq!(store.bare_version_of("uptime").unwrap(), "0.2.0");
    }

    /// Two rows at one version string (a rebuild) are one *version*, so a qualified removal that
    /// leaves only them moves the pointer to the first of them rather than refusing: the refusal's
    /// candidates are versions, and there is one.
    #[test]
    fn a_rebuild_is_one_survivor_for_the_pointer_rule() {
        let mut store = Store::new();
        store.install(row("uptime", "uptime", "0.1.0", 1));
        // A rebuild of 0.1.0: a second row at the same version string (ruling 2).
        store.install(row("uptime", "uptime", "0.1.0", 5));
        store.install(row("uptime", "uptime", "0.2.0", 2));
        assert_eq!(store.bare_version_of("uptime").unwrap(), "0.2.0");

        store.remove_version("uptime", "0.2.0").unwrap();
        // The pointer's version went; two rows remain and they are one version, so the pointer
        // moves to the first of them instead of refusing.
        assert_eq!(store.live_versions_of("uptime"), ["0.1.0", "0.1.0"]);
        assert_eq!(
            store.bare_version_of("uptime").unwrap(),
            "0.1.0",
            "the sole remaining version"
        );
        assert_eq!(
            lookup(store.table(), "uptime").unwrap().unwrap().digest,
            [1; 32],
            "the first row of the surviving version"
        );
    }

    /// Two programs installed, one upgraded beside itself, one removed; selecting the generation
    /// before both changes brings back the old version of the first and the presence of the second
    /// together. The pointer rides along: a generation is one snapshot of rows and pointer
    /// (milestone 614, ruling 5).
    #[test]
    fn a_rollback_restores_the_whole_set() {
        let mut store = Store::new();
        store.install(row("uptime", "uptime", "0.1.0", 1));
        store.install(row("date", "date", "1.0.0", 2));
        let before = store.current();

        store.install(row("uptime", "uptime", "0.2.0", 3));
        store.remove("date").unwrap();
        assert_eq!(store.bare_version_of("uptime").unwrap(), "0.2.0");
        assert_eq!(store.bare_version_of("date"), None);

        store.select(before);
        assert_eq!(store.bare_version_of("uptime").unwrap(), "0.1.0");
        assert_eq!(store.bare_version_of("date").unwrap(), "1.0.0");
        // And rolling forward again is the same act.
        let newest = store.newest();
        store.select(newest);
        assert_eq!(store.bare_version_of("date"), None);
    }

    /// An install appends; the pointer names the newest. (Until milestone 614 this test asserted
    /// the opposite, that an upgrade replaced in place, which is what one entry per name could do.)
    #[test]
    fn an_install_appends_and_the_pointer_follows_the_newest() {
        let mut store = Store::new();
        store.install(row("a", "a", "1", 1));
        store.install(row("b", "b", "1", 2));
        store.install(row("a", "a", "2", 3));
        let programs: std::vec::Vec<_> =
            entries(store.table()).map(|e| e.unwrap().program).collect();
        assert_eq!(programs, ["a", "b", "a"]);
        assert_eq!(lookup(store.table(), "a").unwrap().unwrap().digest, [3; 32]);
        assert_eq!(
            lookup_version(store.table(), "a", "1")
                .unwrap()
                .unwrap()
                .digest,
            [1; 32]
        );
    }

    #[test]
    fn removing_what_is_not_installed_is_reported_and_writes_nothing() {
        let mut store = Store::new();
        store.install(row("a", "a", "1", 1));
        let newest = store.newest();
        assert_eq!(store.remove("b"), Err(Error::NotInstalled));
        assert_eq!(store.newest(), newest);
    }

    #[test]
    fn a_malformed_line_anywhere_makes_the_table_vouch_for_nothing() {
        let good = format!("sha256:{} a 1 a\n", "11".repeat(32));
        let pointer = format!("{good}default a sha256:{}\n", "11".repeat(32));
        // The bare name needs its pointer: rows without one answer `None`, never a guess.
        assert!(lookup(&good, "a").unwrap().is_none());
        assert!(lookup(&pointer, "a").unwrap().is_some());
        for bad in [
            // A bad row, after a good one.
            format!("{good}b 2 b nothex\n"),
            format!("{good}b 2 b\n"),
            format!("{good}b 2 b {} extra\n", "22".repeat(32)),
            format!("{good}b 2 b {}\n", "2".repeat(63)),
            // A digest with no label, as every row was written before 2026-10-07, and one under a
            // label this reader does not know (DECISIONS §197's digest ruling).
            format!("{good}{} b 2 b\n", "22".repeat(32)),
            format!("{good}merkle-sha256-8k:{} b 2 b\n", "22".repeat(32)),
            format!("{good}default a {}\n", "11".repeat(32)),
            // A bad pointer line.
            format!("{good}default a\n"),
            format!("{good}default a nothex\n"),
            format!("{good}default a {} extra\n", "22".repeat(32)),
            // A pointer naming a digest no row of the program carries.
            format!("{good}default a {}\n", "33".repeat(32)),
        ] {
            assert_eq!(lookup(&bad, "a"), Err(Error::Malformed), "{bad:?}");
        }
        // Comments and blank lines are not rows, and the pointer survives them.
        let commented = format!("# installed\n\n{pointer}");
        assert!(lookup(&commented, "a").unwrap().is_some());
        // Two pointers for one program is no single answer.
        let twice = format!("{pointer}default a {}\n", "33".repeat(32));
        assert_eq!(lookup(&twice, "a"), Err(Error::Malformed));
    }

    #[test]
    fn a_name_that_would_not_read_back_is_refused() {
        let mut out = [0u8; 512];
        for (program, version, package) in [
            ("", "1", "p"),
            ("a b", "1", "p"),
            ("a", "", "p"),
            ("a", "1 2", "p"),
            ("a", "1", "p\nq"),
            ("#a", "1", "p"),
        ] {
            let e = Entry {
                program,
                version,
                package,
                digest: [0; 32],
                manager: "jig",
            };
            assert_eq!(
                with_entry("", &e, false, &mut out),
                Err(Error::BadName),
                "{program:?} {version:?}"
            );
        }
    }

    /// **A row records its manager, and a row from before the column reads as unrecorded**
    /// (milestone 809, calef's 2026-10-06 ruling). The four-word row is what every table on a disk
    /// written before this change holds, so refusing it would make each one unreadable.
    #[test]
    fn a_row_records_its_manager_and_an_older_row_reads_as_unrecorded() {
        let mut out = [0u8; 512];
        let n = with_entry(
            "",
            &row("greeting", "greeting", "0.1.0", 3),
            false,
            &mut out,
        )
        .unwrap();
        let table = core::str::from_utf8(&out[..n]).unwrap();
        assert!(table.lines().next().unwrap().ends_with(" greeting jig"));
        assert_eq!(lookup(table, "greeting").unwrap().unwrap().manager, "jig");
        let older = format!("sha256:{} a 1 a\n", "11".repeat(32));
        assert_eq!(entries(&older).next().unwrap().unwrap().manager, NO_MANAGER);
        let six = format!("sha256:{} a 1 a jig extra\n", "11".repeat(32));
        assert_eq!(entries(&six).next().unwrap(), Err(Error::Malformed));
    }

    /// **A manager may not undo another manager's rows** (calef, 2026-10-06), and the owner may
    /// undo anything. `foreign_change` is what a rollback asks of the two generations it moves
    /// between, and `may_edit` what a removal asks of the row.
    #[test]
    fn a_manager_may_not_undo_another_managers_rows() {
        assert!(may_edit("jig", "jig"));
        assert!(may_edit(OWNER, "jig"));
        assert!(may_edit(NO_MANAGER, "jig"));
        assert!(!may_edit("other", "jig"));
        assert!(may_edit("other", OWNER));
        let mut a = [0u8; 512];
        let n = with_entry("", &row("greeting", "greeting", "0.1.0", 3), false, &mut a).unwrap();
        let older = core::str::from_utf8(&a[..n]).unwrap();
        let theirs = Entry {
            manager: "other",
            ..row("hello", "hello", "1", 4)
        };
        let mut b = [0u8; 512];
        let n = with_entry(older, &theirs, false, &mut b).unwrap();
        let newer = core::str::from_utf8(&b[..n]).unwrap();
        assert_eq!(
            foreign_change(older, newer, "jig")
                .unwrap()
                .unwrap()
                .program,
            "hello"
        );
        assert_eq!(foreign_change(older, newer, OWNER).unwrap(), None);
        assert_eq!(foreign_change(older, newer, "other").unwrap(), None);
        // The same generation twice changes nothing.
        assert_eq!(foreign_change(newer, newer, "jig").unwrap(), None);
    }

    #[test]
    fn a_buffer_too_small_is_refused_rather_than_truncated() {
        let mut out = [0u8; 40];
        let e = row("uptime", "uptime", "0.1.0", 1);
        assert_eq!(with_entry("", &e, false, &mut out), Err(Error::TooSmall));
        assert_eq!(format_current(1234, &mut [0u8; 4]), Err(Error::TooSmall));
    }

    /// **A digest finds its row, and a malformed line elsewhere still vouches for nothing**
    /// (§219 D). The miss is a real `None` rather than an error, because "not installed" is the
    /// ordinary answer for a binary somebody just built.
    #[test]
    fn a_digest_finds_its_row_and_a_miss_is_none() {
        let mut out = [0u8; 512];
        let n = with_entry("", &row("uptime", "uptime", "0.1.0", 3), false, &mut out).unwrap();
        let mut two = [0u8; 512];
        let text = core::str::from_utf8(&out[..n]).unwrap();
        let n = with_entry(text, &row("date", "date", "0.1.0", 4), false, &mut two).unwrap();
        let table = core::str::from_utf8(&two[..n]).unwrap();

        let hit = lookup_digest(table, &[4u8; 32]).unwrap().unwrap();
        assert_eq!(hit.program, "date");
        assert!(lookup_digest(table, &[9u8; 32]).unwrap().is_none());

        let mut broken = String::from(table);
        broken.push_str("not a line\n");
        assert_eq!(lookup_digest(&broken, &[4u8; 32]), Err(Error::Malformed));
    }

    /// The row a version set resolves to: first in file order, never a vouch, never another
    /// program's.
    #[test]
    fn a_version_finds_its_row() {
        let mut out = [0u8; 512];
        let n = with_entry("", &row("uptime", "uptime", "0.1.0", 1), false, &mut out).unwrap();
        let text = core::str::from_utf8(&out[..n]).unwrap().to_string();
        let mut two = [0u8; 512];
        let n = with_entry(&text, &row("uptime", "uptime", "0.2.0", 2), false, &mut two).unwrap();
        let text = core::str::from_utf8(&two[..n]).unwrap().to_string();
        let mut three = [0u8; 512];
        let n = with_entry(&text, &vouch("uptime", 7), false, &mut three).unwrap();
        let table = core::str::from_utf8(&three[..n]).unwrap();
        assert_eq!(
            lookup_version(table, "uptime", "0.1.0")
                .unwrap()
                .unwrap()
                .digest,
            [1; 32]
        );
        assert_eq!(
            lookup_version(table, "uptime", "0.2.0")
                .unwrap()
                .unwrap()
                .digest,
            [2; 32]
        );
        assert_eq!(lookup_version(table, "uptime", "0.3.0").unwrap(), None);
        // Neither is the vouch's own column, which is not a version anyone asked for.
        assert_eq!(lookup_version(table, "uptime", NO_VERSION).unwrap(), None);
        assert_eq!(lookup_version(table, "date", "0.1.0").unwrap(), None);
        assert_eq!(versions_of(table, "uptime").count(), 2);
        assert_eq!(versions_of(table, "date").count(), 0);
    }

    #[test]
    fn a_generation_is_named_by_its_number() {
        let mut buf = [0u8; 10];
        assert_eq!(generation_name(0, &mut buf), "0");
        assert_eq!(generation_name(17, &mut buf), "17");
        assert_eq!(generation_name(u32::MAX, &mut buf), "4294967295");
    }

    #[test]
    fn the_current_line_reads_back() {
        for n in [0, 7, 10, 4_294_967_295] {
            let mut out = [0u8; 16];
            let len = format_current(n, &mut out).unwrap();
            assert_eq!(
                parse_current(core::str::from_utf8(&out[..len]).unwrap()),
                Some(n)
            );
        }
        for bad in ["", "\n", "-1", "1 ", "4294967296", "0x1"] {
            assert_eq!(parse_current(bad), None, "{bad:?}");
        }
    }

    /// A row is refused for any one bad name, not only when all three are bad: each of program,
    /// version and package is checked on its own.
    #[test]
    fn a_row_with_one_bad_name_is_malformed() {
        let hex = format!("sha256:{}", "22".repeat(32));
        for bad in [
            format!("{hex} #p 1 pkg\n"),
            format!("{hex} p #1 pkg\n"),
            format!("{hex} p 1 #pkg\n"),
        ] {
            assert_eq!(lookup(&bad, "p"), Err(Error::Malformed), "{bad:?}");
        }
        assert!(
            lookup_digest(&format!("{hex} p 1 pkg\n"), &[0x22; 32])
                .unwrap()
                .is_some()
        );
    }

    /// `parse_current` takes decimal digits only: `u32::from_str` would also take a leading `+`.
    #[test]
    fn a_current_line_with_a_plus_sign_is_not_a_generation() {
        assert_eq!(parse_current("+5"), None);
        assert_eq!(parse_current("+5\n"), None);
        assert_eq!(parse_current("5\n"), Some(5));
    }

    /// Two default pointers for one program are no single answer, and a removal that needs the
    /// pointer's digest says so rather than reading the last one.
    #[test]
    fn removing_a_version_from_a_table_with_two_pointers_is_malformed() {
        let (a, b) = ("11".repeat(32), "22".repeat(32));
        let table = format!("{a} p 1 pkg\n{b} p 2 pkg\ndefault p {a}\ndefault p {b}\n");
        let mut out = [0u8; 512];
        assert_eq!(
            without_version(&table, "p", "1", &mut out),
            Err(Error::Malformed)
        );
    }

    /// Removing one program's version looks only at that program's rows: another program, and
    /// the owner's vouch for this one, are not survivors, so they cannot make the removal
    /// ambiguous or take the pointer.
    #[test]
    fn other_rows_are_not_survivors_of_a_qualified_removal() {
        let mut store = Store::new();
        store.install(row("date", "date", "1.0.0", 9));
        store.install(vouch("uptime", 7));
        store.install(row("uptime", "uptime", "0.1.0", 1));
        store.install(row("uptime", "uptime", "0.2.0", 2));
        store.remove_version("uptime", "0.2.0").unwrap();
        assert_eq!(store.bare_version_of("uptime").unwrap(), "0.1.0");
        assert_eq!(store.bare_version_of("date").unwrap(), "1.0.0");
        assert!(
            lookup_digest(store.table(), &[7; 32]).unwrap().is_some(),
            "the vouch stays"
        );
    }

    /// Removing a version that does not hold the pointer leaves the pointer exactly where it was,
    /// even when the first survivor is some other version; and another program's pointer is never
    /// the removal's to move.
    #[test]
    fn a_removal_moves_only_its_own_programs_pointer_and_only_when_it_held_it() {
        let mut store = Store::new();
        store.install(row("date", "date", "1.0.0", 9));
        store.install(row("uptime", "uptime", "0.1.0", 1));
        store.install(row("uptime", "uptime", "0.2.0", 2));
        store.install(row("uptime", "uptime", "0.3.0", 3));
        store.remove_version("uptime", "0.1.0").unwrap();
        assert_eq!(store.bare_version_of("uptime").unwrap(), "0.3.0");
        assert_eq!(store.bare_version_of("date").unwrap(), "1.0.0");

        // The pointer's own version goes, and a second program's pointer sits after it in the file.
        store.install(row("zed", "zed", "1", 8));
        store.remove_version("uptime", "0.3.0").unwrap();
        assert_eq!(store.bare_version_of("uptime").unwrap(), "0.2.0");
        assert_eq!(store.bare_version_of("zed").unwrap(), "1");
        assert_eq!(store.bare_version_of("date").unwrap(), "1.0.0");
    }

    /// A qualified removal takes one row. Two rows sharing a digest (a hand-written table; an
    /// install relabels instead) are not both taken because they agree on it.
    #[test]
    fn a_qualified_removal_takes_one_row_even_when_a_digest_repeats() {
        let a = format!("sha256:{}", "11".repeat(32));
        let table = format!("{a} p 1 pkg\n{a} p 2 pkg\ndefault p {a}\n");
        let mut out = [0u8; 512];
        let n = without_version(&table, "p", "2", &mut out).unwrap();
        let after = core::str::from_utf8(&out[..n]).unwrap();
        assert_eq!(versions_of(after, "p").count(), 1);
        assert!(lookup_version(after, "p", "1").unwrap().is_some());
    }
}
