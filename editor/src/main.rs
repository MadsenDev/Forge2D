// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use forge2d::{
    create_scene, register_builtin_metadata, restore_scene_physics, Command, CommandHistory,
    ComponentMetadataRegistry, PhysicsWorld, World,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

// Project configuration
#[derive(Serialize, Deserialize, Clone, Debug)]
struct ProjectConfig {
    name: String,
    version: String,
    created_at: String,
    // Future: engine version, settings, etc.
}

// Editor state
struct EditorState {
    world: World,
    physics: PhysicsWorld,
    command_history: CommandHistory,
    metadata_registry: ComponentMetadataRegistry,
    scene_dirty: bool,
    is_playing: bool,
    play_snapshot: Option<forge2d::Scene>, // Snapshot taken before play mode
    play_snapshot_entities: Option<
        Vec<(
            forge2d::EntityId,
            forge2d::entities::Transform,
            Option<forge2d::entities::SpriteComponent>,
            Option<forge2d::entities::PhysicsBody>,
        )>,
    >, // Snapshot of entities and components
    play_snapshot_texture_paths: Option<std::collections::HashMap<u32, String>>, // Snapshot of texture paths
    // Texture registry: maps entity ID -> texture file path (for sprites)
    entity_texture_paths: std::collections::HashMap<u32, String>,
    // Project management
    project_path: Option<PathBuf>,
    project_config: Option<ProjectConfig>,
}

impl EditorState {
    fn new() -> Self {
        let mut registry = ComponentMetadataRegistry::new();
        register_builtin_metadata(&mut registry);

        Self {
            world: World::new(),
            physics: PhysicsWorld::new(),
            command_history: CommandHistory::default(),
            metadata_registry: registry,
            scene_dirty: false,
            is_playing: false,
            play_snapshot: None,
            play_snapshot_entities: None,
            play_snapshot_texture_paths: None,
            entity_texture_paths: std::collections::HashMap::new(),
            project_path: None,
            project_config: None,
        }
    }
}

// Global state (in a real app, you'd use proper state management)
static mut EDITOR_STATE: Option<EditorState> = None;

fn get_state() -> &'static mut EditorState {
    unsafe {
        if EDITOR_STATE.is_none() {
            EDITOR_STATE = Some(EditorState::new());
        }
        EDITOR_STATE.as_mut().unwrap()
    }
}

// Helper to find entity by ID (since EntityId constructor is private)
fn find_entity_by_id(state: &EditorState, entity_id: u32) -> Option<forge2d::EntityId> {
    // Query all entities with Transform (most common case)
    for (eid, _) in state.world.query::<forge2d::entities::Transform>() {
        if eid.to_u32() == entity_id {
            return Some(eid);
        }
    }
    // TODO: Also check entities without Transform
    // For now, we only support entities with Transform
    None
}

// IPC Commands

#[derive(Serialize, Deserialize)]
struct EntityInfo {
    id: u32,
    has_transform: bool,
    has_sprite: bool,
    has_physics: bool,
    parent_id: Option<u32>,
    children: Vec<u32>,
}

#[derive(Serialize, Deserialize)]
struct FileNode {
    name: String,
    path: String,
    is_dir: bool,
    children: Vec<FileNode>,
}

#[derive(Serialize, Deserialize)]
struct ProjectFileTree {
    scenes: FileNode,
    assets: FileNode,
}

fn build_file_tree(path: &Path, depth: usize) -> Result<FileNode, String> {
    let metadata = fs::metadata(path)
        .map_err(|e| format!("Failed to read metadata for {}: {}", path.display(), e))?;
    let is_dir = metadata.is_dir();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    let mut node = FileNode {
        name,
        path: path.to_string_lossy().to_string(),
        is_dir,
        children: Vec::new(),
    };

    if is_dir && depth > 0 {
        let mut entries: Vec<PathBuf> = fs::read_dir(path)
            .map_err(|e| format!("Failed to read directory {}: {}", path.display(), e))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| {
                !p.file_name()
                    .map(|n| n.to_string_lossy().starts_with('.'))
                    .unwrap_or(false)
            })
            .collect();

        entries.sort_by(|a, b| {
            let a_dir = a.is_dir();
            let b_dir = b.is_dir();
            match (a_dir, b_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.file_name().cmp(&b.file_name()),
            }
        });

        for entry_path in entries {
            let child = build_file_tree(&entry_path, depth - 1)?;
            node.children.push(child);
        }
    }

    Ok(node)
}

#[tauri::command]
fn entities_list() -> Vec<EntityInfo> {
    let state = get_state();
    let mut entities = Vec::new();

    for (entity_id, transform) in state.world.query::<forge2d::entities::Transform>() {
        let id = entity_id.to_u32();
        let has_transform = true;
        let has_sprite = state
            .world
            .get::<forge2d::entities::SpriteComponent>(entity_id)
            .is_some();
        let has_physics = state
            .world
            .get::<forge2d::entities::PhysicsBody>(entity_id)
            .is_some();
        let parent_id = transform.parent.map(|e| e.to_u32());
        let children = forge2d::hierarchy::get_children(&state.world, entity_id)
            .iter()
            .map(|e| e.to_u32())
            .collect();

        entities.push(EntityInfo {
            id,
            has_transform,
            has_sprite,
            has_physics,
            parent_id,
            children,
        });
    }

    entities
}

#[tauri::command]
fn entity_delete(entity_id: u32) -> Result<(), String> {
    let state = get_state();
    if state.is_playing {
        return Err("Cannot delete entities in play mode".to_string());
    }

    let entity =
        find_entity_by_id(state, entity_id).ok_or_else(|| "Entity not found".to_string())?;

    let cmd = forge2d::DeleteEntity::new(entity);
    state
        .command_history
        .execute(Box::new(cmd), &mut state.world)
        .map_err(|e| e.to_string())?;

    state.scene_dirty = true;
    Ok(())
}

#[tauri::command]
fn entity_duplicate(entity_id: u32) -> Result<u32, String> {
    let state = get_state();
    let source_entity =
        find_entity_by_id(state, entity_id).ok_or_else(|| "Entity not found".to_string())?;

    // Create new entity
    let mut cmd = Box::new(forge2d::CreateEntity::new());
    cmd.execute(&mut state.world)
        .map_err(|e| format!("Failed to create entity: {}", e))?;
    let new_entity_id = cmd
        .entity()
        .ok_or_else(|| "Entity ID not available after creation".to_string())?;

    // Copy Transform component if it exists
    if let Some(transform) = state
        .world
        .get::<forge2d::entities::Transform>(source_entity)
    {
        let mut new_transform = transform.clone();
        // Offset position slightly so it's visible
        new_transform.position.x += 50.0;
        new_transform.position.y += 50.0;
        state.world.insert(new_entity_id, new_transform);
    }

    // Copy SpriteComponent if it exists
    if let Some(sprite) = state
        .world
        .get::<forge2d::entities::SpriteComponent>(source_entity)
    {
        state.world.insert(new_entity_id, sprite.clone());
    }

    // Copy PhysicsBody if it exists
    if let Some(physics) = state
        .world
        .get::<forge2d::entities::PhysicsBody>(source_entity)
    {
        state.world.insert(new_entity_id, *physics);
    }

    // Add command to history
    state
        .command_history
        .execute(cmd, &mut state.world)
        .map_err(|e| format!("Failed to add command to history: {}", e))?;

    state.scene_dirty = true;
    Ok(new_entity_id.to_u32())
}

#[tauri::command]
fn entity_create() -> Result<u32, String> {
    let state = get_state();
    if state.is_playing {
        return Err("Cannot create entities in play mode".to_string());
    }
    let state = get_state();

    // Create entity via command
    let mut cmd = Box::new(forge2d::CreateEntity::new());

    // Execute the command first to get the entity ID
    cmd.execute(&mut state.world)
        .map_err(|e| format!("Failed to create entity: {}", e))?;

    let entity_id = cmd
        .entity()
        .ok_or_else(|| "Entity ID not available after creation".to_string())?;

    // Add a Transform component so the entity shows up in the list
    // This should also be done via command, but for now we'll do it directly
    state.world.insert(
        entity_id,
        forge2d::entities::Transform::new(forge2d::Vec2::ZERO),
    );

    // Now add the command to history (it's already executed, so this won't execute again)
    // Actually, the history will execute it again, but CreateEntity is idempotent
    state
        .command_history
        .execute(cmd, &mut state.world)
        .map_err(|e| format!("Failed to add command to history: {}", e))?;

    state.scene_dirty = true;
    Ok(entity_id.to_u32())
}

#[tauri::command]
fn undo() -> Result<(), String> {
    let state = get_state();
    state
        .command_history
        .undo(&mut state.world)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn redo() -> Result<(), String> {
    let state = get_state();
    state
        .command_history
        .redo(&mut state.world)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn can_undo() -> bool {
    let state = get_state();
    state.command_history.can_undo()
}

#[tauri::command]
fn can_redo() -> bool {
    let state = get_state();
    state.command_history.can_redo()
}

// Selection management
static mut SELECTED_ENTITIES: Vec<u32> = Vec::new();

#[tauri::command]
fn selection_get() -> Vec<u32> {
    unsafe { SELECTED_ENTITIES.clone() }
}

#[tauri::command]
fn selection_set(ids: Vec<u32>) {
    unsafe {
        SELECTED_ENTITIES = ids;
    }
}

#[tauri::command]
fn selection_add(id: u32) {
    unsafe {
        if !SELECTED_ENTITIES.contains(&id) {
            SELECTED_ENTITIES.push(id);
        }
    }
}

#[tauri::command]
fn selection_clear() {
    unsafe {
        SELECTED_ENTITIES.clear();
    }
}

// Transform operations
#[derive(Serialize, Deserialize)]
struct TransformData {
    position: [f32; 2],
    rotation: f32,
    scale: [f32; 2],
}

#[tauri::command]
fn transform_get(entity_id: u32) -> Option<TransformData> {
    let state = get_state();
    if let Some(entity) = find_entity_by_id(state, entity_id) {
        if let Some(transform) = state.world.get::<forge2d::entities::Transform>(entity) {
            return Some(TransformData {
                position: [transform.position.x, transform.position.y],
                rotation: transform.rotation,
                scale: [transform.scale.x, transform.scale.y],
            });
        }
    }
    None
}

#[derive(Serialize, Deserialize)]
struct SpriteData {
    texture_handle: u32,
    texture_path: Option<String>,   // Path to texture file
    texture_size: Option<[u32; 2]>, // Width, height
    tint: [f32; 4],
    sprite_scale: [f32; 2], // Scale from sprite.transform
}

#[tauri::command]
fn sprite_get(entity_id: u32) -> Option<SpriteData> {
    let state = get_state();
    let entity = find_entity_by_id(state, entity_id)?;
    let sprite_comp = state
        .world
        .get::<forge2d::entities::SpriteComponent>(entity)?;

    // Get texture path for this entity
    let texture_path = state.entity_texture_paths.get(&entity_id).cloned();

    Some(SpriteData {
        texture_handle: 0, // Not used in editor
        texture_path,
        texture_size: None, // Will be determined from loaded image
        tint: sprite_comp.sprite.tint,
        sprite_scale: [
            sprite_comp.sprite.transform.scale.x,
            sprite_comp.sprite.transform.scale.y,
        ],
    })
}

// Set texture path for an entity (called when sprite is created/updated)
#[tauri::command]
fn sprite_set_texture_path(entity_id: u32, path: String) -> Result<(), String> {
    let state = get_state();
    let entity =
        find_entity_by_id(state, entity_id).ok_or_else(|| "Entity not found".to_string())?;

    // Verify entity has SpriteComponent
    if state
        .world
        .get::<forge2d::entities::SpriteComponent>(entity)
        .is_none()
    {
        return Err("Entity does not have SpriteComponent".to_string());
    }

    state.entity_texture_paths.insert(entity_id, path);
    state.scene_dirty = true;
    Ok(())
}

#[tauri::command]
fn transform_set(
    entity_id: u32,
    position: [f32; 2],
    rotation: f32,
    scale: [f32; 2],
) -> Result<(), String> {
    println!(
        "Received transform_set: entity_id={}, position=[{}, {}], rotation={}, scale=[{}, {}]",
        entity_id, position[0], position[1], rotation, scale[0], scale[1]
    );
    let state = get_state();
    let entity =
        find_entity_by_id(state, entity_id).ok_or_else(|| "Entity not found".to_string())?;

    let cmd = forge2d::SetTransform::new(
        entity,
        forge2d::Vec2::new(position[0], position[1]),
        rotation,
        forge2d::Vec2::new(scale[0], scale[1]),
    );

    state
        .command_history
        .execute(Box::new(cmd), &mut state.world)
        .map_err(|e| e.to_string())?;

    // Update physics body if it exists (only in edit mode)
    if !state.is_playing {
        if state
            .world
            .get::<forge2d::entities::PhysicsBody>(entity)
            .is_some()
        {
            // Get the updated transform
            if let Some(transform) = state.world.get::<forge2d::entities::Transform>(entity) {
                state.physics.set_body_position(entity, transform.position);
                state.physics.set_body_rotation(entity, transform.rotation);
            }
        }
    }

    state.scene_dirty = true;
    Ok(())
}

// Component metadata
#[derive(Serialize, Deserialize)]
struct ComponentFieldInfo {
    name: String,
    type_name: String,
    value: serde_json::Value,
}

#[tauri::command]
fn component_fields(entity_id: u32, component_type: String) -> Option<Vec<ComponentFieldInfo>> {
    let state = get_state();
    let entity = find_entity_by_id(state, entity_id)?;
    let handler = state.metadata_registry.get(&component_type)?;
    let fields = handler.fields();

    Some(
        fields
            .into_iter()
            .map(|field| {
                let value = handler
                    .get_field(&state.world, entity, &field.name)
                    .unwrap_or(serde_json::Value::Null);

                ComponentFieldInfo {
                    name: field.name,
                    type_name: field.type_name,
                    value,
                }
            })
            .collect(),
    )
}

#[tauri::command]
fn component_set_field(
    entity_id: u32,
    component_type: String,
    field_name: String,
    value: serde_json::Value,
) -> Result<(), String> {
    let state = get_state();
    let entity =
        find_entity_by_id(state, entity_id).ok_or_else(|| "Entity not found".to_string())?;

    let handler = state
        .metadata_registry
        .get(&component_type)
        .ok_or_else(|| "Component type not found".to_string())?;

    handler
        .set_field(&mut state.world, entity, &field_name, value)
        .map_err(|e| e.to_string())?;
    state.scene_dirty = true;
    Ok(())
}

#[tauri::command]
fn component_types() -> Vec<String> {
    let state = get_state();
    state.metadata_registry.type_names()
}

// Project operations
#[derive(Serialize, Deserialize)]
struct ProjectInfo {
    name: String,
    path: String,
    version: String,
}

#[tauri::command]
fn project_create(name: String) -> Result<(), String> {
    // Get Documents folder path
    let documents_path =
        dirs::document_dir().ok_or_else(|| "Could not find Documents folder".to_string())?;

    // Create Forge2D projects folder
    let projects_folder = documents_path.join("Forge2D");
    fs::create_dir_all(&projects_folder)
        .map_err(|e| format!("Failed to create Forge2D projects folder: {}", e))?;

    // Create project folder: Documents/Forge2D/{name}
    let project_path = projects_folder.join(&name);

    // Check if project folder already exists
    if project_path.exists() {
        return Err(format!("Project '{}' already exists", name));
    }

    // Create project directory
    fs::create_dir_all(&project_path)
        .map_err(|e| format!("Failed to create project directory: {}", e))?;

    // Create subdirectories
    fs::create_dir_all(project_path.join("scenes"))
        .map_err(|e| format!("Failed to create scenes directory: {}", e))?;
    fs::create_dir_all(project_path.join("assets"))
        .map_err(|e| format!("Failed to create assets directory: {}", e))?;
    fs::create_dir_all(project_path.join("assets").join("textures"))
        .map_err(|e| format!("Failed to create textures directory: {}", e))?;

    // Create project config
    let config = ProjectConfig {
        name: name.clone(),
        version: "1.0.0".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let config_path = project_path.join("forge2d_project.json");
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize project config: {}", e))?;
    fs::write(&config_path, config_json)
        .map_err(|e| format!("Failed to write project config: {}", e))?;

    // Load the project
    project_open(project_path.to_string_lossy().to_string())
}

#[tauri::command]
fn project_open(path: String) -> Result<(), String> {
    let state = get_state();
    let project_path = PathBuf::from(&path);

    // Verify project directory exists
    if !project_path.exists() {
        return Err("Project directory does not exist".to_string());
    }

    // Load project config
    let config_path = project_path.join("forge2d_project.json");
    let config: ProjectConfig = if config_path.exists() {
        let config_json = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read project config: {}", e))?;
        serde_json::from_str(&config_json)
            .map_err(|e| format!("Failed to parse project config: {}", e))?
    } else {
        // Create default config for old projects
        ProjectConfig {
            name: project_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Untitled Project")
                .to_string(),
            version: "1.0.0".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        }
    };

    state.project_path = Some(project_path);
    state.project_config = Some(config);

    // Reset scene
    scene_new()?;

    Ok(())
}

#[tauri::command]
fn project_get_current() -> Option<ProjectInfo> {
    let state = get_state();
    state.project_path.as_ref().and_then(|path| {
        state.project_config.as_ref().map(|config| ProjectInfo {
            name: config.name.clone(),
            path: path.to_string_lossy().to_string(),
            version: config.version.clone(),
        })
    })
}

#[tauri::command]
fn project_close() -> Result<(), String> {
    let state = get_state();

    // Check if scene is dirty
    if state.scene_dirty {
        return Err("Scene has unsaved changes. Save before closing project.".to_string());
    }

    state.project_path = None;
    state.project_config = None;
    scene_new()?;

    Ok(())
}

#[tauri::command]
fn project_files_tree() -> Result<ProjectFileTree, String> {
    let state = get_state();
    let project_path = state
        .project_path
        .as_ref()
        .ok_or_else(|| "No project open".to_string())?;

    let scenes_path = project_path.join("scenes");
    let assets_path = project_path.join("assets");

    if !scenes_path.exists() {
        fs::create_dir_all(&scenes_path)
            .map_err(|e| format!("Failed to create scenes folder: {}", e))?;
    }

    if !assets_path.exists() {
        fs::create_dir_all(&assets_path)
            .map_err(|e| format!("Failed to create assets folder: {}", e))?;
    }

    let scenes = build_file_tree(&scenes_path, 6)?;
    let assets = build_file_tree(&assets_path, 6)?;

    Ok(ProjectFileTree { scenes, assets })
}

#[tauri::command]
fn project_list() -> Result<Vec<ProjectInfo>, String> {
    // Get Documents folder path
    let documents_path =
        dirs::document_dir().ok_or_else(|| "Could not find Documents folder".to_string())?;

    let projects_folder = documents_path.join("Forge2D");

    // Return empty list if folder doesn't exist
    if !projects_folder.exists() {
        return Ok(Vec::new());
    }

    let mut projects = Vec::new();

    // Read all directories in Forge2D folder
    let entries = fs::read_dir(&projects_folder)
        .map_err(|e| format!("Failed to read projects folder: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();

        // Check if it's a directory and has a project config file
        if path.is_dir() {
            let config_path = path.join("forge2d_project.json");
            if config_path.exists() {
                // Try to load project config
                if let Ok(config_json) = fs::read_to_string(&config_path) {
                    if let Ok(config) = serde_json::from_str::<ProjectConfig>(&config_json) {
                        projects.push(ProjectInfo {
                            name: config.name,
                            path: path.to_string_lossy().to_string(),
                            version: config.version,
                        });
                    }
                }
            }
        }
    }

    // Sort by name
    projects.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(projects)
}

// Scene operations
#[tauri::command]
fn scene_save(path: Option<String>) -> Result<String, String> {
    let state = get_state();
    let scene = create_scene(&state.physics);

    // TODO: Serialize entities and components manually
    // For now, we'll just save physics

    let json = serde_json::to_string_pretty(&scene).map_err(|e| e.to_string())?;

    // Determine save path
    let save_path = if let Some(p) = path {
        PathBuf::from(p)
    } else if let Some(project_path) = &state.project_path {
        // Default to scenes/scene.json in project
        project_path.join("scenes").join("scene.json")
    } else {
        return Err("No project open and no path provided".to_string());
    };

    // Ensure directory exists
    if let Some(parent) = save_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
    }

    fs::write(&save_path, json).map_err(|e| e.to_string())?;

    state.scene_dirty = false;
    Ok(save_path.to_string_lossy().to_string())
}

#[tauri::command]
fn scene_load(path: String) -> Result<(), String> {
    let state = get_state();

    let json = fs::read_to_string(&path).map_err(|e| e.to_string())?;

    let scene: forge2d::Scene = serde_json::from_str(&json).map_err(|e| e.to_string())?;

    // Clear world and physics
    state.world = World::new();
    restore_scene_physics(&mut state.physics, &scene).map_err(|e| e.to_string())?;

    // Clear command history
    state.command_history.clear();
    state.scene_dirty = false;

    // TODO: Restore entities and components from scene.entities
    // For now, we'll just restore physics

    Ok(())
}

#[tauri::command]
fn scene_new() -> Result<(), String> {
    let state = get_state();
    state.world = World::new();
    state.physics = PhysicsWorld::new();
    state.command_history.clear();
    state.scene_dirty = false;
    Ok(())
}

#[tauri::command]
fn scene_is_dirty() -> bool {
    let state = get_state();
    state.scene_dirty
}

#[tauri::command]
fn play_start() -> Result<(), String> {
    let state = get_state();
    if state.is_playing {
        return Err("Already in play mode".to_string());
    }

    // Take snapshot of current scene (physics)
    let scene = create_scene(&state.physics);
    state.play_snapshot = Some(scene);

    // Snapshot all entities and their components
    let mut entity_snapshot = Vec::new();
    for (entity_id, transform) in state.world.query::<forge2d::entities::Transform>() {
        let transform_clone = transform.clone();
        let sprite_clone = state
            .world
            .get::<forge2d::entities::SpriteComponent>(entity_id)
            .cloned();
        let physics_clone = state
            .world
            .get::<forge2d::entities::PhysicsBody>(entity_id)
            .copied();
        entity_snapshot.push((entity_id, transform_clone, sprite_clone, physics_clone));
    }
    state.play_snapshot_entities = Some(entity_snapshot);

    // Store texture paths snapshot
    state.play_snapshot_texture_paths = Some(state.entity_texture_paths.clone());

    // Enable physics simulation
    state.is_playing = true;

    Ok(())
}

#[tauri::command]
fn play_stop() -> Result<(), String> {
    let state = get_state();
    if !state.is_playing {
        return Err("Not in play mode".to_string());
    }

    // Restore snapshot
    if let Some(snapshot) = state.play_snapshot.take() {
        let entities_snapshot = state.play_snapshot_entities.take();
        let texture_paths_snapshot = state.play_snapshot_texture_paths.take();

        // Clear world and physics
        state.world = World::new();
        state.physics = PhysicsWorld::new();

        // Restore physics first
        restore_scene_physics(&mut state.physics, &snapshot)
            .map_err(|e| format!("Failed to restore scene physics: {}", e))?;

        // Restore entities and components
        // We need to preserve entity IDs for physics world mapping to work correctly
        if let Some(entities) = entities_snapshot {
            for (entity_id, transform, sprite, physics) in entities {
                // Restore entity with its original ID
                state.world.restore_entity(entity_id);

                // Insert Transform
                state.world.insert(entity_id, transform);

                // Insert SpriteComponent if it existed
                if let Some(sprite_comp) = sprite {
                    state.world.insert(entity_id, sprite_comp);
                }

                // Insert PhysicsBody if it existed
                if let Some(physics_comp) = physics {
                    state.world.insert(entity_id, physics_comp);
                }
            }
        }

        // Restore texture paths
        if let Some(texture_paths) = texture_paths_snapshot {
            state.entity_texture_paths = texture_paths;
        }

        // Clear command history after restore
        state.command_history.clear();
    }

    state.is_playing = false;
    Ok(())
}

#[tauri::command]
fn play_is_playing() -> bool {
    let state = get_state();
    state.is_playing
}

#[tauri::command]
fn play_step_physics(dt: f32) -> Result<(), String> {
    let state = get_state();
    if !state.is_playing {
        return Err("Not in play mode".to_string());
    }

    // Step physics simulation
    state.physics.step(dt);

    // Sync physics positions back to Transform components
    // Collect entity IDs first to avoid borrow checker issues
    let entity_ids: Vec<_> = state
        .world
        .query::<forge2d::entities::Transform>()
        .iter()
        .map(|(eid, _)| *eid)
        .collect();

    for entity_id in entity_ids {
        if let Some(transform) = state
            .world
            .get_mut::<forge2d::entities::Transform>(entity_id)
        {
            if let Some(pos) = state.physics.body_position(entity_id) {
                transform.position = pos;
            }
            if let Some(rot) = state.physics.body_rotation(entity_id) {
                transform.rotation = rot;
            }
        }
    }

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            project_create,
            project_open,
            project_get_current,
            project_close,
            project_files_tree,
            project_list,
            entities_list,
            entity_create,
            entity_delete,
            entity_duplicate,
            undo,
            redo,
            can_undo,
            can_redo,
            selection_get,
            selection_set,
            selection_add,
            selection_clear,
            transform_get,
            transform_set,
            sprite_get,
            sprite_set_texture_path,
            component_fields,
            component_set_field,
            component_types,
            scene_save,
            scene_load,
            scene_new,
            play_start,
            play_stop,
            play_is_playing,
            play_step_physics,
            scene_is_dirty,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
