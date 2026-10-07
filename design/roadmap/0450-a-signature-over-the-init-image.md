---
status: REFUSED
raised: 2026-09-20
refused_by: 104, 448
---
# 450. A signature over the init image, in place of a compiled-in digest

Refused by milestone 104 (design/roadmap/104-init-measures-what-init-loads.md),
and recorded there on 2026-09-03. Backfilled here on 2026-09-20 by
milestone 448 (design/roadmap/448-a-refusal-gets-a-number.md), which gave a refusal that names work a number, a
status and a condition that would change it. *(Number provisional until the merge queue lands it.)*

**The date is when the refusal was written down, not necessarily when it was made.** Most of this
tree's `- **Refused.**` bullets were written in one sweep on 2026-09-03, so the decision is usually
older than the bullet and its reasoning sits in the block's own prose above it.

## The refusal, in its own words

From '104. The measurement continues past init', under `## Follow-on`:

> The signature variant stays deferred, reaffirmed by calef on 2026-08-03. A signature over init
> in place of a compiled-in digest puts keys, a certificate chain and signature-verification code
> inside the trusted computing base, which is exactly what the digest approach avoids. This
> milestone extends the measurement's reach, not its mechanism, and DECISIONS §26's natural
> sequence still holds: signatures in addition to measurement, never instead of it.
>
> -- design/roadmap/104-init-measures-what-init-loads.md

## Why it is here rather than only there

The measured-boot chain in this tree compares what it loaded against a digest compiled into the
verifier. A signature would instead check that somebody with a key vouched for the image, which is a
different property: it lets the thing being verified change without the verifier changing, and it
pays for that with keys, a certificate chain and verification code inside the trusted computing
base.

A note on the citation inside the quote, which is not this block's to fix: it names
`DECISIONS §26` for the sequence, and §26 ("the signature variant we did not build") does carry
that section, under a title about the fault endpoint. It resolves, and a reader going by title will
not find it, so it is flagged here rather than corrected.

## Revisit

- **Unstated.** The refusal names no trigger. It names a reason (a signature enlarges the trusted
  computing base with exactly what the digest approach avoids), an authority (calef reaffirmed it on
  2026-08-03) and a sequence (signatures in addition to measurement, never instead of it), and none of
  those is a thing that could become true. That is a gap somebody could close in one sentence, and the
  sentence is probably about who needs to verify an image without recompiling the verifier.

## Index row

A digest compiled into the verifier and a signature checked by it buy different things, and this
tree deliberately took the smaller trusted computing base. The refusal has been reaffirmed once and
states no condition that would reopen it, which is recorded here rather than papered over.
