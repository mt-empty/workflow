use crate::schema::sql_types::EngineStatus;
use diesel::deserialize::{self, FromSql};
use diesel::pg::Pg;
use diesel::pg::{Pg, PgValue};
use diesel::prelude::*;
use diesel::serialize::{self, IsNull, Output, ToSql};
use diesel::*;
use std::io::Write;

// #[derive(SqlType)]
// #[diesel(postgres_type(name = "My_Type"))]
// pub struct MyType;

// #[derive(Debug, PartialEq, FromSqlRow, AsExpression, Eq)]
// #[diesel(sql_type = MyType)]
// pub enum MyEnum {
//     Running,
//     Stopped,
// }

// pub enum EngineStatus {
// }

// impl ToSql<MyType, Pg> for MyEnum {
//     fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
//         match *self {
//             MyEnum::Foo => out.write_all(b"foo")?,
//             MyEnum::Bar => out.write_all(b"bar")?,
//         }
//         Ok(IsNull::No)
//     }
// }

// impl FromSql<MyType, Pg> for MyEnum {
//     fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
//         match bytes.as_bytes() {
//             b"foo" => Ok(MyEnum::Foo),
//             b"bar" => Ok(MyEnum::Bar),
//             _ => Err("Unrecognized enum variant".into()),
//         }
//     }
// }
#[derive(Queryable, Selectable)]
#[diesel(table_name = crate::schema::engines)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Engine {
    pub id: i32,
    pub name: String,
    pub ip_address: String,
    pub status: EngineStatus,
    pub stop_signal: bool,
    pub started_at: chrono::NaiveDateTime,
    pub stopped_at: chrono::NaiveDateTime,
}

// #[derive(Insertable)]
// #[diesel(table_name = engines)]
// pub struct NewEngine<'a> {
//     pub name: &'a str,
//     pub ip_address: &'a str,
// }
