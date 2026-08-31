use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run(env::args_os().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("stylist-lynx-cssc: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: impl Iterator<Item = std::ffi::OsString>) -> Result<(), String> {
    let mut input = None;
    let mut class = None;
    let mut output = None;
    let mut args = args;

    while let Some(argument) = args.next() {
        let slot = match argument.to_str() {
            Some("--input") => &mut input,
            Some("--class") => &mut class,
            Some("--output") => &mut output,
            Some(other) => return Err(format!("unexpected argument: {other}")),
            None => return Err("arguments must be valid UTF-8".into()),
        };
        if slot.is_some() {
            return Err(format!(
                "duplicate argument: {}",
                argument.to_string_lossy()
            ));
        }
        *slot = Some(
            args.next()
                .ok_or_else(|| format!("missing value for {}", argument.to_string_lossy()))?,
        );
    }

    let input = PathBuf::from(input.ok_or("missing required --input")?);
    let class = class
        .ok_or("missing required --class")?
        .into_string()
        .map_err(|_| "class must be valid UTF-8")?;
    let output = PathBuf::from(output.ok_or("missing required --output")?);
    let css = fs::read_to_string(&input)
        .map_err(|error| format!("failed to read {}: {error}", input.display()))?;
    let rules = stylist_lynx_cssc::convert(&css, &class)?;
    let mut json = serde_json::to_string_pretty(&rules)
        .map_err(|error| format!("failed to serialize ruleList: {error}"))?;
    json.push('\n');
    fs::write(&output, json)
        .map_err(|error| format!("failed to write {}: {error}", output.display()))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::run;

    #[test]
    fn rejects_non_exact_arguments() {
        for args in [
            vec![],
            vec!["--input", "in.css"],
            vec!["--unknown", "value"],
            vec!["--input"],
            vec![
                "--input",
                "one.css",
                "--input",
                "two.css",
                "--class",
                "counter",
                "--output",
                "rules.json",
            ],
            vec!["in.css", "--class", "counter", "--output", "rules.json"],
        ] {
            assert!(
                run(args.iter().map(|argument| OsString::from(*argument))).is_err(),
                "accepted {args:?}"
            );
        }
    }
}
