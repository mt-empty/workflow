-- This file should undo anything in `up.sql`

DROP TRIGGER update_engine_status_trigger ON engines;
DROP FUNCTION update_engine_status();

ALTER TABLE engines DROP COLUMN task_process_status;
ALTER TABLE engines DROP COLUMN event_process_status;
