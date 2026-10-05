---
status: DECIDED
raised: 2026-09-04
decided: 2026-09-04
ratified_by: calef
---

# 141. Application is grant: what a command line means in a capability system

Ratified 2026-09-03 by calef, who asked what the phrase meant and then asked for
it written down. The sentence had lived since 2026-07-30 in one paragraph of milestone 47's block,
under a heading about shell syntax, which is the last place a reader looks for the system's
mental model. *(Number provisional until the merge queue lands it.)*

## What is being decided

**Not syntax.** This records the model a reader needs in order to understand what typing a command
here does, and it is true whichever notation wins. The syntax fork it was buried inside is settled
separately and refused; see "What this does not decide" below.

The sentence: passing an argument to a program and giving that program permission are the same
act.

## Why it needs saying, which is that Unix trains the opposite

```
wc report.txt
```

In Unix the shell hands `wc` the *string* `"report.txt"`. That string is not permission. `wc`
calls `open`, and the kernel checks `wc`'s own ambient authority, which is the whole user
account; it could as easily have opened `~/.ssh/id_rsa`. The argument is a hint about what was
wanted. Authority and argument are unrelated, and `wc` could reach everything before anything was
typed.

Here the shell resolves `report.txt` to a capability and hands *that* to the child at spawn. `wc`
can read exactly that and cannot name anything else, because there is no ambient authority to fall
back on. `crates/grant_plan`'s `RunSpec` says it at the type: *"reading this is reading the whole
endowment."*

So the act of applying the program to its argument is the act of granting. Same keystrokes,
different semantics.

## What follows, and this is the part worth carrying

A line is a complete description of the authority it hands out. In
`wc (cat report.txt)` the nesting *is* the delegation tree: `cat` gets `report.txt`, `wc` gets what
`cat` produces, neither gets anything else, and it reads off the line with nothing else consulted.
No other shell can say that, because in Unix both `f(x)` and `f x` mean "f can already reach
everything, here is a string", so nesting describes data flow and is silent about permission.

**Quoting decides how much authority, not how a line is lexed.** From `RunSpec::quoted`:

> `rm "*.txt"` hands over one name where `rm *.txt` hands over a set

In bash those differ in which files are deleted. Here they differ in what `rm` receives. Quoting
is the user deciding how much to grant.

An operator is the shell granting, not two programs connecting. `cat report.txt | wc` is not
"`cat`'s output handed to `wc`": `cat` owns no output to give away. The shell mints one
`Rendezvous` and puts one end in each child's slot, per `grant_plan`'s own words, *"which
capability its output slot gets and which its input slot gets"*. Neither child can name the other,
and neither grants anything.

And a writer cannot tell what is underneath. `byte_sink_proto` states the point of its own
restriction as negative: a sink is a `Rendezvous` with `WRITE` and nothing else, so `cat` cannot
distinguish a pipe from a file from a terminal. That is what makes redirection *putting a different
capability in a slot* rather than a second feature: `>` and `|` are one operation over different
objects.

The end of a stream is a message for the same reason. `byte_sink_proto::OP_EOF` exists because
`wc` holds one capability and knows nothing about `cat`, so there is no ambient fact about "the other
end" for a kernel to notice on its behalf. Its own comment: without it, *"the producer is done"*
would be *"a fact about process supervision standing in for a fact about a stream."* The case that
proves it: `crates/system_initializer` sends `OP_EOF` **on a child's behalf when the child was never
built**, which has no Unix equivalent.

## Why this is a decision and not a note

It is the model every program in `user/` is written against, and a newcomer who arrives with Unix
habits writes one that expects ambient authority and cannot see why it fails. AGENTS.md's third
principle is that a competent stranger with only this repository reaches a correct mental model
without opening a chat window, and this is among the first things that model needs.

It is also already load-bearing in code: `grant_plan`'s endowment framing, `RunSpec::quoted`'s
authority framing, `byte_sink_proto`'s substitutability argument and `OP_EOF`'s existence are each
consequences of it, argued separately in four places and derived from a sentence written down
nowhere.

## What this does not decide

The shell's surface syntax, which was the fork this sentence was buried inside (milestone 47,
"Open fork: should the shell be function calls rather than whitespace", raised 2026-07-30). Measured
2026-09-03 and refused:

- `wc(cat(f))` is command substitution, not a pipeline: the inner call must complete and return
  a value, which buffers the output. In a shell with no allocator that is unimplementable rather
  than merely unidiomatic.
- `cat(f).wc()` implies an object with a type, and DECISIONS §50 carries bytes. Over bytes it
  is `| wc` with more punctuation. Typed pipelines are a larger fork and belong in 50 on their own
  merits rather than arriving through notation.
- Parentheses for grouping are buildable and have no customer: nothing today takes a nested
  invocation, which is the same absence that declined hard links (§110).

And the pathology that motivated the fork does not exist here. The stated diagnosis was that a
value containing a space is silently re-split after substitution. `swish` has no variable
mechanism at all; the only substitution is `$?`, which `crates/swish` documents as `'static`
because *"a substituted word has to be a slice with the line's lifetime"*, and which is one of three
values. No substituted value can contain a space. There is no `IFS`, no command substitution, and
nothing to re-split. The fork's first recommendation, kill word splitting, was already true by
construction.

## BUGS

- This describes the shell's line, and the shell is one caller. A program that spawns a child
  through `grant_plan` obeys the same model, but nothing here says what the model is for a program
  that is not `swish`, and `crates/system_initializer` is the other real caller.
- Nothing gates it. A future operator could be added that connects two children without the
  shell minting the object, and no check would notice the model had been broken.
- **The `$?` argument is a reason, not a guarantee.** It holds because there is no allocator; a
  future shell with one could grow substitution and reintroduce exactly the re-splitting pathology
  this section reports as impossible.
