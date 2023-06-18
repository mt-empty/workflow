#! /bin/bash
file_path=./test/taskscases/foo.txt
rm $file_path | true
touch $file_path 