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
| `fix/local-source-final-end` | The source worker re-reads the final audio boundary after input closes so it can transcribe through the last sample before completing. | none filed yet | Full project CI passed locally on this commit (Linux; `just ci`, with incremental compilation and debug info turned off): 2,574 passed, 0 failed, 7 ignored. Independently reviewed. The regression test holds the recognizer until input is observed closed, with no fixed wait; it failed 50 of 50 runs on the unfixed worker and passed 50 of 50 with the change. The same re-read on the cloud path is established by reading, not by a test. Not yet proposed upstream. |

## Work in progress

| Branch | What it changes | Upstream issue | Verification so far | Known limits |
| --- | --- | --- | --- | --- |
| `wip/enrichment-retry` | Adds an explicit retry for saved meeting enrichment. | part of #96 | Full project CI passed locally on this commit (Linux; `just ci`, incremental compilation and debug info turned off): 2,577 passed, 0 failed, 7 ignored. Independent review returned it for rework. | Review found, with failing tests: a title the user changed and then set back to the generated text can be replaced; transcript segments are selected by transcript only, so a library imported from a damaged archive could feed another meeting's text into the summary; an empty engine response can be saved as a successful summary; a retry that overlaps the background enrichment pass can report success and then be marked failed. Being reworked; do not use as-is. |
| `wip/recovery-rows-and-lease` | Two commits: validate and write playback rows before a session folder is archived; hold the capture lease until the live save completes and, when a meeting already exists for the session, report it and keep the source. | none filed yet | Full project CI passed locally on both commits (Linux; `just ci`, incremental compilation and debug info turned off); independently reviewed; not yet proposed upstream. | The dashboard can still say Saved after such a collision (inherited). |

## Integration

`integration/batch-2` combines the six branches above on current upstream main to test them together: full project CI passed locally (2,613 passed, 0 failed, 7 ignored). 28 of 30 ordered pairs apply cleanly; the receipt-write and enrichment-retry changes touch the same settings writer and were combined by hand. It is a test tree and is never proposed as-is. It includes the enrichment-retry change as it stood before review, so it shares that branch's known limits.

## In progress, not yet published

- No meeting for an empty failed start (#271), being reworked after review found two gaps.
- Imported-title sweep (part of #96), being reworked after review.
- Recovery of recordings that were stopped and then interrupted mid-save, being redesigned so that nothing it adds can delete user content.
- Native Windows support, awaiting final qualification.

## Design questions for the maintainer (drafts in preparation)

- Summary provenance, including which template, revision and mode produced a summary.
- Ownership of files in the local OKF export folder, where a renamed meeting currently leaves its old file behind.

Statuses here are updated as work is verified. Nothing on a wip branch is proposed for merge.
