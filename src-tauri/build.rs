use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

fn copy_regular_file(source: &Path, destination: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if !metadata.file_type().is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "expected a regular build-context file: {}",
                source.display()
            ),
        ));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(())
}

fn copy_regular_tree(source: &Path, destination: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(source)?;
    if !metadata.file_type().is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("expected a build-context directory: {}", source.display()),
        ));
    }
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_regular_tree(&entry.path(), &target)?;
        } else if file_type.is_file() {
            copy_regular_file(&entry.path(), &target)?;
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "unsupported build-context entry: {}",
                    entry.path().display()
                ),
            ));
        }
    }
    Ok(())
}

fn prepare_external_tauri_context() -> io::Result<PathBuf> {
    let source = env::current_dir()?;
    let out_dir = env::var_os("OUT_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "OUT_DIR is not set"))?;
    let context = out_dir.join("brewdesk-tauri-context");

    fs::create_dir_all(&context)?;
    copy_regular_file(&source.join("Cargo.toml"), &context.join("Cargo.toml"))?;
    copy_regular_file(
        &source.join("tauri.conf.json"),
        &context.join("tauri.conf.json"),
    )?;
    copy_regular_tree(&source.join("capabilities"), &context.join("capabilities"))?;
    copy_regular_tree(&source.join("icons"), &context.join("icons"))?;
    copy_regular_file(&source.join("../index.html"), &out_dir.join("index.html"))?;

    Ok(context)
}

fn main() {
    println!("cargo:rerun-if-changed=../index.html");
    println!("cargo:rerun-if-changed=capabilities");
    println!("cargo:rerun-if-changed=icons");
    println!("cargo:rerun-if-changed=tauri.conf.json");

    let context = prepare_external_tauri_context()
        .unwrap_or_else(|error| panic!("failed to prepare external Tauri build context: {error}"));
    env::set_current_dir(&context)
        .unwrap_or_else(|error| panic!("failed to enter external Tauri build context: {error}"));
    tauri_build::build()
}
