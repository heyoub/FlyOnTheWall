# FlyOnTheWall contributions: status

This branch indexes work on this fork for nolanmak/FlyOnTheWall. `main` on this fork tracks upstream.

## Branch naming

- `fix/*` or `feat/*` = verified and proposed, or ready to propose, upstream.
- `wip/*` = implemented with tests; see its row for how far verification has gone.
- `integration/*` = several branches combined to test them together, never proposed as-is.

## Merged upstream

- [Skip quoted lines in OKF descriptions](https://github.com/nolanmak/FlyOnTheWall/pull/310)
- [Search only the primary transcript version](https://github.com/nolanmak/FlyOnTheWall/pull/311)

## Proposed upstream, awaiting review

Each passed the full project CI locally on its exact commit (Linux; `just ci`, with incremental compilation and debug info turned off) and was independently reviewed before it was proposed. On the hosted checks every test job passes on Linux and macOS; the one failing job on the first five is the dependency advisory check for RUSTSEC-2026-0592, which upstream's [#312](https://github.com/nolanmak/FlyOnTheWall/pull/312) has since fixed; it clears on a re-run or on merge.

| Pull request | Branch | What it changes | Local verification | Known limits |
| --- | --- | --- | --- | --- |
| [#313](https://github.com/nolanmak/FlyOnTheWall/pull/313) | `fix/receipt-write-consistency` | Receipt updates take the SQLite writer lock before reading the receipt map, so concurrent writers apply changes to fresh entries. | 2,586 passed, 0 failed, 7 ignored. | An ordering problem between title proofs written by overlapping enrichment passes exists on upstream and remains with this change; see the review thread. It needs the title-origin decision below, not a patch here. |
| [#314](https://github.com/nolanmak/FlyOnTheWall/pull/314) | `fix/per-source-audio-loss` | Recording warnings report dropped and abandoned samples separately for system audio and the microphone. | 2,580 passed, 0 failed, 7 ignored. | none known |
| [#317](https://github.com/nolanmak/FlyOnTheWall/pull/317) | `fix/transcript-dedupe-atomicity` | Transcript deduplication saves replacement segments and switches the primary transcript in the same transaction. | 2,576 passed, 0 failed, 7 ignored. | none known |
| [#318](https://github.com/nolanmak/FlyOnTheWall/pull/318) | `fix/export-path-collision` | Meetings whose short export filenames collide get distinct paths by using a full-ID filename for the collision. | 2,580 passed, 0 failed, 7 ignored. | none known |
| [#320](https://github.com/nolanmak/FlyOnTheWall/pull/320) | `fix/local-source-final-end` | The source worker re-reads the final audio boundary after input closes, so it transcribes through the last sample before completing. | 2,574 passed, 0 failed, 7 ignored. The regression test failed 50 of 50 runs on the unfixed worker and passed 50 of 50 with the change. | The same re-read on the cloud path is established by reading, not by a test. |

| [#324](https://github.com/nolanmak/FlyOnTheWall/pull/324) | `fix/playback-rows-before-retirement` | Validates a promoted recording's archive and commits both playback rows in one transaction before the session folder is retired. | 2,576 passed, 0 failed, 7 ignored. | none known |
| [#325](https://github.com/nolanmak/FlyOnTheWall/pull/325) | `fix/capture-lease-through-live-save` | Holds the capture lease until the live save completes and refuses a live save that would overwrite an existing meeting for the same capture. | 2,574 passed, 0 failed, 7 ignored (alone on main). | The dashboard can still say Saved after such a conflict (existing behaviour). |

Related issues filed: [#315](https://github.com/nolanmak/FlyOnTheWall/issues/315), [#316](https://github.com/nolanmak/FlyOnTheWall/issues/316), [#319](https://github.com/nolanmak/FlyOnTheWall/issues/319), [#323](https://github.com/nolanmak/FlyOnTheWall/issues/323).

## Verified, not yet proposed

| Branch | What it changes | Verification | Known limits |
| --- | --- | --- | --- |
| `wip/recover-stopped-recordings` | The two fixes in #324 and #325 plus a third: at start-up, list a recording that was stopped but never saved as an interrupted meeting, keeping its audio and transcript log in place. Adds nothing that deletes or overwrites user content. | Full project CI passed locally on each of the three commits (third: 2,592 passed, 0 failed, 7 ignored); independently reviewed twice. | The third commit changes what the library shows (a session upstream leaves invisible is listed as interrupted), so it waits for the maintainer's view. Background archiving of such recordings, a finish-status report and a quit command were deliberately left out; a recording listed this way keeps its raw session folder until archived by existing means. |

## Agreed with the maintainer, in progress

The maintainer has agreed the direction for each of these and set the order: the remaining correctness fixes and developer-experience fixes first, then the interrupted-recordings change, then title and summary provenance, then Windows.

| Topic | Where it stands |
| --- | --- |
| Interrupted recordings at start-up | Agreed. The third commit on `wip/recover-stopped-recordings` is proposed after #324 and #325 land. |
| Title origin and replacement permission | Agreed: record where a title came from and whether it may be replaced, instead of inferring it from text. It ships in one schema change together with summary provenance, so library archives change format once. Specification in preparation against current main. The imported-title sweep and an explicit enrichment retry (part of #96) are rebuilt on it afterwards; `wip/enrichment-retry` is the rejected attempt, kept for reference only. |
| Summary provenance | Agreed: record which template, template revision and execution mode produced each summary. Same schema change as above. |
| Empty recordings (#271) | Agreed: keep the meeting so late notes have a place to go, and exclude confirmed-empty recordings from automatic processing. |
| Background archiving of recovered recordings | Agreed, after the interrupted-recordings change. |
| Ownership of files in the local OKF export folder | Agreed in principle (a manifest of files the exporter wrote), deferred for now. |

## Fixed upstream

Three summary defects found here were reproduced by the maintainer and fixed upstream as part of the attendee-release work: [#336](https://github.com/nolanmak/FlyOnTheWall/issues/336) (a summary could include another meeting's segments; fixed by #338 and enforced in the schema by #349), [#334](https://github.com/nolanmak/FlyOnTheWall/issues/334) (blank engine output saved as a summary; #337) and [#335](https://github.com/nolanmak/FlyOnTheWall/issues/335) (an older failed pass overwrote a newer success; #339).

## Native Windows support

Agreed with the maintainer; the pull-request order is being settled before anything is opened. Implemented on a private branch, to be re-cut from current upstream main. Release executables build; the scripted application journey (launcher readiness, handover, stale-state recovery, search, export, stub summary) passes; a matched comparison against upstream on the same machine found no regressions. Not yet published: it is waiting for hands-on capture checks on real hardware (two-leg capture, stop and retirement, forced-termination recovery).

## Integration

`integration/batch-3` combines the five proposed changes and the three recovery commits on upstream main as of `b125ed8` to test them together: full project CI passed locally on its head (2,628 passed, 0 failed, 7 ignored). Five of the six merges applied without conflict; the recovery merge had one conflict in the session pump, where both the per-source loss counts and the capture lease are kept. It contains none of the parked work above. `integration/batch-2` is the older tree that still includes the rejected enrichment retry; it is kept for history only. Integration trees are test trees and are never proposed as-is.

Statuses here are updated as work is verified.
