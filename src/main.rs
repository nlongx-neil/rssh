//! SSH 工具：执行命令 / 上传 / 下载
//! 特性：实时彩色输出 + 上传下载进度条
//! 编译要求：Rust edition = 2024
use rssh;

fn main() {
    if let Err(exit_code) = rssh::run() {
        std::process::exit(exit_code);
    }
}