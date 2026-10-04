//! **The boot keyboard report, and what changed between two of them** (HID 1.11 Appendix B.1,
//! HID Usage Tables 1.12 section 10).
//!
//! A boot keyboard does not send key events. It sends its **state**: eight bytes, the modifier
//! keys as a bitmap in byte 0, a reserved byte, then up to six usage ids for the other keys held
//! down, in no promised order. So a keystroke is a *difference* between two reports, and
//! [`changes`] computes it, in Linux's evdev numbering so that `video_terminal::keymap` (which
//! already turns the virtio keyboard's evdev codes into terminal bytes) does the rest. The table
//! from usage id to evdev code is [`evdev_code`]'s, and it is the one in Linux's
//! `drivers/hid/hid-input.c` (`hid_keyboard[]`) for the range a terminal keyboard uses.

/// How long a boot report is.
pub const REPORT_LEN: usize = 8;

/// The usage id a keyboard puts in every key slot when more keys are held than it can report
/// (`ErrorRollOver`, Usage Tables section 10). A report carrying it describes no real state.
pub const ERROR_ROLL_OVER: u8 = 0x01;

/// The evdev codes of the eight modifier keys, in the bit order of byte 0: left control, shift,
/// alt, GUI, then the right-hand four.
pub const MODIFIER_CODES: [u16; 8] = [29, 42, 56, 125, 97, 54, 100, 126];

/// **One boot report**: the modifier bitmap and the six key slots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BootReport {
    /// Byte 0: one bit per modifier key, in [`MODIFIER_CODES`]' order.
    pub modifiers: u8,
    /// Bytes 2..8: usage ids of the keys held, zero for an empty slot.
    pub keys: [u8; 6],
}

impl BootReport {
    /// Nothing held: the state before the first report.
    pub const RELEASED: BootReport = BootReport {
        modifiers: 0,
        keys: [0; 6],
    };

    /// Parse a report. `None` for fewer than [`REPORT_LEN`] bytes, and for a **phantom** report
    /// (any slot holding [`ERROR_ROLL_OVER`] or the two other error usages, 2 and 3), which a
    /// driver must ignore rather than read as every key released.
    pub fn parse(b: &[u8]) -> Option<BootReport> {
        if b.len() < REPORT_LEN {
            return None;
        }
        let mut keys = [0u8; 6];
        keys.copy_from_slice(&b[2..8]);
        if keys.iter().any(|&k| (1..=3).contains(&k)) {
            return None;
        }
        Some(BootReport {
            modifiers: b[0],
            keys,
        })
    }

    fn holds(&self, usage: u8) -> bool {
        usage != 0 && self.keys.contains(&usage)
    }
}

/// **Usage id to evdev code**, for the keyboard page's usages 0x04 through 0x63: the letters,
/// digits, the main block, the function keys, the navigation cluster, the arrows and the keypad.
/// `None` for anything else, including 0 (no key). From Linux's `hid_keyboard[]`, which is the
/// table every evdev consumer was written against.
pub fn evdev_code(usage: u8) -> Option<u16> {
    /// Indexed by usage id from 0x04. Zero where Linux has no code.
    const TABLE: [u8; 0x60] = [
        30, 48, 46, 32, 18, 33, 34, 35, 23, 36, 37, 38, // 0x04 a .. 0x0f l
        50, 49, 24, 25, 16, 19, 31, 20, 22, 47, 17, 45, 21, 44, // 0x10 m .. 0x1d z
        2, 3, 4, 5, 6, 7, 8, 9, 10, 11, // 0x1e 1 .. 0x27 0
        28, 1, 14, 15, 57, 12, 13, 26, 27, 43, 43, 39, 40, 41, 51, 52,
        53, // 0x28 enter .. 0x38 /
        58, // 0x39 caps lock
        59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 87, 88, // 0x3a F1 .. 0x45 F12
        99, 70, 119, // 0x46 print screen, scroll lock, pause
        110, 102, 104, 111, 107, 109, // 0x49 insert, home, page up, delete, end, page down
        106, 105, 108, 103, // 0x4f right, left, down, up
        69, 98, 55, 74, 78, 96, // 0x53 num lock, keypad / * - + enter
        79, 80, 81, 75, 76, 77, 71, 72, 73, 82, 83, // 0x59 keypad 1 .. 9, 0, .
    ];
    let code = *TABLE.get(usize::from(usage).checked_sub(4)?)?;
    (code != 0).then_some(u16::from(code))
}

/// **What changed from `before` to `after`**, as `(evdev code, pressed)` calls to `event`, in the
/// order that types correctly: every modifier change first, then the keys released, then the keys
/// pressed. Shift and a letter arriving in one report must reach the keymap shift first, which is
/// why modifiers lead; a key released and another pressed in one report is a roll, and the release
/// first is what a person did. A usage with no evdev code is skipped.
///
/// At most twenty calls: eight modifiers, six releases, six presses.
pub fn changes(before: &BootReport, after: &BootReport, event: &mut dyn FnMut(u16, bool)) {
    let flipped = before.modifiers ^ after.modifiers;
    for (bit, &code) in MODIFIER_CODES.iter().enumerate() {
        if flipped & (1 << bit) != 0 {
            event(code, after.modifiers & (1 << bit) != 0);
        }
    }
    for &usage in &before.keys {
        if usage != 0
            && !after.holds(usage)
            && let Some(code) = evdev_code(usage)
        {
            event(code, false);
        }
    }
    for (i, &usage) in after.keys.iter().enumerate() {
        // `after.keys[..i]` so a usage a confused device reports twice is pressed once.
        if usage != 0
            && !before.holds(usage)
            && !after.keys[..i].contains(&usage)
            && let Some(code) = evdev_code(usage)
        {
            event(code, true);
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec::Vec;

    use super::*;

    fn report(modifiers: u8, keys: &[u8]) -> BootReport {
        let mut b = [0u8; 8];
        b[0] = modifiers;
        b[2..2 + keys.len()].copy_from_slice(keys);
        BootReport::parse(&b).unwrap()
    }

    fn diff(a: &BootReport, b: &BootReport) -> Vec<(u16, bool)> {
        let mut v = Vec::new();
        changes(a, b, &mut |c, p| v.push((c, p)));
        v
    }

    /// The table's rows land where the keycaps say: an off-by-one row puts `s` on the `a` key.
    #[test]
    fn the_usage_table_matches_the_keycaps() {
        for (usage, code) in [
            (0x04, 30), // a
            (0x1d, 44), // z
            (0x1e, 2),  // 1
            (0x27, 11), // 0
            (0x28, 28), // enter
            (0x2a, 14), // backspace
            (0x2c, 57), // space
            (0x38, 53), // slash
            (0x4f, 106),
            (0x52, 103), // up
            (0x63, 83),  // keypad dot, the table's last row
        ] {
            assert_eq!(evdev_code(usage), Some(code), "usage {usage:#x}");
        }
        assert_eq!(evdev_code(0), None);
        assert_eq!(evdev_code(3), None);
        assert_eq!(evdev_code(0x64), None);
        assert_eq!(evdev_code(0xff), None);
    }

    /// **`echo hello`, typed the way QEMU's `sendkey` presses it**: one key down, then up, eleven
    /// times. What the end-to-end gate types, decoded on the host.
    #[test]
    fn echo_hello_decodes_key_by_key() {
        let mut state = BootReport::RELEASED;
        let mut presses = Vec::new();
        for usage in [
            0x08u8, 0x06, 0x0b, 0x12, 0x2c, 0x0b, 0x08, 0x0f, 0x0f, 0x12, 0x28,
        ] {
            for next in [report(0, &[usage]), BootReport::RELEASED] {
                changes(&state, &next, &mut |code, pressed| {
                    if pressed {
                        presses.push(code);
                    }
                });
                state = next;
            }
        }
        // e c h o space h e l l o enter, in evdev codes.
        assert_eq!(presses, [18, 46, 35, 24, 57, 35, 18, 38, 38, 24, 28]);
    }

    #[test]
    fn shift_and_a_letter_together_deliver_shift_first() {
        assert_eq!(
            diff(&BootReport::RELEASED, &report(0x20, &[0x04])), // right shift + a
            [(54, true), (30, true)]
        );
    }

    /// A roll: `a` still held, `b` pressed, then `a` released while `b` stays.
    #[test]
    fn a_roll_presses_each_key_once_and_releases_it_once() {
        let a = report(0, &[0x04]);
        let ab = report(0, &[0x04, 0x05]);
        let b = report(0, &[0x05]);
        assert_eq!(diff(&a, &ab), [(48, true)]);
        // Slot order is not promised, so `b` moving to slot 0 must not read as a new press.
        assert_eq!(diff(&ab, &b), [(30, false)]);
    }

    #[test]
    fn a_phantom_report_is_refused_rather_than_read_as_a_release() {
        assert_eq!(BootReport::parse(&[0, 0, 1, 1, 1, 1, 1, 1]), None);
        assert_eq!(BootReport::parse(&[0, 0, 0, 0]), None);
    }

    /// A device that reports one usage in two slots presses it once.
    #[test]
    fn a_duplicated_usage_presses_once() {
        assert_eq!(
            diff(&BootReport::RELEASED, &report(0, &[0x04, 0x04])),
            [(30, true)]
        );
    }
}
