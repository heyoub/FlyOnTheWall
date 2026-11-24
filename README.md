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

## Work in progress

| Branch | What it changes | Upstream issue | Verification so far | Known limits |
| --- | --- | --- | --- | --- |
| `wip/local-source-final-end` | The source worker re-reads the final audio boundary after input closes so it can transcribe through the last sample before completing. | none filed yet | Tests, format and lint for the touched crates pass on this commit; full project CI not yet run on this commit; regression test fails on unmodified main and passes with the change | The main regression test relies on a short fixed wait and is being replaced with explicit synchronisation; the same re-read on the cloud path is established by reading, not by a test |
| `wip/receipt-write-consistency-94` | Receipt updates take the SQLite writer lock before reading the receipt map so concurrent writers apply changes to fresh entries. | #94 | Tests, format and lint for the touched crates pass on this commit; full project CI not yet run on this commit | See commit message |
| `wip/per-source-audio-loss-93` | Recording warnings report dropped and abandoned samples separately for system audio and the microphone. | #93 | Tests, format and lint for the touched crates pass on this commit; full project CI not yet run on this commit | Overlaps textually with the planned change for #271; whichever lands second needs a replay |
| `wip/transcript-dedupe-atomicity` | Transcript deduplication saves replacement segments and switches the primary transcript in the same transaction. | none filed yet | Tests, format and lint for the touched crates pass on this commit; full project CI not yet run on this commit | See commit message |
| `wip/export-path-collision` | Meetings whose short export filenames collide get distinct paths by using a full-ID filename for the collision. | none filed yet | Tests, format and lint for the touched crates pass on this commit; full project CI not yet run on this commit | See commit message |
| `wip/enrichment-retry` | Adds an explicit retry for saved meeting enrichment. | part of #96 | Tests, format and lint for the touched crates pass on this commit; full project CI not yet run on this commit; not yet independently reviewed | The title guard relies on the stored record of the generated title's exact text, so a title renamed away and later renamed back to identical text cannot be told from an untouched one |

## In progress, not yet published

- No meeting for an empty failed start (#271), being reworked after review found two gaps.
- Imported-title sweep (part of #96), being reworked after review.
- Recovery of recordings that were stopped and then interrupted mid-save, being redesigned so that nothing it adds can delete user content.
- Native Windows support, awaiting final qualification.

## Design questions for the maintainer (drafts in preparation)

- Summary provenance, including which template, revision and mode produced a summary.
- Ownership of files in the local OKF export folder, where a renamed meeting currently leaves its old file behind.

Statuses here are updated as work is verified. Nothing on a wip branch is proposed for merge.
