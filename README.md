# Rust 代理服务器

这是一个使用Rust语言开发的高性能HTTP代理服务器，可以将请求从本地端口转发到指定的目标服务器。该项目使用Actix Web框架构建，支持HTTPS请求转发、自定义路径前缀、请求超时设置等功能。

## 功能特点

- 支持HTTP/HTTPS协议代理转发
- 可配置的请求超时时间
- 支持自签名证书（开发环境）
- 完整的请求/响应日志记录
- 可自定义代理路径前缀
- 跨域资源共享(CORS)支持
- 灵活的配置文件支持

## 安装说明

### 前置条件

- Rust 环境 (推荐使用 [rustup](https://rustup.rs/) 安装)
- Cargo 包管理器

### 安装步骤

1. 克隆仓库

```bash
git clone <repository-url>
cd rust_proxy
```

2. 编译项目

```bash
cargo build --release
```

3. 运行服务器

```bash
cargo run --release
```

## 配置说明

项目使用`config.toml`文件进行配置，支持以下配置项：

```toml
# 代理服务器配置
[server]
# 本地监听地址和端口
host = "127.0.0.1"
port = 3000

# 目标服务器配置
[target]
# 目标服务器地址
host = "172.22.32.12"
port = 8383
protocol = "https"

# 代理路径配置
[proxy]
# 代理路径前缀
path_prefix = "/federation-server"

# 请求配置
[request]
# 请求超时时间（秒）
timeout = 120
# 是否接受无效证书（仅用于开发环境）
accept_invalid_certs = true

# 日志配置
[log]
# 日志级别: error, warn, info, debug, trace
level = "info"
```

### 配置项说明

- **server**: 代理服务器自身的配置
  - `host`: 本地监听地址
  - `port`: 本地监听端口

- **target**: 目标服务器配置
  - `host`: 目标服务器地址
  - `port`: 目标服务器端口
  - `protocol`: 目标服务器协议(http/https)

- **proxy**: 代理配置
  - `path_prefix`: 代理路径前缀

- **request**: 请求相关配置
  - `timeout`: 请求超时时间(秒)
  - `accept_invalid_certs`: 是否接受无效证书

- **log**: 日志配置
  - `level`: 日志级别(error/warn/info/debug/trace)

## 使用方法

1. 启动服务器

```bash
cargo run --release
```

2. 发送请求

所有发往`http://127.0.0.1:3000/federation-server/...`的请求都会被转发到`https://172.22.32.12:8383/...`

### 示例

```bash
# 发送GET请求
curl http://127.0.0.1:3000/federation-server/api/data

# 发送POST请求
curl -X POST -H "Content-Type: application/json" -d '{"key":"value"}' http://127.0.0.1:3000/federation-server/api/submit
```

## 环境变量

除了配置文件外，还可以使用环境变量覆盖配置：

```bash
# 设置日志级别为debug
APP_LOG_LEVEL=debug cargo run

# 修改本地监听端口
APP_SERVER_PORT=8080 cargo run
```

## 错误处理

服务器会处理以下类型的错误：

- 请求构建错误 (400 Bad Request)
- 代理请求失败 (500 Internal Server Error)
- 读取响应体错误 (500 Internal Server Error)
- 无效的请求头 (400 Bad Request)
- 响应体转换错误 (500 Internal Server Error)
- 配置错误 (500 Internal Server Error)

## 开发说明

### 项目结构

- `src/main.rs`: 主程序代码
- `config.toml`: 配置文件
- `Cargo.toml`: 项目依赖配置

### 主要依赖

- actix-web: Web服务器框架
- reqwest: HTTP客户端
- config: 配置文件处理
- serde: 序列化/反序列化
- log/env_logger: 日志处理
- thiserror: 错误处理

## 许可证

[MIT](LICENSE)