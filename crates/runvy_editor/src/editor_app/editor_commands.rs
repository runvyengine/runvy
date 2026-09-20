use super::*;

pub(super) fn execute(app: &mut EditorApp, cmd: &str, args: &[&str]) -> bool {
    // Support both direct commands and editor.* namespace
    let (actual_cmd, actual_args) = if cmd == "editor" && !args.is_empty() {
        (args[0], &args[1..])
    } else if let Some(sub) = cmd.strip_prefix("editor.") {
        (sub, args)
    } else {
        (cmd, args)
    };

    match actual_cmd {
        "save" => {
            app.save_current_world();
            true
        }
        "play" | "start" => {
            if app.runtime_process.is_some() {
                app.push_output("Already in play mode.");
            } else if app.project_session.is_none() {
                app.push_output("No project loaded. Open a project first.");
            } else {
                app.play_project();
            }
            true
        }
        "stop" => {
            if app.runtime_process.is_some() {
                app.stop_project();
            } else {
                app.push_output("Not in play mode.");
            }
            true
        }
        "build" => {
            if app.project_session.is_none() {
                app.push_output("No project loaded.");
            } else {
                app.build_project();
            }
            true
        }
        "toggle_hierarchy" => {
            app.panels.hierarchy = !app.panels.hierarchy;
            app.push_output(format!("Hierarchy panel: {}", if app.panels.hierarchy { "ON" } else { "OFF" }));
            true
        }
        "toggle_inspector" => {
            app.panels.inspector = !app.panels.inspector;
            app.push_output(format!("Inspector panel: {}", if app.panels.inspector { "ON" } else { "OFF" }));
            true
        }
        "toggle_bottom_bar" => {
            app.panels.bottom_bar = !app.panels.bottom_bar;
            app.push_output(format!("Bottom bar: {}", if app.panels.bottom_bar { "ON" } else { "OFF" }));
            true
        }
        "bottom_tab" => {
            let target = actual_args.first().copied().unwrap_or("");
            match target {
                "console" => {
                    app.bottom_tab = BottomTab::Console;
                    app.push_output("Switched to Console tab.");
                }
                "browser" | "content" => {
                    app.bottom_tab = BottomTab::ContentBrowser;
                    app.push_output("Switched to Content Browser tab.");
                }
                _ => {
                    app.push_output("Usage: bottom_tab <console|browser>");
                }
            }
            true
        }
        "list_objects" | "ls" => {
            let ids = app.world_object_ids();
            if ids.is_empty() {
                app.push_output("No objects in world.");
            } else {
                let names: Vec<(u64, String)> = {
                    let world = app.world.borrow();
                    ids.iter().filter_map(|id| {
                        world.object(*id).map(|obj| (*id, obj.name.clone()))
                    }).collect()
                };
                app.push_output(format!("Objects ({}):", names.len()));
                for (id, name) in &names {
                    app.push_output(format!("  [{}] {}", id, name));
                }
            }
            true
        }
        "select" => {
            if let Some(name) = actual_args.first() {
                let found_id = {
                    let ids = app.world_object_ids();
                    let world = app.world.borrow();
                    ids.iter().find(|id| {
                        world.object(**id).map_or(false, |o| o.name.as_str() == *name)
                    }).copied()
                };
                if let Some(id) = found_id {
                    app.set_primary_selection(Some(id));
                    app.push_output(format!("Selected [{}] {}", id, name));
                } else {
                    app.push_output(format!("No object named '{}' found.", name));
                }
            } else {
                app.push_output("Usage: select <name>");
            }
            true
        }
        "status" => {
            let obj_count = app.world_object_ids().len();
            let has_play = app.runtime_process.is_some();
            let project_info = app.project_session.as_ref().map(|s| {
                (s.project.root_dir.clone(), s.current_world_path.clone())
            });
            app.push_output(format!("Status: {}", app.status_line));
            app.push_output(format!("Objects: {}", obj_count));
            app.push_output(format!("Bottom tab: {:?}", app.bottom_tab));
            app.push_output(format!("Play mode: {}", has_play));
            if let Some((root, world_path)) = project_info {
                app.push_output(format!("Project: {:?}", root));
                app.push_output(format!("World: {:?}", world_path));
            } else {
                app.push_output("Project: none");
            }
            true
        }
        "focus" | "frame" => {
            if app.selection.is_some() {
                app.frame_selected_object();
                app.push_output("Framed selected object.");
            } else {
                app.push_output("No object selected.");
            }
            true
        }
        "delete" | "remove" => {
            if let Some(id) = app.selection {
                app.delete_object(id);
                app.push_output("Deleted selected object.");
            } else {
                app.push_output("No object selected.");
            }
            true
        }
        "duplicate" | "clone" => {
            if let Some(id) = app.selection {
                app.copy_object(id, false);
                app.paste_object(None);
                app.push_output("Duplicated selected object.");
            } else {
                app.push_output("No object selected.");
            }
            true
        }
        "view_grid" => {
            app.show_viewport_grid = !app.show_viewport_grid;
            app.push_output(format!("Viewport grid: {}", if app.show_viewport_grid { "ON" } else { "OFF" }));
            true
        }
        "view_icons" => {
            app.show_component_icons = !app.show_component_icons;
            app.push_output(format!("Component icons: {}", if app.show_component_icons { "ON" } else { "OFF" }));
            true
        }
        _ => false,
    }
}

/// Returns editor-specific command descriptions for help display
pub(super) fn descriptions() -> Vec<(&'static str, &'static str)> {
    vec![
        ("editor.save", "Save current world"),
        ("editor.play", "Start play mode"),
        ("editor.stop", "Stop play mode"),
        ("editor.build", "Build project (release)"),
        ("editor.toggle_hierarchy", "Toggle hierarchy panel"),
        ("editor.toggle_inspector", "Toggle inspector panel"),
        ("editor.toggle_bottom_bar", "Toggle bottom bar"),
        ("editor.bottom_tab", "Switch bottom tab: <console|browser>"),
        ("editor.list_objects", "List all objects in the world"),
        ("editor.select", "Select object by name"),
        ("editor.status", "Show editor status information"),
        ("editor.focus", "Frame selected object in viewport"),
        ("editor.delete", "Delete selected object"),
        ("editor.duplicate", "Duplicate selected object"),
        ("editor.view_grid", "Toggle viewport grid"),
        ("editor.view_icons", "Toggle component icons"),
    ]
}
