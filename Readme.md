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

# 下载文件
```bash
./rssh -H x.x.x.x -p password -u user -P port download --remote "/xxxx/xxx/xxxxx.dat" --local /xxxx/xxx/xx
```
+ 如果 --local 是目录，要加 路径分割符(例如 / ) 结尾：
1. 如果目录存在，则自动下载到该目录
2. 如果目录不存在，则自动创建该目录

# 上传文件
```bash
./rssh -H x.x.x.x -p password -u user -P port upload --local /xxxx/xxx/xxx.jar --remote ${upload_dir}
```