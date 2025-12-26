-- Tilemap editor script
-- Demonstrates tilemap manipulation in Lua
-- Click to place/remove tiles

local BRUSH_TILE = 2  -- Wall tile
local ERASE_TILE = 1  -- Floor tile

function on_start(self)
    print("[tilemap_editor] Tilemap editor script started")
end

function on_update(self, dt)
    local input = self:input()
    local tilemap = self:tilemap()
    
    if not tilemap then
        return
    end
    
    -- Handle mouse clicks
    if input:is_mouse_pressed("Left") then
        local mouse_screen = input:mouse_pos_screen()
        local mouse_world = mouse_world(mouse_screen)
        
        -- Convert world position to tile coordinates
        local tile_coord = tilemap:world_to_tile(mouse_world)
        local tx = tile_coord.x
        local ty = tile_coord.y
        
        -- Get current tile
        local current_tile = tilemap:get_tile(tx, ty)
        
        -- Toggle: if wall, make floor; if floor, make wall
        if current_tile == BRUSH_TILE then
            tilemap:set_tile(tx, ty, ERASE_TILE)
            print("[tilemap_editor] Erased tile at (" .. tx .. ", " .. ty .. ")")
        else
            tilemap:set_tile(tx, ty, BRUSH_TILE)
            print("[tilemap_editor] Placed wall at (" .. tx .. ", " .. ty .. ")")
        end
    end
    
    -- Right click: fill area
    if input:is_mouse_pressed("Right") then
        local mouse_screen = input:mouse_pos_screen()
        local mouse_world = mouse_world(mouse_screen)
        local tile_coord = tilemap:world_to_tile(mouse_world)
        local tx = tile_coord.x
        local ty = tile_coord.y
        
        -- Fill a 3x3 area
        tilemap:fill_rect(tx - 1, ty - 1, 3, 3, BRUSH_TILE)
        print("[tilemap_editor] Filled 3x3 area at (" .. tx .. ", " .. ty .. ")")
    end
end

