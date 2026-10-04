# Login

Authentication that produces capabilities instead of mutating an identity. Milestone 49's login
half; the attribution half is [DECISIONS §109](../design/decisions/109-attribution-is-a-channel-property.md).

The contract is `crates/login_protocol`, the service is `components/src/login.rs`, and its test client is
`fixtures/src/login_test_client.rs`. All three names are provisional, minted for this milestone and
not yet ratified by calef.

## The problem this exists to solve

Unix login authenticates a presented password and then, on success, mutates a global field: the
process's uid becomes the user's. Every future authority decision for that process reads back through
that one number. It is efficient and it is why `setuid` exists, a program running with the union of
its owner's authority and its invoker's intent, which has been a security disaster for fifty years.

This system has no uid to mutate, and milestone 49's own doc names the shape the replacement should
take: authentication should **hand back a capability set** rather than change anything ambient. A
compromised login service then leaks *what it can grant*, not *the ability to become anyone*, which is
the same trade the credential service (milestone 56) already made for the secret store itself.

## The shape

```text
   a client ──login_protocol::LOGIN, identity+secret──► login ──credential_protocol::VERIFY──► credentialer
                                                       │        (unmodified;
                                                       │         milestone 56)
                                                       │
                                                       ├──fs_subtree_caretaker (fresh, per login)──► the file service
                                                       │       (a directory capability)
                                                       │
                                                       └──an untyped, freshly split (a budget)

   client ◄──SEND_CAP × 3: the caretaker's endpoint, the FS shared frame, the budget── login
```

`login` relays the presented identity and secret to the credential service's `VERIFY` unchanged: it
never touches `credential_protocol`'s protocol or `credentialer.rs`. On a match it **builds**, rather than
narrows, a capability set: a fresh `fs_subtree_caretaker` (the same construction
`crates/system_initializer` performs for a directory-granted spawn) and a fresh budget split off its
own construction untyped. Two different successful logins therefore hold two different endpoint
*objects*, which is DECISIONS §109's channel-shaped attribution rather than a badge on a shared one.

### Why a caretaker, not a narrowed copy of the file service's own endpoint

The cheaper-looking move is to hand every authenticated client a `WRITE`-narrowed copy of the same
directory capability `login` itself holds. That fails the property this milestone is for: a narrowed
copy of one shared endpoint is still one endpoint, so the file service (or anything downstream) cannot
tell two principals apart, which is exactly the shared-endpoint anti-pattern DECISIONS §109 names three
times over (the compositor, the FS server's own handle table, the fault endpoint) and refuses. Building
a fresh `fs_subtree_caretaker` per login costs a process per login and buys a real, distinguishable,
independently revocable object per principal.

### Why the same subtree for everyone, in this slice

Every login is attenuated to `filesystem_protocol::fixture::tree::SUB` with the same rights. Wiring a specific
identity to a specific subtree needs a lookup this milestone does not build (a table, a naming
convention, a directory layout), and guessing at its shape would be scope invented rather than found.
Milestone 47's per-shell root is already the isolation mechanism; what is missing is only the wiring
between an authenticated identity and which subtree it should see, named as follow-on in `components/src/login.rs`'s
BUGS.

### The delegation protocol, and why it is not `CALL`

A one-shot reply capability (what a `CALL` hands the server) carries two words and nothing else
(`abi::reply::REPLY`); it cannot deliver a capability. So this is two persistent endpoints and a fixed
message order, the shape `grant_plan::spawnproto` already uses for the same reason (a shell's spawn
request that may carry a delegated budget): a plain `SEND` for the request, a plain `SEND` for the
verdict, and on success three `SEND_CAP`s in a fixed order the client reads with three `RECEIVE_CAP`s.

## What attribution means here, and what it does not

DECISIONS §109 describes two halves: a server that establishes a channel per principal, and (separately)
a server that, serving a request later, can say which channel it arrived on. `login` is the first half.
It keeps a sequence number and sends one record per successful login, on its own audit endpoint,
naming the identity that established each channel. That makes the property checkable: a test can
confirm channel 0 belongs to `chris` and channel 1 to `corinne`, in the order they logged in.

**No server in this tree today needs the second half.** `fs_subtree_caretaker` already serves exactly
one principal by construction, so there is nothing for it to distinguish. The credential service is
anonymous by design and neither wants nor needs to know who is asking. The second half is real, named
follow-on for whenever a genuinely multi-tenant consumer exists; forcing it onto either of those two
would be inventing a requirement neither has.

## EXAMPLES

### Authenticate and use what comes back

From a client holding the request endpoint (`WRITE`) in slot 0 and the result endpoint (`READ`) in
slot 1, with the shared page mapped:

```rust
use login_protocol as proto;

let w0 = proto::place(page, b"chris", b"correct horse battery staple", proto::LOGIN).unwrap();
send(REQUEST, w0, 0, 0);
let (verdict, _, _) = receive(RESULT);
assert_eq!(verdict, proto::OK);

// In this fixed order: the directory, the file service's shared frame, the budget.
let (_, dir_ep, _) = receive_cap(RESULT);
let (_, fs_frame, _) = receive_cap(RESULT);
let (_, budget, _) = receive_cap(RESULT);

map_frame(fs_frame, FS_VA, true, budget);
let (bytes, _) = call(dir_ep, filesystem_protocol::fs::req(filesystem_protocol::fs::READDIR, filesystem_protocol::fs::ROOT, 0), 0);
```

### A refusal sends nothing further

```rust
let w0 = proto::place(page, b"chris", b"wrong", proto::LOGIN).unwrap();
send(REQUEST, w0, 0, 0);
let (verdict, _, _) = receive(RESULT);
assert_eq!(verdict, proto::DENIED);
// No RECEIVE_CAP here. The protocol promises nothing follows a refusal; a client that tried anyway
// would block forever, which `login_test_client.rs` relies on as its own check that the promise
// holds: its wrong-secret run is `LOGIN` with `credential_proto::fixture::WRONG` for a secret, the
// same code as the honest run (milestone 293).
```

## What is proven, and where

Host tests (`cargo test -p login_protocol`, milliseconds, no emulator): the request page's encoding
round-trips through the same `credential_protocol` helpers the credential service uses, and the attribution
hint is stable per identity and distinct across identities.

Guest tests (`kernel::user::login_tests`, both aarch64 and riscv64):

- A correct identity and secret produce a directory capability that answers a real `READDIR` and a
  budget that retypes a real page, not merely that the capabilities arrived.
- A wrong secret is refused, and the client proves nothing followed the refusal by never calling
  `RECEIVE_CAP` on that path.
- Two different identities each get an independently working channel, and the service's own audit
  trail names each correctly, in the order they were established.

## BUGS

See `components/src/login.rs`'s own BUGS for the itemised list (per-principal subtree scoping, no terminal,
not wired into the interactive boot, no measured-boot consultation before loading a caretaker, no
reclamation, one client at a time, and the scope of what the audit trail proves). Summarised in
[design/roadmap/49-users-and-attribution.md](../design/roadmap/49-users-and-attribution.md)'s own BUGS.
