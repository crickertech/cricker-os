# `jig`, the package manager

The appendix to [notes/packages.md](../packages.md) for milestone 809 (the package client becomes a
program), built 2026-10-10 (UTC). `jig` installs, removes and rolls back packages and lists what the
image vouches for. The progenitor stays the installer; `jig` asks it.

## EXAMPLES

From `script/swish-check`'s first boot, on all three architectures:

```
$ jig list
  packages this image vouches for (its catalog; no index yet):
  greeting 0.1.0
  ...
$ cd downloads
$ jig install ./noteless.nifepkg
  installed; generation 1 is live
$ cd /
$ jig install nosuch
  refused: this image's catalog names no such package, so nothing was fetched
$ jig install greeting@0.1.0
  fetched and installed; generation 2 is live
$ installed/jig rollback
  this jig holds no installer endpoint; only the owner's console grants one
```

The verbs are apt's where apt has one; [the record](../../design/naming/command-line-rulings.md#jig-takes-apts-verbs)
has each spelling. A word with a `/` is a file, so a package file in this directory is `./<file>`.
It is installed from the directory it is in, because a word is designated at the current directory,
and the root cannot be narrowed to one name.

## What it holds

`caps jig install greeting@0.1.0` prints it. A `std` program, so the slots are the `std` layout's:

| slot | what | from |
|---|---|---|
| 2, 3 | the network stack, and its own region for each socket's page | `Manifest::network` |
| 8 | its words | `ArgSpec::Words` |
| 15 | the installer endpoint, `WRITE` | `Manifest::installer`, §270 (a package manager holds an installer endpoint, not the spawn endpoint) |
| 16 | the endpoint its answers come back on, `READ` | the same |
| 17, and a page | a copy of the image's catalog | `Manifest::catalog` |
| 4 | the file a word names, read-only | the line, for `install <path>` |

**Each line holds what its verb needs and no more** (option V2, milestone 281 (`watch` holds
exactly what `ps` holds)'s test): `list` the catalog alone, `remove` and `rollback` the installer
alone, `install` all three. `grant_plan::narrow_by_verb` decides it from `grant_plan::Prog::verbs`.
The shell narrows for `caps` and the progenitor for the endowment, each reading the line's second
word. The progenitor reads it off the argv page and refuses the spawn if the child's copy says
otherwise.

## How an install travels

`jig` reads the package into one run of its own pages: from the file, or from the package source
over plain HTTP (`http_response`). A fetched package must then be the one asked for, and the catalog
must vouch for its digest (`package_archive::installable_as`), or nothing is sent.

Then `spawnproto`'s activation request goes out on the installer endpoint, unchanged from the
builtin's: three words, one `READ` frame per page, one answer. The frames are slices of the run
(`abi::page_frame::SLICE`), so the buffer and the frames are the same memory. The progenitor copies
each page before it hashes, checks its copy against its own catalog, places the program and writes
a generation. The row records `jig` in its manager column.

The run needs room for a package beside the heap, so a program declaring the installer is built in a
region one package larger, from the progenitor's image pool (`grant_plan::named_std_region_pages`).

## What moved

- Out of the shell: `Command::Package`, `PackageVerb`, `package_verb`, `PACKAGE_USAGE` and the
  builtin. `vouch` stays a builtin, on the spawn endpoint, which the installer endpoint is not.
- Out of the progenitor: the fetch, `Activation::Fetch` and its three statuses (holes in the wire
  now), and `http_response`, with `init`'s package exception for it.
- Into the activation set: a manager column (`activation_set::Entry::manager`). A row from before it
  reads as `-`.
- Into the progenitor's stack: two pages, to fourteen (`kernel::user::INIT_STACK_PAGES` has the
  measurements).

## BUGS

- `list` reads the image's catalog, not an index: milestone 809's item 6. The copy is one page, so
  `xtask` refuses a catalog of a page or more.
- Its own refusals name no generation, and a `remove <program>@<version>` it cannot pick names no
  candidates. It holds no view of `activation/`.
- The package source is the runners' fixture peer, plain HTTP; outside QEMU there is none.
- Nothing gates its dependencies to the public contracts the block names; `jig/Cargo.toml` lists
  them with that argument.
- A copy run by path carries a note that asks for less than the image's `jig` holds, so it lists
  nothing and installs nothing. That is the point of the `installed/jig` line, and also all a copy
  can do.
