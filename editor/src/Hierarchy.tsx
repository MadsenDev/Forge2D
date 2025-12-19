import { useState } from "react";

interface EntityInfo {
  id: number;
  has_transform: boolean;
  has_sprite: boolean;
  has_physics: boolean;
  parent_id: number | null;
  children: number[];
}

interface HierarchyProps {
  entities: EntityInfo[];
  selectedEntityId: number | null;
  onEntityClick: (entityId: number) => void;
}

function HierarchyNode({
  entity,
  entities,
  selectedEntityId,
  onEntityClick,
  level = 0,
}: {
  entity: EntityInfo;
  entities: EntityInfo[];
  selectedEntityId: number | null;
  onEntityClick: (entityId: number) => void;
  level?: number;
}) {
  const [expanded, setExpanded] = useState(true);
  const hasChildren = entity.children.length > 0;
  const isSelected = selectedEntityId === entity.id;

  const childEntities = entity.children
    .map((childId) => entities.find((e) => e.id === childId))
    .filter((e): e is EntityInfo => e !== undefined);

  return (
    <div>
      <div
        className={`flex items-center gap-1 px-2 py-1 rounded cursor-pointer ${
          isSelected ? "bg-blue-600" : "hover:bg-gray-700"
        }`}
        style={{ paddingLeft: `${level * 16 + 8}px` }}
        onClick={() => onEntityClick(entity.id)}
      >
        {hasChildren && (
          <button
            onClick={(e) => {
              e.stopPropagation();
              setExpanded(!expanded);
            }}
            className="w-4 h-4 flex items-center justify-center text-xs"
          >
            {expanded ? "▼" : "▶"}
          </button>
        )}
        {!hasChildren && <span className="w-4" />}
        <span className="font-mono text-sm">Entity {entity.id}</span>
      </div>
      {hasChildren && expanded && (
        <div>
          {childEntities.map((child) => (
            <HierarchyNode
              key={child.id}
              entity={child}
              entities={entities}
              selectedEntityId={selectedEntityId}
              onEntityClick={onEntityClick}
              level={level + 1}
            />
          ))}
        </div>
      )}
    </div>
  );
}

export default function Hierarchy({
  entities,
  selectedEntityId,
  onEntityClick,
}: HierarchyProps) {
  // Find root entities (those with no parent)
  const rootEntities = entities.filter((e) => e.parent_id === null);

  return (
    <div className="h-full overflow-y-auto">
      {rootEntities.length === 0 ? (
        <p className="text-gray-400 text-sm p-2">No entities</p>
      ) : (
        <div>
          {rootEntities.map((entity) => (
            <HierarchyNode
              key={entity.id}
              entity={entity}
              entities={entities}
              selectedEntityId={selectedEntityId}
              onEntityClick={onEntityClick}
            />
          ))}
        </div>
      )}
    </div>
  );
}

