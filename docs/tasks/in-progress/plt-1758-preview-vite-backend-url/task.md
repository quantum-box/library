# PLT-1758: PR preview VITE_BACKEND_API_URL injection

## Status
In Progress — `feature/plt-1758-preview-vite-backend-url`

## Goal
PR CI builds must target the PR-specific library-api preview (`https://pr{N}--library-api.txcloud.app`), not the production Lambda URL from `vars.VITE_BACKEND_API_URL`.

## Changes
- `.github/workflows/ci.yml`: resolve backend URL from PR number on `pull_request`; keep repo variable for `push` to main

## Acceptance
- [ ] PR workflow logs preview host on Build
- [ ] CI green → merge

## Ref
- Linear PLT-1758
