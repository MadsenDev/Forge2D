export type Tool = "move" | "rotate" | "scale";

interface ToolbarProps {
  currentTool: Tool;
  onToolChange: (tool: Tool) => void;
  isPlaying: boolean;
  onUndo?: () => void;
  onRedo?: () => void;
  canUndo?: boolean;
  canRedo?: boolean;
  onNewScene?: () => void;
  onSave?: () => void;
  onLoad?: () => void;
  onPlay?: () => void;
  onStop?: () => void;
}

export default function Toolbar({
  currentTool,
  onToolChange,
  isPlaying,
  onUndo = () => {},
  onRedo = () => {},
  canUndo = false,
  canRedo = false,
  onNewScene = () => {},
  onSave = () => {},
  onLoad = () => {},
  onPlay = () => {},
  onStop = () => {},
}: ToolbarProps) {
  return (
    <div className="h-12 bg-gray-800 border-b border-gray-700 flex items-center px-4 gap-2">
      {/* Tool Selector */}
      <div className="flex gap-1 bg-gray-700 rounded p-1">
        <button
          onClick={() => onToolChange("move")}
          disabled={isPlaying}
          className={`px-3 py-1 rounded text-sm font-medium transition-colors ${
            currentTool === "move"
              ? "bg-blue-600 text-white"
              : "bg-transparent text-gray-300 hover:bg-gray-600"
          } disabled:opacity-50 disabled:cursor-not-allowed`}
          title="Move Tool (W)"
        >
          Move
        </button>
        <button
          onClick={() => onToolChange("rotate")}
          disabled={isPlaying}
          className={`px-3 py-1 rounded text-sm font-medium transition-colors ${
            currentTool === "rotate"
              ? "bg-blue-600 text-white"
              : "bg-transparent text-gray-300 hover:bg-gray-600"
          } disabled:opacity-50 disabled:cursor-not-allowed`}
          title="Rotate Tool (E)"
        >
          Rotate
        </button>
        <button
          onClick={() => onToolChange("scale")}
          disabled={isPlaying}
          className={`px-3 py-1 rounded text-sm font-medium transition-colors ${
            currentTool === "scale"
              ? "bg-blue-600 text-white"
              : "bg-transparent text-gray-300 hover:bg-gray-600"
          } disabled:opacity-50 disabled:cursor-not-allowed`}
          title="Scale Tool (R)"
        >
          Scale
        </button>
      </div>

      <div className="w-px h-6 bg-gray-600 mx-2" />

      {/* Undo/Redo */}
      <button
        onClick={onUndo}
        disabled={!canUndo || isPlaying}
        className="px-3 py-1 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed rounded text-sm"
        title="Undo (Ctrl+Z)"
      >
        Undo
      </button>
      <button
        onClick={onRedo}
        disabled={!canRedo || isPlaying}
        className="px-3 py-1 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed rounded text-sm"
        title="Redo (Ctrl+Y)"
      >
        Redo
      </button>

      <div className="w-px h-6 bg-gray-600 mx-2" />

      {/* Scene Operations */}
      <button
        onClick={onNewScene}
        disabled={isPlaying}
        className="px-3 py-1 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed rounded text-sm"
      >
        New
      </button>
      <button
        onClick={onSave}
        disabled={isPlaying}
        className="px-3 py-1 bg-blue-600 hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed rounded text-sm"
      >
        Save
      </button>
      <button
        onClick={onLoad}
        disabled={isPlaying}
        className="px-3 py-1 bg-gray-700 hover:bg-gray-600 disabled:opacity-50 disabled:cursor-not-allowed rounded text-sm"
      >
        Load
      </button>

      <div className="flex-1" />

      {/* Play Mode */}
      {!isPlaying ? (
        <button
          onClick={onPlay}
          className="px-4 py-1 bg-green-600 hover:bg-green-700 rounded text-sm font-medium"
        >
          ▶ Play
        </button>
      ) : (
        <button
          onClick={onStop}
          className="px-4 py-1 bg-red-600 hover:bg-red-700 rounded text-sm font-medium"
        >
          ⏹ Stop
        </button>
      )}
    </div>
  );
}

