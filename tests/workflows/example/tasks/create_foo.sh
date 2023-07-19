#! /bin/bash
set -e
file_path=./foo.txt
rm $file_path | true
touch $file_path 