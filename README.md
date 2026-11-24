# FlyOnTheWall contributions: status

This branch indexes work in progress on this fork for nolanmak/FlyOnTheWall, and `main` on this fork tracks upstream.

## Branch naming

- `wip/*` = implemented with tests, not yet fully verified on its current commit.
- `fix/*` or `feat/*` = verified and proposed, or ready to propose, upstream.
- `experimental/*` = exploratory, may change shape.
- `integration/*` = several branches combined to test them together, never proposed as-is.

## Merged upstream

- [Skip quoted lines in OKF descriptions](https://github.com/nolanmak/FlyOnTheWall/pull/310)
- [Search only the primary transcript version](https://github.com/nolanmak/FlyOnTheWall/pull/311)

## Verified, ready to propose

| Branch | What it changes | Upstream issue | Verification |
| --- | --- | --- | --- |
| `fix/receipt-write-consistency` | Receipt updates take the SQLite writer lock before reading the receipt map so concurrent writers apply changes to fresh entries. | #94 | Full project CI passed locally on this commit (Linux; `just ci`, with incremental compilation and debug info turned off): 2,586 passed, 0 failed, 7 ignored. Independently reviewed. Not yet proposed upstream. |
| `fix/per-source-audio-loss` | Recording warnings report dropped and abandoned samples separately for system audio and the microphone. | #93 | Full project CI passed locally on this commit (Linux; `just ci`, with incremental compilation and debug info turned off): 2,580 passed, 0 failed, 7 ignored. Independently reviewed. Not yet proposed upstream. Overlaps textually with the planned change for #271; whichever lands second needs a replay. |
| `fix/transcript-dedupe-atomicity` | Transcript deduplication saves replacement segments and switches the primary transcript in the same transaction. | none filed yet | Full project CI passed locally on this commit (Linux; `just ci`, with incremental compilation and debug info turned off): 2,576 passed, 0 failed, 7 ignored. Independently reviewed. Not yet proposed upstream. |
| `fix/export-path-collision` | Meetings whose short export filenames collide get distinct paths by using a full-ID filename for the collision. | none filed yet | Full project CI passed locally on this commit (Linux; `just ci`, with incremental compilation and debug info turned off): 2,580 passed, 0 failed, 7 ignored. Independently reviewed. Not yet proposed upstream. |

## Work in progress

| Branch | What it changes | Upstream issue | Verification so far | Known limits |
| --- | --- | --- | --- | --- |
| `wip/local-source-final-end` | The source worker re-reads the final audio boundary after input closes so it can transcribe through the last sample before completing. | none filed yet | Tests, format and lint for the touched crates pass on this commit; full project CI not yet run on this commit; regression test fails on unmodified main and passes with the change | The main regression test relies on a short fixed wait and is being replaced with explicit synchronisation; the same re-read on the cloud path is established by reading, not by a test |
| `wip/enrichment-retry` | Adds an explicit retry for saved meeting enrichment. | part of #96 | Full project CI passed locally on this commit (same settings): 2,577 passed, 0 failed, 7 ignored; independent review in progress | The title guard relies on the stored record of the generated title's exact text, so a title renamed away and later renamed back to identical text cannot be told from an untouched one |

## Integration

`integration/batch-2` combines the six branches above on current upstream main to test them together: full project CI passed locally (2,613 passed, 0 failed, 7 ignored). 28 of 30 ordered pairs apply cleanly; the receipt-write and enrichment-retry changes touch the same settings writer and were combined by hand. It is a test tree and is never proposed as-is.

## In progress, not yet published

- No meeting for an empty failed start (#271), being reworked after review found two gaps.
- Imported-title sweep (part of #96), being reworked after review.
- Recovery of recordings that were stopped and then interrupted mid-save, being redesigned so that nothing it adds can delete user content.
- Native Windows support, awaiting final qualification.

## Design questions for the maintainer (drafts in preparation)

- Summary provenance, including which template, revision and mode produced a summary.
- Ownership of files in the local OKF export folder, where a renamed meeting currently leaves its old file behind.

Statuses here are updated as work is verified. Nothing on a wip branch is proposed for merge.
