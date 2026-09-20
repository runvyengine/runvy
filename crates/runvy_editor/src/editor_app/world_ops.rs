use super::*;

impl<'window> EditorApp<'window> {
    pub(super) fn sync_world_serialized_type_metadata(&mut self) {
        let registered_types = self.place_object.registered_types.clone();
        if registered_types.is_empty() {
            return;
        }

        let object_ids = self.world_object_ids();
        for object_id in object_ids {
            let mut world = self.world.borrow_mut();
            let Some(object) = world.object_mut(object_id) else {
                continue;
            };
            let Some(storage) = object.get_component_mut::<SerializedTypeStorage>() else {
                continue;
            };

            for entry in &mut storage.entries {
                let expected_kind = match entry.kind {
                    SerializedTypeKind::Component => ProjectRegisteredTypeKind::Component,
                    SerializedTypeKind::Script => ProjectRegisteredTypeKind::Script,
                };
                let Some(metadata) = registered_types.iter().find(|metadata| {
                    metadata.kind == expected_kind
                        && (metadata.type_name == entry.type_name
                            || short_type_name(&metadata.type_name)
                                == short_type_name(&entry.type_name))
                }) else {
                    continue;
                };

                entry.type_name = metadata.type_name.clone();

                let mut merged_fields = Vec::with_capacity(metadata.default_fields.len());
                for default_field in &metadata.default_fields {
                    if let Some(existing) = entry
                        .fields
                        .iter()
                        .find(|field| field.name == default_field.name)
                    {
                        merged_fields.push(existing.clone());
                    } else {
                        merged_fields.push(default_field.clone());
                    }
                }
                entry.fields = merged_fields;
            }
        }
    }

    pub(super) fn add_registered_type_to_object(
        &mut self,
        object_id: ObjectId,
        type_id: TypeId,
    ) -> bool {
        let registry = self.runtime_registry().clone();
        let mut world = self.world.borrow_mut();
        let Some(object) = world.object_mut(object_id) else {
            return false;
        };
        registry.add_type_to_object(object, type_id)
    }

    pub(super) fn add_project_serialized_type_to_object(
        &mut self,
        object_id: ObjectId,
        metadata: &ProjectRegisteredTypeRecord,
    ) -> bool {
        let mut world = self.world.borrow_mut();
        let Some(object) = world.object_mut(object_id) else {
            return false;
        };

        let kind = match metadata.kind {
            ProjectRegisteredTypeKind::Component => SerializedTypeKind::Component,
            ProjectRegisteredTypeKind::Script => SerializedTypeKind::Script,
        };
        let mut storage = object
            .get_component::<SerializedTypeStorage>()
            .cloned()
            .unwrap_or_default();
        storage.upsert(SerializedTypeEntry {
            type_name: metadata.type_name.clone(),
            kind,
            fields: metadata.default_fields.clone(),
        });
        if object.get_component::<SerializedTypeStorage>().is_some() {
            object.remove_component_type_id(TypeId::of::<SerializedTypeStorage>());
        }
        object.add_component(storage);
        true
    }

    pub(super) fn create_empty_object(&mut self) {
        let object_id = self.world.borrow_mut().spawn(Object::new("Empty"));
        self.set_primary_selection(Some(object_id));
        self.status_line = "Created empty object.".to_string();
    }

    pub(super) fn create_empty_child_object(&mut self, parent_id: ObjectId) {
        let object_id = self.world.borrow_mut().spawn(Object::new("Empty"));
        if self
            .world
            .borrow_mut()
            .set_parent(object_id, Some(parent_id))
        {
            self.hierarchy_expanded.insert(parent_id);
        }
        self.set_primary_selection(Some(object_id));
        self.status_line = "Created child object.".to_string();
    }

    pub(super) fn create_from_archetype_menu_ui(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Create From Object Definition", |ui| {
            self.refresh_project_metadata(false);
            self.poll_place_object_refresh();

            let mut archetypes = self.runtime_registry().archetypes().metadata();
            archetypes.sort_by(|left, right| left.name().cmp(right.name()));
            let mut project_archetypes: Vec<PlaceableObjectDescriptor> = self
                .place_object
                .objects
                .iter()
                .filter(|object| object.category == "Object Definitions")
                .cloned()
                .collect();
            project_archetypes.sort_by(|left, right| left.name.cmp(&right.name));

            if archetypes.is_empty() && project_archetypes.is_empty() {
                ui.label("No registered object definitions.");
                return;
            }

            let has_runtime_archetypes = !archetypes.is_empty();
            for archetype in &archetypes {
                let name = archetype.name().to_string();
                let key = archetype.key().clone();

                if ui.button(&name).clicked() {
                    let spawned_object_id = {
                        let mut world = self.world.borrow_mut();
                        world.spawn_def_by_key(&key)
                    };

                    if let Some(object_id) = spawned_object_id {
                        self.set_primary_selection(Some(object_id));
                        self.status_line = format!("Created object from object definition {name}.");
                    } else {
                        self.status_line =
                            format!("Failed to create object from object definition {name}.");
                    }

                    ui.close();
                }
            }

            if !project_archetypes.is_empty() {
                if has_runtime_archetypes {
                    ui.separator();
                }
                for archetype in project_archetypes {
                    let name = archetype.name.clone();
                    if ui.button(&name).clicked() {
                        self.place_object(&archetype);
                        self.status_line =
                            format!("Created object from project object definition {name}.");
                        ui.close();
                    }
                }
            }
        });
    }

    pub(super) fn ensure_world_runtime_registry(&mut self) {
        if self.world.borrow().runtime_registry().is_none() {
            self.world
                .borrow_mut()
                .set_runtime_registry(Arc::new(self.runtime_engine.runtime_registry().clone()));
        }
        self.world.borrow_mut().refresh_object_world_ptrs();
    }

    pub(super) fn runtime_registry(&self) -> RuntimeRegistry {
        let world = self.world.borrow();

        if let Some(reg) = world.runtime_registry() {
            reg.clone()
        } else {
            self.runtime_engine.runtime_registry().clone()
        }
    }

    pub(super) fn copy_object(&mut self, object_id: ObjectId, cut: bool) {
        let world = self.world.borrow();
        let Some(object) = world.object(object_id) else {
            return;
        };
        self.hierarchy_clipboard = Some(ObjectClipboard {
            asset: WorldObjectAsset::from_object(object),
            cut_id: cut.then_some(object_id),
        });
        self.status_line = if cut {
            "Cut object.".to_string()
        } else {
            "Copied object.".to_string()
        };
    }

    pub(super) fn paste_object(&mut self, target_id: Option<ObjectId>) {
        let Some(clipboard) = self.hierarchy_clipboard.take() else {
            return;
        };
        let project_root = self
            .project_session
            .as_ref()
            .map(|session| session.project.root_dir.as_path());
        let mut object = clipboard.asset.clone().into_object(project_root);
        if let Some(object_id) = clipboard.asset.object_id.clone() {
            if object.get_component::<ObjectDefinitionInstance>().is_none() {
                object.add_component(ObjectDefinitionInstance::new(object_id));
            }
        }

        if let Some(cut_id) = clipboard.cut_id {
            self.world.borrow_mut().despawn(cut_id);
        }
        let new_id = self.world.borrow_mut().spawn(object);
        if let Some(target_id) = target_id {
            self.world.borrow_mut().set_parent(new_id, Some(target_id));
        }
        self.set_primary_selection(Some(new_id));
        self.status_line = "Pasted object.".to_string();
    }

    pub(super) fn delete_object(&mut self, object_id: ObjectId) {
        self.world.borrow_mut().despawn(object_id);
        self.set_primary_selection(self.first_object_id());
        self.status_line = "Deleted object.".to_string();
    }

    pub(super) fn place_object_menu_ui(&mut self, ui: &mut egui::Ui) {
        if self.project_session.is_none() {
            ui.label("Open a project to use Place Object.");
            return;
        }

        self.refresh_project_metadata(false);
        self.poll_place_object_refresh();

        if ui.button("Refresh Project Metadata").clicked() {
            self.refresh_project_metadata(true);
            ui.close();
            return;
        }

        if self.place_object.refresh_in_progress {
            ui.separator();
            ui.label("Refreshing objects...");
            return;
        }

        if self.place_object.objects.is_empty() {
            ui.separator();
            ui.label("No placeable objects found.");
            return;
        }

        ui.separator();
        let mut categories: Vec<String> = self
            .place_object
            .objects
            .iter()
            .map(|object| object.category.clone())
            .collect();
        categories.sort();
        categories.dedup();

        for category in categories {
            ui.menu_button(category.clone(), |ui| {
                let matching_objects: Vec<PlaceableObjectDescriptor> = self
                    .place_object
                    .objects
                    .iter()
                    .filter(|object| object.category == category)
                    .cloned()
                    .collect();
                for object in matching_objects {
                    if ui.button(&object.name).clicked() {
                        self.place_object(&object);
                        ui.close();
                    }
                }
            });
        }
    }

    pub(super) fn place_object(&mut self, object: &PlaceableObjectDescriptor) {
        let Some(mut asset) = self.place_object.templates.get(&object.id).cloned() else {
            self.status_line = format!("Failed to spawn object {}.", object.name);
            return;
        };

        asset.object_id = Some(object.id.clone());
        let project_root = self
            .project_session
            .as_ref()
            .map(|session| session.project.root_dir.as_path());
        let mut world_object = asset.into_object(project_root);
        if world_object
            .get_component::<ObjectDefinitionInstance>()
            .is_none()
        {
            world_object.add_component(ObjectDefinitionInstance::new(object.id.clone()));
        }

        let object_id = self.world.borrow_mut().spawn(world_object);
        self.set_primary_selection(Some(object_id));
        self.status_line = format!("Placed object {}.", object.name);
    }

    pub(super) fn refresh_placeable_objects_if_needed(&mut self, force: bool) {
        let Some(session) = self.project_session.as_ref().cloned() else {
            return;
        };

        self.poll_place_object_refresh();
        if self.place_object.refresh_in_progress {
            return;
        }
        let latest_stamp = placeables::latest_place_object_stamp(&session.project);
        if !force && latest_stamp == self.place_object.source_stamp {
            return;
        }
        if let Err(error) = ensure_editor_bridge_files(&session.project.root_dir) {
            self.status_line = format!("Failed to refresh project bridge files: {error}");
            self.push_output(self.status_line.clone());
            return;
        }
        let latest_stamp = placeables::latest_place_object_stamp(&session.project);

        let project = session.project.clone();
        let output_tx = self.output_tx.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(placeables::query_project_metadata(&project, &output_tx));
        });
        self.place_object.pending_stamp = latest_stamp;
        self.place_object.refresh_in_progress = true;
        self.place_object.refresh_result = Some(rx);
    }

    pub(super) fn poll_place_object_refresh(&mut self) {
        let Some(receiver) = self.place_object.refresh_result.as_ref() else {
            return;
        };

        match receiver.try_recv() {
            Ok(result) => {
                self.place_object.refresh_result = None;
                self.place_object.refresh_in_progress = false;
                match result {
                    Ok(metadata) => {
                        let merged_records =
                            placeables::merge_placeable_object_records(metadata.object_records);
                        self.place_object.objects = merged_records
                            .iter()
                            .map(|record| record.descriptor.clone())
                            .collect();
                        self.place_object.templates = merged_records
                            .into_iter()
                            .map(|record| (record.descriptor.id.clone(), record.object))
                            .collect();
                        self.place_object.registered_types = metadata.registered_types;
                        self.place_object.source_stamp = self.place_object.pending_stamp.take();
                        self.sync_world_serialized_type_metadata();
                        self.status_line = "Project metadata refreshed.".to_string();
                    }
                    Err(error) => {
                        self.place_object.pending_stamp = None;
                        self.status_line = format!("Failed to refresh placeable objects: {error}");
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.place_object.refresh_result = None;
                self.place_object.refresh_in_progress = false;
                self.place_object.pending_stamp = None;
                self.status_line =
                    "Object refresh failed because the background job disconnected.".to_string();
            }
        }
        }
    }

impl<'window> EditorApp<'window> {
    pub(super) fn spawn_from_asset_path(
        &mut self,
        asset_path: &std::path::Path,
        parent_id: Option<ObjectId>,
    ) -> Option<ObjectId> {
        let ext = asset_path.extension()?.to_str()?.to_lowercase();
        let file_name = asset_path
            .file_stem()
            .and_then(|s: &std::ffi::OsStr| s.to_str())
            .unwrap_or("Asset")
            .to_string();

        let project_root = self
            .project_session
            .as_ref()
            .map(|session| session.project.root_dir.as_path());

        match ext.as_str() {
            "world" | "ron" if asset_path.to_string_lossy().ends_with(".world.ron") => {
                let world_asset = match std::fs::read_to_string(asset_path) {
                    Ok(content) => ron::from_str::<runvy_project::WorldAsset>(&content).ok()?,
                    Err(_) => return None,
                };
                let mut last_id = None;
                for obj_asset in &world_asset.objects {
                    let object = obj_asset.clone().into_object(project_root);
                    let id = self.world.borrow_mut().spawn(object);
                    if let Some(pid) = parent_id {
                        self.world.borrow_mut().set_parent(id, Some(pid));
                    }
                    last_id = Some(id);
                }
                self.status_line = format!("Loaded world from {}", asset_path.display());
                last_id
            }
            "png" | "jpg" | "jpeg" | "webp" => {
                let mut object = Object::new(&file_name);
                let texture_path = asset_path
                    .strip_prefix(project_root.unwrap_or(std::path::Path::new("")))
                    .ok()
                    .and_then(|p: &std::path::Path| p.to_str())
                    .map(|s: &str| s.to_string());
                let sprite = SpriteRenderer {
                    texture: None,
                    texture_path,
                    pixels_per_unit: runvy_core::components::DEFAULT_SPRITE_PIXELS_PER_UNIT,
                    uv_rect: SpriteRenderer::FULL_UV_RECT,
                };
                object.add_component(sprite);
                let id = self.world.borrow_mut().spawn(object);
                if let Some(pid) = parent_id {
                    self.world.borrow_mut().set_parent(id, Some(pid));
                }
                self.status_line = format!("Created sprite from {}", asset_path.display());
                Some(id)
            }
            _ => {
                self.status_line = format!("Unsupported asset type: .{ext}");
                None
            }
        }
    }

    pub(super) fn handle_content_browser_drop(
        &mut self,
        _ui: &egui::Ui,
        parent_id: Option<ObjectId>,
    ) -> bool {
        let Some(asset_path) = self.content_browser.take_dragging_asset_path() else {
            return false;
        };
        self.spawn_from_asset_path(&asset_path, parent_id).is_some()
    }
}

fn short_type_name(type_name: &str) -> &str {
    type_name.rsplit("::").next().unwrap_or(type_name)
}
