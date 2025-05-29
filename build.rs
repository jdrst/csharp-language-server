use std::path::Path;
use std::process::Command;

fn main() {
    let marker_path = Path::new("language-server.zip");

    if !marker_path.exists() {
        let status = Command::new("./download-server")
            .status()
            .expect("Failed to run the setup script");

        if !status.success() {
            panic!("Setup script failed with status: {}", status);
        }
    }
}
