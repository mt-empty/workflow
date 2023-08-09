-- This file should undo anything in `up.sql`
ALTER TABLE events DROP COLUMN stdout;
ALTER TABLE events DROP COLUMN stderr;

ALTER TABLE tasks DROP COLUMN stdout;
ALTER TABLE tasks DROP COLUMN stderr;

