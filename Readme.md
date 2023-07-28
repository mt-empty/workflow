# Workflow

A simple readability oriented event driven workflow engine.
## Demo
![](./demo.gif)

## Limitations

When an event gets triggered depending on the resources available it might take a while for the event to be processed.

## Usage

```bash
cargo build --release

cd target/release
./workflow --help
./workflow start
./workflow add ./path/to/workflow.yaml
./workflow stop

```

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

The events will be polled regularly and the tasks will be added to redis queue when an event is successfully triggered.

std out and err logs will be created for task and event process, the logs will be stored in `./logs/` directory.

More examples can be found in `tests/workflows/` directory.

## Setup

`.env` file

```bash
echo "POSTGRES_PASSWORD=$(openssl rand -base64 32)" >> .env
echo DATABASE_URL=postgres://postgres:$POSTGRES_PASSWORD@localhost:5432/postgres >> .env
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

SELECT * FROM engines;
```

Redis

```bash
docker exec -it workflow-redis redis-cli
```

```redis
LRANGE tasks 0 -1
```

## TODO
- [x] Test suite
- [x] Add support for event triggers
- [x] Add cli functionality to 
  - [x] Parse workflow yaml files
  - [x] Add workflow to engine 
  - [ ] Check the status of engine, event and task processes
  - [ ] Control workflows, stop, continue, delete
- [ ] Make it distributed
  - [ ] Add support for multiple engines

---

If you are looking for a powerful workflow platform, check out [Windmill](https://github.com/windmill-labs/windmill)