-- Benchmark script: Simple entity that moves in a circle pattern
-- This tests script performance with many instances

local SPEED = params.speed or 50.0
local RADIUS = params.radius or 100.0
local center_x = params.center_x or 480.0
local center_y = params.center_y or 360.0

local angle = 0.0
local time_accumulator = 0.0

function on_start(self)
    -- Initialize position
    local pos = self:position()
    center_x = pos.x
    center_y = pos.y
end

function on_fixed_update(self, fixed_dt)
    local physics = self:physics()
    if not physics then return end
    
    time_accumulator = time_accumulator + fixed_dt
    angle = angle + SPEED * fixed_dt
    
    -- Calculate circular motion
    local target_x = center_x + math.cos(angle) * RADIUS
    local target_y = center_y + math.sin(angle) * RADIUS
    
    -- Get current position
    local pos = self:position()
    
    -- Calculate velocity toward target
    local dx = target_x - pos.x
    local dy = target_y - pos.y
    local dist = math.sqrt(dx * dx + dy * dy)
    
    if dist > 1.0 then
        local vel_x = (dx / dist) * SPEED * 2.0
        local vel_y = (dy / dist) * SPEED * 2.0
        physics:set_velocity(vec2(vel_x, vel_y))
    else
        physics:set_velocity(vec2(0.0, 0.0))
    end
end

