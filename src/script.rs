use std::{
    collections::{HashMap, HashSet},
    process::exit,
    sync::Arc,
    time::Duration,
};

use rdev::{listen, simulate, Button, Event, EventType, Key, ListenError};
use serde::{Deserialize, Serialize};
use tokio::{
    sync::{mpsc, mpsc::UnboundedSender},
    task::JoinHandle,
};

pub type Title = (Arc<String>, bool);

pub struct ScriptList(pub Vec<Script>);

impl ScriptList {
    /// 监听脚本的触发
    pub fn listening(mut self) -> Result<(), ListenError> {
        let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

        tokio::spawn(async move {
            let mut keys = HashSet::new();
            let mut mouses = HashSet::new();
            for trigger in self.0.iter().flat_map(|f| f.trigger.keys()) {
                match trigger {
                    Trigger::Key(key) => keys.insert(*key),
                    Trigger::Mouse(mouse) => mouses.insert(*mouse),
                };
            }

            while let Some(event) = rx.recv().await {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        if keys.contains(&key) {
                            let key = Trigger::Key(key);
                            self.0.iter_mut().for_each(|item| item.down(&key));
                        }
                    }
                    EventType::KeyRelease(key) => {
                        if keys.contains(&key) {
                            let key = Trigger::Key(key);
                            self.0.iter_mut().for_each(|item| item.up(&key));
                        }
                    }
                    EventType::ButtonPress(button) => {
                        if mouses.contains(&button) {
                            let button = Trigger::Mouse(button);
                            self.0.iter_mut().for_each(|item| item.down(&button));
                        }
                    }
                    EventType::ButtonRelease(button) => {
                        if mouses.contains(&button) {
                            let button = Trigger::Mouse(button);
                            self.0.iter_mut().for_each(|item| item.up(&button));
                        }
                    }
                    _ => {}
                }
            }
        });

        listen(move |event| {
            let _ = tx.send(event);
        })
    }
}

#[derive(Debug)]
pub struct Script {
    pub title: Arc<String>,
    pub delay: u64,
    pub repeat: usize,
    pub methods: Arc<Vec<Method>>,
    pub task: Option<JoinHandle<()>>,
    pub trigger: HashMap<Trigger, bool>,
    pub updater: UnboundedSender<Title>,
}

impl Script {
    pub fn run(&mut self) {
        let title = self.title.clone();
        let updater = self.updater.clone();

        if let Some(task) = self.task.take() {
            if self.repeat == 0 || !task.is_finished() {
                task.abort();
                return updater.send((title.clone(), false)).unwrap();
            }
        }

        let _ = updater.send((title.clone(), true));

        let delay = self.delay;
        let repeat = self.repeat;
        let methods = self.methods.clone();

        let task = tokio::task::spawn(async move {
            if repeat == 0 {
                loop {
                    run_method(&methods, delay).await;
                }
            } else {
                for _ in 0..repeat {
                    run_method(&methods, delay).await;
                }
                let _ = updater.send((title, false));
            }
        });

        self.task = Some(task);
    }

    pub fn down(&mut self, key: &Trigger) {
        if let Some(k) = self.trigger.get_mut(key) {
            *k = true;

            if self.trigger.values().all(|flag| *flag) {
                self.run()
            }
        }
    }

    pub fn up(&mut self, key: &Trigger) {
        if let Some(k) = self.trigger.get_mut(key) {
            *k = false;
        }
    }
}

/// 运行脚本方法
async fn run_method(methods: &Arc<Vec<Method>>, delay: u64) {
    for method in methods.iter() {
        match method {
            Method::Event(event_type) => {
                if let Err(err) = simulate(event_type) {
                    println!("事件 {event_type:?} 执行失败: {err}");
                }
                if let EventType::MouseMove { .. } = event_type {
                    tokio::time::sleep(Duration::from_micros(100)).await;
                } else {
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                };
            }
            Method::Custom(c) => c.run().await,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Hash, Eq, Serialize, Deserialize)]
pub enum Trigger {
    Key(Key),
    Mouse(Button),
}

/// 自定义事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Custom {
    /// 睡眠 ms 毫秒
    Sleep(u64),

    /// 退出
    Exit(i32),
}

impl Custom {
    pub async fn run(&self) {
        match self {
            Custom::Sleep(n) => tokio::time::sleep(Duration::from_millis(*n)).await,
            Custom::Exit(code) => exit(*code),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Method {
    /// 事件
    Event(EventType),
    /// 自定义
    Custom(Custom),
}

impl Method {
    pub fn key_down(key: Key) -> Self {
        Self::Event(EventType::KeyPress(key))
    }
    pub fn key_up(key: Key) -> Self {
        Self::Event(EventType::KeyRelease(key))
    }
    pub fn mouse_down(button: Button) -> Self {
        Self::Event(EventType::ButtonPress(button))
    }
    pub fn mouse_up(button: Button) -> Self {
        Self::Event(EventType::ButtonRelease(button))
    }
}
