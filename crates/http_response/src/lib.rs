//! **`http_response`**: one HTTP/1.x `GET`, written, and the response to it, read as it arrives.
//!
//! Rung 3a of milestone 198 (a package manager, and the trivial install that makes a second
//! customer possible) fetches a package from a host on the same network over plain HTTP. DECISIONS
//! §196 (nife carries TLS) rules HTTPS for the internet rung, and under §195 (a reviewed recipe
//! vouches for a package) a package's digest, not its transport, decides whether its bytes may run,
//! so the LAN rung needs a plain client and nothing more. This crate is that client's two halves
//! that are pure logic: the request line, and a reader for the response that never holds the body.
//!
//! **The reader is incremental and holds only the head.** A response arrives a socket read at a
//! time, at most `socket_protocol::DATA_MAX` bytes each, and the body is a package that may be a
//! megabyte. [`Response::feed`] takes each read, keeps the status line and headers in a fixed
//! buffer, and hands back the part of the read that is body, so a caller can hash it and write it
//! somewhere without this crate owning a byte of it. That is the same borrowing shape
//! `package_archive` uses, for the same reason: the first consumer has no allocator.
//!
//! **It is strict, because every byte it reads came off a network.** A response it cannot read
//! exactly is refused rather than guessed at:
//!
//! - **`Content-Length` is required.** HTTP/1.0 lets a server end a body by closing the
//!   connection, but a client that accepts that cannot tell a complete package from a truncated
//!   one, and the net server reports a peer's close as a failed receive after a bounded wait, not
//!   as an end. A digest would still catch the truncation; refusing it here says why.
//! - **`Transfer-Encoding` is refused**, chunked included. A `GET` sent as HTTP/1.0 is never owed a
//!   chunked reply, so a server sending one is not speaking to us.
//! - **Lines end in CRLF**, a header section is at most [`HEAD_MAX`] bytes, a duplicated
//!   `Content-Length` must agree with itself, and a body longer than it said is refused.
//!
//! # EXAMPLES
//!
//! ```
//! use http_response::{Response, get_request};
//!
//! let mut request = [0u8; 128];
//! let n = get_request("10.0.2.9", "/uptime-0.1.0-aarch64.nifepkg", &mut request).unwrap();
//! assert!(request[..n].starts_with(b"GET /uptime-0.1.0-aarch64.nifepkg HTTP/1.0\r\n"));
//!
//! let mut response = Response::new();
//! // The reads a socket hands back need not line up with anything.
//! let body = response.feed(b"HTTP/1.0 200 OK\r\nContent-Le").unwrap();
//! assert!(body.is_empty());
//! let body = response.feed(b"ngth: 5\r\n\r\nhel").unwrap();
//! assert_eq!(body, b"hel");
//! assert_eq!(response.status(), Some(200));
//! assert!(!response.is_complete());
//! assert_eq!(response.feed(b"lo").unwrap(), b"lo");
//! assert!(response.is_complete());
//! ```
//!
//! # BUGS
//!
//! - **No redirects, no keep-alive, no `Range`.** One request per connection, and a `3xx` is a
//!   status like any other that the caller refuses. A package source that moves is found again by
//!   its catalogue, not by following a server.
//! - **A close-delimited body is refused**, per the list above. A server that omits
//!   `Content-Length` cannot serve a package to nife.
//! - **Header values other than the two it reads are not validated.** They are skipped, not
//!   interpreted, so a malformed `Date` costs nothing; a malformed line (no colon) is refused.
//! - **Fuzzed, not proved.** `fuzz/fuzz_targets/http_response_feed.rs` checks that how the reads
//!   fall never changes the answer (46 million inputs in fifteen minutes on 2026-09-25, no crash).
//!   A Kani harness over two symbolic 22-byte reads was still in the solver after twenty minutes,
//!   and one over a single 36-byte read had not finished unwinding after five: the `contains` and
//!   per-byte `ends_with` scans unroll badly. That is a CI shard's minutes for a bound far below
//!   the 2 KiB head the fuzzer reaches, so the treatment `crates/elf` carries is not proportionate
//!   here yet.
//!
//! Name: provisional 2026-09-24 (milestone 198's rung 3a consumer lane). `http` alone would claim
//! more than a request writer and a response reader; `design/naming.md` is the rule and calef's the
//! call.

#![no_std]

/// The most a status line plus headers may take. A package server's head is a few hundred bytes;
/// this is enough for a real server's, and it is a bound, so a peer that never ends its headers is
/// refused rather than read forever.
pub const HEAD_MAX: usize = 2048;

/// Why a response was refused. Every variant is a peer that is not speaking the HTTP this reader
/// accepts; none of them is recoverable by reading more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    /// The status line and headers ran past [`HEAD_MAX`] without the blank line that ends them.
    HeadTooLong,
    /// The first line is not `HTTP/1.<digit> <three digits> ...`.
    BadStatusLine,
    /// A header line is not CRLF-terminated `name: value`, or a line ends in a bare LF.
    BadHeader,
    /// No `Content-Length`, so the end of the body could not be told from a dropped connection.
    NoContentLength,
    /// `Content-Length` is not a decimal number that fits a `u64`, or appears twice with two values.
    BadContentLength,
    /// A `Transfer-Encoding` header, which a response to an HTTP/1.0 request should never carry.
    TransferEncoding,
    /// More body arrived than `Content-Length` said there would be.
    BodyTooLong,
}

/// Write `GET <path> HTTP/1.0` with a `Host` header into `out`, returning its length, or `None` if
/// it does not fit or if `host` or `path` could smuggle a second line.
///
/// HTTP/1.0 rather than 1.1 so the server owes a plain, length-delimited or close-delimited body
/// and never a chunked one; [`Response`] refuses the close-delimited kind on its own terms.
pub fn get_request(host: &str, path: &str, out: &mut [u8]) -> Option<usize> {
    let clean = |s: &str| !s.is_empty() && s.bytes().all(|b| b > b' ' && b != 0x7f);
    if !clean(host) || !clean(path) || !path.starts_with('/') {
        return None;
    }
    let parts: [&[u8]; 5] = [
        b"GET ",
        path.as_bytes(),
        b" HTTP/1.0\r\nHost: ",
        host.as_bytes(),
        b"\r\n\r\n",
    ];
    let mut at = 0;
    for part in parts {
        let end = at + part.len();
        out.get_mut(at..end)?.copy_from_slice(part);
        at = end;
    }
    Some(at)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Collecting the head; `head_len` bytes of it are in `head`.
    Head,
    /// The head is read; `seen` of `length` body bytes have been handed back.
    Body { length: u64, seen: u64 },
}

/// A response being read. See the crate documentation for what it accepts.
pub struct Response {
    head: [u8; HEAD_MAX],
    head_len: usize,
    status: Option<u16>,
    state: State,
}

/// The head buffer is left out: it is up to [`HEAD_MAX`] bytes of whatever the peer sent.
impl core::fmt::Debug for Response {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Response")
            .field("status", &self.status)
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}

impl Response {
    /// A reader that has seen nothing.
    pub const fn new() -> Self {
        Self {
            head: [0; HEAD_MAX],
            head_len: 0,
            status: None,
            state: State::Head,
        }
    }

    /// Take the next bytes off the connection, and return the part of them that is body.
    ///
    /// Before the head is complete this returns an empty slice; the read that completes the head
    /// returns whatever body followed it in the same read. Once the body is complete, any further
    /// byte is [`Error::BodyTooLong`]. An error is final: the reader does not recover, and neither
    /// should the caller.
    pub fn feed<'a>(&mut self, input: &'a [u8]) -> Result<&'a [u8], Error> {
        let mut rest = input;
        if self.state == State::Head {
            let mut taken = 0;
            let mut ended = false;
            for &byte in input {
                if self.head_len == HEAD_MAX {
                    return Err(Error::HeadTooLong);
                }
                self.head[self.head_len] = byte;
                self.head_len += 1;
                taken += 1;
                if self.head[..self.head_len].ends_with(b"\r\n\r\n") {
                    ended = true;
                    break;
                }
            }
            if !ended {
                return Ok(&[]);
            }
            let (status, length) = parse_head(&self.head[..self.head_len])?;
            self.status = Some(status);
            self.state = State::Body { length, seen: 0 };
            rest = &input[taken..];
        }
        let State::Body { length, seen } = self.state else {
            return Ok(&[]);
        };
        let left = length - seen;
        if rest.len() as u64 > left {
            return Err(Error::BodyTooLong);
        }
        self.state = State::Body {
            length,
            seen: seen + rest.len() as u64,
        };
        Ok(rest)
    }

    /// The status code, once the head has been read.
    pub fn status(&self) -> Option<u16> {
        self.status
    }

    /// The body length the server declared, once the head has been read.
    pub fn content_length(&self) -> Option<u64> {
        match self.state {
            State::Body { length, .. } => Some(length),
            State::Head => None,
        }
    }

    /// Whether every byte the server declared has arrived. A response is only ever used when this
    /// is true; a connection that ends before it is a truncated response, whatever its digest says.
    pub fn is_complete(&self) -> bool {
        matches!(self.state, State::Body { length, seen } if length == seen)
    }
}

/// Parse a complete head (it ends in the blank line) into the status and the body length.
fn parse_head(head: &[u8]) -> Result<(u16, u64), Error> {
    // Drop the final CRLF CRLF's second half, so every line below ends in exactly one CRLF.
    let head = &head[..head.len() - 2];
    let mut lines = head.split_inclusive(|&b| b == b'\n');
    let status_line = lines.next().ok_or(Error::BadStatusLine)?;
    let status_line = strip_crlf(status_line).ok_or(Error::BadStatusLine)?;
    let status = parse_status(status_line)?;

    let mut length = None;
    for line in lines {
        let line = strip_crlf(line).ok_or(Error::BadHeader)?;
        let colon = line
            .iter()
            .position(|&b| b == b':')
            .ok_or(Error::BadHeader)?;
        let (name, value) = (&line[..colon], trim(&line[colon + 1..]));
        if name.is_empty() || name.iter().any(|&b| b <= b' ') {
            return Err(Error::BadHeader);
        }
        if name.eq_ignore_ascii_case(b"transfer-encoding") {
            return Err(Error::TransferEncoding);
        }
        if name.eq_ignore_ascii_case(b"content-length") {
            let n = parse_decimal(value).ok_or(Error::BadContentLength)?;
            if length.is_some_and(|earlier| earlier != n) {
                return Err(Error::BadContentLength);
            }
            length = Some(n);
        }
    }
    Ok((status, length.ok_or(Error::NoContentLength)?))
}

/// `HTTP/1.x NNN reason`, where the reason may be empty.
fn parse_status(line: &[u8]) -> Result<u16, Error> {
    let bad = Error::BadStatusLine;
    let (version, rest) = line.split_at_checked(8).ok_or(bad)?;
    if !(version.starts_with(b"HTTP/1.") && version[7].is_ascii_digit()) {
        return Err(bad);
    }
    let rest = rest.strip_prefix(b" ").ok_or(bad)?;
    let (code, tail) = rest.split_at_checked(3).ok_or(bad)?;
    if !(tail.is_empty() || tail[0] == b' ') {
        return Err(bad);
    }
    let code = parse_decimal(code).ok_or(bad)?;
    if !(100..=599).contains(&code) {
        return Err(bad);
    }
    Ok(code as u16)
}

/// A line without its CRLF, or `None` if it does not end in one (a bare LF is refused).
fn strip_crlf(line: &[u8]) -> Option<&[u8]> {
    line.strip_suffix(b"\r\n")
        .filter(|l| !l.contains(&b'\r') && !l.contains(&b'\n'))
}

fn trim(mut s: &[u8]) -> &[u8] {
    while let [b' ' | b'\t', rest @ ..] = s {
        s = rest;
    }
    while let [rest @ .., b' ' | b'\t'] = s {
        s = rest;
    }
    s
}

fn parse_decimal(digits: &[u8]) -> Option<u64> {
    if digits.is_empty() {
        return None;
    }
    digits.iter().try_fold(0u64, |n, &d| {
        if !d.is_ascii_digit() {
            return None;
        }
        n.checked_mul(10)?.checked_add(u64::from(d - b'0'))
    })
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec::Vec;

    use super::*;

    /// Feed `bytes` in reads of `step`, collecting the body, the way a socket would hand it over.
    fn read_in(bytes: &[u8], step: usize) -> Result<(Response, Vec<u8>), Error> {
        let mut response = Response::new();
        let mut body = Vec::new();
        for chunk in bytes.chunks(step) {
            body.extend_from_slice(response.feed(chunk)?);
        }
        Ok((response, body))
    }

    const OK: &[u8] = b"HTTP/1.0 200 OK\r\nServer: host\r\nContent-Length: 11\r\n\r\nabcdefghijk";

    #[test]
    fn a_response_reads_the_same_however_the_reads_fall() {
        for step in 1..=OK.len() {
            let (response, body) = read_in(OK, step).unwrap();
            assert_eq!(response.status(), Some(200), "step {step}");
            assert_eq!(body, b"abcdefghijk", "step {step}");
            assert!(response.is_complete(), "step {step}");
        }
    }

    #[test]
    fn a_truncated_body_is_not_complete() {
        let (response, body) = read_in(&OK[..OK.len() - 1], 7).unwrap();
        assert_eq!(body, b"abcdefghij");
        assert!(!response.is_complete());
    }

    #[test]
    fn a_body_longer_than_it_said_is_refused() {
        let mut long = OK.to_vec();
        long.push(b'!');
        assert_eq!(read_in(&long, 5).unwrap_err(), Error::BodyTooLong);
        // And a byte after completion, in a later read, is refused the same way.
        let (mut response, _) = read_in(OK, OK.len()).unwrap();
        assert_eq!(response.feed(b"x"), Err(Error::BodyTooLong));
    }

    #[test]
    fn the_body_length_must_be_stated_and_agree_with_itself() {
        let none = b"HTTP/1.0 200 OK\r\n\r\nbody";
        assert_eq!(read_in(none, 4).unwrap_err(), Error::NoContentLength);
        let twice = b"HTTP/1.0 200 OK\r\nContent-Length: 1\r\ncontent-length: 2\r\n\r\n";
        assert_eq!(read_in(twice, 4).unwrap_err(), Error::BadContentLength);
        let same = b"HTTP/1.0 200 OK\r\nContent-Length: 1\r\nCONTENT-LENGTH:1\r\n\r\nx";
        assert_eq!(read_in(same, 4).unwrap().1, b"x");
        let huge = b"HTTP/1.0 200 OK\r\nContent-Length: 99999999999999999999\r\n\r\n";
        assert_eq!(read_in(huge, 4).unwrap_err(), Error::BadContentLength);
        let signed = b"HTTP/1.0 200 OK\r\nContent-Length: -1\r\n\r\n";
        assert_eq!(read_in(signed, 4).unwrap_err(), Error::BadContentLength);
    }

    #[test]
    fn chunked_is_refused() {
        let chunked =
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhello\r\n0\r\n\r\n";
        assert_eq!(read_in(chunked, 9).unwrap_err(), Error::TransferEncoding);
    }

    #[test]
    fn a_status_line_that_is_not_http_1_is_refused() {
        for bad in [
            &b"HTTP/2 200 OK\r\nContent-Length: 0\r\n\r\n"[..],
            b"HTTP/1.0 20 OK\r\nContent-Length: 0\r\n\r\n",
            b"HTTP/1.0 2000 OK\r\nContent-Length: 0\r\n\r\n",
            b"HTTP/1.0 099 Low\r\nContent-Length: 0\r\n\r\n",
            b"http/1.0 200 OK\r\nContent-Length: 0\r\n\r\n",
            b"\r\n\r\n",
        ] {
            assert_eq!(
                read_in(bad, 3).unwrap_err(),
                Error::BadStatusLine,
                "{bad:?}"
            );
        }
        // A status with no reason phrase is legal.
        let bare = b"HTTP/1.1 404\r\nContent-Length: 0\r\n\r\n";
        let (response, _) = read_in(bare, 3).unwrap();
        assert_eq!(response.status(), Some(404));
        assert!(response.is_complete());
    }

    #[test]
    fn a_header_that_is_not_a_header_is_refused() {
        let no_colon = b"HTTP/1.0 200 OK\r\nContent-Length 3\r\n\r\n";
        assert_eq!(read_in(no_colon, 3).unwrap_err(), Error::BadHeader);
        let bare_lf = b"HTTP/1.0 200 OK\r\nX: y\nContent-Length: 0\r\n\r\n";
        assert_eq!(read_in(bare_lf, 3).unwrap_err(), Error::BadHeader);
        let spaced = b"HTTP/1.0 200 OK\r\nContent Length: 0\r\n\r\n";
        assert_eq!(read_in(spaced, 3).unwrap_err(), Error::BadHeader);
    }

    #[test]
    fn a_head_that_never_ends_is_refused_at_the_bound() {
        let mut endless = b"HTTP/1.0 200 OK\r\n".to_vec();
        while endless.len() <= HEAD_MAX {
            endless.extend_from_slice(b"X-Padding: aaaaaaaaaaaaaaaa\r\n");
        }
        assert_eq!(read_in(&endless, 100).unwrap_err(), Error::HeadTooLong);
    }

    #[test]
    fn a_request_cannot_carry_a_second_line() {
        let mut out = [0u8; 256];
        assert!(get_request("host", "/a\r\nX: y", &mut out).is_none());
        assert!(get_request("host\r\n", "/a", &mut out).is_none());
        assert!(get_request("host", "a", &mut out).is_none());
        assert!(get_request("host", "/a b", &mut out).is_none());
        assert!(get_request("", "/a", &mut out).is_none());
        let n = get_request("h", "/p", &mut out).unwrap();
        assert_eq!(&out[..n], b"GET /p HTTP/1.0\r\nHost: h\r\n\r\n");
        // Too small a buffer is `None`, not a truncated request.
        assert!(get_request("h", "/p", &mut out[..n - 1]).is_none());
    }

    /// A bare CR inside a line is not a line ending and not a value: a peer that sends one is
    /// refused, in the status line and in a header alike (the pair a smuggler would use to make two
    /// parsers disagree about where a line stops).
    #[test]
    fn a_bare_carriage_return_inside_a_line_is_refused() {
        let in_header = b"HTTP/1.0 200 OK\r\nX: a\rb\r\nContent-Length: 0\r\n\r\n";
        assert_eq!(read_in(in_header, 64).unwrap_err(), Error::BadHeader);
        let in_status = b"HTTP/1.0 200 O\rK\r\nContent-Length: 0\r\n\r\n";
        assert_eq!(read_in(in_status, 64).unwrap_err(), Error::BadStatusLine);
    }

    /// The declared length is unknown until the head has been read, and is exactly the declared one
    /// after (not zero, not one).
    #[test]
    fn the_content_length_appears_with_the_head_and_not_before() {
        let mut response = Response::new();
        assert_eq!(response.content_length(), None);
        response.feed(b"HTTP/1.0 200 OK\r\nContent-Len").unwrap();
        assert_eq!(response.content_length(), None);
        response.feed(b"gth: 11\r\n\r\n").unwrap();
        assert_eq!(response.content_length(), Some(11));
        let (done, _) = read_in(b"HTTP/1.0 200 OK\r\nContent-Length: 0\r\n\r\n", 64).unwrap();
        assert_eq!(done.content_length(), Some(0));
    }

    /// A reader printed in a log says what it is and where it stands, and never the up to 2 KiB of
    /// head the peer sent.
    #[test]
    fn debug_names_the_status_and_leaves_the_head_out() {
        let (response, _) = read_in(OK, 64).unwrap();
        let shown = std::format!("{response:?}");
        assert!(shown.starts_with("Response"), "{shown}");
        assert!(shown.contains("200"), "{shown}");
        assert!(!shown.contains("Server"), "{shown}");
    }
}
