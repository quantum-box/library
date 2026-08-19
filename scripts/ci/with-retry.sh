#!/usr/bin/env bash
# Run a command with a hard per-attempt timeout and a few retries.
#
# Usage: with-retry.sh --attempts <n> --timeout <seconds> -- <command> [args...]
#
# CI steps that fetch over the network (apt mirrors, the Playwright CDN) can stall
# forever instead of failing: a job then sits until the six hour GitHub Actions
# default, burning runner minutes and blocking the PR. Bounding every attempt turns
# such a stall into an ordinary retryable failure.
set -euo pipefail

attempts=3
timeout_seconds=300

usage() {
	echo "usage: $0 --attempts <n> --timeout <seconds> -- <command> [args...]" >&2
}

while [[ $# -gt 0 ]]; do
	case "$1" in
	--attempts)
		attempts="$2"
		shift 2
		;;
	--timeout)
		timeout_seconds="$2"
		shift 2
		;;
	--)
		shift
		break
		;;
	*)
		usage
		exit 2
		;;
	esac
done

if [[ $# -eq 0 ]]; then
	usage
	exit 2
fi

attempt=1
while true; do
	status=0
	timeout --kill-after=10s "${timeout_seconds}" "$@" || status=$?
	if [[ "${status}" -eq 0 ]]; then
		exit 0
	fi

	if [[ "${status}" -eq 124 || "${status}" -eq 137 ]]; then
		reason="timed out after ${timeout_seconds}s"
	else
		reason="failed with exit ${status}"
	fi

	if [[ "${attempt}" -ge "${attempts}" ]]; then
		echo "::error::'$*' ${reason} (attempt ${attempt}/${attempts}); giving up"
		exit "${status}"
	fi

	backoff=$((attempt * 10))
	echo "::warning::'$*' ${reason} (attempt ${attempt}/${attempts}); retrying in ${backoff}s"
	sleep "${backoff}"
	attempt=$((attempt + 1))
done
