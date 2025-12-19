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
  const [draggingTab, setDraggingTab] = useState<null | TabId>(null);
  const [dockLayout, setDockLayout] = useState<Record<Zone, TabId[]>>({
    left: ["hierarchy", "files"],
    right: ["inspector"],
  });
  const [activeTabs, setActiveTabs] = useState<Record<Zone, TabId | null>>({
    left: "hierarchy",
    right: "inspector",
  });

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

  const TAB_CONFIG: Record<TabId, { title: string; subtitle: string }> = {
    hierarchy: { title: "Hierarchy", subtitle: "Scene graph" },
    files: { title: "File Explorer", subtitle: "Project files" },
    inspector: { title: "Inspector", subtitle: "Selected entity" },
  };

  const getActiveTabForZone = (zone: Zone) => {
    const tabFromState = activeTabs[zone];
    if (tabFromState && dockLayout[zone].includes(tabFromState)) {
      return tabFromState;
    }
    return dockLayout[zone][0] ?? null;
  };

  const moveTabToZone = (tabId: TabId, targetZone: Zone) => {
    setDockLayout(prevLayout => {
      const cleanedLayout: Record<Zone, TabId[]> = {
        left: prevLayout.left.filter(id => id !== tabId),
        right: prevLayout.right.filter(id => id !== tabId),
      };

      const nextLayout: Record<Zone, TabId[]> = {
        ...cleanedLayout,
        [targetZone]: [...cleanedLayout[targetZone], tabId],
      };

      setActiveTabs(prevActive => {
        const nextActive: Record<Zone, TabId | null> = { ...prevActive };
        (Object.keys(nextLayout) as Zone[]).forEach(zone => {
          const available = nextLayout[zone];
          if (!available.length) {
            nextActive[zone] = null;
            return;
          }
          if (!available.includes(nextActive[zone] as TabId)) {
            nextActive[zone] = available[0];
          }
        });
        nextActive[targetZone] = tabId;
        return nextActive;
      });

      return nextLayout;
    });
  };

  const renderTabContent = (tabId: TabId | null) => {
    if (!tabId) {
      return <div className="panel-empty">Drag a tab here to dock it.</div>;
    }

    switch (tabId) {
      case "hierarchy":
        return (
          <Hierarchy
            entities={entities}
            selectedEntityId={selectedEntityId}
            onEntityClick={async (id) => {
              await handleEntityClick(id);
            }}
          />
        );
      case "files":
        return <FileExplorer refreshToken={fileExplorerRefreshToken} />;
      case "inspector":
        return <Inspector selectedEntityId={selectedEntityId} refreshTrigger={inspectorRefreshTrigger} />;
      default:
        return null;
    }
  };

  const renderTabActions = (tabId: TabId | null) => {
    if (!tabId) return null;

    if (tabId === "hierarchy") {
      return (
        <div className="action-row">
          <button onClick={handleCreateEntity} disabled={isPlaying} className="ghost-button">
            + Entity
          </button>
          <button
            onClick={() => selectedEntityId !== null && handleDuplicateEntity(selectedEntityId)}
            disabled={selectedEntityId === null || isPlaying}
            className="command-button subtle"
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
      );
    }

    if (tabId === "files") {
      return (
        <button onClick={() => setFileExplorerRefreshToken((t) => t + 1)} className="ghost-button">
          Refresh
        </button>
      );
    }

    if (tabId === "inspector") {
      return <div className="pill muted">Live</div>;
    }

    return null;
  };

  const renderTabFooter = (tabId: TabId | null) => {
    if (!tabId) return null;

    if (tabId === "hierarchy") {
      return (
        <div className="panel-footer">
          <div className="pill muted">{entities.length} entities</div>
        </div>
      );
    }

    if (tabId === "files") {
      return (
        <div className="panel-footer justify-between">
          <div className="pill muted">Scenes & Assets</div>
          <button onClick={() => setFileExplorerRefreshToken((t) => t + 1)} className="command-button">
            Refresh
          </button>
        </div>
      );
    }

    return null;
  };

  const renderDockZone = (zone: Zone) => {
    const zoneTabs = dockLayout[zone];
    const activeTab = getActiveTabForZone(zone);

    return (
      <div
        className={`panel dock-panel ${draggingTab && draggingTab !== activeTab ? "droppable" : ""}`}
        onDragOver={(e) => {
          if (draggingTab) {
            e.preventDefault();
          }
        }}
        onDrop={(e) => {
          const tabId = e.dataTransfer.getData("text/tab-id") as TabId;
          if (tabId) {
            moveTabToZone(tabId, zone);
          }
          setDraggingTab(null);
        }}
        onDragLeave={() => setDraggingTab(null)}
      >
        <div className="panel-header dock-header">
          <div className="dock-header-titles">
            <div className="dock-tabs">
              {zoneTabs.map((tabId) => (
                <button
                  key={tabId}
                  className={`tab-button ${activeTab === tabId ? "active" : ""}`}
                  onClick={() => setActiveTabs(prev => ({ ...prev, [zone]: tabId }))}
                  draggable
                  onDragStart={(e) => {
                    e.dataTransfer.setData("text/tab-id", tabId);
                    e.dataTransfer.effectAllowed = "move";
                    setDraggingTab(tabId);
                  }}
                  onDragEnd={() => setDraggingTab(null)}
                  title="Drag to another dock"
                >
                  <span className="tab-label">{TAB_CONFIG[tabId].title}</span>
                </button>
              ))}
              {!zoneTabs.length && <div className="pill muted">Drop a tab to pin it here</div>}
            </div>
            {activeTab && (
              <>
                <p className="panel-title">{TAB_CONFIG[activeTab].title}</p>
                <p className="panel-subtitle">{TAB_CONFIG[activeTab].subtitle}</p>
              </>
            )}
          </div>
          <div className="dock-actions">{renderTabActions(activeTab)}</div>
        </div>
        <div className="panel-body">{renderTabContent(activeTab)}</div>
        {renderTabFooter(activeTab)}
      </div>
    );
  };

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
              <div className="brand-subtitle">Design. Iterate. Ship.</div>
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

        <div className="workspace-grid">
          {renderDockZone("left")}

          <div className="workspace-center">
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

          {renderDockZone("right")}
        </div>
      </div>
    </div>
  );
}

export default App;

type TabId = "hierarchy" | "files" | "inspector";
type Zone = "left" | "right";

