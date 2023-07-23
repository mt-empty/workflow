// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "engine_status"))]
    pub struct EngineStatus;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::EngineStatus;

    engines (id) {
        id -> Int4,
        name -> Varchar,
        ip_address -> Varchar,
        status -> EngineStatus,
        stop_signal -> Bool,
        started_at -> Timestamp,
        stopped_at -> Timestamp,
    }
}

diesel::table! {
    events (uid) {
        uid -> Int4,
        name -> Varchar,
        description -> Varchar,
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
        name -> Varchar,
        description -> Varchar,
        path -> Varchar,
        status -> Varchar,
        on_failure -> Nullable<Varchar>,
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
