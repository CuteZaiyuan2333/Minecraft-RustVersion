-- 泥土方块定义
return {
    hardness = 1.0,
    transparent = false,
    solid = true,
    texture = "dirt",
    light_level = 0,
    on_break = function(pos)
        return "Dirt block broken at " .. tostring(pos)
    end
}