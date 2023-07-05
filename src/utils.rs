use dotenv::dotenv;
use redis::RedisResult;
use std::env;

pub fn get_redis_con() -> RedisResult<redis::Connection> {
    dotenv().ok();
    let client = redis::Client::open(env::var("REDIS_URL").expect("Redis url not set"))?;
    let con = client.get_connection()?;
    Ok(con)
}
