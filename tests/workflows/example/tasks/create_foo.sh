#! /bin/bash
set -e
file_path=./tests/tasks/foo.txt
rm $file_path | true
touch $file_path 