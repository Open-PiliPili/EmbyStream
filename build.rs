use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=web/dist");

    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap_or_default());
    let generated = out_dir.join("generated_web_assets.rs");
    let embedded_dist_dir = out_dir.join("embedded_dist");
    let dist_dir = Path::new("web/dist");

    if !dist_dir.exists() {
        fs::write(
            generated,
            concat!(
                "#[derive(Clone, Copy)]\n",
                "pub struct EmbeddedAsset {\n",
                "    pub bytes: &'static [u8],\n",
                "    pub content_type: &'static str,\n",
                "}\n",
                "pub fn embedded_asset(_: &str) -> Option<EmbeddedAsset> { None }\n",
                "pub fn has_embedded_assets() -> bool { false }\n",
            ),
        )?;
        return Ok(());
    }

    if embedded_dist_dir.exists() {
        fs::remove_dir_all(&embedded_dist_dir)?;
    }
    fs::create_dir_all(&embedded_dist_dir)?;

    let mut files = Vec::new();
    collect_files(dist_dir, dist_dir, &mut files)?;
    files.sort();

    let mut output = String::from(
        "#[derive(Clone, Copy)]\n\
         pub struct EmbeddedAsset {\n\
             pub bytes: &'static [u8],\n\
             pub content_type: &'static str,\n\
         }\n\
         pub fn has_embedded_assets() -> bool { true }\n\
         pub fn embedded_asset(path: &str) -> Option<EmbeddedAsset> {\n\
             match path {\n",
    );

    for (relative, absolute) in files {
        let content_type = content_type_for(&relative);
        let embedded_path = embedded_dist_dir.join(&relative);
        if let Some(parent) = embedded_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&absolute, &embedded_path)?;
        let embedded_path = embedded_path.canonicalize()?;
        let embedded_path =
            embedded_path.to_string_lossy().replace('\\', "\\\\");
        output.push_str(&format!(
            "        \"{relative}\" => Some(EmbeddedAsset {{ bytes: include_bytes!(r#\"{embedded_path}\"#), content_type: \"{content_type}\" }}),\n",
        ));
    }

    output.push_str("        _ => None,\n    }\n}\n");
    fs::write(generated, output)?;
    Ok(())
}

fn collect_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<(String, PathBuf)>,
) -> io::Result<()> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .map_err(io::Error::other)?
            .to_string_lossy()
            .replace('\\', "/");
        files.push((relative, path));
    }

    Ok(())
}

fn content_type_for(path: &str) -> &'static str {
    match Path::new(path).extension().and_then(|value| value.to_str()) {
        Some("css") => "text/css; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("map") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}
