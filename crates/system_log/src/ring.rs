//! **The 64 KiB ring**: rendered JSONL lines, oldest out, read by sequence number.
//!
//! Each entry is `len (u16) | seq (u64) | user length (u8) | user | line`, packed end to end and
//! wrapping. The stamped user is kept beside the line, outside the JSON, so a per-user read filters
//! without parsing anything (§242 (a system log): one store, filtered by the stamped user).
//!
//! Sequence numbers are dense, so where the ring starts says exactly what was evicted: a reader
//! whose cursor is below [`Ring::first_seq`] lost `first_seq - cursor` records, and is told so in
//! one line.

use crate::json;

/// The ring's size, ruled by §242 Question 5.
pub const RING_BYTES: usize = 64 * 1024;

const HDR: usize = 2 + 8 + 1;

/// The ring.
pub struct Ring {
    buf: [u8; RING_BYTES],
    head: usize,
    used: usize,
    count: usize,
    first_seq: u64,
    next_seq: u64,
}

impl Default for Ring {
    fn default() -> Self {
        Self::new()
    }
}

impl Ring {
    /// Empty.
    pub const fn new() -> Self {
        Ring {
            buf: [0; RING_BYTES],
            head: 0,
            used: 0,
            count: 0,
            first_seq: 0,
            next_seq: 0,
        }
    }

    /// The oldest sequence number still held (or the next to come, when empty).
    pub fn first_seq(&self) -> u64 {
        self.first_seq
    }

    /// How many records are held.
    pub fn held(&self) -> usize {
        self.count
    }

    fn put(&mut self, at: usize, bytes: &[u8]) {
        for (i, &b) in bytes.iter().enumerate() {
            self.buf[(at + i) % RING_BYTES] = b;
        }
    }

    fn get<const N: usize>(&self, at: usize) -> [u8; N] {
        core::array::from_fn(|i| self.buf[(at + i) % RING_BYTES])
    }

    fn entry_len(&self, at: usize) -> usize {
        u16::from_le_bytes(self.get::<2>(at)) as usize
    }

    fn entry_seq(&self, at: usize) -> u64 {
        u64::from_le_bytes(self.get::<8>(at + 2))
    }

    /// Append record `seq` (the next in sequence) for `user`, evicting the oldest until it fits.
    pub fn append(&mut self, seq: u64, user: &[u8], line: &[u8]) {
        let need = HDR + user.len() + line.len();
        debug_assert!(need <= RING_BYTES);
        while RING_BYTES - self.used < need {
            let len = self.entry_len(self.head);
            self.head = (self.head + len) % RING_BYTES;
            self.used -= len;
            self.count -= 1;
            self.first_seq = if self.count == 0 {
                seq
            } else {
                self.entry_seq(self.head)
            };
        }
        if self.count == 0 {
            self.first_seq = seq;
        }
        let at = (self.head + self.used) % RING_BYTES;
        self.put(at, &(need as u16).to_le_bytes());
        self.put(at + 2, &seq.to_le_bytes());
        self.put(at + 10, &[user.len() as u8]);
        self.put(at + HDR, user);
        self.put(at + HDR + user.len(), line);
        self.used += need;
        self.count += 1;
        self.next_seq = seq + 1;
    }

    /// Copy whole lines from `cursor` into `out`, only `user`'s when `user` is `Some`. Returns the
    /// cursor to read from next and how many bytes were written. A cursor below what the ring
    /// still holds gets one [`json::render_dropped`] line first; a cursor past the newest record is
    /// read as caught up.
    pub fn read(&self, cursor: u64, user: Option<&[u8]>, out: &mut [u8]) -> (u64, usize) {
        let cursor = cursor.min(self.next_seq);
        let mut n = 0;
        if cursor < self.first_seq {
            n += json::render_dropped(
                self.first_seq - cursor,
                cursor,
                self.first_seq - 1,
                &mut out[n..],
            );
        }
        let mut at = self.head;
        for _ in 0..self.count {
            let len = self.entry_len(at);
            let seq = self.entry_seq(at);
            let ulen = self.buf[(at + 10) % RING_BYTES] as usize;
            if seq >= cursor {
                let matches = match user {
                    None => true,
                    Some(u) => {
                        u.len() == ulen
                            && u.iter()
                                .enumerate()
                                .all(|(i, &b)| self.buf[(at + HDR + i) % RING_BYTES] == b)
                    }
                };
                if matches {
                    let line_len = len - HDR - ulen;
                    if n + line_len > out.len() {
                        return (seq, n);
                    }
                    for i in 0..line_len {
                        out[n + i] = self.buf[(at + HDR + ulen + i) % RING_BYTES];
                    }
                    n += line_len;
                }
            }
            at = (at + len) % RING_BYTES;
        }
        (self.next_seq.max(cursor), n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Entries that wrap the end of the buffer read back whole, and a full window stops at a line
    /// boundary with the cursor naming the first line it did not take.
    #[test]
    fn wrapped_entries_read_back_whole_and_a_full_window_stops_on_a_boundary() {
        extern crate std;
        let mut r = Ring::new();
        let line = [b'z'; 1000];
        let mut seq = 0;
        while r.first_seq() < 70 {
            let mut l = line;
            l[0] = b'0' + (seq % 10) as u8;
            l[999] = b'\n';
            r.append(seq, b"", &l);
            seq += 1;
        }
        let mut out = [0u8; 4080];
        let (next, n) = r.read(r.first_seq(), None, &mut out);
        assert_eq!(n, 4 * 1000);
        assert_eq!(next, r.first_seq() + 4);
        for (i, chunk) in out[..n].chunks(1000).enumerate() {
            assert_eq!(chunk[0], b'0' + ((r.first_seq() + i as u64) % 10) as u8);
            assert_eq!(chunk[999], b'\n');
        }
    }

    fn fill_exactly(r: &mut Ring, entries: usize) {
        // 11 bytes of header and 53 of line make a 64-byte entry, and 1,024 of them are the ring.
        let line = [b'x'; 53];
        for seq in 0..entries as u64 {
            r.append(seq, b"", &line);
        }
    }

    /// An entry that exactly fills what is left evicts nothing, and the next one evicts exactly one.
    #[test]
    fn an_exact_fit_evicts_nothing_and_the_next_entry_evicts_one() {
        let mut r = Ring::new();
        fill_exactly(&mut r, 1024);
        assert_eq!((r.held(), r.first_seq(), r.used), (1024, 0, RING_BYTES));
        r.append(1024, b"", &[b'x'; 53]);
        assert_eq!((r.held(), r.first_seq()), (1024, 1));
        fill_exactly(&mut Ring::new(), 0);
        let mut r = Ring::new();
        fill_exactly(&mut r, 1030);
        assert_eq!((r.held(), r.first_seq()), (1024, 6));
    }

    /// An entry is its header, its user and its line, and nothing else.
    #[test]
    fn an_entry_is_eleven_bytes_of_header_and_what_it_holds() {
        let mut r = Ring::new();
        r.append(0, b"ab", b"xyz");
        assert_eq!((r.used, r.held()), (11 + 2 + 3, 1));
        r.append(1, b"", b"q");
        assert_eq!((r.used, r.held()), (11 + 2 + 3 + 11 + 1, 2));
    }

    /// A reader whose cursor is partway into what was evicted is told how many it lost, counted from
    /// where it was and not from the start.
    #[test]
    fn a_dropped_line_counts_from_the_cursor() {
        let mut r = Ring::new();
        fill_exactly(&mut r, 1030);
        let mut out = [0u8; 4080];
        let (_, n) = r.read(2, None, &mut out);
        let text = core::str::from_utf8(&out[..n]).unwrap();
        assert!(
            text.starts_with("{\"dropped\":4,\"from\":2,\"to\":5}\n"),
            "{}",
            &text[..60]
        );
    }

    /// A per-user read matches the whole user, not a prefix of it.
    #[test]
    fn a_user_filter_matches_the_whole_name() {
        let mut r = Ring::new();
        r.append(0, b"alice", b"a\n");
        r.append(1, b"al", b"b\n");
        let mut out = [0u8; 64];
        let (_, n) = r.read(0, Some(b"al"), &mut out);
        assert_eq!(&out[..n], b"b\n");
        let (_, n) = r.read(0, Some(b"alice"), &mut out);
        assert_eq!(&out[..n], b"a\n");
    }

    /// A window that holds a line exactly takes it, and one a byte short leaves it for the next read.
    #[test]
    fn a_window_that_holds_a_line_exactly_takes_it() {
        let mut r = Ring::new();
        r.append(0, b"", &[b'z'; 53]);
        let mut out = [0u8; 53];
        assert_eq!(r.read(0, None, &mut out), (1, 53));
        let mut short = [0u8; 52];
        assert_eq!(r.read(0, None, &mut short), (0, 0));
    }
}
