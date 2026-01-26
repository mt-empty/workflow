# Workflow Engine - AI Coding Assistant Instructions

## Project Overview
This is a **polling-based workflow engine** written in Rust that executes shell scripts based on conditional triggers. It uses a distributed architecture with event polling and task queue processing.

### Core Architecture (Event-Driven + Queue-Based)
```
Events (polling) → Trigger Scripts → Tasks (Redis queue) → Task Execution
     ↓                                      ↓                    ↓
PostgreSQL (state)              Redis Queue (tasks)      Rayon ThreadPool
```

**Key Components:**
- **Event Poller** (`src/engine/event.rs`): Polls events, executes trigger scripts, pushes tasks to Redis queue when successful
- **Task Processor** (`src/engine/task.rs`): Consumes Redis queue using 4-thread Rayon pool, executes task scripts
- **CLI** (`src/cli.rs`): Manages engine lifecycle, workflow YAML parsing, and database queries
- **Models** (`src/models.rs`): Diesel ORM models for engines, events, and tasks
- **Parser** (`src/parser.rs`): Parses workflow YAML files into database entries

### Data Flow
1. User adds workflow YAML → parsed into Events with Tasks → stored in PostgreSQL
2. Event process polls triggers → successful trigger pushes tasks to Redis queue
3. Task process consumes queue → executes scripts in parallel (4 threads)
4. All stdout/stderr captured and stored in PostgreSQL

## Development Workflow

### First-Time Setup
```bash
# Generate random password and create .env
echo "POSTGRES_PASSWORD=$(openssl rand -base64 32)" >> .env
echo "DATABASE_URL=postgres://postgres:$POSTGRES_PASSWORD@172.17.0.3:5432/postgres" >> .env
echo "REDIS_URL=redis://172.17.0.2/" >> .env

# Start containers (PostgreSQL + Redis)
chmod +x ./start_containers.sh
./start_containers.sh

# Run migrations (automatic on 'start' but can be manual)
cargo run migration
```

### Running the Engine
```bash
cargo run start              # Starts engine (runs migrations, spawns event/task processes)
cargo run add tests/workflows/example/workflow.yml
cargo run stop               # Sets stop_signal in DB (processes poll this)
```

### Key Commands
- `cargo run list engines/events/tasks` - Show database state using prettytable
- `cargo run show event 1` - Detailed view with stdout/stderr
- Logs written to `./logs/event_stdout.txt`, `./logs/task_stdout.txt`, etc.

### Database Inspection
```bash
docker exec -it workflow-postgres psql -U postgres -d postgres
# \dt to list tables: engines, events, tasks
# SELECT * FROM events;
```

## Project-Specific Patterns

### Process Architecture
The engine spawns **two child processes** via `cargo run start-event-process <uid>` and `start-task-process <uid>`:
- Processes are **long-running** with AtomicBool + Ctrl-C handlers for graceful shutdown
- Both processes poll `engines.stop_signal` in the database every 2 seconds
- Process status tracked in `engines.task_process_status` and `engines.event_process_status`
- Logs redirected to files in `./logs/` directory (cleared on start)

### Workflow YAML Structure
```yaml
name: workflow name
events:
  - name: Event1
    trigger: ./ping.sh              # Relative to workflow.yml location
    tasks:
      - name: foo
        path: ./tasks/task.sh
        on_failure: ./tasks/cleanup.sh  # Optional (not yet implemented)
```
- **All paths are relative to workflow.yml location** and converted to absolute paths during parsing
- Events re-poll if trigger script exits non-zero (status: Retrying)
- Tasks only execute if event trigger succeeds (exit code 0)

### Database Status Flow
**Events:** Created → Retrying (failed trigger) → Succeeded (trigger passed, tasks queued)
**Tasks:** Pending → Running → Completed/Failed
**Engines:** Starting → Running → Stopped

Status fields are currently **strings** (not enums) - see TODOs in `models.rs`

### Diesel Patterns
- Migrations embedded at compile-time: `embed_migrations!()` in `utils.rs`
- Schema auto-generated in `schema.rs` (don't edit manually)
- Models use `#[derive(Queryable)]` for reads, `#[derive(Insertable)]` for writes
- Foreign key: `tasks.event_uid` references `events.uid` with CASCADE

### Redis Queue Pattern
Tasks serialized with **bincode** (not JSON) and pushed to `"tasks"` queue:
```rust
let serialized: Vec<u8> = bincode::serialize(&light_task)?;
redis_con.rpush("tasks", serialized)?;
```

## Known Limitations & TODOs

1. **Status Fields**: Currently strings, planned migration to enums (see `models.rs:6`)
2. **on_failure**: Defined in schema but not executed (see `engine/task.rs:121`)
3. **Multiple Engines**: Docker Compose has 3 engines but distribution logic incomplete
4. **CLI Commands**: `pause`, `continue`, `abort` subcommands are stubs (`todo!()`)
5. **Error Handling**: Some panics instead of proper error propagation (see `utils.rs:28`)

## Testing

Example workflows in `tests/workflows/`:
- `example/workflow.yml` - Basic ping test
- `weather_checks/workflow.yml` - Weather check demo
- `periodic_checks/workflow.yml` - Time-based checks

No automated tests yet - manually verify by:
```bash
cargo run start
cargo run add tests/workflows/example/workflow.yml
cargo run list events  # Check status progression
```

## Docker Compose (Multi-Engine)
`docker-compose.yaml` defines 3 engines with separate Redis instances sharing one PostgreSQL:
- Each engine has isolated Redis queue (`redis1`, `redis2`, `redis3`)
- Environment variable `ENVIRONMENT=prod` switches from `cargo run` to `./workflow` binary
- Use `.envdocker` for container networking (postgres:5432 vs 172.17.0.3:5432)

## Architecture Decisions

**Why polling instead of cron?** Provides unified event model where triggers are arbitrary shell scripts
**Why Redis queue?** Decouples event triggering from task execution, enables parallel processing
**Why separate processes?** Isolation - event polling failure doesn't stop task execution
**Why Rayon?** Thread pool prevents overwhelming system with concurrent task execution (capped at 4)
