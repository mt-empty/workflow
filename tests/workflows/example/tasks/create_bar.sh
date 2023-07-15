#! /bin/bash
set -e
file_path=./tests/tasks/bar.txt
rm $file_path | true
touch $file_path 