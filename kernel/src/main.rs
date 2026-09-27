//! The kernel binary: a link line and nothing else.
//!
//! Everything, `_start` included, is in the library (`src/lib.rs`), because a second image links
//! the same kernel: the system-test image in `system_tests/` (milestone 609 (the system tests leave
//! the kernel crate)). `use kernel as _` is what makes rustc link the library at all; the linker
//! then pulls `_start` in as the `ENTRY` of `link-<arch>.ld`, which `build.rs` passes.
#![no_std]
#![no_main]
// The binary target sees every dependency of the package and names none of them; the library does.
#![allow(unused_crate_dependencies)]

use kernel as _;
