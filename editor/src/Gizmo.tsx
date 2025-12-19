import { useRef, useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface TransformData {
  position: [number, number];
  rotation: number;
  scale: [number, number];
}

import { Tool } from "./Toolbar";

interface GizmoProps {
  entityId: number | null;
  camera: { x: number; y: number; zoom: number };
  canvasWidth: number;
  canvasHeight: number;
  tool: Tool;
  onTransformChange?: () => void;
  onTransformUpdate?: (transform: TransformData) => void;
}

type GizmoHandle = "x" | "y" | "rotate" | "scale" | null;

export default function Gizmo({
  entityId,
  camera,
  canvasWidth,
  canvasHeight,
  tool,
  onTransformChange,
  onTransformUpdate,
}: GizmoProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [transform, setTransform] = useState<TransformData | null>(null);
  const [isDragging, setIsDragging] = useState(false);
  const [dragHandle, setDragHandle] = useState<GizmoHandle>(null);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [dragStartTransform, setDragStartTransform] = useState<TransformData | null>(null);
  const [dragAxisGrabOffset, setDragAxisGrabOffset] = useState(0);

  // Load transform when entity changes or when transform updates
  useEffect(() => {
    if (entityId === null) {
      setTransform(null);
      return;
    }

    const loadTransform = async () => {
      const t = await invoke<TransformData | null>("transform_get", {
        entityId,
      });
      if (t) {
        setTransform(t);
        // Only update dragStartTransform if we're not currently dragging
        // (to avoid resetting the drag state)
        if (!isDragging) {
          setDragStartTransform(t);
        }
      }
    };

    loadTransform();
  }, [entityId, isDragging]);

  // Convert world to screen coordinates
  const worldToScreen = (worldX: number, worldY: number): [number, number] => {
    if (canvasWidth === 0 || canvasHeight === 0) return [0, 0];
    const screenX =
      (worldX - camera.x) * camera.zoom + canvasWidth / 2;
    const screenY =
      (worldY - camera.y) * camera.zoom + canvasHeight / 2;
    return [screenX, screenY];
  };

  // Convert screen to world coordinates
  const screenToWorld = (screenX: number, screenY: number): [number, number] => {
    if (canvasWidth === 0 || canvasHeight === 0) return [0, 0];
    const worldX =
      (screenX - canvasWidth / 2) / camera.zoom + camera.x;
    const worldY =
      (screenY - canvasHeight / 2) / camera.zoom + camera.y;
    return [worldX, worldY];
  };

  // Continuous gizmo rendering loop
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !transform) {
      // Clear canvas if no transform
      if (canvas) {
        const ctx = canvas.getContext("2d");
        if (ctx) {
          ctx.clearRect(0, 0, canvas.width, canvas.height);
        }
      }
      return;
    }

    // Ensure canvas is sized correctly
    if (canvasWidth > 0 && canvasHeight > 0) {
      canvas.width = canvasWidth;
      canvas.height = canvasHeight;
    } else {
      // Canvas not ready yet, don't draw
      return;
    }

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let animationFrameId: number;

    const draw = () => {
      // Ensure canvas is still sized correctly
      if (canvas.width !== canvasWidth || canvas.height !== canvasHeight) {
        canvas.width = canvasWidth;
        canvas.height = canvasHeight;
      }

      ctx.clearRect(0, 0, canvas.width, canvas.height);

      const [screenX, screenY] = worldToScreen(transform.position[0], transform.position[1]);

      ctx.save();
      ctx.translate(screenX, screenY);
      // Only rotate for rotate tool - move gizmo stays world-aligned
      if (tool === "rotate") {
        ctx.rotate(transform.rotation);
      }

      const arrowLength = 40;
      const hoverColor = "#60a5fa";

      // Only draw gizmo based on current tool
      if (tool === "move") {
        // Draw translate handles (X and Y arrows)
        // X arrow (red)
        ctx.strokeStyle = dragHandle === "x" ? hoverColor : "#ef4444";
        ctx.lineWidth = 3;
        ctx.beginPath();
        ctx.moveTo(0, 0);
        ctx.lineTo(arrowLength, 0);
        ctx.stroke();
        // Arrowhead
        ctx.fillStyle = dragHandle === "x" ? hoverColor : "#ef4444";
        ctx.beginPath();
        ctx.moveTo(arrowLength, 0);
        ctx.lineTo(arrowLength - 10, -5);
        ctx.lineTo(arrowLength - 10, 5);
        ctx.closePath();
        ctx.fill();

        // Y arrow (green)
        ctx.strokeStyle = dragHandle === "y" ? hoverColor : "#22c55e";
        ctx.lineWidth = 3;
        ctx.beginPath();
        ctx.moveTo(0, 0);
        ctx.lineTo(0, -arrowLength);
        ctx.stroke();
        // Arrowhead
        ctx.fillStyle = dragHandle === "y" ? hoverColor : "#22c55e";
        ctx.beginPath();
        ctx.moveTo(0, -arrowLength);
        ctx.lineTo(-5, -arrowLength + 10);
        ctx.lineTo(5, -arrowLength + 10);
        ctx.closePath();
        ctx.fill();
      } else if (tool === "rotate") {
        // Draw rotate handle (circle)
        const rotateRadius = 30;
        ctx.strokeStyle = dragHandle === "rotate" ? hoverColor : "#a855f7";
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.arc(0, 0, rotateRadius, 0, Math.PI * 2);
        ctx.stroke();
        // Draw rotation indicator
        ctx.strokeStyle = dragHandle === "rotate" ? hoverColor : "#a855f7";
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(0, -rotateRadius);
        ctx.lineTo(0, -rotateRadius - 10);
        ctx.stroke();
      } else if (tool === "scale") {
        // Draw scale handles (corners)
        const boxSize = 20;
        ctx.fillStyle = dragHandle === "scale" ? hoverColor : "#f59e0b";
        ctx.fillRect(-boxSize / 2, -boxSize / 2, boxSize, boxSize);
        // Draw scale lines
        ctx.strokeStyle = dragHandle === "scale" ? hoverColor : "#f59e0b";
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(-boxSize / 2, -boxSize / 2);
        ctx.lineTo(-boxSize / 2 - 10, -boxSize / 2 - 10);
        ctx.moveTo(boxSize / 2, -boxSize / 2);
        ctx.lineTo(boxSize / 2 + 10, -boxSize / 2 - 10);
        ctx.moveTo(-boxSize / 2, boxSize / 2);
        ctx.lineTo(-boxSize / 2 - 10, boxSize / 2 + 10);
        ctx.moveTo(boxSize / 2, boxSize / 2);
        ctx.lineTo(boxSize / 2 + 10, boxSize / 2 + 10);
        ctx.stroke();
      }

      ctx.restore();

      animationFrameId = requestAnimationFrame(draw);
    };

    animationFrameId = requestAnimationFrame(draw);

    return () => {
      cancelAnimationFrame(animationFrameId);
    };
  }, [transform, camera, canvasWidth, canvasHeight, dragHandle, tool]);

  const getHandleAt = (x: number, y: number): GizmoHandle | null => {
    if (!transform) return null;

    const [screenX, screenY] = worldToScreen(transform.position[0], transform.position[1]);
    const dx = x - screenX;
    const dy = y - screenY;

    const handleSize = 12; // Increased for easier clicking
    const arrowLength = 40;
    const rotateRadius = 30;
    const boxSize = 20;

    if (tool === "move") {
      // Check X arrow (horizontal, pointing right)
      // X arrow: small Y offset, positive X direction
      if (Math.abs(dy) < handleSize && dx > 0 && dx < arrowLength + 10) {
        return "x";
      }

      // Check Y arrow (vertical, pointing up)
      // Y arrow: small X offset, negative Y direction (screen Y is inverted)
      if (Math.abs(dx) < handleSize && dy < 0 && Math.abs(dy) < arrowLength + 10) {
        return "y";
      }
    } else if (tool === "rotate") {
      // Check rotate circle
      const dist = Math.sqrt(dx * dx + dy * dy);
      if (Math.abs(dist - rotateRadius) < handleSize) {
        return "rotate";
      }
    } else if (tool === "scale") {
      // Check scale box
      if (
        Math.abs(dx) < boxSize / 2 + handleSize &&
        Math.abs(dy) < boxSize / 2 + handleSize
      ) {
        return "scale";
      }
    }

    return null;
  };

  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!transform || entityId === null) return;

    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;

    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const handle = getHandleAt(x, y);
    if (handle) {
      console.log("Gizmo drag started:", handle, { x, y });
      setIsDragging(true);
      setDragHandle(handle);
      setDragStart({ x, y });
      setDragStartTransform({ ...transform });
      
      // Calculate grab offset to prevent jumping
      const [wx, wy] = screenToWorld(x, y);
      if (handle === "x") {
        setDragAxisGrabOffset(wx - transform.position[0]);
      } else if (handle === "y") {
        setDragAxisGrabOffset(wy - transform.position[1]);
      } else {
        setDragAxisGrabOffset(0);
      }
    }
  };

  const handleMouseMove = async (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (!transform || entityId === null) return;

    const rect = canvasRef.current?.getBoundingClientRect();
    if (!rect) return;

    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    if (isDragging && dragHandle && dragStartTransform) {
      let newTransform = { ...dragStartTransform };

      if (tool === "move") {
        // Convert mouse position to world space
        const [wx, wy] = screenToWorld(x, y);
        
        if (dragHandle === "x") {
          // Use grab offset to prevent jumping
          newTransform.position[0] = wx - dragAxisGrabOffset;
          // Keep Y unchanged
          newTransform.position[1] = dragStartTransform.position[1];
        } else if (dragHandle === "y") {
          // Use grab offset to prevent jumping
          newTransform.position[1] = wy - dragAxisGrabOffset;
          // Keep X unchanged
          newTransform.position[0] = dragStartTransform.position[0];
        }
      } else if (tool === "rotate" && dragHandle === "rotate") {
        // Rotate around entity center
        const [entityScreenX, entityScreenY] = worldToScreen(
          dragStartTransform.position[0],
          dragStartTransform.position[1]
        );
        const startAngle = Math.atan2(dragStart.y - entityScreenY, dragStart.x - entityScreenX);
        const currentAngle = Math.atan2(y - entityScreenY, x - entityScreenX);
        newTransform.rotation = dragStartTransform.rotation + (currentAngle - startAngle);
      } else if (tool === "scale" && dragHandle === "scale") {
        // Scale: make the scale handle follow the cursor
        // Convert mouse position to world space
        const [currentWorldX, currentWorldY] = screenToWorld(x, y);
        const [entityWorldX, entityWorldY] = [dragStartTransform.position[0], dragStartTransform.position[1]];
        
        // Calculate distance from entity center to cursor in world space
        const currentDist = Math.sqrt(
          Math.pow(currentWorldX - entityWorldX, 2) +
          Math.pow(currentWorldY - entityWorldY, 2)
        );
        
        // Calculate initial distance (when drag started)
        const [startWorldX, startWorldY] = screenToWorld(dragStart.x, dragStart.y);
        const startDist = Math.sqrt(
          Math.pow(startWorldX - entityWorldX, 2) +
          Math.pow(startWorldY - entityWorldY, 2)
        );
        
        console.log("Scale drag:", { currentDist, startDist, scaleFactor: startDist > 0 ? currentDist / startDist : 1 });
        
        // Scale factor based on distance change
        if (startDist > 0.001) { // Use small epsilon instead of 0
          const scaleFactor = currentDist / startDist;
          newTransform.scale[0] = dragStartTransform.scale[0] * scaleFactor;
          newTransform.scale[1] = dragStartTransform.scale[1] * scaleFactor;
        } else {
          // If startDist is too small, keep original scale
          newTransform.scale = [...dragStartTransform.scale];
        }
      }

      // Update transform via IPC
      try {
        console.log("Sending transform_set", { entityId, position: newTransform.position, rotation: newTransform.rotation, scale: newTransform.scale });
        await invoke("transform_set", {
          entityId,
          position: newTransform.position,
          rotation: newTransform.rotation,
          scale: newTransform.scale,
        });
        setTransform(newTransform);
        // Update viewport cache immediately for smooth rendering
        if (onTransformUpdate) {
          onTransformUpdate(newTransform);
        }
      } catch (error) {
        console.error("Failed to update transform:", error);
      }
    }
    // Note: Hover detection could be added here in the future for cursor changes
  };

  const handleMouseUp = () => {
    setIsDragging(false);
    setDragHandle(null);
    setDragAxisGrabOffset(0);
    // Call transform change callback when drag ends (refresh entities once)
    if (onTransformChange) {
      onTransformChange();
    }
  };

  // Resize canvas to match viewport dimensions
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    // Use the props directly to ensure correct sizing
    if (canvasWidth > 0 && canvasHeight > 0) {
      canvas.width = canvasWidth;
      canvas.height = canvasHeight;
    } else {
      // Fallback to client dimensions if props aren't available yet
      canvas.width = canvas.clientWidth;
      canvas.height = canvas.clientHeight;
    }
  }, [canvasWidth, canvasHeight]);

  // Always render canvas (even if no transform) so we can debug
  // The draw loop will handle clearing if there's no transform
  if (entityId === null) {
    return null;
  }

  return (
    <canvas
      ref={canvasRef}
      className="absolute top-0 left-0 pointer-events-auto"
      style={{ 
        width: "100%", 
        height: "100%",
        zIndex: 10,
      }}
      onMouseDown={(e) => {
        e.stopPropagation();
        handleMouseDown(e);
      }}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={handleMouseUp}
    />
  );
}

