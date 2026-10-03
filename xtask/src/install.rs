//! **The install gate**: a stick boots, puts itself on a disk, and the machine boots from that disk
//! with the stick gone (milestone 198 (a package manager, and the trivial install that makes a
//! second customer possible), rung 2a.
//!
//! Milestone 515 (a stick that puts itself on the machine's disk) is the proposal it follows.
//!
//! # Why it is two boots
//!
//! **A sequence that only works while the stick is still attached has not installed anything.** The
//! second boot is therefore the whole gate: the first one is a setup step for it, and everything the
//! first one asserts is only there so a failure says which half broke.
//!
//! Three properties are deliberately arranged so that the second boot cannot succeed by accident:
//!
//! - **The stick is not attached at all**, not merely deprioritised. `NIFE_UEFI_NO_STICK=1` removes
//!   the vvfat drive from the machine.
//! - **The firmware variable store is deleted before each boot.** The default store persists across
//!   runs in a checkout, so boot 1 would leave a boot option that boot 2 then rode. Deleting it is
//!   what makes "the firmware found it on its own" a claim about the disk rather than about our
//!   leftovers, and it is the measurement milestone 515 calls B1 and calls unmeasured.
//! - **The NVMe image is created empty**, every run, so nothing survives from the last one.
//!
//! # EXAMPLES
//!
//! ```console
//! $ cargo xtask install-boot
//! --- boot 1 of 2: the stick installs itself onto an empty NVMe disk ---
//!   install     : installed. nife data at LBA 2048, EFI system at LBA 1046528.
//! --- boot 2 of 2: the same machine with the stick detached ---
//! BdsDxe: loading Boot0001 "UEFI QEMU NVMe Ctrl nife-nvme 1" ...
//! $ ls
//!   made-on-target
//! install-boot: PASS
//! ```
//!
//! # BUGS
//!
//! - **It is one disk of one size**, a 1 GiB image. The installer's layout arithmetic has a floor
//!   and a rounding step that nothing here varies, and a disk under about 600 MiB is refused by
//!   code no test exercises.
//! - **It proves nothing about a real firmware.** OVMF is one implementation and a generous one.
//!   Rung 2b is the bench half and belongs to somebody with xenon in front of them.
//! - **It runs nowhere on its own.** It was left out of `script/test`'s default legs because the
//!   two boots were said to take several minutes under TCG. Measured 2026-10-03 on an Apple M-series
//!   host, warm: 42.5 seconds for the whole gate. Nothing schedules it either, so a break is found
//!   by whoever next runs it by hand: `design/roadmap/proposals/the-install-gates-run-nowhere.md`.
//! - **A failure leaves the NVMe image behind**, on purpose: it is the evidence, and
//!   `hdiutil attach -imagekey diskimage-class=CRawDiskImage` on the partition reads the EFI system
//!   partition it wrote.

use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use crate::X86_TARGET;
use crate::disk::redoxfs_server_build;
use crate::host::workspace_root;
use crate::uefi::{uefi_kernel, uefi_stage};

/// The disk the install goes onto. **One gibibyte**: comfortably over the installer's own floor
/// (a 512 MiB EFI system partition plus its 64 MiB minimum for data), and a sparse file, so it
/// costs what is written to it and nothing else.
const DISK_BYTES: u64 = 1024 * 1024 * 1024;

/// Where the boot files are staged for this gate. Its own directory rather than `target/esp`,
/// because that one is what the bench procedure copies to a stick and nothing here should decide
/// when it is rebuilt.
pub(crate) fn install_esp_dir() -> PathBuf {
    workspace_root().join("target/esp-install")
}

/// An empty directory to hand the runner as its ESP argument on the second boot. The runner still
/// requires one and still has to find it; what it does not do is attach it.
pub(crate) fn empty_esp_dir() -> PathBuf {
    workspace_root().join("target/esp-detached")
}

/// The NVMe image this gate installs onto, written empty at the start of every run.
pub(crate) fn install_disk_path() -> PathBuf {
    workspace_root().join("target/nife-install.img")
}

/// This gate's own firmware variable store. See the module header: deleting it is a load-bearing
/// part of the second boot's claim.
pub(crate) fn vars_path() -> PathBuf {
    workspace_root().join("target/ovmf-vars-install.fd")
}

/// **The gate.** `true` only if both boots did what they were supposed to.
pub(crate) fn install_boot() -> bool {
    if !prepare() {
        return false;
    }
    if install_once().is_none() {
        return false;
    }

    eprintln!();
    eprintln!("--- boot 2 of 2: the same machine with the stick detached ---");
    let Some(second) = boot(
        &empty_esp_dir(),
        true,
        &[
            // The progenitor's own last line, which is printed after the prompt exists.
            ("the progenitor is running at ring 3", "ls\r"),
            // And then read the file back, which is what makes this an install rather than a boot.
            ("made-on-target", "wc made-on-target\r"),
        ],
        // `wc`'s answer for `filesystem_protocol::fixture::blank::MADE_BODY`: one line, ten words,
        // fifty-seven bytes. Written out rather than derived, because a gate that computed the
        // number from the same constant the program wrote would be checking its own arithmetic.
        "1 10 57",
        300,
    ) else {
        return false;
    };

    let mut ok = true;
    for wanted in [
        // The firmware found the file on the disk, with no boot variable and no boot index.
        "UEFI QEMU NVMe Ctrl",
        // The loader's own first line, which it has printed since milestone 87 (the x86_64
        // bare-metal machine) and which says the firmware started what was written rather than
        // something it found elsewhere.
        "nife uefi_loader: milestone 87",
        // Rung 2b: the installed disk's own chooser picked the slot the install marked good, and
        // the image it started knows which slot it came from.
        "uefi_loader: starting boot slot 0",
        "uefi_loader: started from boot slot 0",
        "nife: handing the system to the userspace progenitor.",
        // A prompt.
        "nife capability shell.",
        // The file `mkfs` wrote before the reboot, listed and then read.
        "made-on-target",
        "1 10 57",
    ] {
        if !second.contains(wanted) {
            eprintln!("install-boot: boot 2's transcript is missing {wanted:?}");
            ok = false;
        }
    }
    // **The offer must not be made to a machine that already carries nife**, or every boot of every
    // installed machine would pause to ask whether to wipe itself.
    if second.contains("EVERYTHING ON THAT DISK WILL BE DESTROYED") {
        eprintln!("install-boot: boot 2 offered to install over an installed machine");
        ok = false;
    }
    if ok {
        eprintln!("install-boot: PASS");
    }
    ok
}

/// **Build everything and write an empty disk**, which is what any gate starting from a bare
/// machine needs. Shared with `cargo xtask rollback-boot`, which begins with the same install.
pub(crate) fn prepare() -> bool {
    // `mkfs` and the FS server, which the archive packs only if something built them for this
    // target. Without `mkfs` the install partitions the disk and leaves the data partition empty,
    // and the second boot then has nothing to read back.
    if !redoxfs_server_build(X86_TARGET) {
        eprintln!("install-boot: could not build the FS server and mkfs for {X86_TARGET}");
        return false;
    }
    // `uefi_kernel` packs the archive first and then builds the kernel, which is the order the
    // measured-boot seal requires; see its own comment.
    let Some(kernel) = uefi_kernel(None) else {
        return false;
    };
    if !uefi_stage(
        &kernel,
        &install_esp_dir(),
        "the loader, the kernel and the archive",
        false,
    ) {
        return false;
    }
    if std::fs::create_dir_all(empty_esp_dir()).is_err() {
        eprintln!(
            "install-boot: could not create {}",
            empty_esp_dir().display()
        );
        return false;
    }
    write_empty_disk()
}

/// **Boot the stick once and let it install itself.** The transcript, or `None` if anything in the
/// install did not happen.
pub(crate) fn install_once() -> Option<String> {
    eprintln!();
    eprintln!("--- the stick installs itself onto an empty NVMe disk ---");
    let first = boot(
        &install_esp_dir(),
        false,
        // The one thing a person does, typed when the question is on the wire. The marker is the
        // prompt itself, which ends without a newline, so the reader below cannot be line-based.
        &[("install     : > ", "INSTALL\r")],
        // The whole line, not a prefix of it: the loop below stops the machine the instant this
        // appears, so a prefix leaves the rest of the line unread and the assertion for the full
        // sentence then fails on a boot that did everything right.
        "install     : DONE. Remove the installation medium and reboot.",
        420,
    )?;

    for wanted in [
        "install     : this system was booted from a file and can install itself.",
        "install     :   EVERYTHING ON THAT DISK WILL BE DESTROYED.",
        "install     : installed. nife data at LBA 2048",
        "install     : filesystem created.",
        "install     : DONE. Remove the installation medium and reboot.",
    ] {
        if !first.contains(wanted) {
            eprintln!("install: the install transcript is missing {wanted:?}");
            return None;
        }
    }
    Some(first)
}

/// Write `DISK_BYTES` of nothing, replacing whatever the last run left.
pub(crate) fn write_empty_disk() -> bool {
    match std::fs::File::create(install_disk_path()).and_then(|f| f.set_len(DISK_BYTES)) {
        Ok(()) => true,
        Err(e) => {
            eprintln!(
                "install-boot: could not write {}: {e}",
                install_disk_path().display()
            );
            false
        }
    }
}

/// **Boot the machine once**, typing `sends` when each one's marker appears on the wire, and stop
/// as soon as `until` has been printed.
///
/// Reads bytes rather than lines, which is not a detail: the installer's question ends in `> ` with
/// no newline, so a line-based reader would not see it until after the answer was due.
pub(crate) fn boot(
    esp: &PathBuf,
    detached: bool,
    sends: &[(&str, &str)],
    until: &str,
    timeout_secs: u64,
) -> Option<String> {
    // The firmware's variables, fresh. See the module header for why this is the load-bearing line
    // of the second boot.
    let _ = std::fs::remove_file(vars_path());

    let mut command = Command::new("helpers/qemu-uefi-x86_64.sh");
    command
        .arg(esp)
        .current_dir(workspace_root())
        .env("NIFE_NVME", install_disk_path())
        .env("NIFE_OVMF_VARS_OUT", vars_path())
        .env("NIFE_UEFI_TIMEOUT", timeout_secs.to_string())
        // The machine is a disk and a firmware and nothing else, whether or not a suite leg set
        // these in this process first. One boot with two machines is one boot nobody can compare.
        .env_remove("NIFE_DISK")
        .env_remove("NIFE_UEFI_REDOXFS")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if detached {
        command.env("NIFE_UEFI_NO_STICK", "1");
    }
    let mut child = match command.spawn() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("install-boot: failed to run helpers/qemu-uefi-x86_64.sh: {e}");
            return None;
        }
    };

    let mut serial = child.stdin.take().expect("piped stdin");
    let mut output = child.stdout.take().expect("piped stdout");
    // stderr is the runner's own complaints, drained on a thread so a full pipe cannot wedge it.
    let errors = {
        let mut from = child.stderr.take().expect("piped stderr");
        std::thread::spawn(move || {
            let mut mine = String::new();
            let _ = from.read_to_string(&mut mine);
            mine
        })
    };

    let mut transcript = String::new();
    let mut since_send = String::new();
    let mut pending = sends.iter();
    let mut next = pending.next();
    let mut buf = [0u8; 4096];
    loop {
        let n = match output.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        let chunk = String::from_utf8_lossy(&buf[..n]).into_owned();
        print!("{chunk}");
        let _ = std::io::stdout().flush();
        transcript.push_str(&chunk);
        since_send.push_str(&chunk);

        if let Some((marker, text)) = next
            && since_send.contains(marker)
        {
            // The machine has just printed a prompt; the byte after it is the one being waited for.
            // A short settle keeps the answer out of the same read the firmware is still draining.
            std::thread::sleep(std::time::Duration::from_millis(500));
            if serial.write_all(text.as_bytes()).is_err() || serial.flush().is_err() {
                eprintln!("install-boot: could not type {text:?} at the machine");
                break;
            }
            // Cleared rather than kept, so two sends that share a marker substring cannot both
            // fire on one appearance of it.
            since_send.clear();
            next = pending.next();
        }

        if next.is_none() && transcript.contains(until) {
            break;
        }
    }

    // The kernel never exits, so the machine is stopped rather than waited out: SIGTERM to the
    // wrapper, which is what `qemu-bounded.sh` forwards to QEMU, with its own killer as the
    // backstop.
    let _ = Command::new("kill")
        .args(["-TERM", &child.id().to_string()])
        .status();
    let _ = child.wait();
    transcript.push_str(&errors.join().unwrap_or_default());

    if !transcript.contains(until) {
        eprintln!("install-boot: the boot never printed {until:?}");
        return None;
    }
    Some(transcript)
}
