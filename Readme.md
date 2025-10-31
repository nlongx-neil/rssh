# 帮助说明
```
Rust SSH 工具

Usage: rssh [OPTIONS] --host <HOST> --user <USER> --password <PASSWORD> <COMMAND>

Commands:
  exec      执行命令
  upload    上传文件
  download  下载文件
  help      Print this message or the help of the given subcommand(s)

Options:
  -H, --host <HOST>          远程主机 IP
  -u, --user <USER>          用户名
  -p, --password <PASSWORD>  密码
  -P, --port <PORT>          端口号 [default: 22]
  -h, --help                 Print help
  -V, --version              Print version
```

# 执行命令
```bash
./rssh  -H x.x.x.x -p password -u user -P port exec "ls -alh"
```

# 文件传输
```bash
# 下载文件
./rssh -H x.x.x.x -p password -u user -P port download --remote remote_file --local local_path
# 上传文件
./rssh -H x.x.x.x -p password -u user -P port upload --local local_file --remote remote_path
```
1. local_path , remote_path 支持文件和目录，如果目录不存在，需要以 / 结尾
2. 支持sftp