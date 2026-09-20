use std::path::Path;

use super::*;

pub(super) fn latest_source_stamp(root: &PathBuf) -> Option<SystemTime> {
    fn visit(path: &std::path::Path, latest: &mut Option<SystemTime>) {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(&path, latest);
                continue;
            }

            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = metadata.modified() else {
                continue;
            };

            match latest {
                Some(current) if *current >= modified => {}
                _ => *latest = Some(modified),
            }
        }
    }

    let mut latest = None;
    visit(root, &mut latest);
    latest
}

pub(super) fn latest_place_object_stamp(project: &ProjectPaths) -> Option<SystemTime> {
    let mut latest = latest_source_stamp(&project.scripts_dir());
    let hidden_project_dir = project.root_dir.join(".proj");
    if let Some(hidden_stamp) = latest_source_stamp(&hidden_project_dir) {
        match latest {
            Some(current) if current >= hidden_stamp => {}
            _ => latest = Some(hidden_stamp),
        }
    }
    latest
}

fn run_bridge_binary(
    bridge_path: &Path,
    _project: &ProjectPaths,
    output_tx: &Sender<String>,
) -> Result<ProjectMetadataSnapshot, String> {
    let _ = output_tx.send("Running cached bridge binary...".to_string());

    let mut command = Command::new(bridge_path);
    command.args(["--project-metadata"]);
    configure_background_command(&mut command);

    let output = command.output().map_err(|error| error.to_string())?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let _ = output_tx.send(format!("Bridge binary failed: {error}"));
        return Err(error);
    }

    ron::from_str(&String::from_utf8_lossy(&output.stdout)).map_err(|error| error.to_string())
}

fn compile_and_run_bridge(
    project: &ProjectPaths,
    output_tx: &Sender<String>,
) -> Result<ProjectMetadataSnapshot, String> {
    let _ = output_tx.send("Compiling bridge (first build may take several minutes)...".to_string());

    // First compile the bridge binary with stderr piped for progress
    let _proj_dir = project.root_dir.join(".proj");
    let mut compile = std::process::Command::new("cargo");
    compile
        .args(["build", "--bin", "runvy_object_bridge"])
        .current_dir(&project.root_dir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    configure_background_command(&mut compile);

    let mut child = compile.spawn().map_err(|error| error.to_string())?;

    // Read stderr (cargo build output) and send to output_tx
    if let Some(stderr) = child.stderr.take() {
        let tx = output_tx.clone();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stderr).lines() {
                if let Ok(line) = line {
                    let trimmed = line.trim().to_string();
                    if !trimmed.is_empty() {
                        let _ = tx.send(format!("[cargo] {trimmed}"));
                    }
                }
            }
        });
    }

    // Read stdout too
    if let Some(stdout) = child.stdout.take() {
        let tx = output_tx.clone();
        std::thread::spawn(move || {
            for line in std::io::BufReader::new(stdout).lines() {
                if let Ok(line) = line {
                    let trimmed = line.trim().to_string();
                    if !trimmed.is_empty() {
                        let _ = tx.send(format!("[cargo] {trimmed}"));
                    }
                }
            }
        });
    }

    // Wait for compilation with timeout (5 minutes)
    let start = std::time::Instant::now();
    let timeout = std::time::Duration::from_secs(300);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = output_tx.send("Bridge compilation timed out after 5 minutes.".to_string());
                    return Err("Bridge compilation timed out after 5 minutes.".to_string());
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            Err(error) => {
                return Err(format!("Failed to wait for bridge compilation: {error}"));
            }
        }
    };

    match status {
        Some(s) if s.success() => {}
        Some(_) => {
            let _ = output_tx.send("Bridge compilation failed.".to_string());
            return Err("Bridge compilation failed. See console output for details.".to_string());
        }
        None => {
            return Err("Bridge process terminated unexpectedly.".to_string());
        }
    }

    // Copy binary from target to .proj/
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    let target_binary = project.root_dir.join("target").join(profile).join(
        if cfg!(target_os = "windows") {
            "runvy_object_bridge.exe"
        } else {
            "runvy_object_bridge"
        },
    );
    let cached = runvy_project::cached_bridge_path(&project.root_dir);
    if target_binary.exists() {
        let _ = std::fs::copy(&target_binary, &cached);
    }

    let _ = output_tx.send("Bridge compiled successfully. Running...".to_string());

    // Run the cached binary
    run_bridge_binary(&cached, project, output_tx)
}

pub(super) fn query_project_metadata(
    project: &ProjectPaths,
    output_tx: &Sender<String>,
) -> Result<ProjectMetadataSnapshot, String> {
    let _ = output_tx.send("Refreshing project metadata...".to_string());

    // Try running cached bridge binary first (fast path)
    let cached = runvy_project::cached_bridge_path(&project.root_dir);
    if cached.exists() {
        let result = run_bridge_binary(&cached, project, output_tx);
        if result.is_ok() {
            return result;
        }
        let _ = output_tx.send("Cached bridge binary failed, recompiling...".to_string());
    }

    compile_and_run_bridge(project, output_tx)
}

pub(super) fn attach_child_output(
    child: &mut Child,
    output_tx: Sender<String>,
    prefix: &'static str,
) {
    if let Some(stdout) = child.stdout.take() {
        let tx = output_tx.clone();
        std::thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let _ = tx.send(format!("[{prefix}] {line}"));
            }
        });
    }

    if let Some(stderr) = child.stderr.take() {
        std::thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines().map_while(Result::ok) {
                let _ = output_tx.send(format!("[{prefix}] {line}"));
            }
        });
    }
}

#[cfg_attr(not(target_os = "windows"), allow(unused_variables))]
pub(super) fn configure_background_command(command: &mut Command) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
}

pub(super) fn merge_placeable_object_records(
    project_records: Vec<PlaceableObjectRecord>,
) -> Vec<PlaceableObjectRecord> {
    let mut merged = HashMap::new();
    for record in runvy_project::placeable_objects::default_records() {
        merged.insert(record.descriptor.id.clone(), record);
    }
    for record in project_records {
        if record.descriptor.id == "player"
            && record.descriptor.name == "Player"
            && record.descriptor.category == "Gameplay"
        {
            continue;
        }
        merged.insert(record.descriptor.id.clone(), record);
    }

    let mut records: Vec<_> = merged.into_values().collect();
    records.sort_by(|left, right| {
        left.descriptor
            .category
            .cmp(&right.descriptor.category)
            .then(left.descriptor.name.cmp(&right.descriptor.name))
    });
    records
}
