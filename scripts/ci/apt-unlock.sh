#!/usr/bin/env bash
# Clear the apt state a killed install leaves behind.
#
# Usage: apt-unlock.sh
#
# apt-get runs as root under sudo, so an unprivileged `timeout` that gives up on a
# stalled install cannot signal it: the orphan keeps /var/lib/apt/lists/lock and
# every retry then dies instantly with "Could not get lock" (see run 32234771446's
# node job). Terminate the leftovers as root and drop the stale locks so the next
# attempt starts clean. Best effort throughout -- nothing here should fail a job.
set -uo pipefail

echo "Clearing leftover apt processes and locks before retrying"

sudo pkill -TERM -x 'apt-get|apt|dpkg' >/dev/null 2>&1 || true
sleep 5
sudo pkill -KILL -x 'apt-get|apt|dpkg' >/dev/null 2>&1 || true

sudo rm -f \
	/var/lib/apt/lists/lock \
	/var/cache/apt/archives/lock \
	/var/lib/dpkg/lock \
	/var/lib/dpkg/lock-frontend

sudo timeout --kill-after=10s 60 dpkg --configure -a >/dev/null 2>&1 || true

exit 0
