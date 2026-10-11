# `mkfs` on the target, and the divergence it needed

An appendix to [notes/fs-server.md](../fs-server.md). It holds milestone 57 (partitioning and
formatting a real drive, and extended attributes): why creating a filesystem on the target needed a
change to the vendored engine, the first attempt that did not work, and the program.

## The server never creates; the system now does

The parent's "Never create on-device" still holds, and it is narrower than it reads. The *server*
never creates, and that is a property of the server rather than of the system. Since 2026-08-03 a
different program in the same package does create one, on the target, over the same blk wire.

## The blocker was entropy, not `std`, and the first fix for it did not work

`FileSystem::create` and `create_reserved` carry `#[cfg(feature = "std")]`. Un-gating them is
mechanical for every call but one: `Header::new` stamps a v4 UUID through `Uuid::new_v4`, which is
`getrandom`. So the blocker is that a filesystem needs a unique identifier, and a `no_std` engine has
no randomness. That is the same wall `crates/globally_unique_identifier_partition_table` hits one
crate over. notes/globally-unique-identifier-partition-table.md refuses to invent a partition GUID
for the identical reason.

The first divergence taken for it, on the morning of 2026-08-03, made `Header::update_hash` public.
The reasoning was that every `Header` field is already `pub`, so a `no_std` caller could build a
header and then finish it. The first half is true. The second does not follow, and the reason is
worth keeping because it is a general one about vendored code:

> Making a filesystem is not making a header. `create_reserved` lays down the tree list, the
> allocation list and the root node through `Transaction::write_block` and
> `FileSystem::reset_allocator`, both **private**. `Transaction::new` is `pub(crate)`, `sync_block`
> takes an `AllocCtx` the crate does not export, and three of `FileSystem`'s fields are
> `pub(crate)`, so the struct cannot be constructed from outside at all. A caller holding a finished
> `Header` has nowhere to put it.

You cannot tell that from the API docs; it took reading the private half of two modules. The lesson
for the next vendored-engine divergence is to write the caller first and let it fail to compile,
rather than reasoning about what the exposed surface implies.

## What it is now

`Header::new_with_uuid(size, uuid)` and `FileSystem::create_reserved_with_uuid(.., uuid)` hold the
bodies. The `std` entry points keep their signatures and pass `Uuid::new_v4()`, so no upstream caller
changes. This is the shape upstream already uses one line away. `create` takes `ctime` as a parameter
because the engine has no clock, and it now takes the disk id as one because it has no randomness.
The encryption branch stays behind `std` (`Salt::new` and `Key::new` are `getrandom` too). A
`no_std` create with a password returns `ENOSYS`, rather than quietly producing an unencrypted
filesystem, which would withhold the one property the caller asked for.

The design rule this keeps: the library takes the value, the program holds the capability, and no
randomness enters vendored code. `vendor/README.md` divergence 4 records it.
`patches/redoxfs-no-std-create-uuid.patch` is the upstream submission, and it applies to the
published 0.9.1 with zero fuzz.

## The program

`mkfs` (provisional name) is `redoxfs_server`'s opposite in the one package. That one opens an image
and never creates; this one creates one and never serves. Its whole authority is a block-service
endpoint for one disk and an entropy endpoint. The kernel test withholds each in turn from the same
binary, with the same budget, stack and shared page.

Two details are worth carrying:

- It formats a partition, not a disk. `PartitionDisk` puts the partition's first block at block zero
  and reports the partition's *length* from `size()`. So the engine sizes its allocator from the
  partition, and cannot address a byte outside it by arithmetic error. It refuses a partition that is
  not 4096-aligned at both ends. A filesystem whose blocks straddle the partition's first byte reads
  back perfectly through the offset disk that wrote it, and is unreadable by anything else.
- Every timestamp on a filesystem made this way is zero, because the program holds no clock
  capability. Passing zero is the honest answer where inventing a plausible number is not. Adding a
  `clock_protocol` endpoint is one slot, and was deliberately not taken, because it would blur the
  two-capability claim.

The evidence is not the guest's. After the run, `cargo xtask test` parses the table the guest wrote
with `crates/globally_unique_identifier_partition_table`. It checks that every unique GUID is
distinct and version 4, slices the data partition out of the image, and has the host engine read back
the file the guest wrote into the filesystem it made.
