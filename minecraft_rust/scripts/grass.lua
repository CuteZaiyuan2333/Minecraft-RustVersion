-- 草方块定义（顶部草纹理，侧面先用草顶/或后续扩展）
return {
    hardness = 1.2,
    transparent = false,
    solid = true,
    texture = "grass_block_top",
    light_level = 0,
    on_break = function(pos)
        return "Grass block broken at " .. tostring(pos)
    end
}