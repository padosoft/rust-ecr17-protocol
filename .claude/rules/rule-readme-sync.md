# README sync

The repository home page on GitHub renders the root `README.md`. The crates.io front
page renders `crates/ecr17-protocol/README.md`. Both must show the same polished page.

Because the README uses **absolute URLs** for every badge, image, and link (it ships to
crates.io), the two files are byte-identical — the mirror is a plain copy, no path
rewriting.

> **Status:** the automated sync **script** and the CI **readme-check** described below
> are introduced in **MACRO 8** (the wow-README task, see `docs/PLAN.md` T8.1) — they do
> not exist yet during MACROs 0–7. Until then, keep the two copies in sync **by hand**
> (they are byte-identical). This rule documents the target workflow.

When you change the README:

- Edit `crates/ecr17-protocol/README.md` (the canonical, crates.io-published copy), then
  regenerate the root copy with the sync script (added in MACRO 8, e.g. `scripts/sync-readme`).
  Never hand-edit the root `README.md` once the script exists.
- Commit both files together.
- CI (`frontend-checks`, from MACRO 8) runs the readme check and fails if the root
  `README.md` is out of sync, so a stale mirror never lands.

Cross-port links: the README must link the sibling ports
(`padosoft/react-native-ecr17-protocol`, `padosoft/laravel-ecr17`), and those repos must
link this Tauri/Rust package (see `docs/PLAN.md` T8.3 — align the sibling repos to their
latest `main` before editing them).
