use std::env;

use dotenvy::dotenv;
use diesel::{Connection, PgConnection};

// TODO: Connection pooling
pub fn establish_connection() -> Result<PgConnection, diesel::result::ConnectionError> {
    dotenv().ok();
    let db_url = env::var("DATABASE_URL")
    .map_err(
        |_| diesel::result::ConnectionError::BadConnection("DATABASE_URL not set".into())
    )?;
    PgConnection::establish(&db_url)
}
