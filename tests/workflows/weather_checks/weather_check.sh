#!/bin/bash
set -e
weather_data=$(curl -s "http://wttr.in/Melbourne?format=j1")
chance_of_rain=$(echo "$weather_data"  | jq -r ".weather[0].hourly[0].chanceofsunshine")
if [[ $chance_of_rain -gt 90 ]]; then
    exit 0
else
    exit 1
fi
