#!/bin/bash
set -e
weather_data=$(curl -s "http://wttr.in/Melbourne?format=j1")
echo "$weather_data" > ./tests/tasks/weather_data.txt
