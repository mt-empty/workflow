#! /bin/bash
set -e
file_path=./bar.txt
rm $file_path | true
touch $file_path 