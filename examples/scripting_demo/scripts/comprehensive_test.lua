-- Comprehensive scripting test
-- Tests all engine components and scripting features

local test_state = {
    start_called = false,
    update_count = 0,
    fixed_update_count = 0,
    collision_count = 0,
    trigger_count = 0,
    last_position = nil,
}

function on_start(self)
    print("[TEST] on_start called")
    test_state.start_called = true
    
    -- Test sprite facet
    local sprite = self:sprite()
    if sprite ~= nil then
        sprite:set_tint({1.0, 0.5, 0.0, 1.0}) -- Orange tint
        print("[TEST] Sprite facet works")
    end
    
    -- Test transform facet
    local transform = self:transform()
    if transform ~= nil then
        local pos = transform:position()
        test_state.last_position = pos
        print("[TEST] Transform facet works, position: " .. tostring(pos.x) .. ", " .. tostring(pos.y))
    end
    
    -- Test physics facet
    local physics = self:physics()
    if physics ~= nil then
        local vel = physics:velocity()
        print("[TEST] Physics facet works, velocity: " .. tostring(vel.x) .. ", " .. tostring(vel.y))
    end
    
    -- Test input facet
    local input = self:input()
    if input ~= nil then
        print("[TEST] Input facet works")
    end
    
    -- Test time facet
    local time = self:time()
    if time ~= nil then
        local dt = time:delta()
        local fixed_dt = time:fixed_delta()
        print("[TEST] Time facet works, dt: " .. tostring(dt) .. ", fixed_dt: " .. tostring(fixed_dt))
    end
    
    -- Test world facet
    local world = self:world()
    if world ~= nil then
        print("[TEST] World facet works")
    end
    
    print("[TEST] Entity ID: " .. tostring(self:entity()))
end

function on_update(self, dt)
    test_state.update_count = test_state.update_count + 1
    
    -- Test input every 60 frames
    if test_state.update_count % 60 == 0 then
        local input = self:input()
        if input:is_key_down("W") then
            print("[TEST] W key is down")
        end
        if input:is_key_pressed("Space") then
            print("[TEST] Space key was pressed")
        end
    end
end

function on_fixed_update(self, fixed_dt)
    test_state.fixed_update_count = test_state.fixed_update_count + 1
    
    local physics = self:physics()
    if not physics then return end
    
    -- Test physics manipulation
    if test_state.fixed_update_count == 1 then
        -- Apply initial impulse
        physics:apply_impulse(vec2(50.0, -100.0))
        print("[TEST] Applied impulse")
    end
    
    -- Test velocity reading and setting
    if test_state.fixed_update_count % 120 == 0 then
        local vel = physics:velocity()
        print("[TEST] Current velocity: " .. tostring(vel.x) .. ", " .. tostring(vel.y))
    end
    
    -- Test transform manipulation
    local transform = self:transform()
    if transform then
        local pos = transform:position()
        if test_state.last_position == nil or 
           math.abs(pos.x - test_state.last_position.x) > 10.0 or
           math.abs(pos.y - test_state.last_position.y) > 10.0 then
            test_state.last_position = pos
            print("[TEST] Position changed: " .. tostring(pos.x) .. ", " .. tostring(pos.y))
        end
    end
end

function on_collision_enter(self, other_entity)
    test_state.collision_count = test_state.collision_count + 1
    print("[TEST] Collision enter with entity: " .. tostring(other_entity))
    
    -- Test sprite tint change on collision
    local sprite = self:sprite()
    if sprite ~= nil then
        local tint = {0.0, 1.0, 0.0, 1.0} -- Green on collision
        sprite:set_tint(tint)
    end
end

function on_collision_exit(self, other_entity)
    print("[TEST] Collision exit with entity: " .. tostring(other_entity))
    
    -- Reset sprite tint
    local sprite = self:sprite()
    if sprite ~= nil then
        sprite:set_tint({1.0, 0.5, 0.0, 1.0}) -- Back to orange
    end
end

function on_trigger_enter(self, other_entity)
    test_state.trigger_count = test_state.trigger_count + 1
    print("[TEST] Trigger enter with entity: " .. tostring(other_entity))
end

function on_trigger_exit(self, other_entity)
    print("[TEST] Trigger exit with entity: " .. tostring(other_entity))
end

function on_destroy(self)
    print("[TEST] on_destroy called")
    print("[TEST] Summary:")
    print("  - Updates: " .. tostring(test_state.update_count))
    print("  - Fixed Updates: " .. tostring(test_state.fixed_update_count))
    print("  - Collisions: " .. tostring(test_state.collision_count))
    print("  - Triggers: " .. tostring(test_state.trigger_count))
end

