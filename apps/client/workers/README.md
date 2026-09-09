# Library Rust Workers (PLT-4464)

Both Workers are implemented in Rust and compiled to Wasm with
`worker` / `worker-build` 0.8.5. The generated JavaScript contains the
Cloudflare/wasm-bindgen adapters; request handling and Durable Object state
machines live in Rust. This Cargo workspace is separate from the API and
Tauri workspaces so their native dependencies do not enter the Wasm build.

- `sync/src`: Engine proxy, generic Yjs relay, Live authorization/tickets,
  room generations and checkpoint journal. `yrs` reads and writes Yjs v1.
- `public-docs/src`: anonymous API reads, public HTML/SEO, robots and sitemap.
  `lol_html` rewrites the bounded app shell in Rust.
- `common/src`: bounded streams, HTTP helpers, URL encoding and cryptography.
- `tests/runtime.mjs`: test-only storage fixtures. Never included in deployments.

## Build and verify

From `apps/client`, with Rust stable and Node 22 installed:

```sh
rustup target add wasm32-unknown-unknown
cargo install worker-build --version 0.8.5 --locked
npm ci
npm run build:worker
npm run build:public-docs
npm run test:worker
npm run type-check:worker
cargo fmt --manifest-path workers/Cargo.toml --all --check
cargo clippy --manifest-path workers/Cargo.toml --target wasm32-unknown-unknown --workspace --locked -- -D warnings
npm run test:e2e:live
```

`worker:dev` and `worker:deploy` invoke the Rust build through Wrangler's
custom build command. Production, preview and Live E2E configs all load
`workers/sync/build/index.js` and its Wasm module. Existing class names,
binding names, migration tags and origins remain configured in those files.
No Durable Object migration or namespace reset is required.

`build:cloud` resolves the same Vite environment for the browser and the
public-docs Rust build, including the preview overrides. It emits the Pages
advanced-mode directory `dist/_worker.js/` (`index.js`, `index_bg.wasm`) and
`dist/_routes.json`. Upload the entire directory through a Pages Functions
capable deployment path. Cloud App install commands install the Rust build
toolchain using `scripts/install-worker-toolchain.sh`; CI provisions it too.

The old private package's TypeScript `./worker` export is removed. Worker
consumers now build the Rust crate and deploy the generated module directory;
`build:package` continues to build the browser-facing package without Rust.
Tauri uses `build:native` to bundle only the app frontend; native release jobs
do not build or bundle the server-side Workers.

## Persistence and concurrency

The Rust implementation preserves the TypeScript JSON field names, ArrayBuffer
values, snapshot/update keys, pending-checkpoint chunks, one-time tickets,
session references, WebSocket attachments, result index and generation pointer.
It reads both legacy single-value snapshots and chunked snapshots. Binary
updates are validated before storage; snapshot compaction and log removal are
one transaction. A corrupt/missing snapshot fails closed instead of replacing
a persisted document with an empty snapshot.

Each document has a storage mutex. Checkpoints have a separate queue; neither
authorization nor checkpoint HTTP calls hold the document mutex. Editing can
continue during a save. Updates and their working-version metadata commit
atomically. A pending initialization is replayed idempotently after restart.
The checkpoint journal survives failures, preserves operation identity, pins
results needed for ACK-loss recovery, and rejects stale or conflicting saves.

Snapshot replay works in batches of 64 rows, folds at most 512 rows per pass,
and continues on the next request or alarm. Snapshots use 96 KiB chunks;
checkpoint bodies use 64 KiB chunks. Replay results are capped at 128 entries
and 15 minutes, except a result pinned by a pending operation. Ticket and
session cleanup scans at most 128 keys per prefix per alarm.

The integration suites execute the compiled Wasm in Miniflare/workerd, with
JavaScript Yjs clients and a fixture API. They exercise multiple sockets,
process restart with the same persistent directory, legacy storage, concurrent
network waits, retries, generation rotation, UTF-16 text and multi-value
snapshots. They do not constitute production deployment or real-data proof.
