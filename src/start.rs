use std::{env, fs, fs::File, io, io::Write, os::windows::fs::OpenOptionsExt, path::PathBuf, process, time::Duration};

use serde::{Deserialize, Serialize};

/// 通过独享 lock 文件删写权限实现的单例程序
///
/// 此数据被 drop 将释放权限
#[cfg(target_os = "windows")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Restart {
    Prev,
    Curr,
    Many,
}

impl Restart {
    pub fn run(&self) -> io::Result<Option<File>> {
        match self {
            Restart::Prev => match _lock_file() {
                Ok(file) => Ok(Some(file)),
                Err(_) => Err(io::Error::other("已有实例在运行")),
            },
            Restart::Curr => match _lock_file() {
                Ok(file) => Ok(Some(file)),
                Err(_) => Ok(_about().and_then(|_| _lock_file().map(Some))?),
            },
            Restart::Many => Ok(None),
        }
    }
}

/// lock 文件路径
fn _lock_path() -> PathBuf {
    let exe = env::current_exe().unwrap();
    env::temp_dir()
        .join(exe.file_name().unwrap())
        .with_extension("exe.lock")
}

/// 获取 lock 文件读写权限并写入当前进程 pid
fn _lock_file() -> io::Result<File> {
    File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .share_mode(1) // 保留读取权限
        .open(_lock_path())
        .and_then(|mut file| {
            write!(file, "{}", process::id())?;
            Ok(file)
        })
}

/// 根据 lock 文件记录的 pid 结束进程
fn _about() -> io::Result<()> {
    let pid = fs::read_to_string(_lock_path())?;
    let status = process::Command::new("taskkill")
        .arg("/F") // 使用 /F 标志强制杀死进程
        .arg("/PID")
        .arg(pid)
        .status()?;

    std::thread::sleep(Duration::from_millis(500));
    match status.success() {
        true => Ok(()),
        false => Err(io::Error::other("获取 pid 失败无法结束进程")),
    }
}
