# README sync

The repository home page on GitHub renders the root `README.md`. The crates.io front
page renders `crates/ecr17-protocol/README.md`. Both must show the same polished page.

Because the README uses **absolute URLs** for every badge, image, and link (it ships to
crates.io), the two files are byte-identical — the mirror is a plain copy, no path
rewriting.

When you change the README:

- Edit `crates/ecr17-protocol/README.md` (the canonical, crates.io-published copy), then
  regenerate the root copy with the sync script (`bun run sync:readme` /
  `scripts/sync-readme`). Never hand-edit the root `README.md`.
- Commit both files together.
- CI (`frontend-checks`) runs the readme check and fails if the root `README.md` is out
  of sync, so a stale mirror never lands.

Cross-port links: the README must link the sibling ports
(`padosoft/react-native-ecr17-protocol`, `padosoft/laravel-ecr17`), and those repos must
link this Tauri/Rust package (see `docs/PLAN.md` T8.3 — align the sibling repos to their
latest `main` before editing them).
