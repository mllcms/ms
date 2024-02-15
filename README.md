# macro script

### 使用
```shell
# 运行脚本
./ms.exe
./ms.exe run ./config.toml

# 获取按键代码
./ms.exe event

# 获取坐标: AltGr(右) 获取当前鼠标坐标
./ms.exe point
```

###  在某些软件/游戏上可能没反应
- 这些软件可能是 root 权限打开的
- ms 也需要 root 权限打开才能生效
- 快速运行指定配置文件可以用快捷方式配置命令

### 配置说明
```toml
# 全局延迟
delay = 20
# 缩放比例
scaling = 1.5
# 偏移位置(一般双屏才用)
offset = [0, 0]
# 窗口位置
point = [800, 80]
# 字体大小
font_size = 20
# 字体颜色
font_color = [97, 218, 217]

# 脚本 XXX
[[scripts]]
# 显示标题
title = "配置说明"
# 使用单独延迟(可选)
delay = 10
# 重复次数(0 不会停止; 再次触发时循环结束会重启, 循环未结束会强制停止, 强制停止可能会导致事件按下未松发释放)
repeat = 1
# 触发按键(键盘 Key 鼠标 Mouse)
trigger = [{ Key = "Home" }]
# 脚本方法(每种事件后面都可以设置 Await 等待时间, 时间到才会继续执行下一个事件。单位 ms)
methods = [
    # 鼠标点击
    { Click = "Left", Await = 30 },
    # 鼠标松开
    { ClickUp = "Left" },
    # 鼠标按下
    { ClickDown = "Left" },
    # 鼠标点击指定位置
    { ClickOn = ["Left", 800, 500] },
    # 鼠标拖拽
    { ClickTo = ["Left", 200, 100, 240, 120] },
    # 按键点击
    { Key = "KeyA" },
    # 按键松开
    { KeyUp = "KeyA" },
    # 按键按下
    { KeyDown = "KeyA" },
    # 同时点击多个按键
    { Keys = ["KeyA", "KeyB"] },
    # 鼠标移动
    { Move = [400, 500] },
    # 滚轮移动
    { Scroll = [400, 500] },
    # 休眠时间
    { Sleep = 100 },
    # 退出程序
    { Exit = 0 },
    # 脚本块(命名: 需要在 blocks 中定义同名脚本块)
    { Block = { repeat = 10, block = "测试显示" } },
    # 脚本块(具体: 直接嵌套写入)
    { Block = { repeat = 10, block = [
        { Sleep = 100 }
    ] } }
]

# 程序退出
[[scripts]]
title = "程序退出"
repeat = 1
trigger = [{ Key = "End" }]
methods = [{ Exit = 0 }]

# 测试显示
[[scripts]]
title = "测试显示"
repeat = 1
trigger = [{ Key = "KpPlus" }]
methods = [{ Block = { repeat = 10, block = "测试显示" }, Await = 3000 }]

# 脚本块(复用脚本事件或者需要循环一部分事件时使用)
# 单处使用时 repeat 应大于 1
[blocks]
"测试显示" = [
    { Sleep = 100 }
]
```