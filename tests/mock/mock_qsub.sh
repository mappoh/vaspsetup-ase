#!/bin/bash
# Mock qsub for testing. Simulates SGE job submission.
#
# Usage:
#   mock_qsub.sh <script>          → success (prints job ID)
#   mock_qsub.sh --fail <script>   → failure (prints error to stderr)

if [ "$1" = "--fail" ]; then
    echo "Unable to run job: no suitable queue" >&2
    exit 1
fi

if [ -z "$1" ]; then
    echo "usage: qsub <script>" >&2
    exit 1
fi

# Simulate successful submission
JOB_ID=$((RANDOM % 900000 + 100000))
SCRIPT_NAME=$(basename "$1")
echo "Your job $JOB_ID (\"$SCRIPT_NAME\") has been submitted"
exit 0
