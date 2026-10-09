# Login: the appendices

*Name: provisional, minted 2026-10-09 (UTC) by milestone 860's lane (comments state the constraint
as it is now), for the directory and every stem in it. Naming is calef's; `script/names
--unratified` lists each stem.*

[`notes/login.md`](../login.md) is the page to read. The files here hold the history that page
and `components/src/login.rs`'s module doc used to carry inline. It moved out on 2026-10-09 (UTC)
by milestone 860 (comments state the constraint as it is now), so the module doc could state only
what holds today. Nothing was dropped. Every resolved entry moved whole, with its dates.

- [teardown-and-channels-history.md](teardown-and-channels-history.md): the session-reclamation
  and channel sagas: the logout ticket's two refused candidates, the rendezvous leak, the
  capability-table leak, and the destroy order that was found by a test failing silently.
- [boot-wiring-history.md](boot-wiring-history.md): the boot-wiring sagas: the milestone 233 (`login` dies on every boot)
  every-real-boot death, measured boot, the entropy grant chain, and the three pieces still
  missing between the prompt and a real password.
