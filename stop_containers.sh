#! /bin/bash

docker container stop workflow-redis workflow-postgres | true
docker container rm workflow-redis workflow-postgres | true