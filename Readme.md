# Workflow

Readability oriented workflow engine  

## Setup

Postgres password

```bash
echo "POSTGRES_PASSWORD=$(openssl rand -base64 32)" >> .env
```

Clippy 

```bash
cargo clippy --fix -- -W clippy::pedantic -W clippy::nursery -W clippy::unwrap_used -W clippy::todo -W clippy::dbg_macro -W clippy::print_stdout -W clippy::unimplemented

```

## TODO
- [o] test suite
- [ ] Add a cli tool to 
  - [ ] parse workflow files
  - [ ] queue workflow after parsing
- [ ] Add a cli tool to 
  - [ ] check the status of workflows
  - [ ] Control workflows
- [ ] Event driven
- [ ] Make it distributed
