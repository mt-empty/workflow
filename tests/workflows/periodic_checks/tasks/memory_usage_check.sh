#!/bin/bash
set -e
memory_usage=$(free -h | awk '/^Mem:/ {print $3}')
echo "Current memory usage: $memory_usage"
