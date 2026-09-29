//! **A program's manifest, as the bytes of an ELF note it carries** (milestone 597 (a program
//! carries its manifest in an ELF note), whose number is provisional).
//!
//! DECISIONS §197 (a package is one archive file) asked where a program's manifest travels, and
//! calef ruled option M2 on 2026-09-26 (UTC): inside the executable, as a note found through a
//! `PT_NOTE` program header. So the manifest is in the same bytes the progenitor hashes to decide
//! whether anybody vouched for them, and it cannot drift from the code or be separated from it.
//! This crate is the wire format two programs agree on: the program that carries a note (through
//! [`carry!`]) and the two that read one (the shell, which binds a line against it, and the
//! progenitor, which endows from it). AGENTS.md rule 7 is why it is a crate.
//!
//! Finding the note is `crates/elf`'s job (`elf::Elf::note` and its streaming half, `elf::NoteSearch`). This crate
//! starts at the descriptor.
//!
//! # The note
//!
//! | | |
//! |---|---|
//! | owner | [`OWNER`], `nife` (with its NUL, `namesz` 5) |
//! | type | [`MANIFEST`], `1`, "manifest" |
//! | descriptor | [`DESCRIPTOR_LEN`] bytes, the layout below |
//! | section | `.note.nife.manifest`, which `crates/user_mode_runtime/link.ld` keeps in a `PT_NOTE` inside the read-only load segment |
//!
//! # The descriptor, version 1
//!
//! A version word, then little-endian fields in a fixed order (the encoding calef ratified
//! 2026-09-26). Every field is one of `grant_plan::Manifest`'s, in bytes; nothing here is a new
//! authority.
//!
//! | offset | size | field | values |
//! |---|---|---|---|
//! | 0 | 4 | version | `1` |
//! | 4 | 1 | `arg` | 0 forbidden, 1 required; hears words (milestone 205): 2 read-only, 3 read-write, 4 create |
//! | 5 | 1 | `mem` | 0 forbidden, 1 required |
//! | 6 | 1 | `file` | 0 forbidden, 1 read-only, 2 read-write |
//! | 7 | 1 | `dir` | 0 forbidden, 1 required |
//! | 8 | 8 | `mem` minimum pages | 0 unless `mem` is required |
//! | 16 | 8 | `mem` maximum pages | 0 unless `mem` is required; at least the minimum |
//! | 24 | 1 | `output` | 0 silent, 1 words, 2 bytes, 3 bytes and diagnostics (at `grant_plan::DIAGNOSTICS_SLOT`) |
//! | 25 | 1 | `input` | 0 forbidden, 1 required |
//! | 26 | 1 | `input` writes while reading | 0 or 1; 0 unless `input` is required |
//! | 27 | 1 | `dir`'s subtree option | 0 for none, else one of the declared option letters; 0 unless `dir` is required |
//! | 28 | 1 | `reports` | 0 or 1 |
//! | 29 | 1 | `interruptible` | 0 or 1 |
//! | 30 | 1 | `clock` | 0 or 1 |
//! | 31 | 1 | `domain` | 0 or 1 |
//! | 32 | 1 | `config` | 0 or 1 |
//! | 33 | 1 | `entropy` | 0 or 1 |
//! | 34 | 1 | `network` | 0 or 1 |
//! | 35 | 1 | `runtime` | 0 native, 1 std |
//! | 36 | 1 | option count | at most `grant_plan::MAX_DECLARED_FLAGS` (16) |
//! | 37 | 16 | option letters | the first *count* are the letters in bit order, the rest 0 |
//! | 53 | 3 | zero | |
//!
//! **One manifest has exactly one encoding**, and [`decode`] enforces it: an unknown version, a
//! value outside its field's range, a nonzero byte where the layout says zero, a descriptor shorter
//! or longer than [`DESCRIPTOR_LEN`], all refuse. That is what lets two readers not disagree: if
//! `decode` accepts bytes, `encode` of the answer is those bytes again
//! (`verification::a_decoded_manifest_encodes_back_to_its_bytes`), so there is no second spelling
//! for one of them to read differently. A later version is a new version word, not a tolerated
//! extension of this one.
//!
//! **Version 1 was amended once, in place**, on 2026-09-27 (UTC): the `arg` field gained `2` for
//! milestone 205 (how a foreign program is told what to do)'s `ArgSpec::Words`, with no version
//! bump, on calef's ruling ("I think an incompatible change is probably fine. It has just been a
//! few hours."). Nothing outside this tree had acted on version 1 by then.
//!
//! **And a second time, later the same day**: `3` and `4` for the `WordGrant` a program that hears
//! words declares (milestone 205's designation half), under the same ruling and before anything
//! outside the tree had acted on it. `2` kept its meaning, read-only. calef confirmed the ruling
//! covers these on 2026-09-27 at 15:14Z (UTC), on #1402. The rule above holds from here on.
//!
//! # EXAMPLES
//!
//! ```
//! let m = grant_plan::Prog::Uptime.manifest();
//! let bytes = manifest_note::encode(&m);
//! assert_eq!(manifest_note::decode(&bytes), Ok(m));
//!
//! let mut later = bytes;
//! later[0] = 2;
//! assert_eq!(manifest_note::decode(&later), Err(manifest_note::Error::UnknownVersion));
//! ```
//!
//! A program carries its note with one line at module scope:
//!
//! ```ignore
//! manifest_note::carry!(grant_plan::Prog::Uptime.manifest());
//! ```
//!
//! # BUGS
//!
//! - **A foreign build carries a note only by linking an object that holds one.** #1319 measured
//!   `-Clink-arg=note.o` working and `llvm-objcopy --add-section` not: it adds a section with no
//!   program header, which a program-header reader cannot see. Nothing in the tree writes that
//!   object for a foreign program yet.
//! - **Changing a manifest means relinking**, and a new digest. That is M2's recorded cost, and
//!   §197 has why it was accepted.
//!
//! Name: provisional, milestone 597's lane, 2026-09-26. The owner string, the type number and the
//! encoding are ratified; the crate name, [`carry!`] and the public functions are not.

#![no_std]

use grant_plan::{
    ArgSpec, DIAGNOSTICS_SLOT, DirSpec, FileSpec, Flags, InputSpec, MAX_DECLARED_FLAGS, Manifest,
    MemSpec, OutputSpec, Runtime, WordGrant,
};

/// **The note's owner string**, without its NUL: the project's name, lowercase as
/// `design/naming.md` rules it spelled everywhere.
///
/// Name: ratified 2026-09-26 (calef, answering "Yes" to the maintainer, who relayed it to milestone 597's lane).
pub const OWNER: &[u8] = b"nife";

/// **The note type that carries a manifest**: `1`, "manifest".
///
/// Types are per owner, so `1` collides with nothing outside `nife`'s own notes.
///
/// Name: ratified 2026-09-26 (calef, answering "Yes" to the maintainer, who relayed it to milestone 597's lane).
pub const MANIFEST: u32 = 1;

/// The descriptor version this crate writes and the only one it reads. The encoding it numbers (a
/// version word, then little-endian fields in a fixed order) is calef's ruling of 2026-09-26; the
/// field order of version 1 is milestone 597's lane's design, in the module documentation.
pub const VERSION: u32 = 1;

/// How long a version 1 descriptor is, exactly.
pub const DESCRIPTOR_LEN: usize = 56;

/// `OWNER` with its NUL, padded to the note's 4-byte alignment.
const NAME_FIELD: usize = 8;

/// The note's three header words.
const HEADER_LEN: usize = 12;

/// How long a whole manifest note is: header, owner, descriptor. Every field is already a multiple
/// of four, so there is no trailing pad.
pub const NOTE_LEN: usize = HEADER_LEN + NAME_FIELD + DESCRIPTOR_LEN;

/// Why a descriptor was refused. Provisional names.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Error {
    /// Fewer than four bytes, so not even a version.
    NoVersion,
    /// A version this reader does not know. A later layout is refused rather than guessed at.
    UnknownVersion,
    /// A version 1 descriptor that is not exactly [`DESCRIPTOR_LEN`] bytes: cut short, or with
    /// bytes after the last field.
    WrongLength,
    /// A field holds a value outside its range, or a byte the layout says is zero is not. The
    /// offset is the first such byte.
    BadField(usize),
}

// Offsets, named once so `encode` and `decode` cannot disagree about where a field is.
const ARG: usize = 4;
const MEM: usize = 5;
const FILE: usize = 6;
const DIR: usize = 7;
const MEM_MIN: usize = 8;
const MEM_MAX: usize = 16;
const OUTPUT: usize = 24;
const INPUT: usize = 25;
const WRITES_WHILE_READING: usize = 26;
const SUBTREE: usize = 27;
const REPORTS: usize = 28;
const INTERRUPTIBLE: usize = 29;
const CLOCK: usize = 30;
const DOMAIN: usize = 31;
const CONFIG: usize = 32;
const ENTROPY: usize = 33;
const NETWORK: usize = 34;
const RUNTIME: usize = 35;
const FLAG_COUNT: usize = 36;
const FLAG_LETTERS: usize = 37;
const TAIL: usize = FLAG_LETTERS + MAX_DECLARED_FLAGS;

const _: () = assert!(TAIL + 3 == DESCRIPTOR_LEN);

const fn put8(out: &mut [u8; DESCRIPTOR_LEN], at: usize, v: u64) {
    let b = v.to_le_bytes();
    let mut i = 0;
    while i < 8 {
        out[at + i] = b[i];
        i += 1;
    }
}

/// **`m` as a version 1 descriptor.**
///
/// A `const fn`, so a program's note is computed by the compiler from the same constant its source
/// declares ([`carry!`]). It panics on the two manifests the layout has no spelling for, and in a
/// constant that panic is a compile error rather than a note nobody can read: a declared second
/// stream at a slot other than `grant_plan::DIAGNOSTICS_SLOT`, and a subtree option that is not
/// one of the declared letters (or is declared without a directory grant).
pub const fn encode(m: &Manifest) -> [u8; DESCRIPTOR_LEN] {
    let mut out = [0u8; DESCRIPTOR_LEN];
    let v = VERSION.to_le_bytes();
    out[0] = v[0];
    out[1] = v[1];
    out[2] = v[2];
    out[3] = v[3];
    out[ARG] = match m.arg {
        ArgSpec::Forbidden => 0,
        ArgSpec::Required => 1,
        // A program whose line is its argv (milestone 205 (how a foreign program is told what to
        // do), §170 (how a foreign program is told what to do)). Added to version 1 in place on 2026-09-27; see the module's "one encoding"
        // paragraph for calef's ruling.
        ArgSpec::Words(WordGrant::ReadOnly) => 2,
        ArgSpec::Words(WordGrant::ReadWrite) => 3,
        ArgSpec::Words(WordGrant::Create) => 4,
    };
    if let MemSpec::Required { min, max } = m.mem {
        out[MEM] = 1;
        put8(&mut out, MEM_MIN, min);
        put8(&mut out, MEM_MAX, max);
    }
    out[FILE] = match m.file {
        FileSpec::Forbidden => 0,
        FileSpec::Required { writable: false } => 1,
        FileSpec::Required { writable: true } => 2,
    };
    let letters = m.flags.letters();
    if let DirSpec::Required { subtree_flag } = m.dir {
        out[DIR] = 1;
        if let Some(letter) = subtree_flag {
            let mut i = 0;
            let mut declared = false;
            while i < letters.len() {
                declared |= letters[i] == letter;
                i += 1;
            }
            assert!(
                declared,
                "a subtree option must be one of the declared letters"
            );
            out[SUBTREE] = letter;
        }
    }
    out[OUTPUT] = match m.output {
        OutputSpec::Silent => 0,
        OutputSpec::Words => 1,
        OutputSpec::Bytes => 2,
        OutputSpec::BytesAndDiagnostics { slot } => {
            assert!(
                slot == DIAGNOSTICS_SLOT,
                "a second stream lands at grant_plan::DIAGNOSTICS_SLOT"
            );
            3
        }
    };
    if let InputSpec::Required {
        writes_while_reading,
    } = m.input
    {
        out[INPUT] = 1;
        out[WRITES_WHILE_READING] = writes_while_reading as u8;
    }
    out[REPORTS] = m.reports as u8;
    out[INTERRUPTIBLE] = m.interruptible as u8;
    out[CLOCK] = m.clock as u8;
    out[DOMAIN] = m.domain as u8;
    out[CONFIG] = m.config as u8;
    out[ENTROPY] = m.entropy as u8;
    out[NETWORK] = m.network as u8;
    out[RUNTIME] = match m.runtime {
        Runtime::Native => 0,
        Runtime::Std => 1,
    };
    out[FLAG_COUNT] = letters.len() as u8;
    let mut i = 0;
    while i < letters.len() {
        out[FLAG_LETTERS + i] = letters[i];
        i += 1;
    }
    out
}

fn get8(d: &[u8], at: usize) -> u64 {
    let mut b = [0u8; 8];
    b.copy_from_slice(&d[at..at + 8]);
    u64::from_le_bytes(b)
}

/// A byte that must be 0 or 1.
fn flag(d: &[u8], at: usize) -> Result<bool, Error> {
    match d[at] {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::BadField(at)),
    }
}

/// A byte that must be zero, because the field it belongs to is absent.
fn zero(d: &[u8], at: usize, len: usize) -> Result<(), Error> {
    match d[at..at + len].iter().position(|&b| b != 0) {
        Some(i) => Err(Error::BadField(at + i)),
        None => Ok(()),
    }
}

/// **The manifest a version 1 descriptor spells**, or why it spells none.
///
/// Strict on purpose, for the module documentation's reason: whatever this accepts, [`encode`]
/// turns back into the same bytes, so a descriptor has one reading.
pub fn decode(d: &[u8]) -> Result<Manifest, Error> {
    if d.len() < 4 {
        return Err(Error::NoVersion);
    }
    if u32::from_le_bytes([d[0], d[1], d[2], d[3]]) != VERSION {
        return Err(Error::UnknownVersion);
    }
    if d.len() != DESCRIPTOR_LEN {
        return Err(Error::WrongLength);
    }
    let arg = match d[ARG] {
        0 => ArgSpec::Forbidden,
        1 => ArgSpec::Required,
        2 => ArgSpec::Words(WordGrant::ReadOnly),
        3 => ArgSpec::Words(WordGrant::ReadWrite),
        4 => ArgSpec::Words(WordGrant::Create),
        _ => return Err(Error::BadField(ARG)),
    };
    let mem = match d[MEM] {
        0 => {
            zero(d, MEM_MIN, 16)?;
            MemSpec::Forbidden
        }
        1 => {
            let (min, max) = (get8(d, MEM_MIN), get8(d, MEM_MAX));
            if min > max {
                return Err(Error::BadField(MEM_MIN));
            }
            MemSpec::Required { min, max }
        }
        _ => return Err(Error::BadField(MEM)),
    };
    let file = match d[FILE] {
        0 => FileSpec::Forbidden,
        1 => FileSpec::Required { writable: false },
        2 => FileSpec::Required { writable: true },
        _ => return Err(Error::BadField(FILE)),
    };
    let count = d[FLAG_COUNT] as usize;
    if count > MAX_DECLARED_FLAGS {
        return Err(Error::BadField(FLAG_COUNT));
    }
    let letters = &d[FLAG_LETTERS..FLAG_LETTERS + count];
    let flags = Flags::try_new(letters).ok_or(Error::BadField(FLAG_LETTERS))?;
    zero(d, FLAG_LETTERS + count, MAX_DECLARED_FLAGS - count)?;
    let dir = match d[DIR] {
        0 => {
            zero(d, SUBTREE, 1)?;
            DirSpec::Forbidden
        }
        1 => DirSpec::Required {
            subtree_flag: match d[SUBTREE] {
                0 => None,
                l if letters.contains(&l) => Some(l),
                _ => return Err(Error::BadField(SUBTREE)),
            },
        },
        _ => return Err(Error::BadField(DIR)),
    };
    let output = match d[OUTPUT] {
        0 => OutputSpec::Silent,
        1 => OutputSpec::Words,
        2 => OutputSpec::Bytes,
        3 => OutputSpec::BytesAndDiagnostics {
            slot: DIAGNOSTICS_SLOT,
        },
        _ => return Err(Error::BadField(OUTPUT)),
    };
    let input = match d[INPUT] {
        0 => {
            zero(d, WRITES_WHILE_READING, 1)?;
            InputSpec::Forbidden
        }
        1 => InputSpec::Required {
            writes_while_reading: flag(d, WRITES_WHILE_READING)?,
        },
        _ => return Err(Error::BadField(INPUT)),
    };
    let runtime = match d[RUNTIME] {
        0 => Runtime::Native,
        1 => Runtime::Std,
        _ => return Err(Error::BadField(RUNTIME)),
    };
    zero(d, TAIL, DESCRIPTOR_LEN - TAIL)?;
    Ok(Manifest {
        arg,
        mem,
        file,
        dir,
        flags,
        output,
        input,
        reports: flag(d, REPORTS)?,
        interruptible: flag(d, INTERRUPTIBLE)?,
        clock: flag(d, CLOCK)?,
        domain: flag(d, DOMAIN)?,
        config: flag(d, CONFIG)?,
        entropy: flag(d, ENTROPY)?,
        network: flag(d, NETWORK)?,
        runtime,
    })
}

/// **A whole manifest note**, header and owner and descriptor, aligned as a note must be. What
/// [`carry!`] places in the program's `.note.nife.manifest` section. Provisional name.
#[repr(C, align(4))]
pub struct Note(pub [u8; NOTE_LEN]);

impl Note {
    /// The note carrying `m`.
    pub const fn of(m: &Manifest) -> Self {
        let mut out = [0u8; NOTE_LEN];
        let namesz = ((OWNER.len() + 1) as u32).to_le_bytes();
        let descsz = (DESCRIPTOR_LEN as u32).to_le_bytes();
        let kind = MANIFEST.to_le_bytes();
        let mut i = 0;
        while i < 4 {
            out[i] = namesz[i];
            out[4 + i] = descsz[i];
            out[8 + i] = kind[i];
            out[HEADER_LEN + i] = OWNER[i];
            i += 1;
        }
        let d = encode(m);
        let mut i = 0;
        while i < DESCRIPTOR_LEN {
            out[HEADER_LEN + NAME_FIELD + i] = d[i];
            i += 1;
        }
        Self(out)
    }
}

// The owner and its NUL fit the name field, which keeps the descriptor 4-aligned.
const _: () = assert!(OWNER.len() < NAME_FIELD && NAME_FIELD.is_multiple_of(4));

/// **Carry this manifest in the program's own bytes.** One line at module scope in a program's
/// root; the argument is any constant expression of type `grant_plan::Manifest`.
///
/// It places one [`Note`] in `.note.nife.manifest`, which `crates/user_mode_runtime/link.ld` keeps
/// in a `PT_NOTE` inside the read-only load segment. Used twice in one program it is a duplicate
/// symbol, and if two notes reached one file anyway the readers refuse the file
/// (`elf::NoteError::Duplicate`).
///
/// Name: provisional, milestone 597's lane, 2026-09-26.
#[macro_export]
macro_rules! carry {
    ($manifest:expr) => {
        #[used]
        #[unsafe(link_section = ".note.nife.manifest")]
        static NIFE_MANIFEST_NOTE: $crate::Note = $crate::Note::of(&$manifest);
    };
}

// -------------------------------------------------------------------------------------------
// The second note type: a file server that enforces subtree grants itself (milestone 606 (a
// directory walk costs what it does on Linux), calef's ruling T1 of 2026-09-27, 15:22Z on #1413).
// -------------------------------------------------------------------------------------------

/// **The note type that says a file server enforces subtree grants itself**: `2`. A server carries
/// it when it resolves every path and every handle through a scope crate, and it is what the
/// progenitor reads to choose a bound badge over a caretaker for that server's grants. Absent means
/// "give its clients a caretaker", which is every server but an eligible one.
///
/// Name and number provisional (calef, 2026-09-27: "the note and field names stay provisional").
pub const SUBTREE_GRANTS: u32 = 2;

/// The version of the [`SUBTREE_GRANTS`] descriptor this crate writes and reads.
pub const SUBTREE_GRANTS_VERSION: u32 = 1;

/// How long a version 1 [`SUBTREE_GRANTS`] descriptor is: a version word, then a scope word.
pub const SUBTREE_GRANTS_LEN: usize = 8;

/// **Which scope crate a server enforces grants through.** One value today, so the word is room for
/// a second rather than a flag that would need a second field.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Scope {
    /// `crates/subtree_scope`, the one ruling D (2026-09-27) names.
    SubtreeScope,
}

impl Scope {
    const fn word(self) -> u32 {
        match self {
            Scope::SubtreeScope => 1,
        }
    }

    /// The package declaration's spelling of this scope, the value of its `subtree_grants` field,
    /// which `script/lint` checks against the note a server carries.
    pub const fn declared(self) -> &'static str {
        match self {
            Scope::SubtreeScope => "subtree_scope",
        }
    }
}

/// **A [`SUBTREE_GRANTS`] descriptor, decoded**, or why not. One encoding for one scope, as
/// [`decode`] enforces for a manifest: an unknown version or scope, or the wrong length, refuses.
pub fn decode_subtree_grants(d: &[u8]) -> Result<Scope, Error> {
    let version = d.get(..4).ok_or(Error::NoVersion)?;
    if u32::from_le_bytes([version[0], version[1], version[2], version[3]])
        != SUBTREE_GRANTS_VERSION
    {
        return Err(Error::UnknownVersion);
    }
    if d.len() != SUBTREE_GRANTS_LEN {
        return Err(Error::WrongLength);
    }
    match u32::from_le_bytes([d[4], d[5], d[6], d[7]]) {
        1 => Ok(Scope::SubtreeScope),
        _ => Err(Error::BadField(4)),
    }
}

/// How long a whole [`SUBTREE_GRANTS`] note is.
pub const SUBTREE_GRANTS_NOTE_LEN: usize = HEADER_LEN + NAME_FIELD + SUBTREE_GRANTS_LEN;

/// **A whole [`SUBTREE_GRANTS`] note**, what [`carry_subtree_grants!`] places. Provisional name.
#[repr(C, align(4))]
pub struct SubtreeGrantsNote(pub [u8; SUBTREE_GRANTS_NOTE_LEN]);

impl SubtreeGrantsNote {
    /// The note declaring `scope`.
    pub const fn of(scope: Scope) -> Self {
        let mut out = [0u8; SUBTREE_GRANTS_NOTE_LEN];
        let namesz = ((OWNER.len() + 1) as u32).to_le_bytes();
        let descsz = (SUBTREE_GRANTS_LEN as u32).to_le_bytes();
        let kind = SUBTREE_GRANTS.to_le_bytes();
        let version = SUBTREE_GRANTS_VERSION.to_le_bytes();
        let word = scope.word().to_le_bytes();
        let mut i = 0;
        while i < 4 {
            out[i] = namesz[i];
            out[4 + i] = descsz[i];
            out[8 + i] = kind[i];
            out[HEADER_LEN + i] = OWNER[i];
            out[HEADER_LEN + NAME_FIELD + i] = version[i];
            out[HEADER_LEN + NAME_FIELD + 4 + i] = word[i];
            i += 1;
        }
        Self(out)
    }
}

/// **Declare that this file server enforces subtree grants through `scope`.** One line at module
/// scope in the server's root, beside nothing else: a server carries no [`Manifest`]. It places one
/// [`SubtreeGrantsNote`] in `.note.nife.manifest`, the same section and `PT_NOTE` a manifest uses.
///
/// Carrying it is a claim the progenitor acts on, so `script/lint` holds it to account: the crate
/// must depend on `subtree_scope`, and its package declaration must say so too.
///
/// Name: provisional, milestone 606's lane, 2026-09-27.
#[macro_export]
macro_rules! carry_subtree_grants {
    ($scope:expr) => {
        #[used]
        #[unsafe(link_section = ".note.nife.manifest")]
        static NIFE_SUBTREE_GRANTS_NOTE: $crate::SubtreeGrantsNote =
            $crate::SubtreeGrantsNote::of($scope);
    };
}

#[cfg(kani)]
mod verification {
    use super::*;

    /// **Whatever `decode` accepts, `encode` spells back byte for byte**, over every descriptor the
    /// solver can choose. This is the "one manifest, one encoding" claim: no two byte strings
    /// decode to one manifest, so no two readers can disagree about which bytes meant what. The
    /// unwind bound is the 56-byte comparison, plus one.
    ///
    /// Falsification: replayable `crates/manifest_note/falsifications/verification.a_decoded_manifest_encodes_back_to_its_bytes.patch`
    #[kani::proof]
    #[kani::unwind(57)]
    fn a_decoded_manifest_encodes_back_to_its_bytes() {
        let d: [u8; DESCRIPTOR_LEN] = kani::any();
        if let Ok(m) = decode(&d) {
            assert!(encode(&m) == d);
        }
    }

    /// **`decode` never panics, at any length up to one past the layout.** The bytes come from a
    /// file anyone could have written.
    ///
    /// Falsification: replayable `crates/manifest_note/falsifications/verification.decode_never_panics.patch`
    #[kani::proof]
    #[kani::unwind(18)]
    fn decode_never_panics() {
        let d: [u8; DESCRIPTOR_LEN + 1] = kani::any();
        let len: usize = kani::any();
        kani::assume(len <= d.len());
        let _ = decode(&d[..len]);
    }
}

#[cfg(test)]
mod tests {
    use grant_plan::Prog;

    use super::*;

    /// **The subtree-grants note round-trips, and one encoding is the only encoding** (milestone
    /// 606, ruling T1): the note's own descriptor decodes to its scope, and a wrong version, scope
    /// or length is refused rather than read as "eligible".
    #[test]
    fn a_subtree_grants_note_has_one_encoding() {
        let note = SubtreeGrantsNote::of(Scope::SubtreeScope);
        let desc = &note.0[HEADER_LEN + NAME_FIELD..];
        assert_eq!(decode_subtree_grants(desc), Ok(Scope::SubtreeScope));
        assert_eq!(
            u32::from_le_bytes(note.0[8..12].try_into().unwrap()),
            SUBTREE_GRANTS
        );
        let mut bad = [0u8; SUBTREE_GRANTS_LEN];
        bad.copy_from_slice(desc);
        bad[4] = 2;
        assert_eq!(decode_subtree_grants(&bad), Err(Error::BadField(4)));
        bad[4] = 1;
        bad[0] = 2;
        assert_eq!(decode_subtree_grants(&bad), Err(Error::UnknownVersion));
        assert_eq!(decode_subtree_grants(&desc[..7]), Err(Error::WrongLength));
        assert_eq!(decode_subtree_grants(&[1, 0]), Err(Error::NoVersion));
        assert_eq!(Scope::SubtreeScope.declared(), "subtree_scope");
    }

    /// Every manifest the tree compiles in has a spelling, and reads back as itself. A program
    /// added to `Prog` with a manifest this layout cannot carry fails here rather than in a build
    /// that tries to carry it.
    #[test]
    fn every_compiled_in_manifest_round_trips() {
        for p in Prog::ALL {
            let m = p.manifest();
            assert_eq!(decode(&encode(&m)), Ok(m), "{}", p.name());
        }
        let m = grant_plan::UNVOUCHED_MANIFEST;
        assert_eq!(decode(&encode(&m)), Ok(m));
        // Each word grant a program that hears words may declare (milestone 205).
        for g in [WordGrant::ReadOnly, WordGrant::ReadWrite, WordGrant::Create] {
            let m = Manifest {
                arg: ArgSpec::Words(g),
                ..grant_plan::UNVOUCHED_STD_MANIFEST
            };
            assert_eq!(decode(&encode(&m)), Ok(m), "{g:?}");
        }
    }

    #[test]
    fn a_later_version_is_refused_rather_than_guessed_at() {
        let mut d = encode(&Prog::Uptime.manifest());
        d[0] = 2;
        assert_eq!(decode(&d), Err(Error::UnknownVersion));
        assert_eq!(decode(&d[..3]), Err(Error::NoVersion));
    }

    #[test]
    fn a_descriptor_one_byte_long_or_short_is_refused() {
        let d = encode(&Prog::Uptime.manifest());
        assert_eq!(decode(&d[..DESCRIPTOR_LEN - 1]), Err(Error::WrongLength));
        let mut long = [0u8; DESCRIPTOR_LEN + 1];
        long[..DESCRIPTOR_LEN].copy_from_slice(&d);
        assert_eq!(decode(&long), Err(Error::WrongLength));
    }

    #[test]
    fn every_field_refuses_a_value_outside_its_range() {
        let base = encode(&Prog::Uptime.manifest());
        for at in [
            ARG,
            MEM,
            FILE,
            DIR,
            OUTPUT,
            INPUT,
            REPORTS,
            INTERRUPTIBLE,
            CLOCK,
            DOMAIN,
            CONFIG,
            ENTROPY,
            NETWORK,
            RUNTIME,
        ] {
            let mut d = base;
            d[at] = 0xff;
            assert_eq!(decode(&d), Err(Error::BadField(at)), "offset {at}");
        }
    }

    #[test]
    fn a_byte_the_layout_says_is_zero_is_refused() {
        let base = encode(&Prog::Uptime.manifest());
        // Uptime declares no memory, no input, no directory and no options, so all of these are
        // absent fields, and the three trailing bytes are always zero.
        for at in [
            MEM_MIN,
            MEM_MAX + 7,
            WRITES_WHILE_READING,
            SUBTREE,
            FLAG_LETTERS,
            TAIL,
            DESCRIPTOR_LEN - 1,
        ] {
            let mut d = base;
            d[at] = 1;
            assert_eq!(decode(&d), Err(Error::BadField(at)), "offset {at}");
        }
    }

    #[test]
    fn options_are_distinct_letters_and_the_subtree_option_is_one_of_them() {
        let rm = encode(&Prog::Rm.manifest());
        assert_eq!(&rm[FLAG_LETTERS..FLAG_LETTERS + 3], b"rfv");
        assert_eq!(rm[SUBTREE], b'r');

        let mut twice = rm;
        twice[FLAG_LETTERS + 1] = b'r';
        assert_eq!(decode(&twice), Err(Error::BadField(FLAG_LETTERS)));

        let mut undeclared = rm;
        undeclared[SUBTREE] = b'q';
        assert_eq!(decode(&undeclared), Err(Error::BadField(SUBTREE)));

        let mut too_many = rm;
        too_many[FLAG_COUNT] = MAX_DECLARED_FLAGS as u8 + 1;
        assert_eq!(decode(&too_many), Err(Error::BadField(FLAG_COUNT)));
    }

    #[test]
    fn a_memory_range_upside_down_is_refused() {
        let m = Manifest {
            mem: MemSpec::Required { min: 4, max: 8 },
            ..Prog::Uptime.manifest()
        };
        let mut d = encode(&m);
        assert_eq!(decode(&d), Ok(m));
        d[MEM_MIN] = 9;
        assert_eq!(decode(&d), Err(Error::BadField(MEM_MIN)));
    }

    #[test]
    fn the_note_is_what_elf_finds() {
        let n = Note::of(&Prog::Uptime.manifest());
        assert_eq!(n.0.len(), NOTE_LEN);
        assert_eq!(&n.0[0..4], &5u32.to_le_bytes());
        assert_eq!(&n.0[4..8], &(DESCRIPTOR_LEN as u32).to_le_bytes());
        assert_eq!(&n.0[8..12], &MANIFEST.to_le_bytes());
        assert_eq!(&n.0[12..20], b"nife\0\0\0\0");
        assert_eq!(decode(&n.0[20..]), Ok(Prog::Uptime.manifest()));
    }
}
