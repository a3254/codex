use codex_config::schema::canonicalize;
use codex_config::schema::config_schema_json;
use codex_config::schema::write_config_schema;

#[cfg(all(test, any()))]
#[path = "schema_tests.rs.old"]
mod tests;
