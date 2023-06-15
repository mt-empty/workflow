#! /bin/bash

docker run --name workflow-redis -d redis redis-server --save 60 1 --loglevel warning
POSTGRES_PASSWORD=$(grep -oP '(?<=POSTGRES_PASSWORD=).*' .env)
docker run --name workflow-postgress -e POSTGRES_PASSWORD="$POSTGRES_PASSWORD" -p 5432:5432 -d postgres