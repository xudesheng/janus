use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    janus::evidence::write_schema_files(Path::new("schemas/evidence-ir"))
}
