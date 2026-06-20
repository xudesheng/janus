use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let schema_dir = Path::new("schemas/evidence-ir");

    janus::evidence::write_schema_files(schema_dir)?;
    janus::query::write_schema_files(schema_dir)?;

    Ok(())
}
