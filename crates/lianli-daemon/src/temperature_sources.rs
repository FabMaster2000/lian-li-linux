use anyhow::{anyhow, Context, Result};
use lianli_shared::ipc::{FanTemperatureComponent, FanTemperaturePreview};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug)]
enum TemperatureSource {
    Cpu,
    Gpu,
    Max(Vec<TemperatureSource>),
    Min(Vec<TemperatureSource>),
    Avg(Vec<TemperatureSource>),
}

#[derive(Clone, Debug)]
enum ParsedTemperatureSource {
    Structured(TemperatureSource),
    Command(String),
}

#[derive(Clone, Debug)]
struct EvaluatedSource {
    celsius: Option<f32>,
    missing_labels: Vec<String>,
}

pub fn read_temperature(source: &str) -> Result<f32> {
    let preview = preview_temperature_source(source);
    preview.celsius.ok_or_else(|| {
        anyhow!(
            "{}",
            preview
                .note
                .unwrap_or_else(|| "temperature source is currently unavailable".to_string())
        )
    })
}

pub fn preview_temperature_source(source: &str) -> FanTemperaturePreview {
    let trimmed = source.trim();
    if trimmed.is_empty() {
        return FanTemperaturePreview {
            source: String::new(),
            display_name: "No temperature source".to_string(),
            available: false,
            celsius: None,
            components: Vec::new(),
            note: Some("Select a hardware source or enter a custom command.".to_string()),
        };
    }

    match parse_temperature_source(trimmed) {
        ParsedTemperatureSource::Structured(spec) => preview_structured_source(trimmed, &spec),
        ParsedTemperatureSource::Command(command) => preview_command_source(trimmed, &command),
    }
}

fn preview_structured_source(source: &str, spec: &TemperatureSource) -> FanTemperaturePreview {
    let mut components = BTreeMap::new();
    let evaluated = evaluate_source(spec, &mut components);
    let available_count = components.values().filter(|component| component.available).count();
    let missing_count = components.len().saturating_sub(available_count);
    let note = if evaluated.celsius.is_none() && !evaluated.missing_labels.is_empty() {
        Some(format!(
            "{} unavailable.",
            join_labels(&evaluated.missing_labels)
        ))
    } else if evaluated.celsius.is_some() && missing_count > 0 && !evaluated.missing_labels.is_empty() {
        Some(format!(
            "{} unavailable; using {} only.",
            join_labels(&evaluated.missing_labels),
            join_labels(
                &components
                    .values()
                    .filter(|component| component.available)
                    .map(|component| component.label.clone())
                    .collect::<Vec<_>>(),
            )
        ))
    } else {
        None
    };

    FanTemperaturePreview {
        source: source.to_string(),
        display_name: display_name(spec),
        available: evaluated.celsius.is_some(),
        celsius: evaluated.celsius,
        components: components.into_values().collect(),
        note,
    }
}

fn preview_command_source(source: &str, command: &str) -> FanTemperaturePreview {
    match execute_temperature_command(command) {
        Ok(celsius) => FanTemperaturePreview {
            source: source.to_string(),
            display_name: "Custom command".to_string(),
            available: true,
            celsius: Some(celsius),
            components: vec![FanTemperatureComponent {
                key: "command".to_string(),
                label: "Command".to_string(),
                kind: "command".to_string(),
                available: true,
                celsius: Some(celsius),
                note: None,
            }],
            note: None,
        },
        Err(error) => FanTemperaturePreview {
            source: source.to_string(),
            display_name: "Custom command".to_string(),
            available: false,
            celsius: None,
            components: vec![FanTemperatureComponent {
                key: "command".to_string(),
                label: "Command".to_string(),
                kind: "command".to_string(),
                available: false,
                celsius: None,
                note: Some(error.to_string()),
            }],
            note: Some(error.to_string()),
        },
    }
}

fn evaluate_source(
    source: &TemperatureSource,
    components: &mut BTreeMap<String, FanTemperatureComponent>,
) -> EvaluatedSource {
    match source {
        TemperatureSource::Cpu => evaluate_leaf_source(components, "cpu", "CPU", "cpu", read_cpu_temperature),
        TemperatureSource::Gpu => evaluate_leaf_source(components, "gpu", "GPU", "gpu", read_gpu_temperature),
        TemperatureSource::Max(children) => evaluate_aggregate_source(children, components, |values| {
            values.into_iter().fold(f32::NEG_INFINITY, f32::max)
        }),
        TemperatureSource::Min(children) => evaluate_aggregate_source(children, components, |values| {
            values.into_iter().fold(f32::INFINITY, f32::min)
        }),
        TemperatureSource::Avg(children) => evaluate_aggregate_source(children, components, |values| {
            let total = values.iter().sum::<f32>();
            total / values.len() as f32
        }),
    }
}

fn evaluate_leaf_source(
    components: &mut BTreeMap<String, FanTemperatureComponent>,
    key: &str,
    label: &str,
    kind: &str,
    sample: fn() -> Result<f32>,
) -> EvaluatedSource {
    if let Some(existing) = components.get(key) {
        return EvaluatedSource {
            celsius: existing.celsius,
            missing_labels: if existing.available {
                Vec::new()
            } else {
                vec![label.to_string()]
            },
        };
    }

    let component = match sample() {
        Ok(celsius) => FanTemperatureComponent {
            key: key.to_string(),
            label: label.to_string(),
            kind: kind.to_string(),
            available: true,
            celsius: Some(celsius),
            note: None,
        },
        Err(error) => FanTemperatureComponent {
            key: key.to_string(),
            label: label.to_string(),
            kind: kind.to_string(),
            available: false,
            celsius: None,
            note: Some(error.to_string()),
        },
    };

    let celsius = component.celsius;
    let available = component.available;
    components.insert(key.to_string(), component);
    EvaluatedSource {
        celsius,
        missing_labels: if available {
            Vec::new()
        } else {
            vec![label.to_string()]
        },
    }
}

fn evaluate_aggregate_source(
    children: &[TemperatureSource],
    components: &mut BTreeMap<String, FanTemperatureComponent>,
    aggregate: impl Fn(Vec<f32>) -> f32,
) -> EvaluatedSource {
    let mut values = Vec::new();
    let mut missing = Vec::new();

    for child in children {
        let evaluated = evaluate_source(child, components);
        if let Some(celsius) = evaluated.celsius {
            values.push(celsius);
        }
        missing.extend(evaluated.missing_labels);
    }

    missing.sort();
    missing.dedup();

    let celsius = if values.is_empty() {
        None
    } else {
        Some(aggregate(values))
    };

    EvaluatedSource {
        celsius,
        missing_labels: missing,
    }
}

fn parse_temperature_source(source: &str) -> ParsedTemperatureSource {
    parse_structured_source(source)
        .map(ParsedTemperatureSource::Structured)
        .unwrap_or_else(|| ParsedTemperatureSource::Command(source.to_string()))
}

fn parse_structured_source(source: &str) -> Option<TemperatureSource> {
    let normalized = source.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "cpu" => return Some(TemperatureSource::Cpu),
        "gpu" => return Some(TemperatureSource::Gpu),
        _ => {}
    }

    for prefix in ["max", "min", "avg"] {
        let open = format!("{prefix}(");
        if normalized.starts_with(&open) && normalized.ends_with(')') {
            let inner = &normalized[open.len()..normalized.len() - 1];
            let parts = split_top_level_arguments(inner);
            if parts.is_empty() {
                return None;
            }
            let children = parts
                .iter()
                .map(|part| parse_structured_source(part))
                .collect::<Option<Vec<_>>>()?;
            return Some(match prefix {
                "max" => TemperatureSource::Max(children),
                "min" => TemperatureSource::Min(children),
                "avg" => TemperatureSource::Avg(children),
                _ => unreachable!(),
            });
        }
    }

    None
}

fn split_top_level_arguments(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;

    for (index, character) in input.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let part = input[start..index].trim();
                if !part.is_empty() {
                    parts.push(part);
                }
                start = index + 1;
            }
            _ => {}
        }
    }

    let tail = input[start..].trim();
    if !tail.is_empty() {
        parts.push(tail);
    }

    parts
}

fn display_name(source: &TemperatureSource) -> String {
    match source {
        TemperatureSource::Cpu => "CPU temperature".to_string(),
        TemperatureSource::Gpu => "GPU temperature".to_string(),
        TemperatureSource::Max(children) => format!("Highest of {}", describe_children(children)),
        TemperatureSource::Min(children) => format!("Lowest of {}", describe_children(children)),
        TemperatureSource::Avg(children) => format!("Average of {}", describe_children(children)),
    }
}

fn describe_children(children: &[TemperatureSource]) -> String {
    children
        .iter()
        .map(|child| match child {
            TemperatureSource::Cpu => "CPU".to_string(),
            TemperatureSource::Gpu => "GPU".to_string(),
            _ => display_name(child),
        })
        .collect::<Vec<_>>()
        .join(" and ")
}

fn join_labels(labels: &[String]) -> String {
    match labels {
        [] => "sources".to_string(),
        [single] => single.clone(),
        [left, right] => format!("{left} and {right}"),
        _ => {
            let mut joined = labels[..labels.len() - 1].join(", ");
            joined.push_str(", and ");
            joined.push_str(&labels[labels.len() - 1]);
            joined
        }
    }
}

fn read_cpu_temperature() -> Result<f32> {
    let mut candidates = thermal_zone_temperatures(&[
        "cpu",
        "pkg",
        "package",
        "tdie",
        "tctl",
        "x86_pkg_temp",
    ])?;
    candidates.extend(hwmon_temperatures(
        &["coretemp", "k10temp", "zenpower", "cpu_thermal", "soc_thermal"],
        &["package", "tdie", "tctl", "cpu", "core"],
    )?);

    select_temperature(candidates, "CPU")
}

fn read_gpu_temperature() -> Result<f32> {
    let mut candidates = drm_gpu_temperatures()?;
    candidates.extend(hwmon_temperatures(
        &["amdgpu", "nouveau", "nvidia", "i915", "xe"],
        &["gpu", "edge", "junction", "memory"],
    )?);

    if candidates.is_empty() {
        if let Ok(celsius) = read_nvidia_smi_temperature() {
            candidates.push(celsius);
        }
    }

    select_temperature(candidates, "GPU")
}

fn thermal_zone_temperatures(keywords: &[&str]) -> Result<Vec<f32>> {
    let base = rooted_path("/sys/class/thermal");
    let mut values = Vec::new();
    let Ok(entries) = fs::read_dir(&base) else {
        return Ok(values);
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !file_name.starts_with("thermal_zone") {
            continue;
        }
        let zone_type = read_trimmed(path.join("type")).unwrap_or_default();
        if !matches_keywords(&zone_type, keywords) {
            continue;
        }
        if let Ok(value) = read_temperature_file(path.join("temp")) {
            values.push(value);
        }
    }

    Ok(values)
}

fn hwmon_temperatures(device_keywords: &[&str], label_keywords: &[&str]) -> Result<Vec<f32>> {
    let base = rooted_path("/sys/class/hwmon");
    let mut values = Vec::new();
    let Ok(entries) = fs::read_dir(&base) else {
        return Ok(values);
    };

    for entry in entries.flatten() {
        let hwmon_path = entry.path();
        let name = read_trimmed(hwmon_path.join("name")).unwrap_or_default();
        let device_match = matches_keywords(&name, device_keywords);

        let Ok(temp_entries) = fs::read_dir(&hwmon_path) else {
            continue;
        };

        for temp_entry in temp_entries.flatten() {
            let temp_path = temp_entry.path();
            let Some(file_name) = temp_path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !file_name.starts_with("temp") || !file_name.ends_with("_input") {
                continue;
            }

            let label_name = file_name.trim_end_matches("_input");
            let label = read_trimmed(hwmon_path.join(format!("{label_name}_label"))).unwrap_or_default();
            if !(device_match || matches_keywords(&label, label_keywords)) {
                continue;
            }

            if let Ok(value) = read_temperature_file(&temp_path) {
                values.push(value);
            }
        }
    }

    Ok(values)
}

fn drm_gpu_temperatures() -> Result<Vec<f32>> {
    let base = rooted_path("/sys/class/drm");
    let mut values = Vec::new();
    let Ok(entries) = fs::read_dir(&base) else {
        return Ok(values);
    };

    for entry in entries.flatten() {
        let card_path = entry.path();
        let Some(file_name) = card_path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !file_name.starts_with("card") {
            continue;
        }

        let hwmon_root = card_path.join("device").join("hwmon");
        let Ok(hwmon_dirs) = fs::read_dir(hwmon_root) else {
            continue;
        };

        for hwmon_dir in hwmon_dirs.flatten() {
            let Ok(temp_entries) = fs::read_dir(hwmon_dir.path()) else {
                continue;
            };
            for temp_entry in temp_entries.flatten() {
                let temp_path = temp_entry.path();
                let Some(temp_name) = temp_path.file_name().and_then(|value| value.to_str()) else {
                    continue;
                };
                if !temp_name.starts_with("temp") || !temp_name.ends_with("_input") {
                    continue;
                }
                if let Ok(value) = read_temperature_file(&temp_path) {
                    values.push(value);
                }
            }
        }
    }

    Ok(values)
}

fn read_nvidia_smi_temperature() -> Result<f32> {
    let nvidia_smi = find_nvidia_smi().context("nvidia-smi not found")?;
    let output = Command::new(&nvidia_smi)
        .args([
            "--query-gpu=temperature.gpu",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .with_context(|| format!("executing {}", nvidia_smi.display()))?;

    if !output.status.success() {
        anyhow::bail!("nvidia-smi failed with status {}", output.status);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let values = stdout
        .lines()
        .filter_map(|line| line.trim().parse::<f32>().ok())
        .collect::<Vec<_>>();

    select_temperature(values, "GPU")
}

/// Locate `nvidia-smi` — try `PATH` first, then well-known locations that
/// may not be on the daemon's `PATH` (e.g. `/usr/lib/wsl/lib` in WSL2).
fn find_nvidia_smi() -> Option<PathBuf> {
    // Try PATH first (works when nvidia-smi is on PATH)
    if Command::new("nvidia-smi").arg("--version").output().is_ok_and(|o| o.status.success()) {
        return Some(PathBuf::from("nvidia-smi"));
    }

    // Well-known fallback locations (WSL2, standard Linux installs)
    let candidates = [
        "/usr/lib/wsl/lib/nvidia-smi",
        "/usr/bin/nvidia-smi",
        "/usr/local/bin/nvidia-smi",
    ];
    for candidate in candidates {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Some(path);
        }
    }

    None
}

fn execute_temperature_command(command: &str) -> Result<f32> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(command)
        .output()
        .context("executing temperature command")?;

    if !output.status.success() {
        anyhow::bail!("temperature command failed with status {}", output.status);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let temp_str = stdout.split_whitespace().next().unwrap_or("0");
    let temp = temp_str
        .parse::<f32>()
        .with_context(|| format!("parsing temperature value '{temp_str}'"))?;

    if !temp.is_finite() {
        anyhow::bail!("temperature value '{temp}' is not finite");
    }

    Ok(temp)
}

fn select_temperature(values: Vec<f32>, label: &str) -> Result<f32> {
    values
        .into_iter()
        .reduce(f32::max)
        .ok_or_else(|| anyhow!("{label} temperature sensor not found"))
}

fn rooted_path(path: &str) -> PathBuf {
    let root = std::env::var_os("LIANLI_TEMP_SYSFS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"));
    if root == Path::new("/") {
        PathBuf::from(path)
    } else {
        root.join(path.trim_start_matches('/'))
    }
}

fn read_trimmed(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok().map(|value| value.trim().to_string())
}

fn read_temperature_file(path: impl AsRef<Path>) -> Result<f32> {
    let raw = fs::read_to_string(path.as_ref())
        .with_context(|| format!("reading {}", path.as_ref().display()))?;
    let parsed = raw
        .trim()
        .parse::<f32>()
        .with_context(|| format!("parsing {}", path.as_ref().display()))?;

    if parsed.abs() >= 1_000.0 {
        Ok(parsed / 1_000.0)
    } else {
        Ok(parsed)
    }
}

fn matches_keywords(value: &str, keywords: &[&str]) -> bool {
    let normalized = value.trim().to_ascii_lowercase();
    keywords.iter().any(|keyword| normalized.contains(keyword))
}

#[cfg(test)]
mod tests {
    use super::{
        parse_temperature_source, preview_temperature_source, read_temperature, rooted_path,
        split_top_level_arguments, ParsedTemperatureSource, TemperatureSource,
    };
    use std::fs;
    use std::path::Path;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn parses_builtin_temperature_sources() {
        match parse_temperature_source("max(cpu,gpu)") {
            ParsedTemperatureSource::Structured(TemperatureSource::Max(children)) => {
                assert_eq!(children.len(), 2);
            }
            other => panic!("unexpected parse result: {other:?}"),
        }
    }

    #[test]
    fn keeps_unknown_sources_as_commands() {
        match parse_temperature_source("printf 42") {
            ParsedTemperatureSource::Command(command) => assert_eq!(command, "printf 42"),
            other => panic!("unexpected parse result: {other:?}"),
        }
    }

    #[test]
    fn splits_nested_arguments() {
        assert_eq!(
            split_top_level_arguments("cpu,max(cpu,gpu),avg(cpu,gpu)"),
            vec!["cpu", "max(cpu,gpu)", "avg(cpu,gpu)"]
        );
    }

    #[test]
    fn previews_cpu_and_gpu_from_fake_sysfs() {
        let _guard = env_lock().lock().expect("lock env");
        let tempdir = tempfile::tempdir().expect("create tempdir");
        let root = tempdir.path();
        write_file(
            root,
            "sys/class/hwmon/hwmon0/name",
            "k10temp\n",
        );
        write_file(
            root,
            "sys/class/hwmon/hwmon0/temp1_input",
            "54000\n",
        );
        write_file(
            root,
            "sys/class/hwmon/hwmon0/temp1_label",
            "Tctl\n",
        );
        write_file(
            root,
            "sys/class/drm/card0/device/hwmon/hwmon1/temp1_input",
            "61000\n",
        );

        std::env::set_var("LIANLI_TEMP_SYSFS_ROOT", root);
        let preview = preview_temperature_source("max(cpu,gpu)");

        assert!(preview.available);
        assert_eq!(preview.celsius, Some(61.0));
        assert_eq!(preview.components.len(), 2);
        assert!(preview
            .components
            .iter()
            .any(|component| component.key == "cpu" && component.celsius == Some(54.0)));
        assert!(preview
            .components
            .iter()
            .any(|component| component.key == "gpu" && component.celsius == Some(61.0)));

        std::env::remove_var("LIANLI_TEMP_SYSFS_ROOT");
    }

    #[test]
    fn read_temperature_supports_custom_commands() {
        let celsius = read_temperature("printf 47.5").expect("read command temperature");
        assert_eq!(celsius, 47.5);
    }

    #[test]
    fn rooted_path_uses_override_root() {
        let _guard = env_lock().lock().expect("lock env");
        std::env::set_var("LIANLI_TEMP_SYSFS_ROOT", "/tmp/lianli-root");
        assert_eq!(
            rooted_path("/sys/class/hwmon"),
            Path::new("/tmp/lianli-root").join("sys/class/hwmon")
        );
        std::env::remove_var("LIANLI_TEMP_SYSFS_ROOT");
    }

    fn write_file(root: &Path, relative_path: &str, content: &str) {
        let path = root.join(relative_path);
        fs::create_dir_all(path.parent().expect("file parent")).expect("create parent");
        fs::write(path, content).expect("write file");
    }
}
