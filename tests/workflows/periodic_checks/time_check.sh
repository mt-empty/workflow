#!/bin/bash
set -e
current_seconds=$(date +%S)
if (( current_seconds % 300 == 0 )); then
    exit 0
else
    exit 1
fi
