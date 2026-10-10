//! **The installer's half of the spawn service**: an edit to the activation set, asked on the spawn
//! endpoint by the owner's console or on an installer endpoint by a package manager (milestone 198
//! (a package manager) rung 3a, DECISIONS §208 (installing a package is granting it, and the
//! activation set is versioned), §270 (a package manager holds an installer endpoint, not the spawn
//! endpoint)). Moved out of the crate's root by milestone 809 (the package client becomes a
//! program), when the root outgrew §266 (a Rust source file stays under 2,000 lines)'s
//! ratchet; the spawn loop calls in.

use super::*;

/// **Serve one request that arrived on the installer endpoint of the job labeled `label`**
/// (milestone 809 (the package client becomes a program)): install, remove or rollback through
/// [`activate`], recorded under the holder's name; anything else answered
/// `ActivationStatus::NotServedHere` with nothing read past its first message. The answer goes to
/// the holder's own reply endpoint, unless the holder died mid-request. Out of line, for
/// [`activate`]'s reason: the spawn loop's frame is under every request.
#[inline(never)]
pub(super) fn serve_installer(
    label: u64,
    (w0, w1, w2): (u64, u64, u64),
    spawn: &mut SpawnEndpoint,
    (own_ut, images_ut, fs, catalog): (u64, u64, Option<Fs>, &'static str),
    fs_mapped: &mut bool,
) {
    let Some((reply, manager)) = spawn.reply_for(label) else {
        // A holder this process has no record of: its reap came first. Nobody to answer.
        return;
    };
    let (status, live) = match spawnproto::activation(w1, w2) {
        Some(Some(verb)) if spawnproto::installer_serves(verb) => activate(
            Some(verb),
            w0,
            spawn,
            &Activating {
                own_ut,
                images_ut,
                fs,
                catalog,
                manager,
            },
            fs_mapped,
        ),
        _ => (spawnproto::ActivationStatus::NotServedHere, 0),
    };
    if !spawn.abandoned {
        let (r0, r1, r2) = spawnproto::activation_reply(status, live);
        send(reply, r0, r1, r2);
    }
}

/// **A package manager's two grants, made in the job's region `region`** (milestone 809): the
/// installer endpoint and its reply endpoint when `installer` (the request came from the owner's
/// console and the manifest asked), and a copy of the catalog's `lines` when `catalog`. The
/// endpoint is a copy of `spawn_ep` badged with the job's `label`; the reply endpoint is made from
/// the region, so the reap frees it. Either half of the installer failing yields neither half. Out
/// of line, for [`activate`]'s reason.
#[inline(never)]
pub(super) fn package_manager_parts(
    region: u64,
    (installer, catalog): (bool, bool),
    (spawn_ep, label): (u64, u64),
    own_ut: u64,
    lines: &str,
) -> (Option<(u64, u64)>, Option<u64>) {
    let endpoints = if installer {
        let reply = retype_obj(region, abi::objtype::RENDEZVOUS).ok();
        let inst = badged(spawn_ep, spawnproto::installer_badge(label)).ok();
        match (inst, reply) {
            (Some(i), Some(rp)) => Some((i, rp)),
            (i, rp) => {
                for c in [i, rp].into_iter().flatten() {
                    cap_delete(c);
                }
                None
            }
        }
    } else {
        None
    };
    let page = if catalog {
        copy_catalog(own_ut, region, lines)
    } else {
        None
    };
    (endpoints, page)
}

/// **The image's package catalog, copied into a fresh page of the child's own region**, for a
/// child that declares `grant_plan::Manifest::catalog` (milestone 809 (the package client becomes
/// a program)), and mapped read-only at `std_runtime_protocol::CATALOG_PAGE`. The rest of the page
/// is zero, which is where the text ends. `None` if the catalog does not fit in a page or the page
/// could not be made; the spawn is then refused rather than started with half a catalog.
fn copy_catalog(own_ut: u64, region: u64, catalog: &str) -> Option<u64> {
    if catalog.len() >= spawnproto::IMAGE_PAGE as usize {
        return None;
    }
    let page = retype_page_frame(region).ok()?;
    let Ok(ours) = supervision_protocol::map_scratch(page, true, own_ut) else {
        cap_delete(page);
        return None;
    };
    // SAFETY: `ours` is a fresh page mapped read/write just above, and the catalog is shorter than
    // a page. A retyped page is zeroed, so the bytes after the text are zero.
    unsafe {
        core::ptr::copy_nonoverlapping(catalog.as_ptr(), ours as *mut u8, catalog.len());
    }
    supervision_protocol::give_up_own_page(ours);
    Some(page)
}

/// What [`activate`] needs from the spawn service, named so the call site says it.
pub(super) struct Activating {
    pub(super) own_ut: u64,
    /// [`IMAGE_POOL_PAGES`]: a package is staged where an image is.
    pub(super) images_ut: u64,
    pub(super) fs: Option<Fs>,
    pub(super) catalog: &'static str,
    /// **Who is asking, as a row's manager column records it** (milestone 809 (the package client
    /// becomes a program)): `activation_set::OWNER` on the spawn endpoint, the holder's program
    /// name on an installer endpoint.
    pub(super) manager: &'static str,
}

/// **Serve one activation request** (milestone 198 (a package manager) rung 3a's installer,
/// DECISIONS §208 (installing a package is granting it, and the activation set is versioned)).
/// Returns the reply's status and the generation live afterwards.
///
/// - **Install**: the package's frames are staged exactly as an image's are ([`take_frames`], [`stage_frames`]), so
///   what is checked is this process's copy. `package_archive::installable` decides on the bytes:
///   the image's catalog must vouch for the whole file, and the member named after the package
///   is the program. Its bytes go to `packages/<stem>/<program>`, a place per package version that
///   is never rewritten with other bytes, and a new generation records the program's digest.
/// - **Remove**: a new generation without the program. Its bytes stay where they are, so a rollback
///   can bring it back; nothing collects them yet.
/// - **Rollback**: `current` names the generation one below the live one. Nothing else is written.
/// - **Vouch** (DECISIONS §221 (the boot prompt is the owner's console)): the executable's frames
///   are staged as an install's are, and a new generation records this process's hash of its own
///   copy under the name that follows, marked `activation_set::OWNER`. Nothing is placed: the
///   digest vouches for the bytes wherever they are, and a rollback undoes it.
///
/// Each of the four ends in [`FsCalls::commit`], so the only thing that ever changes what runs is
/// one rename of `current`. **Who may do this** is whoever holds the spawn endpoint, which is the
/// boot prompt alone, and DECISIONS §221 ruled that whoever holds that prompt is the machine's
/// owner, who may also write `activation/` directly; and, for install, remove and rollback, a
/// program the owner's console granted an installer endpoint (milestone 809 (the package client
/// becomes a program), DECISIONS §270 (a package manager holds an installer endpoint, not the spawn
/// endpoint)). A fetch was a fifth verb until milestone 809: `jig` fetches now, and this process
/// parses no network input.
///
/// **Out of line on purpose**, for [`build_grant`]'s reason: it holds a page-sized buffer and
/// [`edit`] another beneath it, and since milestone 809 the spawn loop calls it from two places
/// (the spawn endpoint and an installer endpoint). Inlined at both, the loop's frame carried both
/// copies, and `script/swish-check`'s stack gauge measured the progenitor under its floor.
#[inline(never)]
pub(super) fn activate(
    verb: Option<spawnproto::Activation>,
    w0: u64,
    spawn: &mut SpawnEndpoint,
    a: &Activating,
    fs_mapped: &mut bool,
) -> (spawnproto::ActivationStatus, u32) {
    use spawnproto::{Activation, ActivationStatus as S};
    // The request's own trailing messages come off the endpoint first, whatever happens next, so a
    // refusal never leaves words behind that the next request would read as its own. The frames
    // are copied only once the name after them is read too ([`take_frames`]).
    let frames = match verb {
        Some(Activation::Install | Activation::Vouch) => Some(take_frames(spawn, w0, a.own_ut)),
        _ => None,
    };
    let mut named = [0u8; filesystem_protocol::grant::MAX_NAME];
    // A vouch's name follows its frames, which `take_frames` has just taken.
    let named_len = if matches!(verb, Some(Activation::Remove | Activation::Vouch)) {
        let (lo, hi, len) = spawn.receive();
        filesystem_protocol::grant::unpack_name(lo, hi, len as usize, &mut named)
    } else {
        0
    };
    let staging = frames
        .and_then(|f| stage_frames(spawn, &f, w0, a.own_ut, a.images_ut, true).map(|st| (st, w0)));
    let named = &named[..named_len];

    let outcome = (|| {
        let Some(verb) = verb else {
            return (S::Unknown, 0);
        };
        // An installer holder that died mid-request sent nothing more, and is owed no answer.
        if spawn.abandoned {
            return (S::Unknown, 0);
        }
        let Some(files) = FsCalls::map(a.fs, a.own_ut, fs_mapped) else {
            return (S::StoreFailed, 0);
        };
        let act = files.directory(fs_operation::ROOT, activation_set::DIRECTORY);
        if act < 0 {
            return (S::StoreFailed, 0);
        }
        let act_h = act as u64;
        let mut old = [0u8; PAGE_BYTES];
        let answer = match files.live_generation(act_h, &mut old) {
            Err(()) => (S::StoreFailed, 0),
            // A live table that is not text is a table nothing can edit, the same answer as one
            // that cannot be read.
            Ok((live, n)) => match core::str::from_utf8(&old[..n]) {
                Ok(table) => edit(
                    &files, act_h, verb, live, table, staging, a.catalog, a.manager, named,
                ),
                Err(_) => (S::StoreFailed, live),
            },
        };
        files.close(act);
        answer
    })();

    // The staged package goes back to the image pool, whose most recent carve it is.
    if let Some((st, _)) = staging {
        supervision_protocol::memory_region_destroy(st);
        cap_delete(st);
    }
    outcome
}

/// The part of [`activate`] that decides and writes, with the live generation already read.
#[allow(clippy::too_many_arguments)]
fn edit(
    files: &FsCalls,
    act: u64,
    verb: spawnproto::Activation,
    live: u32,
    table: &str,
    staging: Option<(u64, u64)>,
    catalog: &str,
    manager: &str,
    named: &[u8],
) -> (spawnproto::ActivationStatus, u32) {
    use spawnproto::{Activation, ActivationStatus as S};
    // The next generation's number: the first one above the live generation with no file. After a
    // rollback the numbers above the live one are taken, and a generation is never rewritten.
    let next = || {
        let mut m = live + 1;
        while files.generation_exists(act, m) && m < u32::MAX {
            m += 1;
        }
        m
    };
    let mut new = [0u8; PAGE_BYTES];
    match verb {
        Activation::Rollback => {
            if live <= 1 {
                return (S::NoEarlier, live);
            }
            let back = live - 1;
            // **A rollback undoes rows; a manager may undo only its own and the owner's**
            // (`activation_set::foreign_change`, calef's 2026-10-06 ruling on milestone 809). The
            // generation below is read into the page the next generation would have used, since a
            // rollback writes none.
            let Ok(n) = files.generation(act, back, &mut new) else {
                return (S::StoreFailed, live);
            };
            let Ok(older) = core::str::from_utf8(&new[..n]) else {
                return (S::StoreFailed, live);
            };
            match activation_set::foreign_change(older, table, manager) {
                Ok(None) => {}
                Ok(Some(_)) => return (S::NotYours, live),
                Err(_) => return (S::StoreFailed, live),
            }
            if !files.commit(act, back, None) {
                return (S::StoreFailed, live);
            }
            (S::Done, back)
        }
        Activation::Remove => {
            let Ok(operand) = core::str::from_utf8(named) else {
                return (S::NotInstalled, live);
            };
            // **The verb's object is a program or a program at a version** (milestone 614 (two
            // installed versions of one program, each runnable, and a caller granted the one it
            // needs), ruling 5). `@` is the qualified form's separator and is reserved for it: a
            // word with one is never a program name, so the two cannot be mistaken for each other.
            // **Another manager's rows are not this one's to remove** (calef, 2026-10-06; milestone
            // 809). Checked over every row the edit would take, before anything is written.
            let (program, version) = match operand.split_once('@') {
                Some((p, v)) if !p.is_empty() && !v.is_empty() => (p, Some(v)),
                _ => (operand, None),
            };
            let foreign = activation_set::entries(table).flatten().any(|e| {
                e.program == program
                    && e.package != activation_set::OWNER
                    && version.is_none_or(|v| e.version == v)
                    && !activation_set::may_edit(e.manager, manager)
            });
            if foreign {
                return (S::NotYours, live);
            }
            let removed = match operand.split_once('@') {
                Some((program, version)) if !program.is_empty() && !version.is_empty() => {
                    activation_set::without_version(table, program, version, &mut new)
                }
                _ => activation_set::without_entry(table, operand, &mut new),
            };
            let n = match removed {
                Ok(n) => n,
                Err(activation_set::Error::NotInstalled) => return (S::NotInstalled, live),
                Err(activation_set::Error::SeveralVersions) => return (S::Ambiguous, live),
                Err(_) => return (S::StoreFailed, live),
            };
            let m = next();
            if !files.commit(act, m, Some(&new[..n])) {
                return (S::StoreFailed, live);
            }
            (S::Done, m)
        }
        // **The owner's vouch** (DECISIONS §221 ruling 1; §195 clause 3). The digest is this
        // process's hash of its own copy, the one a spawn of the same file computes, so the entry
        // is found by the next run of those bytes and by nothing else.
        Activation::Vouch => {
            let Some((_, len)) = staging else {
                return (S::Unknown, live);
            };
            let bytes = staged_image(len);
            let Ok(program) = core::str::from_utf8(named) else {
                return (S::NotExecutable, live);
            };
            if elf::Elf::parse(bytes).is_err() {
                return (S::NotExecutable, live);
            }
            let entry = activation_set::Entry {
                program,
                // **A vouch claims no version**: the owner vouches for bytes (DECISIONS §221
                // (the boot prompt is the owner's console), ruling 1), so the version column
                // carries `activation_set::NO_VERSION` and no pointer names the row.
                version: activation_set::NO_VERSION,
                package: activation_set::OWNER,
                digest: measured_boot::sha256(bytes),
                manager,
            };
            // A vouch claims no name, so whether the image carries one does not matter to it.
            let n = match activation_set::with_entry(table, &entry, false, &mut new) {
                Ok(n) => n,
                Err(activation_set::Error::BadName) => return (S::NotExecutable, live),
                Err(_) => return (S::StoreFailed, live),
            };
            let m = next();
            if !files.commit(act, m, Some(&new[..n])) {
                return (S::StoreFailed, live);
            }
            (S::Done, m)
        }
        Activation::Install => {
            let Some((_, len)) = staging else {
                return (S::Unknown, live);
            };
            let bytes = staged_image(len);
            // Whoever fetched it (`jig`, since milestone 809) checked it was the package asked
            // for; what decides here is only whether the image vouches for these bytes.
            let got = match package_archive::installable(catalog, bytes) {
                Ok(got) => got,
                Err(package_archive::Refusal::NoProgram) => return (S::NoProgram, live),
                Err(_) => return (S::NotCataloged, live),
            };
            let Ok(package) = package_archive::Package::parse(bytes) else {
                return (S::NotCataloged, live);
            };
            // The program's bytes, where a person can run them (DECISIONS §219 option D hashes
            // whatever they run, so where they live is a convenience, not a trust decision).
            let packages = files.directory(fs_operation::ROOT, activation_set::PACKAGES);
            if packages < 0 {
                return (S::StoreFailed, live);
            }
            // `packages/<name>/<version>/<program>`: one component per field, because a prompt
            // component is at most sixteen bytes and a stem is longer. The architecture is this
            // machine's; the generation's row does not carry it, because the digest is of
            // target-specific bytes and two ISAs never collide in one table.
            let name = files.directory(packages as u64, package.name());
            let version = if name >= 0 {
                files.directory(name as u64, package.version())
            } else {
                -1
            };
            let placed = version >= 0 && {
                let f = files.open_or_create(version as u64, got.program);
                let ok = f >= 0 && files.replace(f as u64, got.bytes);
                if f >= 0 {
                    files.close(f);
                }
                ok
            };
            for h in [version, name, packages] {
                if h >= 0 {
                    files.close(h);
                }
            }
            if !placed {
                return (S::StoreFailed, live);
            }
            let entry = activation_set::Entry {
                program: got.program,
                // **The row is digest-keyed; name and version are label columns** (milestone 614
                // (two installed versions of one program, each runnable, and a caller granted the
                // one it needs), ruling 2). The version is the upstream developer's claim as the
                // package header carries it; the package column is the package's name, the
                // stem's first field, because the version now has a column of its own and no row
                // carries the architecture.
                version: package.version(),
                package: package.name(),
                digest: got.digest,
                // Who asked (milestone 809): `jig`, or the owner's console.
                manager,
            };
            // **A bare name belongs to one package, and never to one the image carries** (DECISIONS
            // §229 (how a bare name at the prompt reaches an installed program), B2 and calef's
            // ruling of 2026-09-27). Either is refused here, after the bytes are placed under
            // `packages/` (where they still run by path) and before any generation names them. A
            // base program is updated through the boot slot under §235 (the OS is built and updated
            // from packages), so the image refusal blocks no update. The prompt still refuses a
            // name that is both, because a later base can add a name a package already holds.
            let image = Prog::from_name(got.program.as_bytes()).is_some();
            let n = match activation_set::with_entry(table, &entry, image, &mut new) {
                Ok(n) => n,
                Err(activation_set::Error::ImageName) => return (S::ImageName, live),
                Err(activation_set::Error::Taken) => return (S::NameTaken, live),
                Err(_) => return (S::StoreFailed, live),
            };
            let m = next();
            if !files.commit(act, m, Some(&new[..n])) {
                return (S::StoreFailed, live);
            }
            (S::Done, m)
        }
    }
}
