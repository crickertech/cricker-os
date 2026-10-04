//! **The port status and control register** (`PORTSC`, xHCI 1.2 section 5.4.8), and the trap in it.
//!
//! Half its bits are write-one-to-clear, and one of those is Port Enabled: writing back what was
//! read, with a single bit added, **disables the port** and acknowledges every pending change
//! nobody has looked at yet. So a write is always [`write_value`]: the bits that are safe to write
//! back, plus the one the caller means.

/// Current Connect Status: a device is attached.
pub const CONNECTED: u32 = 1 << 0;
/// Port Enabled. **Write one to clear, which disables the port.**
pub const ENABLED: u32 = 1 << 1;
/// Port Reset: write one to reset the port, reads one until the reset finishes.
pub const RESET: u32 = 1 << 4;
/// Port Power.
pub const POWER: u32 = 1 << 9;
/// Connect Status Change, write one to clear.
pub const CONNECT_CHANGE: u32 = 1 << 17;
/// Port Enabled/Disabled Change, write one to clear.
pub const ENABLE_CHANGE: u32 = 1 << 18;
/// Warm Port Reset Change, write one to clear.
pub const WARM_RESET_CHANGE: u32 = 1 << 19;
/// Over-current Change, write one to clear.
pub const OVER_CURRENT_CHANGE: u32 = 1 << 20;
/// Port Reset Change, write one to clear.
pub const RESET_CHANGE: u32 = 1 << 21;
/// Port Link State Change, write one to clear.
pub const LINK_CHANGE: u32 = 1 << 22;
/// Port Config Error Change, write one to clear.
pub const CONFIG_ERROR_CHANGE: u32 = 1 << 23;

/// Every change bit: what a driver writes back to acknowledge all of them.
pub const CHANGES: u32 = CONNECT_CHANGE
    | ENABLE_CHANGE
    | WARM_RESET_CHANGE
    | OVER_CURRENT_CHANGE
    | RESET_CHANGE
    | LINK_CHANGE
    | CONFIG_ERROR_CHANGE;

/// The bits a write may carry back unchanged: the read-only ones (ignored on write) and the
/// read-write ones that must keep their value (power, indicator, link state without its strobe,
/// wake enables). The same set Linux's `xhci_port_state_to_neutral` keeps.
pub const PRESERVE: u32 =
    1 | 1 << 3 | 0xf << 10 | 1 << 30 | 0xf << 5 | 1 << 9 | 0x3 << 14 | 0x7 << 25;

/// Port speed ids, the default ones every xHCI uses (xHCI 1.2 Table 7-13).
pub mod speed {
    /// Full speed, 12 Mb/s: most keyboards.
    pub const FULL: u8 = 1;
    /// Low speed, 1.5 Mb/s: cheap keyboards.
    pub const LOW: u8 = 2;
    /// High speed, 480 Mb/s.
    pub const HIGH: u8 = 3;
    /// `SuperSpeed`, 5 Gb/s.
    pub const SUPER: u8 = 4;
    /// `SuperSpeedPlus`, 10 Gb/s.
    pub const SUPER_PLUS: u8 = 5;
}

/// The attached device's speed id, from bits 13:10.
pub const fn speed_of(portsc: u32) -> u8 {
    ((portsc >> 10) & 0xf) as u8
}

/// **What to write to set `bits`** without disabling the port or acknowledging a change nobody
/// asked to: `portsc`'s preserved bits, plus `bits`.
pub const fn write_value(portsc: u32, bits: u32) -> u32 {
    (portsc & PRESERVE) | bits
}

/// **Endpoint 0's packet size before the device has said**, by speed (xHCI 1.2 section 4.3): 8 for low
/// and full speed (a full-speed device may want up to 64, which the driver learns from the first
/// eight bytes of its descriptor and fixes with Evaluate Context), 64 for high speed, 512 above.
/// `None` for a speed id this table does not know.
pub const fn default_control_packet(speed: u8) -> Option<u16> {
    match speed {
        speed::LOW | speed::FULL => Some(8),
        speed::HIGH => Some(64),
        speed::SUPER | speed::SUPER_PLUS => Some(512),
        _ => None,
    }
}

/// The name a bench transcript prints for a speed id.
pub const fn speed_name(speed: u8) -> &'static str {
    match speed {
        speed::LOW => "low speed",
        speed::FULL => "full speed",
        speed::HIGH => "high speed",
        speed::SUPER => "SuperSpeed",
        speed::SUPER_PLUS => "SuperSpeedPlus",
        _ => "an unknown speed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **The trap, closed**: a port that is enabled with a reset change pending, written to start
    /// another reset, keeps its enable bit OFF in the written value (writing it would disable the
    /// port) and does not acknowledge the change.
    #[test]
    fn a_write_never_carries_the_enable_bit_or_a_change() {
        let read = CONNECTED | ENABLED | POWER | RESET_CHANGE | 3 << 10;
        let w = write_value(read, RESET);
        assert_eq!(w & ENABLED, 0);
        assert_eq!(w & CHANGES, 0);
        assert_ne!(w & POWER, 0);
        assert_ne!(w & RESET, 0);
        assert_eq!(speed_of(read), speed::HIGH);
    }

    #[test]
    fn the_preserved_set_is_linuxs() {
        assert_eq!(PRESERVE, 0x4e00_ffe9);
    }

    #[test]
    fn endpoint_zero_starts_at_the_speeds_minimum() {
        assert_eq!(default_control_packet(speed::LOW), Some(8));
        assert_eq!(default_control_packet(speed::FULL), Some(8));
        assert_eq!(default_control_packet(speed::HIGH), Some(64));
        assert_eq!(default_control_packet(speed::SUPER), Some(512));
        assert_eq!(default_control_packet(0), None);
    }
}
