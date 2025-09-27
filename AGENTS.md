# Repository Guidelines

## Project Structure & Module Organization
Rust source lives in `src/`, with modules split by concern (`scene.rs` for gameplay setup, `offscreen.rs` for Web Worker canvas plumbing, `p2p.rs` gated behind the `p2p` feature). Browser assets and the Vite entrypoint are under `web/` (`main.js`, service worker, and generated `pkg/` bindings). Shipping artifacts land in `dist/` after a release build, while `docs/` collects architecture notes, deployment steps, and P2P design references. Place any new developer notes beside the existing topic files.

## Build, Test, and Development Commands
- `npm run dev`: builds the debug WebAssembly module and starts Vite with hot reload.
- `npm run build`: produces an optimized wasm build and bundles the PWA into `dist/`.
- `npm run wasm:debug` / `npm run wasm:release`: regenerate `web/pkg/` without starting Vite, useful for profiling or CI steps.
- `npm run preview`: serve the contents of `dist/` locally for final smoke tests.
- `cargo fmt && cargo clippy --all-targets --all-features`: enforce Rust formatting and linting before opening a PR.

## Coding Style & Naming Conventions
Use stable Rust (1.85+) with `cargo fmt`’s default 4-space indentation. Keep modules small and cohesive, exposing types via `pub` only when needed. Prefer descriptive CamelCase for Rust types (`CanvasSize`) and snake_case for functions. In `web/`, follow modern ES modules with `const`/`let`, two-space indentation, and kebab-case filenames. Avoid introducing globals; wire configuration through exported helpers instead.

## Testing Guidelines
Add Rust integration tests under `tests/` and run them with `cargo test`. Browser-specific scenarios can live in `web/tests/` using Playwright or Vitest; document new tooling inside `docs/`. When touching wasm bindings, pair changes with a minimal `wasm-bindgen-test` to prevent regressions. Aim to keep manual QA steps in sync with `docs/deployment.md` and update it when workflows change.

## Commit & Pull Request Guidelines
The repository has no git history yet, so set the tone with Conventional Commit prefixes (`feat:`, `fix:`, `chore:`) and imperative subject lines under 72 characters. PRs should describe the user-facing impact, list testing performed (`cargo test`, `npm run build`), and link any relevant doc updates or follow-up issues. Include screenshots or GIFs for UI changes in `web/` to help reviewers triage quickly.

## Feature Flags & Configuration Notes
Enable the optional P2P stack by building with `--features p2p` and ensure matchbox signaling endpoints are configurable through environment variables or launch parameters. Toggle the OffscreenCanvas worker by appending `?worker=1` during local testing to mirror production defaults.
