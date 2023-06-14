#! /bin/bash

docker run --name workflow-redis -d redis redis-server --save 60 1 --loglevel warning
docker run --name workflow-postgress -e POSTGRES_PASSWORD=$POSTGRES_PASSWORD -d postgres