use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs, mem,
    path::Path,
    sync::Arc,
};

use rdev::{Button, EventType, Key};
use serde::{Deserialize, Serialize};

use crate::{
    script::{Custom, Method, Script, ScriptList, Trigger},
    window::WindowList,
};

/// 脚本配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// 延迟
    pub delay: u64,
    /// 缩放
    pub scaling: f64,
    /// 偏移位置
    pub offset: (f64, f64),
    /// 窗口位置
    pub point: (f64, f64),
    /// 字体大小
    pub font_size: f64,
    /// 字体颜色
    pub font_color: (u8, u8, u8),
    /// 脚本列表
    pub scripts: Vec<ScriptConfig>,
    /// 脚本块
    #[serde(default)]
    pub blocks: HashMap<String, Vec<MethodConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptConfig {
    /// 脚本标题
    pub title: String,

    /// 循环次数
    pub repeat: usize,

    /// 单独配置延迟
    pub delay: Option<u64>,

    /// 触发按键
    pub trigger: Vec<Trigger>,

    /// 脚本事件
    pub methods: Vec<MethodConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodConfig {
    #[serde(flatten)]
    event: ScriptEvent,
    #[serde(rename = "Await")]
    await_: Option<u64>,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<(ScriptList, WindowList), Box<dyn Error>> {
        let data = fs::read_to_string(path)?;
        let mut config: Self = toml::from_str(&data)?;

        let hs: HashSet<&str> = config.scripts.iter().map(|m| m.title.as_str()).collect();
        if config.scripts.len() != hs.len() {
            return Err("title 不可重复".into());
        }

        let mut scripts = vec![];
        mem::swap(&mut config.scripts, &mut scripts);

        let window_list = WindowList::init(config.point, config.font_size, config.font_color);
        let script_list: Result<Vec<Script>, Box<dyn Error>> = scripts
            .into_iter()
            .map(|item| {
                Ok(Script {
                    title: Arc::new(item.title),
                    delay: item.delay.unwrap_or(config.delay),
                    trigger: item.trigger.into_iter().map(|m| (m, false)).collect(),
                    repeat: item.repeat,
                    task: None,
                    methods: Arc::new(config.to_methods(item.methods)?),
                    updater: window_list.updater.clone(),
                })
            })
            .collect();

        Ok((ScriptList(script_list?), window_list))
    }

    pub fn mouse_move(&self, x: f64, y: f64) -> EventType {
        let x = (x + self.offset.0) / self.scaling;
        let y = (y + self.offset.1) / self.scaling;
        EventType::MouseMove { x, y }
    }

    fn to_methods(&self, methods: Vec<MethodConfig>) -> Result<Vec<Method>, Box<dyn Error>> {
        let mut res = vec![];
        for method in methods {
            match method.event {
                ScriptEvent::ClickDown(button) => res.push(Method::mouse_down(button)),
                ScriptEvent::ClickUp(button) => res.push(Method::mouse_up(button)),
                ScriptEvent::Click(button) => {
                    res.push(Method::mouse_down(button));
                    res.push(Method::mouse_up(button));
                }
                ScriptEvent::ClickOn(button, x, y) => {
                    res.push(Method::Event(self.mouse_move(x, y)));
                    res.push(Method::mouse_down(button));
                    res.push(Method::mouse_up(button));
                }
                ScriptEvent::ClickTo(button, x, y, x2, y2) => {
                    res.push(Method::Event(self.mouse_move(x, y)));
                    res.push(Method::mouse_down(button));
                    res.push(Method::Event(self.mouse_move(x2, y2)));
                    res.push(Method::mouse_up(button));
                }
                ScriptEvent::KeyDown(key) => res.push(Method::key_down(key)),
                ScriptEvent::KeyUp(key) => res.push(Method::key_up(key)),
                ScriptEvent::Key(key) => {
                    res.push(Method::key_down(key));
                    res.push(Method::key_up(key));
                }
                ScriptEvent::Keys(keys) => {
                    keys.iter().for_each(|key| res.push(Method::key_down(*key)));
                    keys.iter().for_each(|key| res.push(Method::key_up(*key)));
                }
                ScriptEvent::Scroll(delta_x, delta_y) => res.push(Method::Event(EventType::Wheel { delta_x, delta_y })),
                ScriptEvent::Move(x, y) => res.push(Method::Event(self.mouse_move(x, y))),
                ScriptEvent::Sleep(n) => res.push(Method::Custom(Custom::Sleep(n))),
                ScriptEvent::Exit(n) => res.push(Method::Custom(Custom::Exit(n))),
                ScriptEvent::Block { repeat, block } => {
                    let block = match block {
                        Block::Name(name) => {
                            let block = self
                                .blocks
                                .get(&name)
                                .ok_or_else(|| format!("没有找到名为 {name:?} 的 block"))?
                                .to_owned();
                            if block.iter().any(|a| a.event.block_has(&name)) {
                                return Err(format!("不能引用自身同名 {name:?} 的 block").into());
                            }
                            block
                        }
                        Block::Block(block) => block,
                    };
                    let block = self.to_methods(block)?;
                    for _ in 0..repeat {
                        res.extend(block.iter().cloned());
                    }
                    res.pop();
                }
            }
            if let Some(n) = method.await_ {
                res.push(Method::Custom(Custom::Sleep(n)))
            }
        }
        Ok(res)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScriptEvent {
    /// 鼠标点击
    Click(Button),

    /// 鼠标松开
    ClickUp(Button),

    /// 鼠标按下
    ClickDown(Button),

    /// 点击指定位置
    ClickOn(Button, f64, f64),

    /// 拖拽到指定位置
    ClickTo(Button, f64, f64, f64, f64),

    /// 触发单按键
    Key(Key),

    /// 键盘松开
    KeyUp(Key),

    /// 键盘按下
    KeyDown(Key),

    /// 触发多个按键
    Keys(Vec<Key>),

    /// 移动鼠标到指定位置
    Move(f64, f64),

    /// 滚轮
    Scroll(i64, i64),

    /// 休眠时间
    Sleep(u64),

    /// 退出程序
    Exit(i32),

    /// 脚本块
    Block { repeat: usize, block: Block },
}

impl ScriptEvent {
    pub fn block_has(&self, name: &str) -> bool {
        matches!(self,ScriptEvent::Block { block: Block::Name(n), .. } if n == name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Block {
    Name(String),
    Block(Vec<MethodConfig>),
}
