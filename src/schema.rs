// @generated automatically by Diesel CLI.

diesel::table! {
    engines (uid) {
        uid -> Int4,
        name -> Varchar,
        ip_address -> Varchar,
        status -> Varchar,
        stop_signal -> Bool,
        started_at -> Timestamp,
        stopped_at -> Timestamp,
        task_process_status -> Varchar,
        event_process_status -> Varchar,
    }
}

diesel::table! {
    events (uid) {
        uid -> Int4,
        name -> Nullable<Varchar>,
        description -> Nullable<Varchar>,
        trigger -> Varchar,
        status -> Varchar,
        created_at -> Timestamp,
        triggered_at -> Nullable<Timestamp>,
        deleted_at -> Nullable<Timestamp>,
    }
}

diesel::table! {
    tasks (uid) {
        uid -> Int4,
        event_uid -> Int4,
        name -> Nullable<Varchar>,
        description -> Nullable<Varchar>,
        path -> Varchar,
        on_failure -> Nullable<Varchar>,
        status -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        deleted_at -> Nullable<Timestamp>,
        completed_at -> Nullable<Timestamp>,
    }
}

diesel::joinable!(tasks -> events (event_uid));

diesel::allow_tables_to_appear_in_same_query!(
    engines,
    events,
    tasks,
);
