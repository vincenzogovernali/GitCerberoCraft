# GitCerberoCraft – behaviour and layout sheet

This is an application (a Git client), not a game, so there is no movement,
weapons or scoring. The "mechanics" are the layout sizes, colours, controls and
rules of each action. Values measured from the reference pictures are marked
`reference/picture-N`; everything else is **estimated** (clean room: the
original program was never opened).

## Window and layout
| Value | Number | Source |
|---|---|---|
| Default window size | 1232 × 760 | reference/picture-10..13 (window width) |
| Menu bar height | 24 px, colour #1F1F1F | reference/picture-11 (sampled) |
| Tab bar height | 36 px, colour #2A2D34, active tab #33373F | reference/picture-11 (sampled) |
| Repository toolbar height | 46 px, colour #33373F | reference/picture-12 (sampled) |
| Content background | #1C1E23 | reference/picture-11/12 (sampled) |
| Left panel width | 206 px, colour #272A31 | reference/picture-12 |
| Graph header height | 22 px, colour #272A31 | reference/picture-12 |
| Commit row height | 28 px | reference/picture-12 (rows at y 176/204/232) |
| Branch/Tag column | 140 px | reference/picture-12 |
| Graph column | 84 px minimum, +22 px per extra lane | reference/picture-3/12 |
| First lane centre | 36 px into the graph column | reference/picture-12 |
| Lane spacing | 22 px | reference/picture-3 |
| Commit node radius | 11 px (merge commits 6 px) | reference/picture-12; merge size estimated |
| Lane colours | #15A0BF, #0669F7, #8E00C2, then a rainbow | first three sampled from reference/picture-3; rest estimated |
| Row tint | lane colour at 14 % over the background | reference/picture-12 (#1B2B32 sampled) |
| Branch label | lane colour at 45 % | reference/picture-12 (#195F71 sampled) |
| Checked-out branch row (left panel) | #37553E with green check #5CB85C | reference/picture-10 (sampled) |
| Selected switch (List/Agents) | #2E3D5A | reference/picture-12 (sampled) |
| Status bar height | 24 px, colour #272A31 | reference/picture-11 |
| Dialog header / left list / selected | #3D424D / #32363F / #374665 | reference/picture-4 (sampled) |
| Clone and welcome pages background | #0D1017 | reference/picture-2/8 (sampled) |
| "Clone the repo!" button | #44088B | reference/picture-2 (sampled) |

## Controls
| Action | Key | Source |
|---|---|---|
| New tab / close / reopen | Ctrl+T / Ctrl+W / Ctrl+Shift+T | reference/picture-6 (File menu) |
| Clone / Init / Open repo | Ctrl+N / Ctrl+I / Ctrl+O | reference/picture-6 |
| Repo management | Alt+Ctrl+O | reference/picture-6 |
| External terminal / file manager | Alt+T / Alt+O | reference/picture-6 |
| Preferences | Ctrl+, | reference/picture-6 |
| Undo / Redo | Ctrl+Z / Ctrl+Y (also toolbar) | reference/picture-5 |
| Move selection in graph | Up / Down | estimated |
| Zoom | Ctrl+= / Ctrl+- / Ctrl+0, steps of 10 % | estimated |
| Checkout a branch | double-click | estimated |
| Context actions | right-click | estimated |

## Rules
- Pull button default: merge, fast-forward if possible; arrow menu offers Fetch All, fast-forward only, rebase (estimated).
- Push sets the upstream the first time; force push uses "with lease" and asks first.
- Undo covers: commit (soft reset, changes kept), checkout, branch create/delete, reset.
- The working tree is re-checked every 2.5 s; auto fetch is off by default (0 min).
- Graph shows at most 2000 commits by default (setting).
- Language folders are re-scanned every 2 s; a new `*.json` file appears automatically.
- Commit summary counter starts at 72 characters.
- AI diff limit: 60 000 characters sent per question.
