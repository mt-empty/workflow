# Workflow

A simple readability oriented event driven workflow engine.
## Features

- Event driven via  
## Limitations

When an event gets triggered depending on the resources available it might take a while for the event to be processed.

## Setup

`.env` file

```bash
echo "POSTGRES_PASSWORD=$(openssl rand -base64 32)" >> .env
echo "REDIS_URL=redis://localhost" >> .env
```

Start the containers

```bash
chmod +x ./start.sh
./start.sh
```

Start the engine
```
cargo run -- --help
cargo run start
cargo run add tests/workflows/weather_checks/workflow.yml
```

### Accessing containers

Postgres

```bash
docker exec -it workflow-postgres psql -U postgres -d postgres
```

```sql
\dt

SELECT * FROM engine_status;
```

Redis

```bash
docker exec -it workflow-redis redis-cli
```

```redis
LRANGE tasks 0 -1
```

## TODO
- [o] test suite
- [x] Add support for event triggers
- [x] Add a cli tool to 
  - [x] parse workflow yaml files
  - [x] add tasks to workflow queue 
- [o] Add a cli tool to 
  - [o] check the status of workflows
  - [ ] Control workflows, stop, continue, delete
- [o] Use Diesel for database access
- [ ] Make it distributed


## Usage

Example workflow yaml file

```yaml
name: check file exists
description: check if file exists
events:
  - name: Event1
    description: First event
    trigger: ./ping.sh
    tasks:
      - name: foo
        description: First task
        path: ./tasks/create_foo.sh
        on_failure: ./tasks/ls.sh
      - path: ./tasks/create_bar.sh
        on_failure: ./tasks/ls.sh
      - path: ./tasks/free.sh
```

```bash
cargo run add ./path/to/workflow.yaml
```
The events and tasks will be added to redis queue.

More examples can be found in `tests/workflows` directory.

---

If you are looking for a powerful workflow platform, check out [Windmill](https://github.com/windmill-labs/windmill)