use super::*;

impl<'window> EditorApp<'window> {
    pub(super) fn return_to_welcome(&mut self) {
        self.stop_project();
        if let Some(mut child) = self.build_process.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        self.world = helpers::create_preview_world();
        self.world
            .borrow_mut()
            .set_runtime_registry(Arc::new(self.runtime_engine.runtime_registry().clone()));
        self.set_primary_selection(self.first_object_id());
        self.project_session = None;
        self.project_version_prompt = None;
        self.place_object = PlaceObjectState::default();
        self.hierarchy_clipboard = None;
        self.content_browser.set_project_root(
            dirs::document_dir().unwrap_or_else(helpers::default_browse_root),
            &self.settings,
        );
        self.status_line = "Returned to Welcome screen.".to_string();
        self.push_output(self.status_line.clone());
    }

    pub(super) fn refresh_project_metadata(&mut self, force: bool) {
        let Some(session) = self.project_session.as_ref() else {
            if force {
                self.status_line = "Open a project before refreshing project metadata.".to_string();
            }
            return;
        };

        if force {
            self.content_browser.refresh(&self.settings);
            self.status_line = format!(
                "Refreshing project metadata for {}...",
                session.project.manifest.name
            );
            self.push_output(self.status_line.clone());
        }

        self.refresh_placeable_objects_if_needed(force);
    }

    pub(super) fn new_world(&mut self) {
        self.world = create_empty_world();
        self.ensure_world_runtime_registry();
        self.set_primary_selection(self.first_object_id());
        if let Some(session) = self.project_session.as_mut() {
            session.current_world_path = None;
        }
        self.status_line = "Created a new world.".to_string();
    }

    pub(super) fn open_project_dialog(&mut self) {
        if let Some(path) = FileDialog::new()
            .add_filter("Runvy Project", &["runvyproj"])
            .pick_file()
        {
            self.begin_load_project(path);
        }
    }

    pub(super) fn begin_load_project(&mut self, path: PathBuf) {
        println!("Begin load project");
        let output_tx = self.output_tx.clone();
        let (tx, rx) = mpsc::channel();
        self.project_load = Some(rx);
        self.status_line = format!("Opening project {}...", path.display());
        self.push_output(format!("Opening project {}...", path.display()));

        std::thread::spawn(move || {
            let _ = output_tx.send(format!("Loading project manifest: {}", path.display()));
            let result = (|| -> Result<ProjectLoadResult, String> {
                let project = load_project(&path).map_err(|error| error.to_string())?;
                ensure_editor_bridge_files(&project.root_dir).map_err(|error| error.to_string())?;
                let metadata = placeables::query_project_metadata(&project, &output_tx)?;
                println!("Project Metadata loaded!");

                Ok(ProjectLoadResult { project, metadata })
            })();
            let _ = tx.send(result);
        });
    }

    pub(super) fn apply_loaded_project(&mut self, result: ProjectLoadResult) {
        let editor_version = env!("CARGO_PKG_VERSION").to_string();
        let mut project = result.project.clone();
        if project.manifest.engine_version != editor_version {
            project.manifest.engine_version = editor_version.clone();
            let _ = project.save_manifest();
        }
        let startup_world_path = result.project.startup_world_path();
        self.project_session = Some(ProjectSession {
            current_world_path: Some(startup_world_path.clone()),
            project: project.clone(),
        });

        let runtime_registry = self.runtime_engine.runtime_registry().clone();
        let world = if startup_world_path.exists() {
            match load_world_with_runtime_registry(&startup_world_path, &runtime_registry) {
                Ok(world) => world,
                Err(error) => {
                    self.push_output(format!(
                        "Failed to load startup world: {error}. Using an empty world instead."
                    ));
                    create_empty_world()
                }
            }
        } else {
            create_empty_world()
        };
        self.world = world;
        self.ensure_world_runtime_registry();
        self.world.borrow_mut().start(self.world.clone());
        self.log_file = std::fs::File::create(project.root_dir.join("editor.log")).ok();
        self.set_primary_selection(self.first_object_id());
        self.content_browser
            .set_project_root(project.root_dir.clone(), &self.settings);
        let merged_records =
            placeables::merge_placeable_object_records(result.metadata.object_records);
        self.place_object.objects = merged_records
            .iter()
            .map(|record| record.descriptor.clone())
            .collect();
        self.place_object.templates = merged_records
            .into_iter()
            .map(|record| (record.descriptor.id.clone(), record.object))
            .collect();
        self.place_object.registered_types = result.metadata.registered_types;
        self.sync_world_serialized_type_metadata();
        self.place_object.source_stamp = None;
        self.settings
            .remember_project(project.manifest_path.clone(), project.manifest.name.clone());
        let _ = self.settings.save();
        self.project_version_prompt = None;
        self.status_line = format!("Opened project {}.", project.manifest.name);
        self.push_output(self.status_line.clone());
    }

    pub(super) fn open_world_dialog(&mut self) {
        let start_dir = self
            .project_session
            .as_ref()
            .map(|session| session.project.worlds_dir())
            .unwrap_or_else(helpers::default_browse_root);
        if let Some(path) = FileDialog::new()
            .set_directory(start_dir)
            .add_filter("Runvy World", &["ron"])
            .pick_file()
        {
            self.open_world_from_path(path);
        }
    }

    pub(super) fn open_world_from_path(&mut self, path: PathBuf) {
        let runtime_registry = self.runtime_engine.runtime_registry().clone();
        match load_world_with_runtime_registry(&path, &runtime_registry) {
            Ok(world) => {
                self.world = world;
                self.ensure_world_runtime_registry();
                self.set_primary_selection(self.first_object_id());
                if let Some(session) = self.project_session.as_mut() {
                    session.current_world_path = Some(path.clone());
                }
                self.status_line = format!("Opened world {}.", path.display());
            }
            Err(error) => {
                self.status_line = format!("Failed to open world: {error}");
            }
        }
    }

    pub(super) fn save_current_world(&mut self) {
        if let Some(path) = self.current_world_path() {
            self.save_world_to_path(path);
        } else {
            self.save_world_as_dialog();
        }
    }

    fn save_current_world_for_play_traced(&mut self, project_root: &std::path::Path) -> bool {
        self.play_launch_trace(project_root, "save_current_world: started");
        let Some(path) = self.current_world_path() else {
            self.status_line = "Save the world before starting Play mode.".to_string();
            self.play_launch_trace(
                project_root,
                "save_current_world: failed, no current world path",
            );
            return false;
        };
        self.play_launch_trace(
            project_root,
            format!("save_current_world: resolved path {}", path.display()),
        );

        match self.save_world_data_to_path_traced(path, Some(project_root)) {
            Ok(()) => {
                self.play_launch_trace(project_root, "save_current_world: completed");
                true
            }
            Err(error) => {
                self.status_line = format!("Failed to save world before Play mode: {error}");
                self.play_launch_trace(
                    project_root,
                    format!("save_current_world: failed, {error}"),
                );
                false
            }
        }
    }

    pub(super) fn save_world_as_dialog(&mut self) {
        let suggested_dir = self
            .project_session
            .as_ref()
            .map(|session| session.project.worlds_dir())
            .unwrap_or_else(helpers::default_browse_root);
        if let Some(path) = FileDialog::new()
            .set_directory(suggested_dir)
            .set_file_name("main.world.ron")
            .add_filter("Runvy World", &["ron"])
            .save_file()
        {
            self.save_world_to_path(helpers::ensure_world_extension(path));
        }
    }

    pub(super) fn save_world_to_path(&mut self, path: PathBuf) {
        match self.save_world_data_to_path(path.clone()) {
            Ok(()) => {
                if let Err(error) = self.save_project_preview() {
                    self.push_output(format!("Project preview update skipped: {error}"));
                }
                self.status_line = format!("Saved world to {}.", path.display());
            }
            Err(error) => {
                self.status_line = format!("Failed to save world: {error}");
            }
        }
    }

    fn save_world_data_to_path(&mut self, path: PathBuf) -> Result<(), String> {
        self.save_world_data_to_path_traced(path, None)
    }

    fn save_world_data_to_path_traced(
        &mut self,
        path: PathBuf,
        play_log_project_root: Option<&std::path::Path>,
    ) -> Result<(), String> {
        if let Some(project_root) = play_log_project_root {
            self.play_launch_trace(
                project_root,
                "save_world_data: refresh_object_world_ptrs started",
            );
        }
        self.world.borrow_mut().refresh_object_world_ptrs();
        if let Some(project_root) = play_log_project_root {
            self.play_launch_trace(
                project_root,
                "save_world_data: refresh_object_world_ptrs completed",
            );
        }
        if let Some(project_root) = play_log_project_root {
            self.play_launch_trace(project_root, "save_world_data: repair_hierarchy started");
        }
        self.world.borrow_mut().repair_hierarchy();
        if let Some(project_root) = play_log_project_root {
            let object_count = self.world.borrow().query::<Transform>().len();
            self.play_launch_trace(
                project_root,
                format!("save_world_data: repair_hierarchy completed, objects={object_count}"),
            );
            self.play_launch_trace(
                project_root,
                format!(
                    "save_world_data: save_world started, path={}",
                    path.display()
                ),
            );
        }
        save_world(&path, &self.world.borrow()).map_err(|error| error.to_string())?;
        if let Some(project_root) = play_log_project_root {
            self.play_launch_trace(project_root, "save_world_data: save_world completed");
        }
        let startup_world = self.project_session.as_ref().and_then(|session| {
            path.strip_prefix(&session.project.root_dir)
                .ok()
                .map(|relative| relative.to_string_lossy().replace('\\', "/"))
        });

        if let Some(startup_world) = startup_world.as_ref() {
            if let Some(project_root) = play_log_project_root {
                self.play_launch_trace(
                    project_root,
                    format!(
                        "save_world_data: manifest startup_world update started, value={startup_world}"
                    ),
                );
            }
        } else if let Some(project_root) = play_log_project_root {
            self.play_launch_trace(
                project_root,
                "save_world_data: manifest startup_world update skipped, path is outside project root",
            );
        }

        let manifest_update_result = if let Some(session) = self.project_session.as_mut() {
            session.current_world_path = Some(path.clone());
            if let Some(startup_world) = startup_world.as_ref() {
                session.project.manifest.startup_world = startup_world.clone();
                session.project.save_manifest().map_err(|error| {
                    format!("world saved but failed to update startup world: {error}")
                })
            } else {
                Ok(())
            }
        } else {
            Ok(())
        };
        manifest_update_result?;

        if startup_world.is_some() {
            if let Some(project_root) = play_log_project_root {
                self.play_launch_trace(
                    project_root,
                    "save_world_data: manifest startup_world update completed",
                );
            }
        }
        Ok(())
    }

    pub(super) fn play_project(&mut self) {
        let Some(session) = self.project_session.as_ref().cloned() else {
            self.status_line = "Open a project before starting Play mode.".to_string();
            return;
        };
        let project_root = session.project.root_dir.clone();
        self.reset_play_launch_log(&project_root);
        self.play_launch_trace(
            &project_root,
            format!(
                "play_project: clicked, project={}, root={}, binary={}, world={}",
                session.project.manifest.name,
                project_root.display(),
                session.project.manifest.binary_name,
                session
                    .current_world_path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<none>".to_string())
            ),
        );

        if self.runtime_process.is_some() {
            self.play_launch_trace(&project_root, "stop_existing_runtime: started");
            self.stop_project();
            self.play_launch_trace(&project_root, "stop_existing_runtime: completed");
        } else {
            self.play_launch_trace(&project_root, "stop_existing_runtime: skipped, no process");
        }

        self.play_launch_trace(&project_root, "ensure_editor_bridge_files: started");
        if let Err(error) = ensure_editor_bridge_files(&session.project.root_dir) {
            self.status_line = format!("Failed to refresh project runtime bootstrap: {error}");
            self.play_launch_trace(
                &project_root,
                format!("ensure_editor_bridge_files: failed, {error}"),
            );
            return;
        }
        self.play_launch_trace(&project_root, "ensure_editor_bridge_files: completed");

        if !self.save_current_world_for_play_traced(&project_root) {
            self.play_launch_trace(&project_root, "play_project: aborted before command spawn");
            return;
        }

        self.play_launch_trace(&project_root, "command_setup: started");
        let mut command = Command::new("cargo");
        command
            .args(["run", "--bin", &session.project.manifest.binary_name])
            .current_dir(&session.project.root_dir)
            // Play mode should not keep live cargo output pipes attached to the editor UI loop.
            // This avoids native crashes in pipe-reader callbacks while the game process starts.
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        self.play_launch_trace(
            &project_root,
            format!(
                "command_setup: completed, command=cargo run --bin {}, cwd={}",
                session.project.manifest.binary_name,
                session.project.root_dir.display()
            ),
        );
        self.play_launch_trace(&project_root, "configure_background_command: started");
        placeables::configure_background_command(&mut command);
        self.play_launch_trace(&project_root, "configure_background_command: completed");

        self.play_launch_trace(&project_root, "command_spawn: started");
        match command.spawn() {
            Ok(child) => {
                let child_id = child.id();
                self.play_launch_trace(
                    &project_root,
                    format!("command_spawn: completed, pid={child_id}"),
                );
                self.runtime_process = Some(child);
                self.play_launch_trace(&project_root, "runtime_process_store: completed");
                self.status_line =
                    format!("Started Play mode for {}.", session.project.manifest.name);
                self.play_launch_trace(
                    &project_root,
                    format!("play_project: completed, {}", self.status_line),
                );
            }
            Err(error) => {
                self.status_line = format!("Failed to start Play mode: {error}");
                self.play_launch_trace(&project_root, format!("command_spawn: failed, {error}"));
            }
        }
    }

    pub(super) fn stop_project(&mut self) {
        let Some(mut child) = self.runtime_process.take() else {
            return;
        };

        match child.kill() {
            Ok(()) => {
                let _ = child.wait();
                self.status_line = "Stopped Play mode.".to_string();
                self.push_output(self.status_line.clone());
            }
            Err(error) => {
                self.status_line = format!("Failed to stop Play mode: {error}");
                self.push_output(self.status_line.clone());
            }
        }
    }

    pub(super) fn update_runtime_process_state(&mut self) {
        let Some(child) = self.runtime_process.as_mut() else {
            return;
        };

        match child.try_wait() {
            Ok(Some(status)) => {
                self.runtime_process = None;
                self.status_line = format!("Play mode exited with status {status}.");
                self.push_output(self.status_line.clone());
            }
            Ok(None) => {}
            Err(error) => {
                self.runtime_process = None;
                self.status_line = format!("Failed to poll Play mode: {error}");
                self.push_output(self.status_line.clone());
            }
        }
    }

    pub(super) fn build_project(&mut self) {
        let Some(session) = self.project_session.as_ref().cloned() else {
            self.status_line = "Open a project before building.".to_string();
            return;
        };

        if self.build_process.is_some() {
            self.status_line = "Build is already running.".to_string();
            return;
        }

        if let Err(error) = ensure_editor_bridge_files(&session.project.root_dir) {
            self.status_line = format!("Failed to refresh project runtime bootstrap: {error}");
            self.push_output(self.status_line.clone());
            return;
        }

        if let Err(error) = ensure_release_windows_subsystem(
            &session.project.root_dir,
            session
                .project
                .manifest
                .build
                .hide_console_window_on_windows,
        ) {
            self.status_line = format!("Failed to prepare release main.rs: {error}");
            self.push_output(self.status_line.clone());
            return;
        }

        let release = session.project.manifest.build.release;
        let profile_label = if release { "release" } else { "debug" };
        let mut command = Command::new("cargo");
        command
            .arg("build")
            .arg("--bin")
            .arg(&session.project.manifest.binary_name)
            .current_dir(&session.project.root_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if release {
            command.arg("--release");
        }
        placeables::configure_background_command(&mut command);

        match command.spawn() {
            Ok(child) => {
                self.build_process = Some(child);
                if let Some(child_ref) = self.build_process.as_mut() {
                    placeables::attach_child_output(child_ref, self.output_tx.clone(), "build");
                }
                self.status_line = format!("Started {profile_label} build.");
                self.push_output(self.status_line.clone());
            }
            Err(error) => {
                self.status_line = format!("Failed to start build: {error}");
                self.push_output(self.status_line.clone());
            }
        }
    }

    pub(super) fn update_build_process_state(&mut self) {
        let Some(child) = self.build_process.as_mut() else {
            return;
        };

        match child.try_wait() {
            Ok(Some(status)) => {
                let Some(session) = self.project_session.as_ref().cloned() else {
                    self.build_process = None;
                    return;
                };
                self.build_process = None;

                if !status.success() {
                    self.status_line = format!("Build failed with status {status}.");
                    self.push_output(self.status_line.clone());
                    return;
                }

                let profile_dir = if session.project.manifest.build.release {
                    "release"
                } else {
                    "debug"
                };
                let executable_name = if cfg!(target_os = "windows") {
                    format!("{}.exe", session.project.manifest.binary_name)
                } else {
                    session.project.manifest.binary_name.clone()
                };
                let source_binary = session
                    .project
                    .root_dir
                    .join("target")
                    .join(profile_dir)
                    .join(&executable_name);
                let output_dir = session
                    .project
                    .root_dir
                    .join(&session.project.manifest.build.output_dir);
                let destination_binary = output_dir.join(&executable_name);

                match std::fs::create_dir_all(&output_dir)
                    .and_then(|_| std::fs::copy(&source_binary, &destination_binary).map(|_| ()))
                {
                    Ok(()) => {
                        self.status_line =
                            format!("Build finished: {}", destination_binary.display());
                        self.push_output(self.status_line.clone());
                    }
                    Err(error) => {
                        self.status_line =
                            format!("Build finished but failed to copy artifact: {error}");
                        self.push_output(self.status_line.clone());
                    }
                }
            }
            Ok(None) => {}
            Err(error) => {
                self.build_process = None;
                self.status_line = format!("Failed to poll build process: {error}");
                self.push_output(self.status_line.clone());
            }
        }
    }

    pub(super) fn poll_project_load(&mut self) {
        let Some(receiver) = self.project_load.as_ref() else {
            return;
        };

        match receiver.try_recv() {
            Ok(result) => {
                self.project_load = None;
                match result {
                    Ok(result) => {
                        let editor_version = env!("CARGO_PKG_VERSION").to_string();
                        let project_version = result.project.manifest.engine_version.clone();
                        if !project_version.trim().is_empty() && project_version != editor_version {
                            let project_root = result.project.root_dir.clone();
                            let project_name = result.project.manifest.name.clone();
                            self.project_version_prompt = Some(ProjectVersionPromptState {
                                pending_result: result,
                                project_root,
                                project_name,
                                project_version,
                                editor_version,
                            });
                            self.status_line = format!(
                                "Project version differs from editor version for {}.",
                                self.project_version_prompt
                                    .as_ref()
                                    .map(|state| state.project_name.as_str())
                                    .unwrap_or("project")
                            );
                        } else {
                            self.apply_loaded_project(result);
                        }
                    }
                    Err(error) => {
                        let error_msg = format!("Failed to open project: {error}");
                        self.push_output(&error_msg);
                        self.project_load_error = Some(error_msg);
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.project_load = None;
                self.status_line = "Project loading task disconnected.".to_string();
                self.push_output(self.status_line.clone());
            }
        }
    }

    pub(super) fn poll_output(&mut self) {
        while let Ok(line) = self.output_rx.try_recv() {
            self.push_output(line);
        }
    }

    pub(super) fn push_output(&mut self, line: impl Into<String>) {
        let line = line.into();
        self.console.add_message(line.clone());
        use std::io::Write;
        if let Some(file) = &mut self.global_log_file {
            writeln!(file, "{}", line).ok();
        }
        if let Some(file) = &mut self.log_file {
            writeln!(file, "{}", line).ok();
        }
    }

    pub(super) fn current_world_path(&self) -> Option<PathBuf> {
        self.project_session
            .as_ref()
            .and_then(|session| session.current_world_path.clone())
    }

    fn reset_play_launch_log(&mut self, project_root: &std::path::Path) {
        let path = play_launch_log_path(project_root);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::File::create(&path) {
            Ok(mut file) => {
                let _ = writeln!(file, "Runvy Editor Play launch log");
                let _ = writeln!(file, "started_at_unix_ms={}", unix_time_millis());
                let _ = writeln!(file, "log_path={}", path.display());
            }
            Err(error) => {
                self.push_output(format!(
                    "[play-launch] failed to reset log {}: {error}",
                    path.display()
                ));
            }
        }
    }

    fn play_launch_trace(&mut self, project_root: &std::path::Path, message: impl Into<String>) {
        let line = format!("[play-launch] {}", message.into());
        self.push_output(line.clone());

        let path = play_launch_log_path(project_root);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = writeln!(file, "{} {}", unix_time_millis(), line);
            let _ = file.flush();
        }
    }

    pub(super) fn window_title(&self) -> String {
        let version = env!("CARGO_PKG_VERSION");
        if let Some(session) = self.project_session.as_ref() {
            format!(
                "Runvy Editor ({version}) - {}",
                session.project.manifest.name
            )
        } else {
            format!("Runvy Editor ({version})")
        }
    }

    pub(super) fn create_project_from_dialog(&mut self) {
        let name = self.project_dialog.name.trim();
        let location = self.project_dialog.location.trim();
        if name.is_empty() || location.is_empty() {
            self.project_dialog.error = Some("Project name and location are required.".to_string());
            self.push_output("Project name and location are required.");
            return;
        }

        let destination = PathBuf::from(location).join(name);
        match create_empty_project(&destination, name) {
            Ok(project) => {
                self.project_dialog.open = false;
                self.begin_load_project(project.manifest_path.clone());
                self.status_line = format!("Created project {}.", project.manifest.name);
            }
            Err(error) => {
                let error_msg = format!("Failed to create project: {error}");
                self.push_output(&error_msg);
                self.project_dialog.error = Some(error_msg);
            }
        }
    }

    fn save_project_preview(&mut self) -> Result<(), String> {
        let Some(session) = self.project_session.as_ref() else {
            return Err("No project is open.".to_string());
        };
        let Some(renderer) = self.renderer.as_ref() else {
            return Err("Renderer is not initialized.".to_string());
        };
        let Some(target) = self.viewport_target.as_ref() else {
            return Err("Viewport target is not initialized.".to_string());
        };

        let (width, height, pixels) = renderer.capture_render_target_rgba8(target)?;
        let image = image::RgbaImage::from_raw(width, height, pixels)
            .ok_or_else(|| "Failed to build preview image.".to_string())?;
        let preview =
            image::imageops::resize(&image, 64, 64, image::imageops::FilterType::Triangle);
        let preview_path = project_preview_path(&session.project.root_dir);
        if let Some(parent) = preview_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        preview
            .save(&preview_path)
            .map_err(|error| error.to_string())
    }

    pub(super) fn open_project_in_explorer(&mut self, project_root: &std::path::Path) {
        match Command::new("explorer").arg(project_root).spawn() {
            Ok(_) => {
                self.status_line = format!("Opened {} in Explorer.", project_root.display());
            }
            Err(error) => {
                self.status_line = format!("Failed to open Explorer: {error}");
            }
        }
    }

    pub(super) fn open_project_in_code_editor(&mut self) {
        let Some(session) = self.project_session.as_ref() else {
            self.status_line = "Open a project first.".to_string();
            return;
        };

        let executable = self.settings.external_editor_executable.trim();
        if executable.is_empty() {
            self.status_line = "External editor is not configured.".to_string();
            return;
        }

        let project = session.project.root_dir.to_string_lossy().to_string();
        let args: Vec<String> = self
            .settings
            .external_editor_project_args
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(|line| line.replace("{project}", &project))
            .collect();

        match Command::new(executable).args(args).spawn() {
            Ok(_) => {
                self.status_line = format!(
                    "Opened project {} in external editor.",
                    session.project.manifest.name
                );
            }
            Err(error) => {
                self.status_line = format!("Failed to open project in editor: {error}");
            }
        }
    }

    pub(super) fn create_project_backup(
        &mut self,
        project_root: &std::path::Path,
    ) -> Result<PathBuf, String> {
        let timestamp = chrono_like_timestamp()?;
        let project_name = project_root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("project");
        let backup_root = project_root
            .parent()
            .unwrap_or(project_root)
            .join(format!("{project_name}_backup_{timestamp}"));
        copy_directory_recursive(project_root, &backup_root)?;
        Ok(backup_root)
    }
}

pub(super) fn project_preview_path(project_root: &std::path::Path) -> PathBuf {
    project_root
        .join(".runvy_editor")
        .join("project-preview.png")
}

pub(super) fn play_launch_log_path(project_root: &std::path::Path) -> PathBuf {
    project_root.join(".runvy_editor").join("play-launch.log")
}

fn unix_time_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn chrono_like_timestamp() -> Result<String, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    Ok(now.to_string())
}

fn copy_directory_recursive(
    source: &std::path::Path,
    destination: &std::path::Path,
) -> Result<(), String> {
    std::fs::create_dir_all(destination).map_err(|error| error.to_string())?;
    for entry in std::fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            if source_path.file_name().and_then(|name| name.to_str()) == Some("target") {
                continue;
            }
            copy_directory_recursive(&source_path, &destination_path)?;
        } else {
            std::fs::copy(&source_path, &destination_path)
                .map(|_| ())
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub(super) fn path_relative_to_project(
    project_root: &std::path::Path,
    path: &std::path::Path,
) -> String {
    path.strip_prefix(project_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
