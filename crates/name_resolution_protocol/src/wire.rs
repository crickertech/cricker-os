//! **What a client and the name resolver put on the wire**, alone, so it can be copied verbatim
//! into std's PAL (`cargo xtask std-src` writes it as `sys/pal/nife/resolveproto.rs`), the way every
//! other contract the PAL speaks is. It names nothing outside itself, which is the condition of that
//! copy: the grant table and the reject codes need `domain_name_system`, and stay in `lib.rs`.
//!
//! Split out 2026-10-09 (UTC) by milestone 801 (packages over the internet)'s lane, whose std
//! programs resolve names through `std::net` (`ToSocketAddrs`) and so need these numbers in std.

/// The longest host name a client may write, in dotted text without a trailing dot. RFC 1035's
/// 255-byte wire limit is 253 characters of dotted text.
pub const NAME_TEXT_MAX: usize = 253;

/// How many addresses one answer carries back, the same bound `domain_name_system::Answer` keeps.
pub const MAX_ADDRESSES: usize = 8;

/// Where the client writes the host name in its page.
pub const OFF_NAME: usize = 0x000;
/// Where the resolver writes the addresses in the client's page, four bytes each, in the order the
/// name server gave them.
pub const OFF_ADDRESSES: usize = 0x100;

/// How many clients one resolver can hold grants for (see the crate's BUGS).
pub const GRANTS_MAX: usize = 8;

// =================================================================================================
// The words.
// =================================================================================================

/// **A client delegates its page.** A `SEND_CAP` of a writable page frame with this request word.
/// The resolver maps it in the window its badge's grant names; a page from a badge with no grant
/// is deleted unmapped.
pub const OPERATION_ATTACH_PAGE_FRAME: u64 = 1;
/// **A client resolves the name in its page.** A `CALL` whose second word is the name's length in
/// bytes. The reply is an [`Outcome`] word and the answer's TTL in seconds.
pub const OPERATION_RESOLVE: u64 = 2;
/// **The spawner grants a badge a zone.** A plain `SEND` through the unbadged capability, whose
/// first word is a [`control_word`] and whose second carries up to eight bytes of the zone's dotted
/// text, little-endian. Offset 0 starts the zone afresh, so a re-grant replaces rather than
/// appends. Starting at 0x10 so that no client opcode is ever also a control opcode.
pub const OPERATION_GRANT: u64 = 0x10;

/// A client's request word.
pub const fn request(operation: u64) -> u64 {
    operation << 56
}

/// The operation in any first word, client or control.
pub const fn operation(w0: u64) -> u64 {
    w0 >> 56
}

/// A control word: `operation << 56 | offset << 48 | total << 40 | badge`, the badge in the low 32
/// bits. `offset` and `total` are bytes of the zone's text.
pub const fn control_word(operation: u64, offset: u8, total: u8, badge: u32) -> u64 {
    (operation << 56) | ((offset as u64) << 48) | ((total as u64) << 40) | badge as u64
}

/// A control word's `(operation, offset, total, badge)`.
pub const fn control_fields(w0: u64) -> (u64, u8, u8, u32) {
    (
        w0 >> 56,
        (w0 >> 48) as u8,
        (w0 >> 40) as u8,
        (w0 & 0xffff_ffff) as u32,
    )
}

// =================================================================================================
// The reply.
// =================================================================================================

/// What a [`OPERATION_RESOLVE`] reply's first word says. Each is a distinct reason, because "no
/// address" is a different fact from "you may not ask", and a client that collapses them cannot
/// tell its own misconfiguration from the network's.
pub mod status {
    /// Resolved. The count is how many addresses are in the page.
    pub const OK: u8 = 0;
    /// **This client may not ask about this name**: no grant for its badge, or the name is outside
    /// the zone it was granted. Nothing was sent to the network.
    pub const DENIED: u8 = 1;
    /// The name server says the name does not exist (NXDOMAIN).
    pub const NO_SUCH_NAME: u8 = 2;
    /// A reply came back and `domain_name_system::Query::accept` refused it. The detail byte is
    /// the protocol crate's `reject_code` of the reason.
    pub const REFUSED: u8 = 3;
    /// No reply the resolver could believe arrived before it gave up.
    pub const NO_ANSWER: u8 = 4;
    /// The name server answered with a failure other than NXDOMAIN. The detail byte is its response
    /// code.
    pub const SERVER_ERROR: u8 = 5;
    /// The page did not hold a host name `domain_name_system::Name::from_dotted` accepts.
    pub const BAD_NAME: u8 = 6;
    /// The client has a grant and has not attached its page yet.
    pub const NOT_ATTACHED: u8 = 7;
    /// The resolver has no source of unguessable transaction ids, so it asks nothing.
    pub const NO_ENTROPY: u8 = 8;
    /// The socket contract refused the resolver. The detail byte names the step.
    pub const NETWORK: u8 = 9;
    /// An operation this protocol does not have, or a resolve from the spawner's own badge.
    pub const BAD_REQUEST: u8 = 10;
}

/// A resolve reply's first word, decoded: `status | detail << 8 | count << 16`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// One of [`status`].
    pub status: u8,
    /// What else the status carries: a reject code, a response code, a network step, or 0.
    pub detail: u8,
    /// How many addresses the resolver wrote at [`OFF_ADDRESSES`]: 0 unless the status is OK.
    pub count: u8,
}

impl Outcome {
    /// A status with no detail and no addresses.
    pub const fn of(status: u8) -> Outcome {
        Outcome {
            status,
            detail: 0,
            count: 0,
        }
    }

    /// The reply word.
    pub const fn word(self) -> u64 {
        self.status as u64 | (self.detail as u64) << 8 | (self.count as u64) << 16
    }

    /// A reply word back to its fields. The resolver never sets the high 40 bits, so a word with any
    /// of them set is not its reply: it is the kernel's error from a `CALL` that failed (a negative
    /// number, read as an enormous one), and decodes to `None`.
    pub const fn from_word(word: u64) -> Option<Outcome> {
        if word >> 24 != 0 {
            return None;
        }
        Some(Outcome {
            status: word as u8,
            detail: (word >> 8) as u8,
            count: (word >> 16) as u8,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_word_carries_its_operation() {
        for op in [
            OPERATION_ATTACH_PAGE_FRAME,
            OPERATION_RESOLVE,
            OPERATION_GRANT,
        ] {
            assert_eq!(operation(request(op)), op);
        }
    }

    #[test]
    fn a_control_word_comes_back_field_for_field() {
        let w = control_word(OPERATION_GRANT, 16, 253, 0x8010);
        assert_eq!(control_fields(w), (OPERATION_GRANT, 16, 253, 0x8010));
        assert_eq!(operation(w), OPERATION_GRANT);
    }

    #[test]
    fn an_outcome_word_round_trips_and_a_kernel_error_is_not_one() {
        let o = Outcome {
            status: status::OK,
            detail: 3,
            count: 2,
        };
        assert_eq!(Outcome::from_word(o.word()), Some(o));
        assert_eq!(Outcome::of(status::DENIED).word(), status::DENIED as u64);
        // A failed CALL returns a negative number, which no reply word looks like.
        assert_eq!(Outcome::from_word(-1i64 as u64), None);
    }
}
