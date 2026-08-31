use std::env;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub fn compile(
    input: impl AsRef<Path>,
    class: &str,
    output: impl AsRef<Path>,
) -> Result<PathBuf, String> {
    let compiler = env::var_os("LYNX_CSSC")
        .ok_or("LYNX_CSSC must point to the pinned lynx-cssc host executable")?;
    let manifest_dir = env::var_os("CARGO_MANIFEST_DIR")
        .ok_or("CARGO_MANIFEST_DIR is unavailable outside a Cargo build script")?;
    let out_dir =
        env::var_os("OUT_DIR").ok_or("OUT_DIR is unavailable outside a Cargo build script")?;

    compile_with(
        compiler,
        manifest_dir,
        out_dir,
        input.as_ref(),
        class,
        output.as_ref(),
    )
}

pub fn compile_with(
    compiler: impl AsRef<Path>,
    manifest_dir: impl AsRef<Path>,
    out_dir: impl AsRef<Path>,
    input: impl AsRef<Path>,
    class: &str,
    output: impl AsRef<Path>,
) -> Result<PathBuf, String> {
    let manifest_dir = manifest_dir.as_ref();
    let input = input.as_ref();
    let input = if input.is_absolute() {
        input.to_owned()
    } else {
        manifest_dir.join(input)
    };
    let output = output.as_ref();
    if output.components().count() != 1
        || !matches!(output.components().next(), Some(Component::Normal(_)))
    {
        return Err("output must be a file name without directory components".into());
    }
    let compiler = compiler.as_ref();
    if !compiler.is_absolute() {
        return Err("LYNX_CSSC must be an absolute path".into());
    }

    println!("cargo:rerun-if-changed={}", input.display());
    println!("cargo:rerun-if-changed={}", compiler.display());
    println!("cargo:rerun-if-env-changed=LYNX_CSSC");

    let css = fs::read_to_string(&input)
        .map_err(|error| format!("failed to read {}: {error}", input.display()))?;
    let rules = stylist_lynx_cssc::convert(&css, class)?;
    let out_dir = out_dir.as_ref();
    fs::create_dir_all(out_dir)
        .map_err(|error| format!("failed to create {}: {error}", out_dir.display()))?;
    let output = out_dir.join(output);
    let temporary_output = with_suffix(&output, ".tmp");
    let rules_path = with_suffix(&output, ".rules.json");
    let mut json = serde_json::to_string_pretty(&rules)
        .map_err(|error| format!("failed to serialize ruleList: {error}"))?;
    json.push('\n');
    fs::write(&rules_path, json)
        .map_err(|error| format!("failed to write {}: {error}", rules_path.display()))?;
    for stale in [&output, &temporary_output] {
        if let Err(error) = fs::remove_file(stale) {
            if error.kind() != std::io::ErrorKind::NotFound {
                return Err(format!("failed to remove {}: {error}", stale.display()));
            }
        }
    }

    let status = Command::new(compiler)
        .arg("--input")
        .arg(&rules_path)
        .arg("--output")
        .arg(&temporary_output)
        .status()
        .map_err(|error| format!("failed to run {}: {error}", compiler.display()))?;
    if !status.success() {
        return Err(format!("{} failed with {status}", compiler.display()));
    }
    if !temporary_output.is_file()
        || fs::metadata(&temporary_output)
            .map_err(|error| format!("failed to inspect {}: {error}", temporary_output.display()))?
            .len()
            == 0
    {
        return Err(format!(
            "{} did not produce a non-empty fragment",
            compiler.display()
        ));
    }
    fs::rename(&temporary_output, &output).map_err(|error| {
        format!(
            "failed to move {} to {}: {error}",
            temporary_output.display(),
            output.display()
        )
    })?;

    Ok(output)
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut file_name = path
        .file_name()
        .expect("validated output has a file name")
        .to_os_string();
    file_name.push(suffix);
    path.with_file_name(file_name)
}

#[cfg(test)]
mod tests {
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use super::compile_with;

    #[cfg(unix)]
    #[test]
    fn compiles_css_into_the_requested_out_dir() {
        let temp = std::env::temp_dir().join(format!(
            "lynx-css-build-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(temp.join("styles")).unwrap();
        fs::write(temp.join("styles/app.css"), "width: 123px;\n").unwrap();
        let compiler = temp.join("lynx-cssc");
        fs::write(
            &compiler,
            "#!/bin/sh\nwhile [ $# -gt 0 ]; do\n  case \"$1\" in\n    --input) input=$2; shift 2 ;;\n    --output) output=$2; shift 2 ;;\n    *) exit 2 ;;\n  esac\ndone\ncp \"$input\" \"$output\"\n",
        )
        .unwrap();
        fs::set_permissions(&compiler, fs::Permissions::from_mode(0o755)).unwrap();

        let output = compile_with(
            &compiler,
            &temp,
            temp.join("out"),
            "styles/app.css",
            "app",
            "app.lynxcss",
        )
        .unwrap();

        let fragment = fs::read_to_string(output).unwrap();
        assert!(fragment.contains("\"value\": \".app\""));
        assert!(fragment.contains("\"name\": \"width\""));
        fs::remove_dir_all(temp).unwrap();
    }

    #[test]
    fn rejects_output_paths_outside_out_dir() {
        let error = compile_with(
            "lynx-cssc",
            ".",
            "out",
            "style.css",
            "app",
            "../app.lynxcss",
        )
        .unwrap_err();
        assert!(error.contains("file name"));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_success_without_a_fresh_fragment() {
        let temp =
            std::env::temp_dir().join(format!("lynx-css-build-stale-{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp);
        fs::create_dir_all(temp.join("out")).unwrap();
        fs::write(temp.join("style.css"), "width: 123px;\n").unwrap();
        fs::write(temp.join("out/app.lynxcss"), "stale").unwrap();
        let compiler = temp.join("lynx-cssc");
        fs::write(&compiler, "#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&compiler, fs::Permissions::from_mode(0o755)).unwrap();

        let error = compile_with(
            &compiler,
            &temp,
            temp.join("out"),
            "style.css",
            "app",
            "app.lynxcss",
        )
        .unwrap_err();

        assert!(error.contains("did not produce"));
        assert!(!temp.join("out/app.lynxcss").exists());
        fs::remove_dir_all(temp).unwrap();
    }
}
