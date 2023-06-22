# Workflow

Readability oriented workflow engine  

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

## TODO
- [o] test suite
- [ ] Add a cli tool to 
  - [ ] parse workflow yaml files
  - [ ] add tasks to workflow queue 
- [ ] Add a cli tool to 
  - [ ] check the status of workflows
  - [ ] Control workflows, stop, continue, delete
- [ ] Add support for event triggers
- [ ] Make it distributed
