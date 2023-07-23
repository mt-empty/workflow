-- Your SQL goes here
CREATE TABLE IF NOT EXISTS events (
    uid             SERIAL PRIMARY KEY,
    name            VARCHAR NOT NULL,
    description     VARCHAR NOT NULL,
    trigger         VARCHAR NOT NULL,
    status          VARCHAR NOT NULL,
    created_at      TIMESTAMP NOT NULL DEFAULT NOW(),
    triggered_at    TIMESTAMP,
    deleted_at      TIMESTAMP
);

CREATE TABLE IF NOT EXISTS tasks (
    uid             SERIAL PRIMARY KEY,
    event_uid       INTEGER NOT NULL,
    name            VARCHAR NOT NULL,
    description     VARCHAR NOT NULL,
    path            VARCHAR NOT NULL,
    status          VARCHAR NOT NULL,
    on_failure      VARCHAR,
    created_at      TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMP NOT NULL DEFAULT NOW(),
    deleted_at      TIMESTAMP,
    completed_at    TIMESTAMP,
    CONSTRAINT fk_event_uid
        FOREIGN KEY(event_uid)
            REFERENCES events(uid) ON DELETE CASCADE ON UPDATE CASCADE
);

-- CREATE TYPE IF NOT EXISTS engine_status AS ENUM ('Stopped', 'Started');
CREATE TABLE IF NOT EXISTS engines (
    id              SERIAL PRIMARY KEY,
    name            VARCHAR NOT NULL,
    ip_address      VARCHAR NOT NULL,
    -- status          engine_status NOT NULL DEFAULT 'Stopped',
    status          VARCHAR NOT NULL DEFAULT 'Stopped',
    stop_signal     BOOLEAN NOT NULL DEFAULT false,
    started_at      TIMESTAMP NOT NULL DEFAULT NOW(),
    stopped_at      TIMESTAMP NOT NULL DEFAULT NOW()
);

-- ALTER TABLE engines ADD CONSTRAINT engine_status_unique CHECK (id = 1);
-- ALTER TABLE engines ADD CONSTRAINT engine_name_unique UNIQUE (name, ip_address);