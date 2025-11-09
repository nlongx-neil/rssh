use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use colored::*;
use indicatif::{ProgressBar, ProgressStyle};
use ssh2::{Session, Sftp};
use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{MAIN_SEPARATOR, Path, PathBuf};
use std::time::Duration;

/// 命令行参数
#[derive(Parser, Debug)]
#[command(name = "rssh", version, about = "Rust SSH 工具")]
pub struct Cli {
    /// 远程主机 IP
    #[arg(short = 'H', long)]
    host: String,

    /// 用户名
    #[arg(short, long)]
    user: String,

    /// 密码
    #[arg(short, long)]
    password: String,

    /// 端口号
    #[arg(short = 'P', long, default_value_t = 22)]
    port: u16,

    /// 子命令
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 执行命令
    Exec { cmd: String },
    /// 上传文件
    Upload {
        #[arg(short, long)]
        local: PathBuf,
        #[arg(short, long)]
        remote: PathBuf,
    },
    /// 下载文件
    Download {
        #[arg(short, long)]
        remote: PathBuf,
        #[arg(short, long)]
        local: PathBuf,
    },
}

pub struct Executor<'a> {
    cli: &'a Cli,
    session: Option<Session>,
}

impl<'a> Executor<'a> {
    pub fn new(cli: &'a Cli) -> Self {
        Self { cli, session: None }
    }

    pub fn remote_login(&mut self) -> Result<Session, i32> {
        if self.session.is_some() {
            return Ok(self.session.as_ref().unwrap().to_owned());
        }
        let addr = format!("{}:{}", self.cli.host, self.cli.port);
        let addr = addr
            .to_socket_addrs()
            .map_err(|e| {
                eprintln!("Failed to resolve address: {}", e);
                1
            })?
            .next()
            .ok_or_else(|| {
                eprintln!("Invalid address: {}", addr);
                1
            })?;

        let tcp = TcpStream::connect_timeout(&addr, Duration::from_secs(10)).map_err(|e| {
            eprintln!("Failed to connect TCP: {}", e);
            1
        })?;

        let mut sess = Session::new().map_err(|e| {
            eprintln!("Failed to create session: {}", e);
            1
        })?;

        sess.set_tcp_stream(tcp);
        sess.handshake().map_err(|e| {
            eprintln!("SSH handshake failed: {}", e);
            1
        })?;
        sess.userauth_password(&self.cli.user, &self.cli.password)
            .map_err(|e| {
                eprintln!("SSH authentication failed: {}", e);
                1
            })?;
        if !sess.authenticated() {
            eprintln!("Authentication failed");
            return Err(1);
        }
        self.session = Some(sess.to_owned());
        Ok(self.session.as_ref().unwrap().to_owned())
    }
}

pub fn run() -> Result<(), i32> {
    let cli = Cli::parse();
    let sess = Executor::new(&cli).remote_login()?;
    match cli.command {
        Commands::Exec { cmd } => run_command(&sess, &cmd).map_err(|exit_code| exit_code)?,
        Commands::Upload { local, remote } => upload_file(&sess, &local, &remote).map_err(|e| {
            println!("{}", format!("\nUpload failed:{}", e).green());
            1
        })?,
        Commands::Download { remote, local } => {
            download_file(&sess, &remote, &local).map_err(|e| {
                println!("{}", format!("\nDownload failed:{}", e).green());
                1
            })?
        }
    };
    Ok(())
}

/// 执行命令，实时彩色输出
fn run_command(sess: &Session, cmd: &str) -> Result<(), i32> {
    let mut channel = sess.channel_session().map_err(|e| {
        eprintln!("Failed to open channel: {}", e);
        1
    })?;
    channel.exec(cmd).map_err(|e| {
        eprintln!("Failed to execute command: {}", e);
        1
    })?;

    let mut buffer = [0u8; 1024];
    while let Ok(n) = channel.read(&mut buffer) {
        if n == 0 {
            break;
        }
        let text = String::from_utf8_lossy(&buffer[..n]);
        print!("{}", text.bright_white());
        std::io::stdout().flush().ok();
    }

    channel.wait_close().map_err(|e| {
        eprintln!("Channel wait close failed: {}", e);
        1
    })?;
    let exit_status = channel.exit_status().unwrap_or(-1);

    if exit_status == 0 {
        println!("{}", format!("\n[exit code: {}]", exit_status).green());
        Ok(())
    } else {
        println!("{}", format!("\n[exit code: {}]", exit_status).red());
        Err(exit_status)
    }
}

/// 上传文件（带进度条 + 自动建目录）
fn upload_file(sess: &Session, local: &Path, remote: &Path) -> Result<()> {
    let sftp = sess.sftp().context("failed to open sftp")?;

    let remote_path = if remote.is_dir() || remote.to_str().unwrap_or("").ends_with(MAIN_SEPARATOR)
    {
        remote.join(local.file_name().context("local  has no filename")?)
    } else {
        remote.to_path_buf()
    };

    if let Some(parent) = remote_path.parent() {
        create_remote_dir_recursive(&sftp, parent)?;
    }

    let mut file = File::open(local)
        .with_context(|| format!("failed to open local file {}", local.display()))?;
    let meta = file.metadata()?;
    let size = meta.len();

    let pb = ProgressBar::new(size);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})",
        )
        .unwrap(),
    );

    let mut remote_file = sftp
        .create(remote_path.as_path())
        .with_context(|| format!("failed to create remote file {}", remote_path.display()))?;

    let mut buf = [0u8; 8192];
    let mut transferred = 0u64;

    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        remote_file.write_all(&buf[..n])?;
        transferred += n as u64;
        pb.set_position(transferred);
    }

    pb.finish_with_message(format!(
        "✅ Uploaded {} → {} ({} bytes)",
        local.display(),
        remote.display(),
        size
    ));

    Ok(())
}

/// 下载文件（带进度条 + 自动建目录）
fn download_file(sess: &Session, remote: &Path, local: &Path) -> Result<()> {
    let sftp = sess.sftp().context("failed to open sftp")?;
    let mut remote_file = sftp
        .open(remote)
        .with_context(|| format!("failed to open remote file {}", remote.display()))?;

    let stat = sftp.stat(remote)?;
    let size = stat.size.unwrap_or(0);

    // 如果是目录，或者以 分割符结尾
    let local_path = if local.is_dir() || local.to_str().unwrap_or("").ends_with(MAIN_SEPARATOR) {
        local.join(remote.file_name().context("remote has no filename")?)
    } else {
        local.to_path_buf()
    };

    if let Some(parent) = local_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create local dir {}", parent.display()))?;
    }

    let pb = ProgressBar::new(size);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.yellow/blue}] {bytes}/{total_bytes} ({eta})",
        )
        .unwrap(),
    );

    let mut out = File::create(&local_path)?;
    let mut buf = [0u8; 8192];
    let mut transferred = 0u64;

    loop {
        let n = remote_file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        transferred += n as u64;
        pb.set_position(transferred);
    }

    pb.finish_with_message(format!(
        "✅ Downloaded {} → {} ({} bytes)",
        remote.display(),
        local_path.display(),
        transferred
    ));
    Ok(())
}

/// 递归创建远程目录
fn create_remote_dir_recursive(sftp: &Sftp, dir: &Path) -> Result<()> {
    let mut current = PathBuf::from("/");
    for comp in dir.components() {
        if let std::path::Component::Normal(name) = comp {
            current.push(name);
            match sftp.stat(&current) {
                Ok(stat) => {
                    if stat.is_dir() {
                        // 已存在且是目录，继续
                        continue;
                    } else {
                        // 已存在但不是目录，报错
                        anyhow::bail!("Remote path exists but is not a directory: {:?}", current);
                    }
                }
                Err(_) => {
                    // 不存在，则创建目录
                    match sftp.mkdir(&current, 0o755) {
                        Ok(_) => {
                            println!("📁 created remote directory: {}", current.display());
                        }
                        Err(e) => {
                            return Err(e).context(format!("failed to mkdir {:?}", current));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
