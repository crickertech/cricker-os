---
status: NOT-STARTED
promoted_from: a-client-cannot-stall-the-file-server-with-a-sparse-file
raised: 2026-10-04
milestone_dependencies: none
decision_dependencies: none
machine_requirements: none
specific_machine: none
needs_person: no
---
# 772. A client cannot stall the file server with a sparse file

Raised by the fuzz lane (`lane/fuzz-service-handlers`, proposal #1592 part a) on 2026-10-04 (UTC),
when the `redoxfs_server_session` fuzz target timed out after about 380,000 sessions. Title and
slug are drafts.

## The defect

RedoxFS frees a file's records one pointer at a time, from the old end down to the new one
(`truncate_node_inner` in `vendor/redoxfs/src/transaction.rs`), whether or not the records exist.
Growing a file writes no zero records, so it is cheap. So a client holding a writable handle can
send `TRUNCATE` to near `redoxfs_server::MAX_FILE_END` (about 128 TiB) and then `TRUNCATE` to 0,
and the second request walks on the order of 2^34 record pointers. The file server answers one
request at a time, so every other client waits. A confined client can do this inside its own grant.
The same walk runs when such a file's last name and handle go away. The limitation is recorded in
the `BUGS` section of `Server::truncate`.

## Options

1. **Skip a null subtree** in `truncate_node_inner`: if a level's pointer is null, step past every
   record under it. Fixes the cost at the root and is upstreamable, but it is a sixth pin
   divergence (`vendor/README.md`), re-applied on every bump; the first five were calef's calls.
2. Refuse a sparse size past the image in `Server::truncate` and `Server::write`: no file may
   end past the filesystem's size. Bounds the walk at the image's records (about 131,000 for a
   1 GiB image) inside our own code. It changes what the server accepts: POSIX allows a sparse file
   larger than its disk, and nothing in this tree uses one today (not measured beyond a grep).
3. Bound the work per request and make a long shrink resumable. Most general, most machinery.

Recommendation, not measured: option 1, with option 2 as the interim if the divergence is refused.
#1606 (`lane/redoxfs-level4`) takes the other vendored fix this lane found, a level-4 constant, as
a divergence of the same pin.

## Proof condition: BUILT when

1. `TRUNCATE` from `MAX_FILE_END` to 0 completes in under a millisecond on the host, by a host test
   in `redoxfs_server`.
2. The `redoxfs_server_session` target no longer clamps sizes into its own small range, and runs
   ten minutes with no timeout.

## Index row

A client holding a writable file can stall the file server: shrinking a huge sparse file walks every
record pointer. Skip null subtrees in the engine, or refuse sizes past the image.
