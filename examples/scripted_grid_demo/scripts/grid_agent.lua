-- Grid-based agent script
-- Controls an agent that moves on a grid using pathfinding

local CELL_SIZE = params.cell_size or 32.0
local MOVE_DURATION = params.move_duration or 0.3

-- Use a global table to persist state across script re-executions
if not _agent_state then
    _agent_state = {
        agent_grid_pos = {x = 5, y = 5},
        target_grid = nil,
        path = {},
        path_index = 0,
        move_timer = 0.0
    }
end

function on_start(self)
    print("[grid_agent] Agent script started")
    local pos = self:position()
    -- Initialize grid position from world position
    local grid_pos = world_to_grid(pos)
    _agent_state.agent_grid_pos.x = grid_pos.x
    _agent_state.agent_grid_pos.y = grid_pos.y
    print("[grid_agent] Starting at grid: (" .. _agent_state.agent_grid_pos.x .. ", " .. _agent_state.agent_grid_pos.y .. ")")
end

function on_update(self, dt)
    local input = self:input()
    
    -- Handle mouse click to set target
    if input:is_mouse_pressed("Left") then
        local mouse_screen = input:mouse_pos_screen()
        -- Convert screen to world coordinates
        local mouse_world = mouse_world(mouse_screen)
        
        -- Convert world to grid coordinates
        local target_grid_pos = world_to_grid(mouse_world)
        
        -- Check if target is walkable
        if is_walkable(target_grid_pos) then
            print("[grid_agent] Target set to grid: (" .. target_grid_pos.x .. ", " .. target_grid_pos.y .. ")")
            
            -- Find path from current position to target
            local found_path = find_path(_agent_state.agent_grid_pos, target_grid_pos)
            
            if found_path then
                -- Convert path to array of grid coordinates
                _agent_state.path = {}
                for i = 1, #found_path do
                    local node = found_path[i]
                    table.insert(_agent_state.path, {x = node.x, y = node.y})
                end
                
                if #_agent_state.path > 1 then
                    -- Skip first node (current position) - create new table without first element
                    local new_path = {}
                    for i = 2, #_agent_state.path do
                        table.insert(new_path, _agent_state.path[i])
                    end
                    _agent_state.path = new_path
                    _agent_state.target_grid = target_grid_pos
                    _agent_state.path_index = 0
                    _agent_state.move_timer = 0.0
                    print("[grid_agent] Path found with " .. #_agent_state.path .. " nodes")
                    print("[grid_agent] First path node: (" .. _agent_state.path[1].x .. ", " .. _agent_state.path[1].y .. ")")
                else
                    print("[grid_agent] Path too short or already at target")
                    _agent_state.path = {}
                    _agent_state.target_grid = nil
                end
            else
                print("[grid_agent] No path found to target")
                _agent_state.path = {}
                _agent_state.target_grid = nil
            end
        else
            print("[grid_agent] Target is not walkable")
        end
    end
    
    -- Move along path
    if _agent_state.target_grid and _agent_state.path_index < #_agent_state.path and #_agent_state.path > 0 then
        _agent_state.move_timer = _agent_state.move_timer + dt
        local t = math.min(_agent_state.move_timer / MOVE_DURATION, 1.0)
        
        -- Smooth interpolation (smoothstep)
        local smooth_t = t * t * (3.0 - 2.0 * t)
        
        -- Get next cell position (Lua arrays are 1-indexed, path_index starts at 0)
        local next_cell = _agent_state.path[_agent_state.path_index + 1]
        if next_cell then
            local next_world = grid_to_world(next_cell)
            
            local current_pos = self:position()
            local new_pos = vec2(
                current_pos.x + (next_world.x - current_pos.x) * smooth_t,
                current_pos.y + (next_world.y - current_pos.y) * smooth_t
            )
            self:set_position(new_pos)
            
            if t >= 1.0 then
                -- Reached next cell
                _agent_state.agent_grid_pos.x = next_cell.x
                _agent_state.agent_grid_pos.y = next_cell.y
                self:set_position(next_world)
                _agent_state.path_index = _agent_state.path_index + 1
                _agent_state.move_timer = 0.0
                print("[grid_agent] Reached cell (" .. next_cell.x .. ", " .. next_cell.y .. "), path_index=" .. _agent_state.path_index .. ", path_length=" .. #_agent_state.path)
                
                if _agent_state.path_index >= #_agent_state.path then
                    -- Reached target
                    print("[grid_agent] Reached target!")
                    _agent_state.target_grid = nil
                    _agent_state.path = {}
                    _agent_state.path_index = 0
                end
            end
        else
            print("[grid_agent] ERROR: next_cell is nil! path_index=" .. _agent_state.path_index .. ", path_length=" .. #_agent_state.path)
        end
    elseif _agent_state.target_grid then
        -- Debug: why aren't we moving?
        if #_agent_state.path == 0 then
            print("[grid_agent] WARNING: target_grid set but path is empty")
        elseif _agent_state.path_index >= #_agent_state.path then
            print("[grid_agent] WARNING: path_index >= path length")
        end
    end
end

