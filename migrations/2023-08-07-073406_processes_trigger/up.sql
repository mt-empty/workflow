-- Your SQL goes here
ALTER TABLE engines ADD COLUMN task_process_status VARCHAR NOT NULL DEFAULT '';
ALTER TABLE engines ADD COLUMN event_process_status VARCHAR NOT NULL DEFAULT '';

CREATE OR REPLACE FUNCTION update_engine_status() RETURNS TRIGGER AS $$
BEGIN
    IF NEW.task_process_status = 'Stopped' AND NEW.event_process_status = 'Stopped' THEN
        NEW.status = 'Stopped';
        NEW.stopped_at = NOW();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_engine_status_trigger
BEFORE UPDATE OF task_process_status, event_process_status ON engines
FOR EACH ROW
EXECUTE FUNCTION update_engine_status();
