#!/bin/bash
set -e
file_path=/etc/passwd
logged_in_user=$(awk -F: '{ if ($3 >= 1000 && $1 != "nobody") print $1 }' "$file_path")
echo "Current logged-in user: $logged_in_user"
