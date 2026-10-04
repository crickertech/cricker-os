#![no_std]
//! **What a USB device says, what a host asks it, and what a boot keyboard's report means**
//! (milestone 242, USB host and HID; notes/usb.md).
//!
//! The half of a USB keyboard driver that is about USB rather than about the host controller. The
//! controller half (rings, contexts, doorbells) is `crates/extensible_host_controller_interface`;
//! the volatile half that touches either is `components/src/usb_keyboard_driver.rs`. This crate
//! touches nothing: bytes in, a decision out.
//!
//! It is also **the part of the driver that reads untrusted input**. Every descriptor and every
//! report is whatever the device chose to send, and a device is whatever a stranger plugged in.
//! So the parsers here are total: they bound every walk, check every length before reading through
//! it, and answer a [`Refusal`] that names what was wrong rather than panicking or guessing. The
//! Kani harnesses at the bottom prove that for every input up to a bound, not for the inputs a
//! test happened to try.
//!
//! Three pieces:
//!
//! - [`descriptor`]: the device descriptor, the configuration header, and the one walk this
//!   driver needs, [`descriptor::find_boot_keyboard`], which finds a keyboard interface that
//!   speaks the boot protocol and its interrupt IN endpoint.
//! - [`request`]: the four standard and class requests the driver sends, as the eight-byte setup
//!   packet a control transfer carries.
//! - [`boot_keyboard`]: the eight-byte boot report, and the difference between two of them as key
//!   presses and releases in Linux's evdev numbering, which is what `video_terminal::keymap`
//!   already turns into terminal bytes for the virtio keyboard. One layout table in the tree, two
//!   devices feeding it.
//!
//! # Examples
//!
//! A boot keyboard's configuration, as QEMU's `usb-kbd` reports it (trimmed to what matters): a
//! configuration header, one HID interface (class 3, subclass 1 "boot", protocol 1 "keyboard"),
//! its HID descriptor, and one interrupt IN endpoint.
//!
//! ```
//! use usb::descriptor::find_boot_keyboard;
//!
//! let config = [
//!     9, 2, 34, 0, 1, 1, 0, 0xa0, 50, // configuration 1, 34 bytes, one interface
//!     9, 4, 0, 0, 1, 3, 1, 1, 0, // interface 0: HID, boot, keyboard, one endpoint
//!     9, 0x21, 0x11, 1, 0, 1, 0x22, 63, 0, // the HID descriptor, which the walk skips
//!     7, 5, 0x81, 3, 8, 0, 10, // endpoint 1 IN, interrupt, 8 bytes, every 10 frames
//! ];
//! let keyboard = find_boot_keyboard(&config).unwrap();
//! assert_eq!(keyboard.configuration, 1);
//! assert_eq!(keyboard.interface, 0);
//! assert_eq!(keyboard.endpoint, 0x81);
//! assert_eq!(keyboard.max_packet, 8);
//! assert_eq!(keyboard.interval, 10);
//! ```
//!
//! And two reports a keystroke apart. Shift and `h` arrive together in one report, so the shift
//! press is delivered first: the order is what makes it an `H`.
//!
//! ```
//! use usb::boot_keyboard::{BootReport, changes};
//!
//! let before = BootReport::parse(&[0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
//! let after = BootReport::parse(&[0x02, 0, 0x0b, 0, 0, 0, 0, 0]).unwrap(); // left shift, usage 0x0b (h)
//! let mut seen = Vec::new();
//! changes(&before, &after, &mut |code, pressed| seen.push((code, pressed)));
//! assert_eq!(seen, [(42, true), (35, true)]); // KEY_LEFTSHIFT, then KEY_H
//! ```
//!
//! # BUGS
//!
//! - **Boot protocol only.** A keyboard is found by its interface triple (3, 1, 1) and read in the
//!   fixed eight-byte layout; nothing here parses a HID *report descriptor*. A keyboard that
//!   declares no boot interface is refused with [`Refusal::NoBootKeyboard`], which is the honest
//!   answer and is milestone 242's own BUGS entry ("boot protocol may not survive contact with real
//!   keyboards").
//! - **The first boot keyboard interface wins.** A device with two (rare) gets its first; a second
//!   keyboard on another port is not looked for by this crate, and the driver serves one.
//! - **No key repeat.** A boot keyboard reports state, not events, and does not repeat. Typematic
//!   repeat is the host's job and nothing here does it; holding a key types it once.
//! - **Caps Lock, Num Lock and the LEDs are not handled**: the lock keys are reported as key codes
//!   and `video_terminal::keymap` maps none of them, so they do nothing.
//!
//! Name: provisional. Introduced 2026-10-04 for milestone 242 (USB host and HID). `usb` and not
//! `universal_serial_bus` by DECISIONS §154 (the acronym test is whether the phrase is spoken), applied the way `pci` was: nobody says
//! the expansion. An architect's call; expect it to be asked.

pub mod boot_keyboard;
pub mod descriptor;
pub mod request;

pub use descriptor::Refusal;

#[cfg(kani)]
mod verification {
    use super::boot_keyboard::{BootReport, changes};
    use super::descriptor::{ConfigurationHeader, DeviceDescriptor, find_boot_keyboard};

    /// **The configuration walk never panics, and anything it accepts is a real interrupt IN
    /// endpoint**, for every configuration of up to 24 bytes the solver can choose. This is the
    /// one function in the driver that walks a device-supplied linked structure.
    /// Falsification: unfalsified
    #[kani::proof]
    #[kani::unwind(26)]
    fn the_configuration_walk_is_total_and_accepts_only_an_interrupt_in() {
        let bytes: [u8; 24] = kani::any();
        let len: usize = kani::any();
        kani::assume(len <= bytes.len());
        if let Ok(k) = find_boot_keyboard(&bytes[..len]) {
            assert!(k.endpoint & 0x80 != 0, "an IN endpoint");
            assert!(k.endpoint & 0x0f != 0, "never endpoint zero");
            assert!(k.max_packet >= 8, "a whole boot report fits one packet");
        }
    }

    /// **The device descriptor and configuration header parsers are total.**
    /// Falsification: unfalsified
    #[kani::proof]
    fn the_header_parsers_are_total() {
        let bytes: [u8; 20] = kani::any();
        let len: usize = kani::any();
        kani::assume(len <= bytes.len());
        let _ = DeviceDescriptor::parse(&bytes[..len]);
        let _ = ConfigurationHeader::parse(&bytes[..len]);
    }

    /// **Two reports a keystroke apart produce at most twenty events**: eight modifier changes and
    /// six releases and six presses at the very most, so a driver's fixed buffer cannot overflow.
    /// Falsification: unfalsified
    #[kani::proof]
    #[kani::unwind(10)]
    fn two_reports_produce_a_bounded_number_of_events() {
        let a: [u8; 8] = kani::any();
        let b: [u8; 8] = kani::any();
        if let (Some(a), Some(b)) = (BootReport::parse(&a), BootReport::parse(&b)) {
            let mut n = 0usize;
            changes(&a, &b, &mut |_, _| n += 1);
            assert!(n <= 20);
        }
    }
}
