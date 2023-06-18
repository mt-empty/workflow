# Workflow

Readability oriented workflow engine  

## Setup

Postgres password

```bash
echo "POSTGRES_PASSWORD=$(openssl rand -base64 32)" >> .env
```

## TODO
- [ ] Add a cli tool to 
  - [ ] parse workflow files
  - [ ] queue workflow after parsing
- [ ] Add a cli tool to 
  - [ ] check the status of workflows
  - [ ] Control workflows
- [ ] Event driven
- [ ] Make it distributed
