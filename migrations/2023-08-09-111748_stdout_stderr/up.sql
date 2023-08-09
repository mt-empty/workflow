-- Your SQL goes here
ALTER TABLE events ADD COLUMN stdout TEXT;
AlTER TABLE events ADD COLUMN stderr TEXT;

ALTER TABLE tasks ADD COLUMN stdout TEXT;
ALTER TABLE tasks ADD COLUMN stderr TEXT;
