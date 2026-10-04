# Spelled-out rulings

*An appendix to [`design/naming.md`](../naming.md), which is the rule. It holds rulings that spell
out a contraction nothing outside the tree owns, after the `receive` ruling in
[vocabulary-rulings.md](vocabulary-rulings.md), which sat in a document with no room left under the
prose budget. The file and its stem are provisional, minted 2026-10-04 by the lane that performed
the first of them; naming is an architect's.*

## The tree spells `operation`, never `op`

calef, 2026-10-04 UTC, after the `receive` ruling: *"Seems like OP should be OPERATION and RECV
should be RECEIVE."*

> `operation` is spelled out in every identifier this tree owns (`OPERATION_READLINE`,
> `request_operation`, `operation(w0)`). `op` is not a ratified abbreviation for it. An identifier
> that mirrors an external API, or where `op` means something else, keeps its spelling.

The rule is the one the `receive` ruling applied: spell out a contraction nothing outside the tree
owns. The `OP_` prefix on request-code constants became `OPERATION_`. The decoder `op(w0)` became
`operation(w0)`. The names `req_op`, `fs_op`, `op_code`, `op_in`, `OPS`, `PROOF_OPS` and `ops`
followed. No constant value, opcode or wire layout changed.

Refused: keeping `OP_` because the prefix is short and every constant carries it, since length is
the cost the tree already pays for `RECEIVE_CAP`. Also refused: `opcode` or `OPCODE_`, which names
the number rather than the thing requested and would be a second ruling.

Kept, because someone else owns the spelling or the word means something else:

- `core::ops` and `std::ops` (`Deref`, `Range`), and every `no-op` or `no_op`, an English idiom.
- `unsafe_op_in_unsafe_fn`, a rustc lint.
- `op` as an instruction mnemonic in `script/stack-depth-check` and `script/fastpath-footprint`,
  and as the TFTP opcode field in `script/board-netboot`.
- DHCP's `op` field (`op = BOOTREQUEST` in `crates/virtio`).
- `is_op` and `is_diag_op` in `crates/grant_plan/src/line.rs`, and the `op` loop in a swish help
  test. Both mean a shell *operator* (`|`, `>`, `2>`).
- `vendor/redoxfs`, a permission-bit `op` that is not ours to edit.

`OPERATION_SENDTO` keeps the POSIX `SENDTO` spelling in its suffix, pending a separate ruling on
whether the tree spells that `SEND_TO`. This ruling does not touch it.

Closed records (`BUILT` roadmap blocks, `design/decisions/`, dated audit reports) keep the name they
used, so a grep for `OP_` finds them.

The live notes and roadmap documents that cited `OP_` were swept on 2026-10-04 UTC (lane `records/rename-pointer-sweep`).
