#!/usr/bin/env bash
# Give every apt invocation on this runner bounded network timeouts.
#
# Usage: apt-harden.sh
#
# Our own apt calls pass these as -o flags, but tools that shell out to apt
# themselves -- `playwright install --with-deps` above all -- do not, and an
# unreachable mirror then wedges them for hours. Dropping the settings into
# apt.conf.d makes them apply to every apt-get the job runs, ours or not.
set -euo pipefail

CONFIG_PATH=/etc/apt/apt.conf.d/99-ci-network-timeouts

sudo tee "${CONFIG_PATH}" >/dev/null <<'CONF'
Acquire::Retries "3";
Acquire::http::Timeout "15";
Acquire::https::Timeout "15";
Acquire::ftp::Timeout "15";
CONF
