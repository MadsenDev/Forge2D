-- Animated agent script
-- Demonstrates animation system in Lua

function on_start(self)
    print("[animated_agent] Agent script started")
    local anim = self:animation()
    if anim then
        anim:play()
        anim:set_speed(1.0)  -- Normal speed
        print("[animated_agent] Animation started")
    else
        print("[animated_agent] WARNING: No animation component found!")
    end
end

function on_update(self, dt)
    local anim = self:animation()
    if anim then
        -- Update animation each frame
        anim:update(dt)
        
        -- Example: Change speed based on movement
        local physics = self:physics()
        if physics then
            local vel = physics:velocity()
            local speed = math.sqrt(vel.x * vel.x + vel.y * vel.y)
            if speed > 50.0 then
                anim:set_speed(2.0)  -- Fast movement = fast animation
            else
                anim:set_speed(1.0)  -- Normal speed
            end
        end
    end
end

