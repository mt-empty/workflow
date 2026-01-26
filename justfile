build:
    #!/usr/bin/env sh
    cargo build -j 6

start-containers:
    #!/usr/bin/env sh

    POSTGRES_PASSWORD=$(grep -oP '(?<=POSTGRES_PASSWORD=).*' .env)
    docker run --name workflow-redis -d redis redis-server --save 60 1 --loglevel warning
    docker run --name workflow-postgres -e POSTGRES_PASSWORD="$POSTGRES_PASSWORD" -p 5432:5432 -d postgres

stop-containers:
    #!/usr/bin/env sh
    docker container stop workflow-redis workflow-postgres | true
    docker container rm workflow-redis workflow-postgres | true

