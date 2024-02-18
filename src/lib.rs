use std::{
    ops::Sub,
    path::PathBuf,
    process::exit,
    time::{Duration, Instant},
};

use anyhow::anyhow;
use clap::{Parser, Subcommand};
use rdev::{listen, Event, EventType, Key};
use tokio::task::spawn_blocking;

use crate::config::{Config, ScriptEvent};

pub mod config;
pub mod script;
pub mod start;
pub mod window;

/// 键鼠宏脚本(无反应或需 root 启动)
#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    sub_command: Option<Commands>,
}

impl Cli {
    pub async fn run(self) {
        match self.sub_command.unwrap_or_default() {
            Commands::Run { config } => {
                if let Err(err) = run(config).await {
                    println!("{err}");
                    tokio::time::sleep(Duration::from_secs(60)).await;
                }
            }
            Commands::Event => event(),
            Commands::Point => point(),
            Commands::Record => record(),
        }
    }
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// 运行脚本
    Run {
        /// 配置文件所在路径
        config: PathBuf,
    },
    /// 获取事件代码
    Event,
    /// 获取鼠标坐标 PS: Alt 输出当前坐标; Escape 清屏
    Point,
    /// 录制事件
    Record,
}

impl Default for Commands {
    fn default() -> Self {
        Self::Run { config: PathBuf::from("config.toml") }
    }
}

/// 获取事件代码
fn event() {
    fn callback(event: Event) {
        match event.event_type {
            EventType::KeyRelease(key) => {
                println!("🖮 -> {key:?}");
            }
            EventType::ButtonRelease(button) => {
                println!("🖰 -> {button:?}");
            }
            _ => {}
        }
    }
    let _ = listen(callback);
}

async fn run(path: PathBuf) -> anyhow::Result<()> {
    let config = Config::parse(path)?;
    let _only_app = config.start.run().map_err(|err| anyhow!("启动失败: {err}"))?;

    let (script, window) = config.load()?;
    tokio::select! {
        res = spawn_blocking(move || script.listening()) => {
            res?.map_err(|err|anyhow!("监听异常: {err:?}"))
        }
        res = spawn_blocking(move || window.run()) => {
            res?.map_err(|err|anyhow!("窗口异常: {err}"))
        }
    }
}

/// 获取坐标
fn point() {
    let mut point = (0.0, 0.0);
    let callback = move |event: Event| match event.event_type {
        EventType::MouseMove { x, y } => {
            point = (x, y);
        }
        EventType::KeyRelease(Key::AltGr) => {
            println!("{}, {}", point.0, point.1)
        }
        EventType::KeyRelease(Key::Escape) => {
            println!("\x1B[2J\x1B[1;1H");
        }
        _ => {}
    };
    let _ = listen(callback);
}

/// 录制事件
fn record() {
    let mut point = (0.0, 0.0);
    let mut prev = Instant::now();
    let mut res = vec![];
    let callback = move |event: Event| {
        if let EventType::MouseMove { x, y } = event.event_type {
            if x + y < 1_f64 {
                for item in res.iter() {
                    println!("{}", toml::to_string_pretty(&item).unwrap())
                }
                exit(0);
            }
            point = (x, y);
            return;
        }

        let curr = Instant::now();
        res.push(ScriptEvent::Sleep(curr.sub(prev).as_millis() as u64));
        prev = curr;

        match event.event_type {
            EventType::KeyPress(key) => res.push(ScriptEvent::KeyDown(key)),
            EventType::KeyRelease(key) => res.push(ScriptEvent::KeyUp(key)),
            EventType::ButtonPress(button) => {
                res.push(ScriptEvent::Move(point.0, point.1));
                res.push(ScriptEvent::ClickDown(button))
            }
            EventType::ButtonRelease(button) => {
                res.push(ScriptEvent::Move(point.0, point.1));
                res.push(ScriptEvent::ClickUp(button))
            }
            EventType::Wheel { delta_x, delta_y } => res.push(ScriptEvent::Scroll(delta_x, delta_y)),
            _ => {}
        }
    };
    let _ = listen(callback);
}
