//! Looking at what was built: `gdb`, `objdump`, and the flat `image` with its header dumped.

use std::process::Command;

use crate::host::{kernel_elf, llvm_tool, run, workspace_root};
use crate::{RISCV_TARGET, RUNNER, TARGET, X86_TARGET, build};

/// Boot the kernel with QEMU frozen and a GDB stub listening.
///
/// `-s` opens the stub on :1234, `-S` holds the CPU before the first instruction.
/// The kernel ELF carries symbols and DWARF, so GDB shows Rust source lines rather
/// than raw addresses (notes/elf.md). Point GDB at the **ELF**, even though QEMU is
/// running the flat image: the image has no symbols, and the addresses match.
///
/// This is the tool that will save you at milestone 4, when the MMU comes on and
/// `println!` stops being an option.
pub(crate) fn gdb() -> bool {
    if !build() {
        return false;
    }

    let elf = kernel_elf();
    eprintln!("QEMU is paused, waiting for a debugger on localhost:1234.");
    eprintln!("In another terminal:");
    eprintln!();
    eprintln!("    gdb {elf}");
    eprintln!("    (gdb) target remote :1234");
    eprintln!("    (gdb) break kernel_main");
    eprintln!("    (gdb) continue");
    eprintln!();
    eprintln!("To watch boot.s set up the stack and zero .bss:");
    eprintln!();
    eprintln!("    (gdb) break _boot");
    eprintln!("    (gdb) layout asm");
    eprintln!("    (gdb) si          # step one instruction");
    eprintln!();

    run(RUNNER, &[&elf, "-s", "-S"])
}

pub(crate) fn objdump() -> bool {
    if !build() {
        return false;
    }
    match llvm_tool("llvm-objdump") {
        Some(tool) => run(
            &tool,
            &[
                "-d",
                "--no-show-raw-insn",
                "-M",
                "no-aliases",
                &kernel_elf(),
            ],
        ),
        None => false,
    }
}

/// Build the flat arm64 Image and show its 64-byte header.
///
/// Useful when the header is wrong, which is a failure mode with no diagnostics at
/// all: QEMU simply falls back to treating the file as an anonymous blob, boots it,
/// and hands you a zero in x0. See notes/boot-protocol.md.
pub(crate) fn image() -> bool {
    if !build() {
        return false;
    }
    let Some(objcopy) = llvm_tool("llvm-objcopy") else {
        return false;
    };

    let elf = kernel_elf();
    let img = format!("{elf}.img");
    // --remove-section: see the matching comment in helpers/qemu-runner-aarch64.sh. The ELF keeps
    // `.eh_frame`/`.eh_frame_hdr` (milestone: CFI in hand-written asm) for a debugger to read;
    // this flat Image, the thing a bootloader actually loads, does not need them along for the
    // ride.
    if !run(
        &objcopy,
        &[
            "-O",
            "binary",
            "--remove-section=.eh_frame",
            "--remove-section=.eh_frame_hdr",
            &elf,
            &img,
        ],
    ) {
        return false;
    }

    match std::fs::read(&img) {
        Ok(bytes) if bytes.len() >= 64 => {
            let magic = u32::from_le_bytes(bytes[56..60].try_into().unwrap());
            let text_offset = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
            let image_size = u64::from_le_bytes(bytes[16..24].try_into().unwrap());

            eprintln!("{img}  ({} bytes)", bytes.len());
            eprintln!();
            eprintln!("  text_offset  {text_offset:#x}");
            eprintln!("  image_size   {image_size:#x}");
            eprintln!(
                "  magic        {magic:#010x}  {}",
                if magic == 0x644d5241 {
                    "ok (\"ARM\\x64\")"
                } else {
                    "WRONG - QEMU will not treat this as a kernel"
                }
            );
            magic == 0x644d5241
        }
        Ok(_) => {
            eprintln!("image is shorter than its own 64-byte header");
            false
        }
        Err(e) => {
            eprintln!("cannot read {img}: {e}");
            false
        }
    }
}

/// Locate an LLVM tool inside the rustup sysroot.
///
/// These ship with the `llvm-tools` component, which `rust-toolchain.toml` pins. We
/// do NOT use the `rust-objdump` / `rust-objcopy` wrappers, because those require a
/// separate `cargo install cargo-binutils` that nothing else in the project needs,
/// and its absence produces a confusing "command not found" rather than a real error.
/// The namespace `read_stripped` files a stripped copy under, from the triple in `path`.
///
/// Only a triple names an architecture or a std build; the checkout's own directory name is not
/// evidence of anything, so no arm may match on a bare `nife`.
fn strip_tag(path: &str) -> &'static str {
    if path.contains(RISCV_TARGET) {
        "riscv"
    } else if path.contains(X86_TARGET) {
        "x86"
    } else if path.contains(TARGET) {
        // Provisional name: the aarch64 bare-metal tag, so `host` means real host builds only.
        "aarch64"
    } else if path.contains("riscv64-unknown-nife") {
        "std-riscv"
    } else if path.contains("x86_64-unknown-nife") {
        "std-x86"
    } else if path.contains("aarch64-unknown-nife") {
        "std"
    } else {
        "host"
    }
}

/// **Read a program ELF for packing, with its debug information removed.**
///
/// The initrd is *reserved RAM*: the frame allocator never owns those pages, so every byte in the
/// archive is a byte the running system does not have. And a debug build is almost entirely debug
/// information: `rust_swappable` is 720 KB, of which **3 KB** is `.text` plus `.rodata` and the
/// other 717 KB is `.debug_*`. Twenty-odd programs like that made a 26 MB archive out of well under
/// a megabyte of code, on a machine with 128 MB.
///
/// Nothing ever read those bytes. `crates/elf` parses **program headers only** (it has no
/// section-header code at all), so the loader cannot see a debug section on either side of the
/// boundary; the kernel prints a raw `pc` on a fault and symbolisation is done offline, against the
/// unstripped binary that is still sitting in `target/`. So this is pure waste, and milestone 23 is
/// where it stopped being free: five more programs pushed the archive 4 MB up and a *later,
/// unrelated* test could no longer find a contiguous eight-megabyte region for the progenitor.
///
/// `--strip-debug` rather than `--strip-all`, deliberately: it takes the `.debug_*` sections, which
/// is all of the bulk, and leaves the symbol table for anything that later wants to read it out of
/// the archive rather than out of `target/`.
///
/// A missing `llvm-objcopy` is a hard failure rather than a silent fallback to unstripped bytes,
/// because the measured-boot digest (§26's phase B.1) is taken over what this returns: a build that
/// quietly packed different bytes depending on which tools were installed would be a build whose
/// trust root means something different on each machine.
pub(crate) fn read_stripped(path: &str) -> std::io::Result<Vec<u8>> {
    let objcopy = llvm_tool("llvm-objcopy").ok_or_else(|| {
        std::io::Error::other("llvm-objcopy not found; the llvm-tools rustup component provides it")
    })?;
    let out = workspace_root().join("target/stripped");
    std::fs::create_dir_all(&out)?;
    let stem = std::path::Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("program");
    // Namespaced by the target directory the binary came from, so two architectures' builds of a
    // program cannot overwrite each other's stripped copy.
    //
    // **x86_64 needs its own arm and the bug was latent rather than absent** (milestone 161). Before
    // this port packed an archive, an `x86_64-unknown-none` path fell through to `"host"` and so
    // shared a filename with every aarch64 build of the same program. Nothing noticed, because
    // nothing ever asked for both; the moment an x86 archive is packed in the same run as an aarch64
    // one, whichever ran second would silently read the other's bytes back out of `target/stripped`
    // and measure them. That is the worst shape a bug can have here: the digest in a trust root
    // would be a real digest of a real program, and the wrong one.
    //
    // The order matters too. `X86_TARGET` is `x86_64-unknown-none`, and a *host* path on an x86
    // Linux CI runner contains `x86_64-unknown-linux-gnu`, which the naive `contains` would also
    // match. The check is anchored on the full triple, so the two cannot be confused.
    //
    // **The two `*-unknown-nife` triples need separating too, for the same reason** (milestone
    // 121). `std_exerciser` is built for both and lands here under one name; `rg` now is as well.
    // Sequentially that is harmless, because each call writes the file it then reads, but it is
    // the same shape of latent bug the paragraph above describes and it costs one arm to close.
    // A third `*-unknown-nife` arm joined at milestone 184, for the same reason as the second.
    //
    // **The aarch64 std arm names its triple too, and a bare `contains("nife")` is the bug that
    // replaced it.** Every checkout lives under a directory called `nife`, CI's included, so that
    // substring matched the absolute path of every aarch64 bare-metal program and stripped it under
    // the `std-` tag. See `strip_tag`.
    let tag = strip_tag(path);
    let dst = out.join(format!("{tag}-{stem}"));
    // Fail before running the tool if the input is missing, so the caller's error message names the
    // binary it wanted rather than objcopy's exit status.
    std::fs::metadata(path)?;
    let status = Command::new(&objcopy)
        .arg("--strip-debug")
        .arg(path)
        .arg(&dst)
        .status()?;
    if !status.success() {
        return Err(std::io::Error::other(format!(
            "{objcopy} --strip-debug {path} failed ({status})"
        )));
    }
    std::fs::read(&dst)
}

#[cfg(test)]
mod strip_tag_tests {
    use super::strip_tag;

    const CHECKOUT: &str = "/home/runner/work/nife/nife";

    #[test]
    fn std_program_is_tagged_std() {
        let p = format!("{CHECKOUT}/std_exerciser/target/aarch64-unknown-nife/release/x");
        assert_eq!(strip_tag(&p), "std");
    }

    #[test]
    fn bare_metal_program_gets_its_own_tag_despite_a_checkout_named_nife() {
        let p = format!("{CHECKOUT}/target/aarch64-unknown-none-softfloat/release/init");
        assert_eq!(strip_tag(&p), "aarch64");
    }

    #[test]
    fn host_build_is_tagged_host() {
        let p = format!("{CHECKOUT}/target/release/xtask");
        assert_eq!(strip_tag(&p), "host");
    }
}
