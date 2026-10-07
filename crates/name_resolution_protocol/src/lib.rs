//! **What a name resolver and the programs around it agree on** (milestone 384 (in a capability
//! system the resolver is a grant); §248 (the name resolver is its own confined program)).
//!
//! Three parties share one endpoint, the resolver's, and each holds a different capability to it:
//!
//! ```text
//!   spawner ──unbadged WRITE──┐          (GRANT: badge B may resolve names in zone Z)
//!                             ▼
//!   client ──WRITE, badge B──► name_resolver ──Stack──► net_stack ──► a name server
//!   (ATTACH a page, then RESOLVE)    │
//!                                    └──entropy──► the transaction id
//! ```
//!
//! - **The spawner** holds the one unbadged capability. It mints a badged copy for each client
//!   (`abi::rendezvous::BADGE`), and tells the resolver what that badge may resolve with
//!   [`OPERATION_GRANT`] messages ([`grant_messages`] builds them). Only a message arriving with
//!   badge 0 is read as a grant, so a client cannot widen its own.
//! - **A client** holds a badged copy and nothing else of the resolver. It delegates one page
//!   ([`OPERATION_ATTACH_PAGE_FRAME`], a `SEND_CAP`), writes a host name into it, and `CALL`s
//!   [`OPERATION_RESOLVE`] with the name's length. The reply word is a [`status`] (with the address
//!   count and a detail byte, [`Outcome`]), the second word is the TTL, and the addresses are in
//!   the page at [`OFF_ADDRESSES`].
//! - **The resolver** keeps a [`Grants`] table, judges each name against the zone its caller's
//!   badge was granted **before it sends anything**, and asks the name server only about a name
//!   that passes.
//!
//! # What a grant means
//!
//! A grant is a **zone**: the zone's own name and every name under it, compared label by label
//! (`domain_name_system::Name::is_within`), so a client granted `nife.test` may resolve
//! `packages.nife.test` and may not resolve `evilnife.test`. A zone of one host name is an exact-name
//! grant in effect, and the root zone (an empty zone text) is "any name", for the client that truly
//! needs it. Those are the three options of Fork 2 in `notes/name-resolution.md`, expressed as one
//! mechanism rather than three.
//!
//! **The check applies to the name asked, not to the chain the answer follows.** A client granted
//! `packages.example.org` whose name is a CNAME into a CDN still gets the CDN's address, because the
//! answer is `domain_name_system::Query::accept`'s business and the grant is about what the client
//! may ask. What the grant stops is a client asking about a name outside its zone at all.
//!
//! # The page
//!
//! | bytes | field | written by |
//! |---|---|---|
//! | [`OFF_NAME`].. | the host name, dotted ASCII, at most [`NAME_TEXT_MAX`] bytes | the client |
//! | [`OFF_ADDRESSES`].. | up to [`MAX_ADDRESSES`] IPv4 addresses, four bytes each | the resolver |
//!
//! # EXAMPLES
//!
//! A spawner grants badge 7 the zone `nife.test`, and the resolver's table then answers for it:
//!
//! ```
//! use domain_name_system::Name;
//! use name_resolution_protocol::{Denied, Grants, grant_messages};
//!
//! let mut grants = Grants::new();
//! for (w0, w1) in grant_messages(7, b"nife.test").unwrap() {
//!     grants.control(w0, w1);
//! }
//! let asked = Name::from_dotted("packages.nife.test").unwrap();
//! assert!(grants.check(7, &asked).is_ok());
//! let elsewhere = Name::from_dotted("evilnife.test").unwrap();
//! assert_eq!(grants.check(7, &elsewhere), Err(Denied::OutsideZone));
//! assert_eq!(grants.check(8, &asked), Err(Denied::NoGrant));
//! ```
//!
//! # BUGS
//!
//! - **A grant lasts as long as the resolver.** There is no verb to withdraw one, and a badge's
//!   window is never reused, so a resolver serves at most [`GRANTS_MAX`] clients in its life. A
//!   spawner that outlives its clients wants a forget verb, and that verb wants a way to unmap the
//!   client's page that the resolver does not yet have.
//! - **One zone per badge.** A client that needs two unrelated domains needs two badges, or a
//!   grant of their common parent, which may be the root.
//! - **Grant messages carry eight bytes each**, because the resolver receives with `RECEIVE_CAP`
//!   (its clients `CALL` and `SEND_CAP`), which returns two data words, and the first is the control
//!   word. A 253-byte zone is 32 messages. It is the spawner's one-time cost per client.
//!
//! Name: provisional 2026-10-06 (UTC), milestone 384's lane. Spelled the way
//! `network_time_protocol` and `domain_name_system` are, by what it is the protocol of. Refused
//! `resolver_protocol`, which names the program rather than the act, and `dns_protocol` under §154
//! (the acronym test): the act is name resolution, and the wire format underneath is
//! `domain_name_system`'s.

#![no_std]

use domain_name_system::Name;

/// The longest host name a client may write, in dotted text without a trailing dot. RFC 1035's
/// 255-byte wire limit is 253 characters of dotted text.
pub const NAME_TEXT_MAX: usize = 253;

/// How many addresses one answer carries back, the same bound `domain_name_system::Answer` keeps.
pub const MAX_ADDRESSES: usize = domain_name_system::MAX_ADDRESSES;

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

/// The [`OPERATION_GRANT`] messages that grant `badge` the zone `zone` (dotted text, a trailing dot
/// allowed; empty for the root, which is every name). `None` when the badge is 0, which is the
/// spawner's own, or the text is longer than [`NAME_TEXT_MAX`] plus a trailing dot.
pub fn grant_messages(badge: u32, zone: &[u8]) -> Option<impl Iterator<Item = (u64, u64)> + '_> {
    if badge == 0 || zone.len() > NAME_TEXT_MAX + 1 {
        return None;
    }
    let total = zone.len() as u8;
    // `max(1)` so the root (no bytes) is still one message: a zero-length grant is a grant.
    let messages = zone.len().div_ceil(8).max(1);
    Some((0..messages).map(move |i| {
        let start = i * 8;
        let part = zone.get(start..).unwrap_or(&[]);
        let n = part.len().min(8);
        let mut bytes = [0u8; 8];
        bytes[..n].copy_from_slice(&part[..n]);
        (
            control_word(OPERATION_GRANT, start as u8, total, badge),
            u64::from_le_bytes(bytes),
        )
    }))
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
    /// [`crate::reject_code`] of the reason.
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

/// A stable small number for each `domain_name_system::Reject`, the detail of a
/// [`status::REFUSED`]: which check refused the reply, so that a forged answer and a broken server
/// are not one word.
pub const fn reject_code(reject: domain_name_system::Reject) -> u8 {
    use domain_name_system::Reject;
    match reject {
        Reject::Malformed(_) => 1,
        Reject::NotAResponse => 2,
        Reject::IdMismatch => 3,
        Reject::Truncated => 4,
        Reject::QuestionMismatch => 5,
        Reject::NoSuchName => 6,
        Reject::ServerError(_) => 7,
        Reject::ChainTooLong => 8,
        Reject::NoAddress => 9,
    }
}

// =================================================================================================
// The resolver's endowment.
// =================================================================================================

/// **What the resolver is told at spawn**, in its three start words. Which name server to ask is
/// the spawner's decision, made from the DHCP lease (`socket_protocol::lease`) or from a
/// configuration, never the resolver's, the way the network time client is told its server.
pub mod endowment {
    /// Ask over UDP, and again over TCP when the reply comes back truncated (RFC 1035 and RFC 7766's
    /// ordinary behavior).
    pub const TRANSPORT_UDP: u64 = 0;
    /// Ask over TCP only (RFC 7766 lets a stub resolver do this). For a name server that does not
    /// answer UDP at all, which is the QEMU runners' peer: slirp forwards only TCP to a host process.
    pub const TRANSPORT_TCP: u64 = 1;

    /// The first start word: the name server's IPv4 address, big-endian in the low 32 bits, the
    /// format `socket_protocol::lease::ipv4_word` reports a lease's name server in.
    pub const fn server(octets: [u8; 4]) -> u64 {
        u32::from_be_bytes(octets) as u64
    }

    /// The first start word back to an address.
    pub const fn server_octets(word: u64) -> [u8; 4] {
        (word as u32).to_be_bytes()
    }

    /// The second start word: the name server's port and the transport.
    pub const fn port_and_transport(port: u16, transport: u64) -> u64 {
        port as u64 | transport << 16
    }

    /// The second start word's `(port, transport)`.
    pub const fn port_and_transport_fields(word: u64) -> (u16, u64) {
        (word as u16, word >> 16)
    }
}

/// **The boot test's cases**, in one place for the two programs that must agree on them: the test
/// client asks each name in order, and the kernel's test (`system_tests/src/user/name_resolver_tests.rs`)
/// judges each reply by its index. `socket_protocol::fixture`'s split: the peer that answers,
/// `helpers/name-server-peer`, spells its zone again in Python, so the two sides are written
/// independently.
pub mod fixture {
    use domain_name_system::{Error, Reject};

    use super::{reject_code, status};

    /// The zone the test grants its client.
    pub const ZONE: &str = "nife.test";
    /// The runners' name server peer, a `guestfwd` to `helpers/name-server-peer` on DNS's port, and
    /// the address its zone answers with.
    pub const SERVER: [u8; 4] = [10, 0, 2, 9];

    /// The test client's `arg0` when it was given a stack and ends by connecting to what it
    /// resolved.
    pub const WITH_NETWORK: u64 = 1;
    /// The test client's last report carries one of these: the echo through the resolved address
    /// came back,
    pub const ECHO_OK: u64 = 1;
    /// it was given no stack,
    pub const ECHO_SKIPPED: u64 = 2;
    /// the first name did not resolve, so there was nowhere to connect,
    pub const ECHO_NO_ADDRESS: u64 = 3;
    /// or the connection or the echo failed.
    pub const ECHO_FAILED: u64 = 4;

    /// One name, and the reply the client must get for it.
    pub struct Case {
        /// The host name the client writes in its page.
        pub name: &'static str,
        /// The status the reply must carry.
        pub status: u8,
        /// Its detail byte.
        pub detail: u8,
    }

    const fn case(name: &'static str, status: u8, detail: u8) -> Case {
        Case {
            name,
            status,
            detail,
        }
    }

    /// The cases, in the order the client asks them. The first resolves, and is the address the
    /// client then connects to.
    pub const CASES: [Case; 10] = [
        case("packages.nife.test", status::OK, 0),
        // A compressed CNAME to the name above, the way a recursive server answers.
        case("mirror.nife.test", status::OK, 0),
        case("nosuch.nife.test", status::NO_SUCH_NAME, 0),
        // The peer's catch-all answer is REFUSED, response code 5.
        case("unlisted.nife.test", status::SERVER_ERROR, 5),
        // The right answer under the wrong transaction id.
        case(
            "forged.nife.test",
            status::REFUSED,
            reject_code(Reject::IdMismatch),
        ),
        // An address for bank.nife.test, and none for the name asked.
        case(
            "poisoned.nife.test",
            status::REFUSED,
            reject_code(Reject::NoAddress),
        ),
        // An owner that is a compression pointer to itself.
        case(
            "loop.nife.test",
            status::REFUSED,
            reject_code(Reject::Malformed(Error::PointerForward)),
        ),
        // Outside the zone, so never asked.
        case("example.com", status::DENIED, 0),
        // Ends in the zone's text and is not under it.
        case("evilnife.test", status::DENIED, 0),
        // Not a host name at all.
        case("two words.nife.test", status::BAD_NAME, 0),
    ];
}

// =================================================================================================
// The grant table.
// =================================================================================================

/// Why [`Grants::check`] refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denied {
    /// No grant was ever completed for this badge.
    NoGrant,
    /// The badge has a grant, and the name is not within its zone.
    OutsideZone,
}

/// What a control message did, for the resolver to ignore and a test to read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    /// A piece of a grant was taken; more are due.
    Partial,
    /// The last piece arrived and the zone parsed: the badge's grant is in force in this window.
    Granted(usize),
    /// Refused, and the badge's grant (if this was a re-grant) is gone rather than half-written.
    Refused(Refusal),
}

/// Why a control message was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// Not an operation the spawner has.
    UnknownOperation,
    /// Badge 0 is the spawner's own capability, so it cannot be granted anything.
    BadgeZero,
    /// A piece arrived out of order, or a continuation for a grant nobody started.
    OutOfOrder,
    /// The zone is longer than a name can be.
    TooLong,
    /// The zone's text is not a name `Name::from_dotted` accepts.
    BadZone,
    /// Every window already belongs to another badge.
    Full,
}

#[derive(Clone, Copy)]
struct Entry {
    /// 0 for a free entry: no client's badge is 0.
    badge: u32,
    /// In force only when `complete`.
    zone: Name,
    complete: bool,
    /// The zone's text so far, while the spawner's messages arrive.
    text: [u8; NAME_TEXT_MAX + 1],
    text_len: u8,
    text_total: u8,
}

impl Entry {
    const FREE: Entry = Entry {
        badge: 0,
        zone: Name::root(),
        complete: false,
        text: [0; NAME_TEXT_MAX + 1],
        text_len: 0,
        text_total: 0,
    };
}

/// **Which badge may resolve which zone**, and which window its page lives in.
///
/// An entry's index is its client's window, fixed for the resolver's life (see the crate's BUGS),
/// so the resolver maps a client's page at the same place every time and a re-grant does not move
/// it.
pub struct Grants {
    entries: [Entry; GRANTS_MAX],
}

impl Default for Grants {
    fn default() -> Self {
        Self::new()
    }
}

impl Grants {
    /// No grants: every badge is denied.
    pub const fn new() -> Grants {
        Grants {
            entries: [Entry::FREE; GRANTS_MAX],
        }
    }

    /// **Take one control message from the spawner.** The caller has already established that it
    /// arrived with badge 0; this reads only the words.
    pub fn control(&mut self, w0: u64, w1: u64) -> Control {
        let (op, offset, total, badge) = control_fields(w0);
        if op != OPERATION_GRANT {
            return Control::Refused(Refusal::UnknownOperation);
        }
        if badge == 0 {
            return Control::Refused(Refusal::BadgeZero);
        }
        if total as usize > NAME_TEXT_MAX + 1 {
            return self.refuse(badge, Refusal::TooLong);
        }
        let at = match self.position(badge) {
            Some(at) => at,
            None if offset == 0 => match self.entries.iter().position(|e| e.badge == 0) {
                Some(free) => free,
                None => return Control::Refused(Refusal::Full),
            },
            None => return Control::Refused(Refusal::OutOfOrder),
        };
        let e = &mut self.entries[at];
        if offset == 0 {
            // A fresh grant, or a re-grant: the old zone stops applying now, not when the new one
            // completes, so there is never a moment where a half-sent zone is in force.
            e.badge = badge;
            e.complete = false;
            e.text_len = 0;
            e.text_total = total;
        } else if e.complete || offset != e.text_len || total != e.text_total {
            return self.refuse(badge, Refusal::OutOfOrder);
        }
        let bytes = w1.to_le_bytes();
        let n = (e.text_total - e.text_len).min(8) as usize;
        let start = e.text_len as usize;
        e.text[start..start + n].copy_from_slice(&bytes[..n]);
        e.text_len += n as u8;
        if e.text_len < e.text_total {
            return Control::Partial;
        }
        let zone = core::str::from_utf8(&e.text[..e.text_len as usize])
            .ok()
            .and_then(|text| Name::from_dotted(text).ok());
        match zone {
            Some(zone) => {
                e.zone = zone;
                e.complete = true;
                Control::Granted(at)
            }
            None => self.refuse(badge, Refusal::BadZone),
        }
    }

    /// **May `badge` ask about `name`?** The window its page is in, or why not.
    pub fn check(&self, badge: u32, name: &Name) -> Result<usize, Denied> {
        let at = self.granted(badge).ok_or(Denied::NoGrant)?;
        if name.is_within(&self.entries[at].zone) {
            Ok(at)
        } else {
            Err(Denied::OutsideZone)
        }
    }

    /// The window of `badge`'s grant, if one is in force. Where its page goes when it attaches one.
    pub fn granted(&self, badge: u32) -> Option<usize> {
        self.position(badge).filter(|&at| self.entries[at].complete)
    }

    fn position(&self, badge: u32) -> Option<usize> {
        if badge == 0 {
            return None;
        }
        self.entries.iter().position(|e| e.badge == badge)
    }

    /// A refused grant leaves nothing behind: the badge keeps its window (it is still the badge's,
    /// see the BUGS), and has no zone.
    fn refuse(&mut self, badge: u32, why: Refusal) -> Control {
        if let Some(at) = self.position(badge) {
            let e = &mut self.entries[at];
            e.complete = false;
            e.text_len = 0;
            e.text_total = 0;
        }
        Control::Refused(why)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec::Vec;

    use super::*;

    fn grant(g: &mut Grants, badge: u32, zone: &str) -> Control {
        // The first refusal, or the last message's outcome: once a piece is refused, the ones after
        // it are continuations of nothing, and their own refusal would hide the reason.
        let mut last = Control::Partial;
        for (w0, w1) in grant_messages(badge, zone.as_bytes()).unwrap() {
            last = g.control(w0, w1);
            if matches!(last, Control::Refused(_)) {
                break;
            }
        }
        last
    }

    fn name(text: &str) -> Name {
        Name::from_dotted(text).unwrap()
    }

    #[test]
    fn a_zone_grant_answers_for_the_zone_and_everything_under_it_and_nothing_beside_it() {
        let mut g = Grants::new();
        assert_eq!(grant(&mut g, 5, "nife.test"), Control::Granted(0));
        assert_eq!(g.check(5, &name("nife.test")), Ok(0));
        assert_eq!(g.check(5, &name("packages.nife.test")), Ok(0));
        assert_eq!(g.check(5, &name("a.b.c.NIFE.Test")), Ok(0));
        assert_eq!(g.check(5, &name("evilnife.test")), Err(Denied::OutsideZone));
        assert_eq!(
            g.check(5, &name("nife.test.evil")),
            Err(Denied::OutsideZone)
        );
        assert_eq!(g.check(5, &name("test")), Err(Denied::OutsideZone));
        assert_eq!(g.check(5, &name("example.com")), Err(Denied::OutsideZone));
    }

    #[test]
    fn a_badge_with_no_grant_and_the_spawners_own_badge_are_denied() {
        let mut g = Grants::new();
        grant(&mut g, 5, "");
        assert_eq!(g.check(6, &name("anything.test")), Err(Denied::NoGrant));
        assert_eq!(g.check(0, &name("anything.test")), Err(Denied::NoGrant));
        assert!(grant_messages(0, b"nife.test").is_none());
        assert_eq!(
            g.control(control_word(OPERATION_GRANT, 0, 0, 0), 0),
            Control::Refused(Refusal::BadgeZero)
        );
    }

    #[test]
    fn the_root_zone_is_every_name_and_one_host_name_is_exactly_itself() {
        let mut g = Grants::new();
        assert_eq!(grant(&mut g, 1, ""), Control::Granted(0));
        assert_eq!(grant(&mut g, 2, "packages.nife.test."), Control::Granted(1));
        assert_eq!(g.check(1, &name("example.com")), Ok(0));
        assert_eq!(g.check(2, &name("packages.nife.test")), Ok(1));
        assert_eq!(
            g.check(2, &name("mirror.nife.test")),
            Err(Denied::OutsideZone)
        );
    }

    #[test]
    fn a_longest_zone_crosses_in_thirty_two_messages_and_still_parses() {
        // Four 63-byte labels and their dots: 255 characters, past what a name can be.
        let label = "a".repeat(63);
        let too_long = [label.as_str(); 4].join(".");
        assert!(grant_messages(3, too_long.as_bytes()).is_none());
        // 61 + 63 * 3 + 3 dots = 253, the longest dotted name there is.
        let longest = std::format!("{}.{label}.{label}.{label}", "b".repeat(61));
        assert_eq!(longest.len(), NAME_TEXT_MAX);
        assert_eq!(
            grant_messages(3, longest.as_bytes()).unwrap().count(),
            NAME_TEXT_MAX.div_ceil(8)
        );
        let mut g = Grants::new();
        assert_eq!(grant(&mut g, 3, &longest), Control::Granted(0));
        assert_eq!(g.check(3, &name(&longest)), Ok(0));
    }

    #[test]
    fn a_regrant_replaces_the_zone_in_the_same_window_and_is_not_in_force_until_whole() {
        let mut g = Grants::new();
        grant(&mut g, 9, "nife.test");
        let messages: Vec<_> = grant_messages(9, b"example.org").unwrap().collect();
        assert_eq!(messages.len(), 2);
        assert_eq!(g.control(messages[0].0, messages[0].1), Control::Partial);
        // Half way through, neither the old zone nor a prefix of the new one is in force.
        assert_eq!(g.check(9, &name("nife.test")), Err(Denied::NoGrant));
        assert_eq!(g.granted(9), None);
        assert_eq!(g.control(messages[1].0, messages[1].1), Control::Granted(0));
        assert_eq!(g.check(9, &name("www.example.org")), Ok(0));
        assert_eq!(g.check(9, &name("nife.test")), Err(Denied::OutsideZone));
    }

    #[test]
    fn pieces_out_of_order_or_unstarted_refuse_and_leave_no_grant() {
        let mut g = Grants::new();
        let messages: Vec<_> = grant_messages(4, b"packages.nife.test").unwrap().collect();
        // A continuation for a grant nobody started.
        assert_eq!(
            g.control(messages[1].0, messages[1].1),
            Control::Refused(Refusal::OutOfOrder)
        );
        g.control(messages[0].0, messages[0].1);
        // The third piece before the second.
        assert_eq!(
            g.control(messages[2].0, messages[2].1),
            Control::Refused(Refusal::OutOfOrder)
        );
        assert_eq!(
            g.check(4, &name("packages.nife.test")),
            Err(Denied::NoGrant)
        );
    }

    #[test]
    fn a_zone_that_is_not_a_name_is_refused_and_so_is_an_unknown_operation() {
        let mut g = Grants::new();
        assert_eq!(
            grant(&mut g, 2, "nife..test"),
            Control::Refused(Refusal::BadZone)
        );
        assert_eq!(
            grant(&mut g, 2, "nife test"),
            Control::Refused(Refusal::BadZone)
        );
        assert_eq!(g.granted(2), None);
        assert_eq!(
            g.control(control_word(0x7f, 0, 0, 2), 0),
            Control::Refused(Refusal::UnknownOperation)
        );
    }

    #[test]
    fn the_ninth_badge_finds_every_window_taken() {
        let mut g = Grants::new();
        for badge in 1..=GRANTS_MAX as u32 {
            assert_eq!(
                grant(&mut g, badge, "nife.test"),
                Control::Granted(badge as usize - 1)
            );
        }
        assert_eq!(
            grant(&mut g, 99, "nife.test"),
            Control::Refused(Refusal::Full)
        );
        assert_eq!(g.check(99, &name("nife.test")), Err(Denied::NoGrant));
    }

    #[test]
    fn an_outcome_round_trips_and_a_kernel_error_is_not_one() {
        let o = Outcome {
            status: status::REFUSED,
            detail: reject_code(domain_name_system::Reject::IdMismatch),
            count: 0,
        };
        assert_eq!(Outcome::from_word(o.word()), Some(o));
        let ok = Outcome {
            status: status::OK,
            detail: 0,
            count: 8,
        };
        assert_eq!(Outcome::from_word(ok.word()), Some(ok));
        // A `CALL` on an empty slot returns a small negative error, read as a huge u64.
        assert_eq!(Outcome::from_word(-3i64 as u64), None);
    }

    #[test]
    fn the_endowment_words_round_trip() {
        let w = endowment::server([10, 0, 2, 9]);
        assert_eq!(endowment::server_octets(w), [10, 0, 2, 9]);
        let w = endowment::port_and_transport(53, endowment::TRANSPORT_TCP);
        assert_eq!(
            endowment::port_and_transport_fields(w),
            (53, endowment::TRANSPORT_TCP)
        );
    }

    #[test]
    fn every_reject_has_its_own_code() {
        use domain_name_system::{Error, Reject};
        let all = [
            Reject::Malformed(Error::Truncated),
            Reject::NotAResponse,
            Reject::IdMismatch,
            Reject::Truncated,
            Reject::QuestionMismatch,
            Reject::NoSuchName,
            Reject::ServerError(2),
            Reject::ChainTooLong,
            Reject::NoAddress,
        ];
        let mut codes: Vec<u8> = all.iter().map(|&r| reject_code(r)).collect();
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes.len(), all.len());
        assert!(!codes.contains(&0));
    }
}
