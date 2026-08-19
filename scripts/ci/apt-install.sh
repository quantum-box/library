#!/usr/bin/env bash
# Install apt packages on a GitHub Actions runner without risking an unbounded hang.
#
# Usage: apt-install.sh <package> [package...]
#
# `apt-get update` waits forever on an unreachable mirror by default, which is how
# the CI runs for #199 sat in "Install protoc" for hours. Every fetch here is bounded
# twice over: apt's own Acquire timeouts, plus an outer `timeout` that kills a stuck
# apt-get outright. A failing update is only a warning, so the install can still
# proceed from the package lists baked into the runner image; a failing install is
# fatal after its retries.
set -euo pipefail

if [[ $# -eq 0 ]]; then
	echo "usage: $0 <package> [package...]" >&2
	exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WITH_RETRY="${SCRIPT_DIR}/with-retry.sh"

export DEBIAN_FRONTEND=noninteractive

APT_OPTS=(
	-o Acquire::Retries=3
	-o Acquire::http::Timeout=15
	-o Acquire::https::Timeout=15
	-o Acquire::ftp::Timeout=15
)

# `timeout` runs under sudo so that it signals apt-get itself; the outer timeout in
# with-retry.sh is a backstop in case sudo is the process that wedges.
if ! "${WITH_RETRY}" --attempts 2 --timeout 120 -- \
	sudo timeout --kill-after=10s 90 apt-get update "${APT_OPTS[@]}"; then
	echo "::warning::apt-get update did not succeed; installing from the runner image's cached package lists"
fi

"${WITH_RETRY}" --attempts 3 --timeout 180 -- \
	sudo timeout --kill-after=10s 150 apt-get install -y "${APT_OPTS[@]}" "$@"
