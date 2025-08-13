-- 基岩方块定义
return {
    hardness = 999.0,
    transparent = false,
    solid = true,
    texture = "bedrock",
    light_level = 0,
    on_break = function(pos)
        return "Cannot break bedrock!"
    end
}