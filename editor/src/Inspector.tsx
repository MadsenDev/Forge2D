import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface ComponentFieldInfo {
  name: string;
  type_name: string;
  value: any;
}

interface InspectorProps {
  selectedEntityId: number | null;
  refreshTrigger?: number; // Increment this to force refresh
}

export default function Inspector({ selectedEntityId, refreshTrigger }: InspectorProps) {
  const [componentTypes, setComponentTypes] = useState<string[]>([]);
  const [fields, setFields] = useState<Record<string, ComponentFieldInfo[]>>({});

  useEffect(() => {
    if (selectedEntityId === null) {
      setFields({});
      return;
    }

    const loadComponentTypes = async () => {
      const types = await invoke<string[]>("component_types");
      setComponentTypes(types);

      // Load fields for each component type
      const fieldMap: Record<string, ComponentFieldInfo[]> = {};
      for (const type of types) {
        const componentFields = await invoke<ComponentFieldInfo[] | null>(
          "component_fields",
          { entityId: selectedEntityId, componentType: type }
        );
        if (componentFields) {
          fieldMap[type] = componentFields;
        }
      }
      setFields(fieldMap);
    };

    loadComponentTypes();
  }, [selectedEntityId, refreshTrigger]);

  const handleFieldChange = async (
    componentType: string,
    fieldName: string,
    value: any
  ) => {
    if (selectedEntityId === null) return;

    await invoke("component_set_field", {
      entityId: selectedEntityId,
      componentType,
      fieldName,
      value,
    });

    // Reload fields
    const componentFields = await invoke<ComponentFieldInfo[] | null>(
      "component_fields",
      { entityId: selectedEntityId, componentType }
    );
    if (componentFields) {
      setFields((prev) => ({ ...prev, [componentType]: componentFields }));
    }
  };

  if (selectedEntityId === null) {
    return (
      <div className="p-4 text-gray-400 text-sm">
        Select an entity to inspect
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto p-4">
      <div className="mb-4">
        <div className="text-xs text-gray-400 mb-1">Entity ID</div>
        <div className="font-mono text-sm">{selectedEntityId}</div>
      </div>

      {componentTypes.length === 0 ? (
        <div className="text-gray-400 text-sm">No components</div>
      ) : (
        componentTypes.map((type) => {
          const componentFields = fields[type];
          if (!componentFields || componentFields.length === 0) return null;

          return (
            <div key={type} className="mb-6">
              <div className="text-sm font-semibold mb-2 text-blue-400">
                {type}
              </div>
              <div className="space-y-3">
                {componentFields.map((field) => (
                  <div key={field.name}>
                    <label className="block text-xs text-gray-400 mb-1">
                      {field.name} ({field.type_name})
                    </label>
                    {field.type_name === "f32" ? (
                      <input
                        type="number"
                        step="0.1"
                        value={field.value as number}
                        onChange={(e) =>
                          handleFieldChange(
                            type,
                            field.name,
                            parseFloat(e.target.value) || 0
                          )
                        }
                        className="w-full px-2 py-1 bg-gray-700 rounded text-sm"
                      />
                    ) : field.type_name === "Vec2" ? (
                      <div className="grid grid-cols-2 gap-2">
                        <input
                          type="number"
                          step="0.1"
                          value={(field.value as any)?.x || 0}
                          onChange={(e) =>
                            handleFieldChange(type, field.name, {
                              x: parseFloat(e.target.value) || 0,
                              y: (field.value as any)?.y || 0,
                            })
                          }
                          placeholder="X"
                          className="px-2 py-1 bg-gray-700 rounded text-sm"
                        />
                        <input
                          type="number"
                          step="0.1"
                          value={(field.value as any)?.y || 0}
                          onChange={(e) =>
                            handleFieldChange(type, field.name, {
                              x: (field.value as any)?.x || 0,
                              y: parseFloat(e.target.value) || 0,
                            })
                          }
                          placeholder="Y"
                          className="px-2 py-1 bg-gray-700 rounded text-sm"
                        />
                      </div>
                    ) : (
                      <div className="px-2 py-1 bg-gray-700 rounded text-sm text-gray-300">
                        {JSON.stringify(field.value)}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          );
        })
      )}
    </div>
  );
}

