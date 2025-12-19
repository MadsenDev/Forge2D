import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import Inspector from "./Inspector";
import Viewport from "./Viewport";
import Hierarchy from "./Hierarchy";
import FileExplorer from "./FileExplorer";
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
  const [fileExplorerRefreshToken, setFileExplorerRefreshToken] = useState(0);

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
    <div className="unity-shell">
      <header className="unity-menu-bar">
        <div className="menu-left">
          <div className="menu-logo">Unity</div>
          <nav className="menu-items">
            {["File", "Edit", "Assets", "GameObject", "Component", "Window", "Help"].map(item => (
              <span key={item} className="menu-item">
                {item}
              </span>
            ))}
          </nav>
        </div>
        <div className="menu-right">
          <span className="menu-status">Layout: 2 by 3</span>
          <span className="menu-status">{projectName ?? "No project"}</span>
        </div>
      </header>

      <div className="unity-toolbar-row">
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

      <div className="unity-grid">
        <section className="panel unity-panel hierarchy-area">
          <header className="panel-header tight">
            <div className="panel-tabs">
              <span className="panel-tab active">Hierarchy</span>
            </div>
            <div className="panel-actions">
              <button onClick={handleCreateEntity} disabled={isPlaying} className="unity-button">
                Create
              </button>
              <button
                onClick={() => selectedEntityId !== null && handleDuplicateEntity(selectedEntityId)}
                disabled={selectedEntityId === null || isPlaying}
                className="unity-button muted"
              >
                Duplicate
              </button>
              <button
                onClick={() => selectedEntityId !== null && handleDeleteEntity(selectedEntityId)}
                disabled={selectedEntityId === null || isPlaying}
                className="unity-button danger"
              >
                Delete
              </button>
            </div>
          </header>
          <div className="panel-body muted-bg">
            <Hierarchy
              entities={entities}
              selectedEntityId={selectedEntityId}
              onEntityClick={async (id) => {
                await handleEntityClick(id);
              }}
            />
          </div>
          <footer className="panel-footer tight">
            <span className="panel-footnote">{entities.length} objects in scene</span>
          </footer>
        </section>

        <section className={`panel unity-panel scene-area ${isPlaying ? "playing" : ""}`}>
          <header className="panel-header tight">
            <div className="panel-tabs">
              <span className="panel-tab active">Scene</span>
              <span className="panel-tab">Game</span>
            </div>
            <div className="panel-actions">
              <span className="panel-footnote">Shaded</span>
            </div>
          </header>
          <div className="scene-viewport">
            {isPlaying && <div className="mode-banner">Play Mode</div>}
            <div className="viewport-toolbar">
              <span className="viewport-pill">Tool: {currentTool}</span>
              <span className="viewport-pill">Selection: {selectedEntityId ?? "None"}</span>
              <span className="viewport-pill muted">
                Undo: {canUndo ? "Available" : "-"} / Redo: {canRedo ? "Available" : "-"}
              </span>
            </div>
            <div className="viewport-surface">
              <Viewport
                entities={entities}
                selectedEntityId={selectedEntityId}
                onEntityClick={handleEntityClick}
                onTransformChange={async () => {
                  await refreshEntities();
                  setInspectorRefreshTrigger(prev => prev + 1);
                }}
                isPlaying={isPlaying}
                tool={currentTool}
              />
            </div>
          </div>
        </section>

        <section className="panel unity-panel inspector-area">
          <header className="panel-header tight">
            <div className="panel-tabs">
              <span className="panel-tab active">Inspector</span>
              <span className="panel-tab">Services</span>
            </div>
            <div className="panel-actions">
              <span className="panel-footnote muted">Static</span>
            </div>
          </header>
          <div className="panel-body">
            <Inspector selectedEntityId={selectedEntityId} refreshTrigger={inspectorRefreshTrigger} />
          </div>
        </section>

        <section className="panel unity-panel project-area">
          <header className="panel-header tight">
            <div className="panel-tabs">
              <span className="panel-tab active">Project</span>
              <span className="panel-tab">Console</span>
              <span className="panel-tab">Animator</span>
            </div>
            <div className="panel-actions">
              <button onClick={() => setFileExplorerRefreshToken((t) => t + 1)} className="unity-button muted">
                Refresh
              </button>
            </div>
          </header>
          <div className="panel-body muted-bg">
            <FileExplorer refreshToken={fileExplorerRefreshToken} />
          </div>
        </section>

        <section className="panel unity-panel console-area">
          <header className="panel-header tight">
            <div className="panel-tabs">
              <span className="panel-tab active">Console</span>
            </div>
            <div className="panel-actions">
              <span className="panel-footnote muted">Clear</span>
            </div>
          </header>
          <div className="panel-body console-body">
            <div className="console-line">Project: {projectName ?? "No project open"}</div>
            <div className="console-line">Play state: {isPlaying ? "Running" : "Stopped"}</div>
            <div className="console-line">Selection: {selectedEntityId ?? "None"}</div>
            <div className="console-line">Layout: Unity 2 by 3</div>
          </div>
        </section>
      </div>
    </div>
  );
}

export default App;

