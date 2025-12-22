import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import Inspector from "./Inspector";
import Viewport from "./Viewport";
import Hierarchy from "./Hierarchy";
import FileExplorer from "./FileExplorer";
import Toolbar, { Tool } from "./Toolbar";
import Welcome from "./Welcome";
import ResizablePanel from "./ResizablePanel";
import { TabData } from "./Tab";
import SplitterOverlay from "./SplitterOverlay";
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

  // Track selection and handlers for global shortcuts
  const selectedEntityIdRef = useRef<number | null>(null);
  const shortcutHandlersRef = useRef({
    handleUndo: async () => {},
    handleRedo: async () => {},
    handleDeleteEntity: async (_: number) => {},
  });

  // Grid column and row sizes for resizing
  const [columnSizes, setColumnSizes] = useState<number[]>([1.4, 260, 320, 340]);
  const [rowSizes, setRowSizes] = useState<number[]>([1.2, 0.9]);
  const gridRef = useRef<HTMLDivElement>(null);
  const sceneRef = useRef<HTMLDivElement>(null);
  const gameRef = useRef<HTMLDivElement>(null);
  const hierarchyRef = useRef<HTMLDivElement>(null);
  const projectRef = useRef<HTMLDivElement>(null);
  const inspectorRef = useRef<HTMLDivElement>(null);

  // Panel and tab management
  interface PanelState {
    id: string;
    tabs: TabData[];
    activeTabId: string | null;
    width?: number;
    height?: number;
  }

  const [panels, setPanels] = useState<PanelState[]>([
    {
      id: "scene",
      tabs: [{ id: "scene", label: "Scene", content: null, closable: false }],
      activeTabId: "scene",
      width: undefined,
      height: undefined,
    },
    {
      id: "game",
      tabs: [{ id: "game", label: "Game", content: null, closable: false }],
      activeTabId: "game",
      width: undefined,
      height: undefined,
    },
    {
      id: "hierarchy",
      tabs: [{ id: "hierarchy", label: "Hierarchy", content: null, closable: false }],
      activeTabId: "hierarchy",
      width: 260,
      height: undefined,
    },
    {
      id: "project",
      tabs: [
        { id: "project", label: "Project", content: null, closable: false },
      ],
      activeTabId: "project",
      width: 320,
      height: undefined,
    },
    {
      id: "inspector",
      tabs: [
        { id: "inspector", label: "Inspector", content: null, closable: false },
      ],
      activeTabId: "inspector",
      width: 340,
      height: undefined,
    },
  ]);

  const handleTabActivate = (panelId: string, tabId: string) => {
    setPanels((prev) =>
      prev.map((panel) =>
        panel.id === panelId ? { ...panel, activeTabId: tabId } : panel
      )
    );
  };

  const handleTabClose = (panelId: string, tabId: string) => {
    setPanels((prev) =>
      prev.map((panel) => {
        if (panel.id !== panelId) return panel;
        const newTabs = panel.tabs.filter((tab) => tab.id !== tabId);
        if (newTabs.length === 0) return panel; // Don't close if it's the last tab
        const newActiveTabId =
          panel.activeTabId === tabId
            ? newTabs[0]?.id || null
            : panel.activeTabId;
        return { ...panel, tabs: newTabs, activeTabId: newActiveTabId };
      })
    );
  };

  const handleTabDragStart = (e: React.DragEvent, panelId: string, tabId: string) => {
    e.dataTransfer.setData("panelId", panelId);
    e.dataTransfer.setData("tabId", tabId);
  };

  const handleTabDrop = (e: React.DragEvent, targetPanelId: string) => {
    e.preventDefault();
    const sourcePanelId = e.dataTransfer.getData("panelId");
    const tabId = e.dataTransfer.getData("tabId");

    if (!sourcePanelId || !tabId || sourcePanelId === targetPanelId) return;

    setPanels((prev) => {
      const sourcePanel = prev.find((p) => p.id === sourcePanelId);
      if (!sourcePanel) return prev;

      const tab = sourcePanel.tabs.find((t) => t.id === tabId);
      if (!tab) return prev;

      return prev.map((panel) => {
        if (panel.id === sourcePanelId) {
          // Remove tab from source panel
          const newTabs = panel.tabs.filter((t) => t.id !== tabId);
          const newActiveTabId =
            panel.activeTabId === tabId
              ? newTabs[0]?.id || null
              : panel.activeTabId;
          return { ...panel, tabs: newTabs, activeTabId: newActiveTabId };
        } else if (panel.id === targetPanelId) {
          // Add tab to target panel
          const newTabs = [...panel.tabs, tab];
          return { ...panel, tabs: newTabs, activeTabId: tabId };
        }
        return panel;
      });
    });
  };

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

  // Keep shortcut handler refs up to date
  useEffect(() => {
    selectedEntityIdRef.current = selectedEntityId;
    shortcutHandlersRef.current = {
      handleUndo,
      handleRedo,
      handleDeleteEntity,
    };
  }, [selectedEntityId, handleUndo, handleRedo, handleDeleteEntity]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      // Avoid interfering with text inputs
      const target = e.target as HTMLElement | null;
      if (target && ["INPUT", "TEXTAREA"].includes(target.tagName)) return;

      const { handleUndo, handleRedo, handleDeleteEntity } =
        shortcutHandlersRef.current;

      // Ctrl+Z or Cmd+Z for undo
      if ((e.ctrlKey || e.metaKey) && e.key === "z" && !e.shiftKey) {
        e.preventDefault();
        await handleUndo();
        return;
      }
      // Ctrl+Shift+Z or Cmd+Shift+Z for redo
      if ((e.ctrlKey || e.metaKey) && e.key === "z" && e.shiftKey) {
        e.preventDefault();
        await handleRedo();
        return;
      }
      // Ctrl+Y for redo (alternative)
      if ((e.ctrlKey || e.metaKey) && e.key === "y") {
        e.preventDefault();
        await handleRedo();
        return;
      }
      // Delete key to delete selected entity
      if (e.key === "Delete" && selectedEntityIdRef.current !== null) {
        e.preventDefault();
        await handleDeleteEntity(selectedEntityIdRef.current);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

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

      <div 
        ref={gridRef}
        className="unity-grid"
        style={{
          gridTemplateColumns: `${columnSizes[0]}fr ${columnSizes[1]}px ${columnSizes[2]}px ${columnSizes[3]}px`,
          gridTemplateRows: `${rowSizes[0]}fr ${rowSizes[1]}fr`,
        }}
      >
        <ResizablePanel
          id="scene"
          ref={sceneRef}
          tabs={panels.find((p) => p.id === "scene")?.tabs || []}
          activeTabId={panels.find((p) => p.id === "scene")?.activeTabId || null}
          onTabActivate={handleTabActivate}
          onTabClose={handleTabClose}
          onTabDragStart={handleTabDragStart}
          onTabDrop={handleTabDrop}
          headerActions={<span className="panel-footnote">Shaded</span>}
          className={`scene-area ${isPlaying ? "playing" : ""}`}
          resizable={{ horizontal: true, vertical: true }}
        >
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
        </ResizablePanel>

        <ResizablePanel
          id="game"
          ref={gameRef}
          tabs={panels.find((p) => p.id === "game")?.tabs || []}
          activeTabId={panels.find((p) => p.id === "game")?.activeTabId || null}
          onTabActivate={handleTabActivate}
          onTabClose={handleTabClose}
          onTabDragStart={handleTabDragStart}
          onTabDrop={handleTabDrop}
          headerActions={<span className="panel-footnote muted">{isPlaying ? "Live" : "Stopped"}</span>}
          className="game-area"
          mutedBg
          resizable={{ horizontal: true, vertical: true }}
        >
          <div className="game-body">
            <div className="game-preview">
              <div className="game-preview-surface">
                <div className="game-preview-frame">
                  <div className="game-overlay">Game view</div>
                </div>
              </div>
              <div className="game-status-row">
                <span className="viewport-pill">Resolution: 1920 x 1080</span>
                <span className="viewport-pill">Play Mode: {isPlaying ? "Running" : "Stopped"}</span>
              </div>
            </div>
          </div>
        </ResizablePanel>

        <ResizablePanel
          id="hierarchy"
          ref={hierarchyRef}
          tabs={panels.find((p) => p.id === "hierarchy")?.tabs || []}
          activeTabId={panels.find((p) => p.id === "hierarchy")?.activeTabId || null}
          onTabActivate={handleTabActivate}
          onTabClose={handleTabClose}
          onTabDragStart={handleTabDragStart}
          onTabDrop={handleTabDrop}
          headerActions={
            <>
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
            </>
          }
          footer={<span className="panel-footnote">{entities.length} objects in scene</span>}
          className="hierarchy-area"
          mutedBg
          resizable={{ horizontal: true, vertical: true }}
        >
          <Hierarchy
            entities={entities}
            selectedEntityId={selectedEntityId}
            onEntityClick={async (id) => {
              await handleEntityClick(id);
            }}
          />
        </ResizablePanel>

        <ResizablePanel
          id="project"
          ref={projectRef}
          tabs={panels.find((p) => p.id === "project")?.tabs || []}
          activeTabId={panels.find((p) => p.id === "project")?.activeTabId || null}
          onTabActivate={handleTabActivate}
          onTabClose={handleTabClose}
          onTabDragStart={handleTabDragStart}
          onTabDrop={handleTabDrop}
          headerActions={
            <button onClick={() => setFileExplorerRefreshToken((t) => t + 1)} className="unity-button muted">
              Refresh
            </button>
          }
          className="project-area"
          mutedBg
          resizable={{ horizontal: true, vertical: true }}
        >
          {panels.find((p) => p.id === "project")?.activeTabId === "project" && (
            <FileExplorer refreshToken={fileExplorerRefreshToken} />
          )}
          {panels.find((p) => p.id === "project")?.activeTabId === "console-tab" && (
            <div className="console-body">
              <div className="console-line">Project: {projectName ?? "No project open"}</div>
              <div className="console-line">Play state: {isPlaying ? "Running" : "Stopped"}</div>
              <div className="console-line">Selection: {selectedEntityId ?? "None"}</div>
              <div className="console-line">Layout: Unity 2 by 3</div>
            </div>
          )}
          {panels.find((p) => p.id === "project")?.activeTabId === "animator" && (
            <div className="panel-body">Animator view</div>
          )}
        </ResizablePanel>

        <ResizablePanel
          id="inspector"
          ref={inspectorRef}
          tabs={panels.find((p) => p.id === "inspector")?.tabs || []}
          activeTabId={panels.find((p) => p.id === "inspector")?.activeTabId || null}
          onTabActivate={handleTabActivate}
          onTabClose={handleTabClose}
          onTabDragStart={handleTabDragStart}
          onTabDrop={handleTabDrop}
          headerActions={<span className="panel-footnote muted">Static</span>}
          className="inspector-area"
          resizable={{ horizontal: true, vertical: true }}
        >
          <Inspector selectedEntityId={selectedEntityId} refreshTrigger={inspectorRefreshTrigger} />
        </ResizablePanel>

        <SplitterOverlay
          gridRef={gridRef}
          sceneRef={sceneRef}
          gameRef={gameRef}
          hierarchyRef={hierarchyRef}
          projectRef={projectRef}
          inspectorRef={inspectorRef}
          onResizeVertical={(delta) => {
            setRowSizes((prev) => {
              const newSizes = [...prev];
              const minSize = 0.3;
              // Resize first row (scene) and second row (game)
              const newFirst = Math.max(minSize, newSizes[0] + delta / 100);
              const totalHeight = newSizes[0] + newSizes[1];
              const newSecond = totalHeight - newFirst;
              if (newSecond < minSize) return prev;
              return [newFirst, Math.max(minSize, newSecond)];
            });
          }}
          onResizeHorizontal1={(delta) => {
            setColumnSizes((prev) => {
              const newSizes = [...prev];
              const minSize = 150;
              // Resize scene/game (column 0, fr) and hierarchy (column 1, px)
              const newHierarchy = Math.max(minSize, newSizes[1] - delta);
              if (newHierarchy === newSizes[1]) return prev; // Hit minimum
              // Adjust scene flex proportionally
              const flexAdjustment = delta / 100;
              const newSceneFlex = Math.max(0.5, newSizes[0] + flexAdjustment);
              return [newSceneFlex, newHierarchy, newSizes[2], newSizes[3]];
            });
          }}
          onResizeHorizontal2={(delta: number) => {
            setColumnSizes((prev) => {
              const newSizes = [...prev];
              const minSize = 150;
              // Resize hierarchy (column 1) and project (column 2)
              if (newSizes[1] + delta < minSize || newSizes[2] - delta < minSize) return prev;
              return [newSizes[0], newSizes[1] + delta, newSizes[2] - delta, newSizes[3]];
            });
          }}
          onResizeHorizontal3={(delta: number) => {
            setColumnSizes((prev) => {
              const newSizes = [...prev];
              const minSize = 150;
              // Resize project (column 2) and inspector (column 3)
              if (newSizes[2] + delta < minSize || newSizes[3] - delta < minSize) return prev;
              return [newSizes[0], newSizes[1], newSizes[2] + delta, newSizes[3] - delta];
            });
          }}
        />
      </div>
    </div>
  );
}

export default App;

