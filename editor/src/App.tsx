import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import Inspector from "./Inspector";
import Viewport from "./Viewport";
import Hierarchy from "./Hierarchy";
import Toolbar, { Tool } from "./Toolbar";
import Welcome from "./Welcome";
import "./App.css";

interface EntityInfo {
  id: number;
  has_transform: boolean;
  has_sprite: boolean;
  has_physics: boolean;
  parent_id: number | null;
  children: number[];
}

function App() {
  const [hasProject, setHasProject] = useState(false);
  const [projectName, setProjectName] = useState<string | null>(null);
  const [entities, setEntities] = useState<EntityInfo[]>([]);
  const [canUndo, setCanUndo] = useState(false);
  const [canRedo, setCanRedo] = useState(false);
  const [selectedEntityId, setSelectedEntityId] = useState<number | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTool, setCurrentTool] = useState<Tool>("move");
  const [inspectorRefreshTrigger, setInspectorRefreshTrigger] = useState(0);

  // Check if project is open on mount
  useEffect(() => {
    const checkProject = async () => {
      try {
        const project = await invoke<{ name: string; path: string; version: string } | null>("project_get_current");
        if (project) {
          setProjectName(project.name);
          setHasProject(true);
        }
      } catch (e) {
        console.error("Failed to check project:", e);
      }
    };
    checkProject();
  }, []);

  const refreshEntities = async () => {
    const list = await invoke<EntityInfo[]>("entities_list");
    setEntities(list);
  };

  const refreshUndoRedo = async () => {
    setCanUndo(await invoke<boolean>("can_undo"));
    setCanRedo(await invoke<boolean>("can_redo"));
  };

  useEffect(() => {
    refreshEntities();
    refreshUndoRedo();
    loadSelection();
    checkPlayMode();
  }, []);

  const checkPlayMode = async () => {
    const playing = await invoke<boolean>("play_is_playing");
    setIsPlaying(playing);
  };

  // Play mode physics loop
  useEffect(() => {
    if (!isPlaying) return;

    let animationFrame: number;
    let lastTime = performance.now();

    const step = async (currentTime: number) => {
      const dt = (currentTime - lastTime) / 1000.0; // Convert to seconds
      lastTime = currentTime;

      try {
        await invoke("play_step_physics", { dt: Math.min(dt, 1.0 / 30.0) }); // Cap at 30fps min
        await refreshEntities();
      } catch (error) {
        console.error("Physics step error:", error);
      }

      animationFrame = requestAnimationFrame(step);
    };

    animationFrame = requestAnimationFrame(step);

    return () => {
      cancelAnimationFrame(animationFrame);
    };
  }, [isPlaying]);

  const handlePlay = async () => {
    try {
      await invoke("play_start");
      setIsPlaying(true);
    } catch (error) {
      console.error("Failed to start play mode:", error);
      alert(`Failed to start play mode: ${error}`);
    }
  };

  const handleStop = async () => {
    try {
      await invoke("play_stop");
      setIsPlaying(false);
      await refreshEntities();
    } catch (error) {
      console.error("Failed to stop play mode:", error);
      alert(`Failed to stop play mode: ${error}`);
    }
  };

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      // Ctrl+Z or Cmd+Z for undo
      if ((e.ctrlKey || e.metaKey) && e.key === "z" && !e.shiftKey) {
        e.preventDefault();
        await handleUndo();
      }
      // Ctrl+Shift+Z or Cmd+Shift+Z for redo
      if ((e.ctrlKey || e.metaKey) && e.key === "z" && e.shiftKey) {
        e.preventDefault();
        await handleRedo();
      }
      // Ctrl+Y for redo (alternative)
      if ((e.ctrlKey || e.metaKey) && e.key === "y") {
        e.preventDefault();
        await handleRedo();
      }
      // Delete key to delete selected entity
      if (e.key === "Delete" && selectedEntityId !== null) {
        e.preventDefault();
        await handleDeleteEntity(selectedEntityId);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedEntityId]);

  const loadSelection = async () => {
    const selection = await invoke<number[]>("selection_get");
    setSelectedEntityId(selection.length > 0 ? selection[0] : null);
  };

  const handleEntityClick = async (entityId: number, e?: React.MouseEvent) => {
    if (e && (e.ctrlKey || e.metaKey)) {
      // Multi-select
      await invoke("selection_add", { id: entityId });
    } else {
      // Single select
      await invoke("selection_set", { ids: [entityId] });
    }
    await loadSelection();
  };

  const handleCreateEntity = async () => {
    try {
      const entityId = await invoke<number>("entity_create");
      console.log("Created entity:", entityId);
      await refreshEntities();
      await refreshUndoRedo();
      // Select the newly created entity
      await invoke("selection_set", { ids: [entityId] });
      await loadSelection();
    } catch (error) {
      console.error("Failed to create entity:", error);
      alert(`Failed to create entity: ${error}`);
    }
  };

  const handleDeleteEntity = async (entityId: number) => {
    if (confirm(`Delete entity ${entityId}?`)) {
      try {
        await invoke("entity_delete", { entityId });
        await refreshEntities();
        await refreshUndoRedo();
        await loadSelection();
      } catch (error) {
        console.error("Failed to delete entity:", error);
        alert(`Failed to delete entity: ${error}`);
      }
    }
  };

  const handleDuplicateEntity = async (entityId: number) => {
    try {
      const newEntityId = await invoke<number>("entity_duplicate", { entityId });
      await refreshEntities();
      await refreshUndoRedo();
      await invoke("selection_set", { ids: [newEntityId] });
      await loadSelection();
    } catch (error) {
      console.error("Failed to duplicate entity:", error);
      alert(`Failed to duplicate entity: ${error}`);
    }
  };

  const handleUndo = async () => {
    await invoke("undo");
    await refreshEntities();
    await refreshUndoRedo();
  };

  const handleRedo = async () => {
    await invoke("redo");
    await refreshEntities();
    await refreshUndoRedo();
  };

  const handleNewScene = async () => {
    if (confirm("Create a new scene? All unsaved changes will be lost.")) {
      await invoke("scene_new");
      await refreshEntities();
      await loadSelection();
    }
  };

  const handleSave = async () => {
    try {
      // If project is open, save to project's scenes folder
      // Otherwise, show save dialog
      const savedPath = await invoke<string>("scene_save", { path: null });
      alert(`Scene saved successfully to: ${savedPath}`);
    } catch (e) {
      console.error("Failed to save scene:", e);
      alert(`Failed to save scene: ${e}`);
    }
  };

  const handleLoad = async () => {
    if (confirm("Load scene? All unsaved changes will be lost.")) {
      try {
        const filePath = await open({
          filters: [
            {
              name: "Scene",
              extensions: ["json"],
            },
          ],
        });

        if (filePath && typeof filePath === "string") {
          await invoke("scene_load", { path: filePath });
          await refreshEntities();
          await loadSelection();
          alert("Scene loaded successfully!");
        }
      } catch (e) {
        console.error("Failed to load scene:", e);
        alert(`Failed to load scene: ${e}`);
      }
    }
  };

  const handleProjectOpen = () => {
    setHasProject(true);
    // Refresh project name
    invoke<{ name: string; path: string; version: string } | null>("project_get_current")
      .then(project => {
        if (project) {
          setProjectName(project.name);
        }
      })
      .catch(console.error);
  };

  // Show welcome screen if no project is open
  if (!hasProject) {
    return <Welcome onProjectOpen={handleProjectOpen} />;
  }

  return (
    <div className="app-shell">
      <div className="app-ambient app-ambient-left" />
      <div className="app-ambient app-ambient-right" />
      <div className="app-frame">
        <header className="command-bar">
          <div className="brand">
            <div className="brand-mark">F</div>
            <div>
              <div className="brand-title">Forge2D Editor</div>
              <div className="brand-subtitle">Unity-inspired workflow</div>
            </div>
          </div>
          <div className="command-actions">
            <div className="command-group">
              <span className="command-label">Scene</span>
              <button onClick={handleNewScene} disabled={isPlaying} className="command-button">
                New Scene
              </button>
              <button onClick={handleSave} disabled={isPlaying} className="command-button primary">
                Save
              </button>
              <button onClick={handleLoad} disabled={isPlaying} className="command-button">
                Load
              </button>
            </div>
            <div className="command-divider" />
            <div className="command-group">
              <span className="command-label">Project</span>
              <div className="command-info">
                <div className="pill">{projectName ?? "No project loaded"}</div>
                {projectName && (
                  <button
                    onClick={async () => {
                      try {
                        await invoke("project_close");
                        setHasProject(false);
                        setProjectName(null);
                      } catch (e) {
                        alert(`Failed to close project: ${e}`);
                      }
                    }}
                    className="command-button danger"
                  >
                    Close
                  </button>
                )}
              </div>
            </div>
          </div>
        </header>

        <div className="flex-1 flex overflow-hidden px-4 pb-4 gap-4">
          {/* Left Sidebar - Entity Hierarchy */}
          <div className="panel w-72 flex flex-col">
            <div className="panel-header">
              <div>
                <p className="panel-title">Hierarchy</p>
                <p className="panel-subtitle">Scene graph</p>
              </div>
              <button
                onClick={handleCreateEntity}
                disabled={isPlaying}
                className="ghost-button"
              >
                + Entity
              </button>
            </div>
            <div className="flex-1 overflow-hidden">
              <div className="panel-body">
                <Hierarchy
                  entities={entities}
                  selectedEntityId={selectedEntityId}
                  onEntityClick={async (id) => {
                    await handleEntityClick(id);
                  }}
                />
              </div>
            </div>
            <div className="panel-footer">
              <div className="pill muted">{entities.length} entities</div>
              <button
                onClick={() => selectedEntityId !== null && handleDuplicateEntity(selectedEntityId)}
                disabled={selectedEntityId === null || isPlaying}
                className="command-button"
              >
                Duplicate
              </button>
              <button
                onClick={() => selectedEntityId !== null && handleDeleteEntity(selectedEntityId)}
                disabled={selectedEntityId === null || isPlaying}
                className="command-button danger"
              >
                Delete
              </button>
            </div>
          </div>

          {/* Main Content Area */}
          <div className="flex-1 flex flex-col gap-3">
            <div className="panel floating">
              <Toolbar
                currentTool={currentTool}
                onToolChange={setCurrentTool}
                isPlaying={isPlaying}
                onUndo={handleUndo}
                onRedo={handleRedo}
                canUndo={canUndo}
                canRedo={canRedo}
                onNewScene={handleNewScene}
                onSave={handleSave}
                onLoad={handleLoad}
                onPlay={handlePlay}
                onStop={handleStop}
              />
            </div>

            <div className={`panel viewport-panel flex-1 overflow-hidden ${isPlaying ? "ring-2 ring-green-500/70" : ""}`}>
              {isPlaying && (
                <div className="mode-banner">PLAY MODE</div>
              )}
              <div className="viewport-hud">
                <div className="pill">Tool: {currentTool}</div>
                <div className="pill">Selection: {selectedEntityId ?? "None"}</div>
                <div className="pill muted">Undo: {canUndo ? "Available" : "-"} / Redo: {canRedo ? "Available" : "-"}</div>
              </div>
              <Viewport
                entities={entities}
                selectedEntityId={selectedEntityId}
                onEntityClick={handleEntityClick}
                onTransformChange={async () => {
                  // Refresh entities to get updated positions
                  await refreshEntities();
                  // Trigger inspector refresh to show updated values
                  setInspectorRefreshTrigger(prev => prev + 1);
                }}
                isPlaying={isPlaying}
                tool={currentTool}
              />
            </div>
          </div>

          {/* Right Sidebar - Inspector */}
          <div className="panel w-80 flex flex-col">
            <div className="panel-header">
              <div>
                <p className="panel-title">Inspector</p>
                <p className="panel-subtitle">Selected entity data</p>
              </div>
              <div className="pill muted">Live</div>
            </div>
            <Inspector selectedEntityId={selectedEntityId} refreshTrigger={inspectorRefreshTrigger} />
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;

