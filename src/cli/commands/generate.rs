//! Generate OpenAPI specification

use anyhow::Result;
use std::io::{self, Write};

/// Output OpenAPI 3.1 JSON specification to stdout
pub async fn run() -> Result<()> {
    let spec = crate::server::openapi::build_openapi();
    let json = serde_json::to_string_pretty(&spec)?;

    let mut stdout = io::stdout().lock();
    writeln!(stdout, "{}", json)?;
    stdout.flush()?;

    Ok(())
}
