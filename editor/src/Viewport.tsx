import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Gizmo from "./Gizmo";
import { Tool } from "./Toolbar";

interface EntityInfo {
  id: number;
  has_transform: boolean;
  has_sprite: boolean;
  has_physics: boolean;
  parent_id: number | null;
  children: number[];
}

interface TransformData {
  position: [number, number];
  rotation: number;
  scale: [number, number];
}

interface SpriteData {
  texture_handle: number;
  texture_path: string | null;
  texture_size: [number, number] | null; // [width, height]
  tint: [number, number, number, number];
  sprite_scale: [number, number];
}

interface ViewportProps {
  entities: EntityInfo[];
  selectedEntityId: number | null;
  onEntityClick?: (entityId: number) => void;
  onTransformChange?: () => void;
  isPlaying?: boolean;
  tool: Tool;
}

export default function Viewport({
  entities,
  selectedEntityId,
  onEntityClick,
  onTransformChange,
  isPlaying = false,
  tool,
}: ViewportProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [camera, setCamera] = useState({ x: 0, y: 0, zoom: 1 });
  const [isDragging, setIsDragging] = useState(false);
  const [dragStart, setDragStart] = useState({ x: 0, y: 0 });
  const [transformCache, setTransformCache] = useState<Map<number, TransformData>>(new Map());
  const [spriteCache, setSpriteCache] = useState<Map<number, SpriteData>>(new Map());
  const [imageCache, setImageCache] = useState<Map<string, HTMLImageElement>>(new Map());
  const [canvasSize, setCanvasSize] = useState({ width: 0, height: 0 });

  // Cache transforms and sprites when entities change (not every frame)
  useEffect(() => {
    const fetchData = async () => {
      const transforms = new Map<number, TransformData>();
      const sprites = new Map<number, SpriteData>();
      
      const promises = entities
        .filter((e) => e.has_transform)
        .map(async (entity) => {
          // Fetch transform
          const transform = await invoke<TransformData | null>("transform_get", {
            entityId: entity.id,
          });
          if (transform) {
            transforms.set(entity.id, transform);
          }
          
          // Fetch sprite if it exists
          if (entity.has_sprite) {
            const sprite = await invoke<SpriteData | null>("sprite_get", {
              entityId: entity.id,
            });
            if (sprite) {
              sprites.set(entity.id, sprite);
            }
          }
        });
      await Promise.all(promises);
      setTransformCache(transforms);
      setSpriteCache(sprites);
      
      // Load images for sprites with texture paths
      const imagesToLoad = new Map<string, HTMLImageElement>();
      const imagePromises: Promise<void>[] = [];
      
      for (const sprite of sprites.values()) {
        if (sprite.texture_path && !imageCache.has(sprite.texture_path)) {
          const texturePath = sprite.texture_path; // TypeScript knows it's not null here
          const img = new Image();
          const promise = new Promise<void>((resolve) => {
            img.onload = () => {
              imagesToLoad.set(texturePath, img);
              resolve();
            };
            img.onerror = () => {
              console.warn(`Failed to load texture: ${texturePath}`);
              resolve(); // Continue even if image fails to load
            };
            // Tauri file paths need special handling - use tauri:// protocol or convert to file://
            // For now, assume absolute paths work
            img.src = texturePath;
          });
          imagePromises.push(promise);
        }
      }
      
      await Promise.all(imagePromises);
      
      // Update image cache
      if (imagesToLoad.size > 0) {
        setImageCache(prev => {
          const next = new Map(prev);
          for (const [path, img] of imagesToLoad) {
            next.set(path, img);
          }
          return next;
        });
      }
    };

    fetchData();
  }, [entities, imageCache]);

  // Continuous rendering loop
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    let animationFrameId: number;
    let lastTime = 0;
    const targetFPS = 60;
    const frameInterval = 1000 / targetFPS;

    const draw = () => {
      // Use display dimensions (not internal canvas dimensions) for all calculations
      const displayWidth = canvasSize.width || canvas.clientWidth;
      const displayHeight = canvasSize.height || canvas.clientHeight;
      
      // Clear canvas (use internal dimensions for clearing)
      ctx.fillStyle = "#0a0a0a";
      ctx.fillRect(0, 0, canvas.width, canvas.height);

      // Draw grid with square cells on screen
      // Use the same pixel spacing for both X and Y to ensure squares
      ctx.strokeStyle = "#1a1a1a";
      ctx.lineWidth = 1;
      
      // Grid size in world units (this defines the actual world grid)
      const gridSizeWorld = 50;
      
      // Convert to screen pixels - use the same conversion for both axes
      // This ensures grid cells are square on screen
      const gridSizeScreen = gridSizeWorld * camera.zoom;
      
      // Calculate where the camera center is in screen space
      const cameraScreenX = canvas.width / 2;
      const cameraScreenY = canvas.height / 2;
      
      // Convert camera world position to screen offset
      // The camera position in world space, when converted to screen, offsets from center
      const screenOffsetX = camera.x * camera.zoom;
      const screenOffsetY = camera.y * camera.zoom;
      
      // Calculate the first grid line position in screen space
      // Start from camera center, offset by world position, then align to grid
      const firstGridX = cameraScreenX - screenOffsetX;
      const firstGridY = cameraScreenY - screenOffsetY;
      
      // Align to grid boundaries
      const startX = Math.floor(firstGridX / gridSizeScreen) * gridSizeScreen;
      const startY = Math.floor(firstGridY / gridSizeScreen) * gridSizeScreen;
      
      // Draw vertical lines - use same gridSizeScreen for spacing
      for (let x = startX; x <= displayWidth + gridSizeScreen; x += gridSizeScreen) {
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, displayHeight);
        ctx.stroke();
      }
      
      // Draw horizontal lines - use same gridSizeScreen for spacing (ensures squares)
      for (let y = startY; y <= displayHeight + gridSizeScreen; y += gridSizeScreen) {
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(displayWidth, y);
        ctx.stroke();
      }

      // Draw entities using cached transforms and sprites (1:1 with engine)
      for (const entity of entities) {
        if (!entity.has_transform) continue;
        const transform = transformCache.get(entity.id);
        if (!transform) continue;

        // Convert world to screen coordinates (matching engine's Camera2D.world_to_screen)
        const screenX =
          (transform.position[0] - camera.x) * camera.zoom + displayWidth / 2;
        const screenY =
          (transform.position[1] - camera.y) * camera.zoom + displayHeight / 2;

        const isSelected = selectedEntityId === entity.id;
        const sprite = spriteCache.get(entity.id);
        
        ctx.save();
        ctx.translate(screenX, screenY);
        ctx.rotate(transform.rotation);
        
        // Apply transform scale (from Transform component) - matches engine's Transform2D
        ctx.scale(transform.scale[0], transform.scale[1]);

        // Draw selection highlight
        if (isSelected) {
          ctx.strokeStyle = "#3b82f6";
          ctx.lineWidth = 3 / (camera.zoom * Math.max(transform.scale[0], transform.scale[1]));
          // Draw outline around sprite bounds
          if (sprite) {
            // Get actual texture size if available
            let w = 32;
            let h = 32;
            if (sprite.texture_path) {
              const img = imageCache.get(sprite.texture_path);
              if (img && img.complete) {
                w = img.width;
                h = img.height;
              }
            }
            w *= sprite.sprite_scale[0];
            h *= sprite.sprite_scale[1];
            ctx.strokeRect(-w / 2, -h / 2, w, h);
          } else {
            ctx.strokeRect(-20, -20, 40, 40);
          }
        }

        if (sprite) {
          // Draw sprite with proper scale and tint (1:1 with engine rendering)
          ctx.save();
          // Apply sprite scale (from Sprite.transform.scale) - matches engine's Sprite.transform
          ctx.scale(sprite.sprite_scale[0], sprite.sprite_scale[1]);
          
          // Get texture image
          let img: HTMLImageElement | null = null;
          let textureWidth = 32; // Default fallback
          let textureHeight = 32;
          
          if (sprite.texture_path) {
            img = imageCache.get(sprite.texture_path) || null;
            if (img && img.complete) {
              textureWidth = img.width;
              textureHeight = img.height;
            }
          }
          
          // Apply tint (matching engine's sprite.tint)
          const [r, g, b, a] = sprite.tint;
          ctx.globalAlpha = a;
          
          if (img && img.complete && img.naturalWidth > 0) {
            // Draw actual texture image (1:1 with engine)
            ctx.drawImage(
              img,
              -textureWidth / 2,
              -textureHeight / 2,
              textureWidth,
              textureHeight
            );
            
            // Apply tint using composite operation (matches engine's multiplicative tint)
            if (r !== 1.0 || g !== 1.0 || b !== 1.0) {
              ctx.globalCompositeOperation = 'multiply';
              ctx.fillStyle = `rgb(${Math.round(r * 255)}, ${Math.round(g * 255)}, ${Math.round(b * 255)})`;
              ctx.fillRect(-textureWidth / 2, -textureHeight / 2, textureWidth, textureHeight);
              ctx.globalCompositeOperation = 'source-over';
            }
          } else {
            // Fallback: draw colored rectangle if image not loaded
            ctx.fillStyle = `rgba(${Math.round(r * 255)}, ${Math.round(g * 255)}, ${Math.round(b * 255)}, ${a})`;
            ctx.fillRect(-textureWidth / 2, -textureHeight / 2, textureWidth, textureHeight);
            
            // Draw sprite outline
            ctx.strokeStyle = "#ffffff";
            ctx.lineWidth = 1 / (camera.zoom * Math.max(transform.scale[0] * sprite.sprite_scale[0], transform.scale[1] * sprite.sprite_scale[1]));
            ctx.strokeRect(-textureWidth / 2, -textureHeight / 2, textureWidth, textureHeight);
          }
          
          ctx.globalAlpha = 1.0;
          ctx.restore();
        } else {
          // Fallback: draw entity box if no sprite
          ctx.fillStyle = isSelected ? "#60a5fa" : "#4b5563";
          ctx.fillRect(-15, -15, 30, 30);
          ctx.strokeStyle = "#ffffff";
          ctx.lineWidth = 1 / (camera.zoom * Math.max(transform.scale[0], transform.scale[1]));
          ctx.strokeRect(-15, -15, 30, 30);
        }

        // Draw rotation indicator
        ctx.strokeStyle = "#ffffff";
        ctx.lineWidth = 2 / (camera.zoom * Math.max(transform.scale[0], transform.scale[1]));
        ctx.beginPath();
        ctx.moveTo(0, 0);
        ctx.lineTo(0, -20);
        ctx.stroke();

        ctx.restore();

        // Draw entity ID
        ctx.fillStyle = "#ffffff";
        ctx.font = "12px monospace";
        ctx.fillText(
          entity.id.toString(),
          screenX + 20,
          screenY - 20
        );
      }
    };

    const renderLoop = (currentTime: number) => {
      // Throttle to target FPS
      if (currentTime - lastTime >= frameInterval) {
        draw();
        lastTime = currentTime;
      }
      animationFrameId = requestAnimationFrame(renderLoop);
    };

    animationFrameId = requestAnimationFrame(renderLoop);

    return () => {
      cancelAnimationFrame(animationFrameId);
    };
  }, [entities, selectedEntityId, camera, tool, transformCache, spriteCache, imageCache]);

  const handleMouseDown = (e: React.MouseEvent<HTMLCanvasElement>) => {
    // Don't interfere with gizmo - let it handle clicks on selected entities
    // The gizmo canvas is on top and will capture the click
    
    if (e.button === 1 || (e.button === 0 && e.altKey)) {
      // Middle mouse or Alt+Left = pan
      setIsDragging(true);
      setDragStart({ x: e.clientX, y: e.clientY });
    } else if (e.button === 0) {
      // Left click = select entity
      const canvas = canvasRef.current;
      if (!canvas) return;

      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;

      // Convert screen to world coordinates
      const worldX =
        (x - canvas.width / 2) / camera.zoom + camera.x;
      const worldY =
        (y - canvas.height / 2) / camera.zoom + camera.y;

      // Find closest entity using cached transforms
      let closestEntity: { id: number; dist: number } | null = null;
      
      for (const entity of entities) {
        if (!entity.has_transform) continue;
        const transform = transformCache.get(entity.id);
        if (!transform) continue;

        const dx = transform.position[0] - worldX;
        const dy = transform.position[1] - worldY;
        const dist = Math.sqrt(dx * dx + dy * dy);

        if (dist < 20 && (!closestEntity || dist < closestEntity.dist)) {
          closestEntity = { id: entity.id, dist };
        }
      }

      if (closestEntity && onEntityClick) {
        onEntityClick(closestEntity.id);
      }
    }
  };

  const handleMouseMove = (e: React.MouseEvent<HTMLCanvasElement>) => {
    if (isDragging) {
      const dx = (e.clientX - dragStart.x) / camera.zoom;
      const dy = (e.clientY - dragStart.y) / camera.zoom;
      setCamera((prev) => ({
        ...prev,
        x: prev.x - dx,
        y: prev.y - dy,
      }));
      setDragStart({ x: e.clientX, y: e.clientY });
    }
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  const handleWheel = (e: React.WheelEvent<HTMLCanvasElement>) => {
    e.preventDefault();
    const delta = e.deltaY > 0 ? 0.9 : 1.1;
    setCamera((prev) => ({
      ...prev,
      zoom: Math.max(0.1, Math.min(5, prev.zoom * delta)),
    }));
  };

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const resize = () => {
      // Get the actual display size (CSS pixels)
      const displayWidth = canvas.clientWidth;
      const displayHeight = canvas.clientHeight;
      
      // Get device pixel ratio to handle high-DPI displays
      const dpr = window.devicePixelRatio || 1;
      
      // Set internal canvas size accounting for device pixel ratio
      // This ensures crisp rendering and prevents stretching
      const internalWidth = Math.floor(displayWidth * dpr);
      const internalHeight = Math.floor(displayHeight * dpr);
      
      if (canvas.width !== internalWidth || canvas.height !== internalHeight) {
        canvas.width = internalWidth;
        canvas.height = internalHeight;
        
        // Scale the context to account for device pixel ratio
        const ctx = canvas.getContext("2d");
        if (ctx) {
          ctx.scale(dpr, dpr);
        }
        
        // Store the display size (not internal size) for calculations
        setCanvasSize({ width: displayWidth, height: displayHeight });
      }
    };

    resize();
    
    // Use ResizeObserver to detect when container size changes
    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(canvas);
    
    window.addEventListener("resize", resize);
    return () => {
      window.removeEventListener("resize", resize);
      resizeObserver.disconnect();
    };
  }, []);

  return (
    <div className="relative w-full h-full">
      <canvas
        ref={canvasRef}
        className="w-full h-full cursor-crosshair"
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        onWheel={handleWheel}
        style={{ display: "block" }}
      />
      {!isPlaying && (
        <Gizmo
          entityId={selectedEntityId}
          camera={camera}
          canvasWidth={canvasSize.width}
          canvasHeight={canvasSize.height}
          tool={tool}
          onTransformUpdate={(transform) => {
            // Update cache immediately during drag for smooth rendering
            if (selectedEntityId !== null) {
              setTransformCache((prev) => {
                const next = new Map(prev);
                next.set(selectedEntityId, transform);
                return next;
              });
            }
          }}
          onTransformChange={async () => {
            // Refresh full entity list when drag ends
            if (onTransformChange) {
              onTransformChange();
            }
          }}
        />
      )}
    </div>
  );
}

