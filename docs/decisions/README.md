# Architecture Decision Records

This directory records approved technical, gameplay, and workflow decisions for 2nd Hand Combat so contributors and agents can rely on the repository rather than chat history.

## Index

| ADR | Title | Status | Target phase |
| --- | --- | --- | --- |
| [0001](0001-performance-targets.md) | Performance targets | Accepted | 0C, 1, 2, 6 |
| [0002](0002-base-materials-and-custom-loadouts.md) | Game-wide base materials and custom loadouts | Accepted | 3 |
| [0003](0003-fire-on-the-move-doctrine.md) | Fire on the move as a doctrine option | Accepted | 3, 4 |
| [0004](0004-battle-time-limit-and-end-battle.md) | Battle time limit, unlimited mode, and End battle | Accepted | 3, 4 |
| [0005](0005-svelte-5-runes-migration.md) | Full migration to Svelte 5 runes | Accepted | 7 |
| [0006](0006-automated-verification-and-ci.md) | Automated verification, generated contracts, and CI | Accepted | 0A, 0B, 0C |
| [0007](0007-agent-workflow-and-human-checkpoints.md) | Agent-maintained workflow and human-approval checkpoints | Accepted | 0D |
| [0008](0008-scaling-spatial-grid-and-storage-layout.md) | Spatial grid, compact binary snapshots, and storage layout evaluation | Accepted | 6 |

## Adding or updating records

- Number new ADRs sequentially (`0008-short-title.md`) and add them to the table above.
- Keep each record short: Context, Decision, Consequences, and any Open Questions.
- Do not silently rewrite an accepted ADR to reverse it. Add a new ADR that supersedes it and cross-link both files.
- Decisions that add dependencies, bump scenario `schemaVersion`, change doctrine meaning, alter scope documents, or change governance checkpoints require human approval (see [ADR 0007](0007-agent-workflow-and-human-checkpoints.md)).
