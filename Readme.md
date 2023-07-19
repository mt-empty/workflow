# Workflow

Readability oriented workflow engine.
## Features

- Event driven via  
## Limitations

When an event gets triggered depending on the resource it might take a while for the event to be processed.

## Setup

`.env` file

```bash
echo "POSTGRES_PASSWORD=$(openssl rand -base64 32)" >> .env
echo "REDIS_URL=redis://localhost" >> .env
```

Clippy 

```bash
cargo clippy --fix -- -W clippy::pedantic -W clippy::nursery -W clippy::unwrap_used -W clippy::todo -W clippy::dbg_macro -W clippy::print_stdout -W clippy::unimplemented

```


Postgres

```bash
docker exec -it workflow-postgres psql -U postgres -d postgres
```

```sql
\dt

SELECT * FROM engine_status;
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
- [ ] Use Diesel for database access
- [ ] Make it distributed


## Usage

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
The events and tasks will be added to the queue.

More examples can be found in the `tests/workflows` directory.