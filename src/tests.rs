

#[test]
fn test_if_redis_is_up() {
    assert!(crate::get_redis_con().is_ok());
}

#[test]
fn test_if_postgres_is_up() {
    assert!(crate::postgres().is_ok());
}

