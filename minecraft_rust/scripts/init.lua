print("[Lua] init.lua loaded")

-- Example global function callable from Rust
function hello()
    print("Hello from Lua!")
end

-- 地面方块定义
blocks = {
    -- 基岩层
    bedrock = {
        hardness = 999.0,
        transparent = false,
        solid = true,
        texture = "stone",
        light_level = 0,
        on_break = function(pos)
            return "Cannot break bedrock!"
        end
    },
    
    -- 泥土层
    dirt = {
        hardness = 1.0,
        transparent = false,
        solid = true,
        texture = "dirt",
        light_level = 0,
        on_break = function(pos)
            return "Dirt block broken at " .. pos
        end
    },
    
    -- 草方块层
    grass = {
        hardness = 1.2,
        transparent = false,
        solid = true,
        texture = "grass",
        light_level = 0,
        on_break = function(pos)
            return "Grass block broken at " .. pos
        end
    },
    
    -- 石头（原始石头）
    stone = {
        hardness = 2.0,
        transparent = false,
        solid = true,
        texture = "stone",
        light_level = 0,
        on_break = function(pos)
            return "Stone block broken at " .. pos
        end
    }
}