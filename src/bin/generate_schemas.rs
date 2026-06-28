use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let evidence_schema_dir = Path::new("schemas/evidence-ir");
    let mcp_schema_dir = Path::new("schemas/mcp");

    janus::evidence::write_schema_files(evidence_schema_dir)?;
    janus::query::write_schema_files(evidence_schema_dir)?;
    janus::mcp::write_schema_files(mcp_schema_dir)?;

    Ok(())
}
