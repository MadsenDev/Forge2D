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
    <div className="flex h-screen bg-gray-900 text-white">
      {/* Left Sidebar - Entity Hierarchy */}
      <div className="w-64 bg-gray-800 border-r border-gray-700 flex flex-col">
        <div className="p-4 border-b border-gray-700">
          <h2 className="text-lg font-semibold">Hierarchy</h2>
        </div>
        <div className="flex-1 overflow-y-auto p-2">
          <Hierarchy
            entities={entities}
            selectedEntityId={selectedEntityId}
            onEntityClick={async (id) => {
              await handleEntityClick(id);
            }}
          />
        </div>
        <div className="p-2 border-t border-gray-700">
          <button
            onClick={handleCreateEntity}
            disabled={isPlaying}
            className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded text-sm disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Create Entity
          </button>
        </div>
      </div>
      {/* Entity List View (Hidden) */}
      <div className="hidden w-64 bg-gray-800 border-r border-gray-700 flex flex-col">
        <div className="p-4 border-b border-gray-700">
          <h2 className="text-lg font-semibold">Entities</h2>
        </div>
        <div className="flex-1 overflow-y-auto p-2">
          {entities.length === 0 ? (
            <p className="text-gray-400 text-sm">No entities</p>
          ) : (
            entities.map((entity) => (
              <div
                key={entity.id}
                className={`p-2 mb-1 rounded ${
                  selectedEntityId === entity.id
                    ? "bg-blue-600 hover:bg-blue-700"
                    : "bg-gray-700 hover:bg-gray-600"
                }`}
              >
                <div
                  onClick={(e) => handleEntityClick(entity.id, e)}
                  className="cursor-pointer"
                >
                  <div className="font-mono text-sm">Entity {entity.id}</div>
                  <div className="text-xs text-gray-400 mt-1">
                    {entity.has_transform && "Transform "}
                    {entity.has_sprite && "Sprite "}
                    {entity.has_physics && "Physics"}
                  </div>
                </div>
                <div className="flex gap-1 mt-2">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDuplicateEntity(entity.id);
                    }}
                    className="flex-1 px-2 py-1 bg-gray-600 hover:bg-gray-500 rounded text-xs"
                    title="Duplicate"
                  >
                    Dup
                  </button>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDeleteEntity(entity.id);
                    }}
                    className="flex-1 px-2 py-1 bg-red-600 hover:bg-red-700 rounded text-xs"
                    title="Delete"
                  >
                    Del
                  </button>
                </div>
              </div>
            ))
          )}
        </div>
        <div className="p-2 border-t border-gray-700">
          <button
            onClick={handleCreateEntity}
            className="w-full px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded text-sm font-medium"
          >
            Create Entity
          </button>
        </div>
      </div>

      {/* Main Content Area */}
      <div className="flex-1 flex flex-col">
        {/* Project Header */}
        {projectName && (
          <div className="bg-gray-800 border-b border-gray-700 px-4 py-2 flex items-center justify-between">
            <div className="flex items-center gap-2">
              <span className="text-sm text-gray-400">Project:</span>
              <span className="text-sm font-medium text-white">{projectName}</span>
            </div>
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
              className="text-xs text-gray-400 hover:text-white px-2 py-1 rounded hover:bg-gray-700"
            >
              Close Project
            </button>
          </div>
        )}
        {/* Top Toolbar */}
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

        {/* Viewport Area */}
        <div className={`flex-1 bg-gray-950 overflow-hidden relative ${isPlaying ? "ring-2 ring-green-500" : ""}`}>
          {isPlaying && (
            <div className="absolute top-2 left-2 bg-green-600 text-white px-3 py-1 rounded text-sm font-bold z-20">
              PLAY MODE
            </div>
          )}
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
      <div className="w-64 bg-gray-800 border-l border-gray-700 flex flex-col">
        <div className="p-4 border-b border-gray-700">
          <h2 className="text-lg font-semibold">Inspector</h2>
        </div>
        <Inspector selectedEntityId={selectedEntityId} refreshTrigger={inspectorRefreshTrigger} />
      </div>
    </div>
  );
}

export default App;

