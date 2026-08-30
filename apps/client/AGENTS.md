# Repository Guidelines

## Project Structure & Module Organization

**library v2** — the React 19 + Vite client that ships as web, desktop, and mobile. Its backend is `apps/api` (library-api), not anything in this directory.

- `src/`: frontend app code. Routes live in `src/router.tsx`; shared state is under `src/contexts/`; Yjs helpers are in `src/lib/yjs/`.
- `src/app/kitConfig.ts`: app settings for branding, navigation, record defaults, storage, and sync.
- `src/lib/recordsApi.ts`: the Library API client. `src/lib/photonEngine/`: the local-first store.
- `src/components/chat/` and `src/components/files/`: chat and file preview features.
- `src-tauri/`: Tauri shell for desktop and mobile.
- `workers/sync/`: Cloudflare Worker — Engine proxy and the Live Durable Object relay.
- `tests/e2e/`: Playwright end-to-end tests, backed by the Node fixture in `tests/e2e/library-api-fixture.mjs`.
- `docs/`: this app's own operational docs. See [`docs/README.md`](./docs/README.md).

### The local-first store

`src/lib/photonEngine/` wires up [`@quantum-box/photon`](https://github.com/quantum-box/photon) — the engine is the package's, not this repo's. Do not reimplement its parts here; the design lives in the photon repo, linked from [`docs/README.md`](./docs/README.md).

## Build, Test, and Development Commands

- `npm install`: install frontend and test dependencies.
- `npm run dev -- --host 127.0.0.1`: start the Vite frontend on port `5173`.
- `npm run build`: type-check and build the frontend.
- `npm run type-check`: run TypeScript checks without emitting files.
- `npm test`: run Vitest unit tests.
- `npm run test:e2e`: run Playwright E2E tests (starts the Library API fixture itself).
- `npm run tauri:dev`: run the desktop app during development.

## Coding Style & Naming Conventions

Use TypeScript, React function components, and hooks. Keep components in PascalCase files such as `CreateRecordModal.tsx`; hooks use `useSomething.ts`.

ESLint is configured in `eslint.config.js`. Use two-space indentation, single quotes, and existing Tailwind utility patterns. Add `data-testid` only for stable user-facing flows that need E2E coverage.

Keep reusable shell code independent of project names. Put labels, defaults, persistence keys, and WebSocket paths in `src/app/kitConfig.ts`.

Every user-facing string goes through the message catalogs in `src/i18n/`, never
into a component as a literal: add the key to `messages/en.ts`, translate it in
the other catalogs, and read it with `useI18n()` (or the module-level `t()`
outside React). Constant tables carry a `labelKey`, not a label. Dates, numbers,
byte sizes, and sorting use the helpers in `src/i18n/format.ts` so they follow
the reader's locale. See [`docs/i18n.md`](docs/i18n.md).

## Testing Guidelines

Use Vitest for focused unit tests in `src/**/*.{test,spec}.{ts,tsx}`. Use Playwright for browser flows in `tests/e2e/*.spec.ts`.

E2E tests should cover critical shell behavior: route navigation, record creation/editing, Kanban movement, chat streaming, file attachment, and persistence/sync behavior. Prefer role, label, and `data-testid` locators.

## Commit & Pull Request Guidelines

History uses concise conventional commits with Linear-style IDs, for example:

- `feat: PLT-348 implement record CRUD operations with Y.Doc integration`
- `feat: PLT-346 add dark mode and theme system with light/dark/system toggle`

For pull requests, include a summary, linked ticket or PLT ID, verification commands, and screenshots for UI changes. Call out migrations, Tauri changes, and setup changes.

## Security & Configuration Tips

Do not commit generated data such as `dist/`, `target/`, local SQLite files, Playwright reports, or secrets. Keep ports and API endpoints explicit so frontend, backend, mobile, and desktop clients share runtime assumptions.
