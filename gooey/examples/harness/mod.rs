use std::path::PathBuf;

/// Returns a path within the `target` directory. This function assumes the exe
/// running is an example.
pub fn snapshot_path(example: &str, name: &str) -> std::io::Result<PathBuf> {
    let exe_path = std::env::current_exe()?;
    let target_dir = exe_path
        .parent()
        .expect("examples dir")
        .parent()
        .expect("debug dir")
        .parent()
        .expect("target dir");

    let examples_dir = target_dir.join("snapshots").join(example);
    if !examples_dir.exists() {
        std::fs::create_dir_all(&examples_dir)?;
    }

    Ok(examples_dir.join(name))
}
