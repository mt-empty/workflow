#!/bin/bash
set -e
current_minute=$(date +%M)
if [[ $((current_minute % 5)) -eq 0 ]]; then
    exit 0
else
    exit 1
fi
