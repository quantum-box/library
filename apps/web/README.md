# Library Web

Vite frontend for Library.

## Development

Run commands from the repository root with Yarn 4:

```bash
corepack yarn dev
corepack yarn build
corepack yarn test
corepack yarn test:e2e
```

The local dev server runs on http://localhost:5010.

Testing docs:

- [Test strategy](../../docs/specs/testing/test-strategy.md)
- [Auth registration coverage](../../docs/specs/testing/auth-registration-tests.md)

GitHub Actions runs lint, Unit tests, type check, build, and Playwright E2E from `.github/workflows/ci.yml`.
