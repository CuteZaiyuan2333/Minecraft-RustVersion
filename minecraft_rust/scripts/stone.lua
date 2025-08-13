-- 石头方块定义
return {
    hardness = 2.0,
    transparent = false,
    solid = true,
    texture = "stone",
    light_level = 0,
    
    -- 破坏时的回调
    on_break = function(pos)
        return "Stone block broken at " .. tostring(pos)
    end,
    
    -- 右键点击时的回调
    on_interact = function(pos, player)
        return "Stone is solid and sturdy"
    end,
    
    -- 方块放置时的回调
    on_place = function(pos)
        return "Stone placed at " .. tostring(pos)
    end
}