#! /bin/bash

docker container stop workflow-redis workflow-postgres
docker container rm workflow-redis workflow-postgres