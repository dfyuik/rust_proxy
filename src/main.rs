// 导入所需的外部库
use actix_cors::Cors; // 用于处理跨域资源共享(CORS)
use actix_web::{App, HttpRequest, HttpResponse, HttpServer, Result, middleware, web}; // Actix Web框架核心组件
use config::{Config, ConfigError, File, FileFormat}; // 用于加载和处理配置文件
use env_logger; // 用于配置和初始化日志
use reqwest::Client; // HTTP客户端，用于发送请求
use serde::Deserialize; // 用于反序列化JSON/TOML等格式
use std::time::Duration; // 用于处理时间和超时
use thiserror::Error; // 简化错误处理的宏

// ==================== 配置结构体定义 ====================

// 服务器配置：定义代理服务器自身的监听地址和端口
#[derive(Debug, Deserialize, Clone)] // 自动实现Debug、Deserialize和Clone特性
struct ServerConfig {
    host: String, // 服务器主机地址
    port: u16,    // 服务器端口号
}

// 目标服务器配置：定义要代理的目标服务器信息
#[derive(Debug, Deserialize, Clone)]
struct TargetConfig {
    host: String,     // 目标服务器主机地址
    port: u16,        // 目标服务器端口号
    protocol: String, // 协议(http/https)
}

// 代理配置：定义代理服务的基本设置
#[derive(Debug, Deserialize, Clone)]
struct ProxyConfig {
    path_prefix: String, // 代理的URL路径前缀
}

// 请求配置：定义HTTP请求的相关设置
#[derive(Debug, Deserialize, Clone)]
struct RequestConfig {
    timeout: u64,               // 请求超时时间(秒)
    accept_invalid_certs: bool, // 是否接受无效的SSL证书
}

// 日志配置：定义日志相关设置
#[derive(Debug, Deserialize, Clone)]
struct LogConfig {
    level: String, // 日志级别(debug/info/warn/error)
}

// 应用总配置：包含所有子配置
#[derive(Debug, Deserialize, Clone)]
struct AppConfig {
    server: ServerConfig,   // 服务器配置
    target: TargetConfig,   // 目标服务器配置
    proxy: ProxyConfig,     // 代理配置
    request: RequestConfig, // 请求配置
    log: LogConfig,         // 日志配置
    #[serde(default = "default_config_path")] // 使用默认函数提供默认值
    config_path: String, // 配置文件路径
}

// 为config_path提供默认值的函数
fn default_config_path() -> String {
    "config.toml".to_string() // 默认配置文件为当前目录下的config.toml
}

// ==================== 初始化函数 ====================

// 加载配置和初始化日志的函数
fn init() -> Result<(AppConfig, Client), ProxyError> {
    // 1. 构建配置加载器
    let settings = Config::builder()
        // 添加配置文件源，不强制要求文件存在
        .add_source(File::new("config.toml", FileFormat::Toml).required(false))
        // 添加环境变量源，以APP_为前缀的环境变量会覆盖配置文件中的同名设置
        .add_source(config::Environment::with_prefix("APP"))
        .build()?; // 构建配置，如果失败则返回错误

    // 2. 将配置反序列化到AppConfig结构体中
    let app_config: AppConfig = settings.try_deserialize()?;

    // 3. 根据配置设置日志级别并初始化日志系统
    env_logger::Builder::from_env(env_logger::Env::new().default_filter_or(&app_config.log.level))
        .init();

    // 4. 构建HTTP客户端
    let client = Client::builder()
        // 设置是否接受无效证书
        .danger_accept_invalid_certs(app_config.request.accept_invalid_certs)
        // 设置请求超时时间
        .timeout(Duration::from_secs(app_config.request.timeout))
        .build()
        .unwrap(); // 如果构建失败则panic

    // 5. 输出配置信息到日志
    log::info!("配置文件路径: {}", app_config.config_path);
    log::info!(
        "服务器配置: {}:{}",
        app_config.server.host,
        app_config.server.port
    );
    log::info!(
        "目标服务器: {}://{}:{}",
        app_config.target.protocol,
        app_config.target.host,
        app_config.target.port
    );
    log::info!("代理路径前缀: {}", app_config.proxy.path_prefix);
    log::info!("请求超时: {}秒", app_config.request.timeout);
    log::info!("接受无效证书: {}", app_config.request.accept_invalid_certs);

    // 6. 返回配置和HTTP客户端
    Ok((app_config, client))
}

// ==================== 错误处理 ====================

// 定义自定义错误类型，用于统一处理各种可能的错误
#[derive(Error, Debug)] // 使用thiserror宏自动实现Error特性
enum ProxyError {
    #[error("请求构建失败: {0}")] // 错误消息模板
    RequestBuilderError(String), // 请求构建错误，如URL解析失败

    #[error("代理请求失败: {0}")]
    RequestError(#[from] reqwest::Error), // HTTP请求错误，#[from]表示可以自动从reqwest::Error转换

    #[error("读取响应体错误: {0}")]
    ResponseReadError(#[from] std::io::Error), // IO错误，如读取响应体失败

    #[error("无效的请求头: {0}")]
    InvalidHeader(String), // 请求头无效错误

    #[error("响应体转换错误")]
    ResponseBodyConversionError, // 响应体转换错误，如非UTF-8编码

    #[error("配置错误: {0}")]
    ConfigError(#[from] ConfigError), // 配置加载错误
}

// 将自定义错误转换为actix_web可以处理的HTTP响应
impl actix_web::error::ResponseError for ProxyError {
    fn error_response(&self) -> HttpResponse {
        match self {
            // 根据不同错误类型返回不同的HTTP状态码和错误信息
            ProxyError::RequestBuilderError(_) => {
                // 请求构建错误返回400 Bad Request
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "请求构建失败",
                    "details": self.to_string()
                }))
            }
            ProxyError::RequestError(_) => {
                // 请求错误返回500 Internal Server Error
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "代理请求失败",
                    "details": self.to_string()
                }))
            }
            ProxyError::ResponseReadError(_) => {
                // 响应读取错误返回500
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "读取响应体错误",
                    "details": self.to_string()
                }))
            }
            ProxyError::InvalidHeader(_) => {
                // 无效请求头返回400
                HttpResponse::BadRequest().json(serde_json::json!({
                    "error": "无效的请求头",
                    "details": self.to_string()
                }))
            }
            ProxyError::ResponseBodyConversionError => {
                // 响应体转换错误返回500
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "响应体转换错误"
                }))
            }
            ProxyError::ConfigError(_) => {
                // 配置错误返回500
                HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": "配置错误",
                    "details": self.to_string()
                }))
            }
        }
    }
}

// ==================== 代理请求处理 ====================

// 构建代理请求：将客户端请求转换为发送给目标服务器的请求
async fn build_proxy_request(
    req: &HttpRequest, // 原始客户端请求
    body: &web::Bytes, // 请求体
    backend_url: &str, // 目标URL
    client: &Client,   // HTTP客户端
) -> Result<reqwest::RequestBuilder, ProxyError> {
    // 1. 解析URL，确保格式正确
    let url = reqwest::Url::parse(backend_url)
        .map_err(|parse_err| ProxyError::RequestBuilderError(parse_err.to_string()))?;

    // 2. 创建请求构建器，使用与原始请求相同的HTTP方法
    let mut proxy_req = client.request(req.method().clone(), url);

    // 3. 复制原始请求的头部信息
    for (key, value) in req.headers() {
        // 跳过特定的头部，这些会由客户端自动处理
        if key != "host" && key != "content-length" && key != "transfer-encoding" {
            // 尝试将头部值转换为字符串
            let value_str = value
                .to_str()
                .map_err(|_| ProxyError::InvalidHeader(key.to_string()))?;
            proxy_req = proxy_req.header(key, value_str);
        }
    }

    // 4. 添加请求体（如果有）
    if !body.is_empty() {
        proxy_req = proxy_req.body(body.clone());
    }

    // 5. 返回构建好的请求
    Ok(proxy_req)
}

// 代理处理函数：处理所有进入的HTTP请求
async fn proxy_handler(
    req: HttpRequest,             // 客户端请求
    body: web::Bytes,             // 请求体
    client: web::Data<Client>,    // HTTP客户端（从应用状态获取）
    config: web::Data<AppConfig>, // 应用配置（从应用状态获取）
) -> Result<HttpResponse, ProxyError> {
    // 1. 构建目标URL
    let backend_url = format!(
        "{}://{}:{}{}",
        config.target.protocol,
        config.target.host,
        config.target.port,
        req.uri()
            .path_and_query() // 获取路径和查询参数
            .map(|pq| pq.as_str())
            .unwrap_or("")
    );

    // 2. 记录请求详情
    log::info!("=== 请求详情 ===");
    log::info!("代理请求地址: {}", backend_url);
    log::info!("请求方法: {}", req.method());
    log::info!("请求头: {:?}", req.headers());
    log::info!("查询参数: {:?}", req.query_string());
    log::info!("客户端IP: {:?}", req.peer_addr());

    // 3. 构建并发送代理请求
    let proxy_req = build_proxy_request(&req, &body, &backend_url, &client).await?;
    let response = proxy_req.send().await?;

    // 4. 获取响应状态码并创建响应构建器
    let status = response.status();
    let mut client_resp = HttpResponse::build(status);

    // 5. 复制响应头
    for (key, value) in response.headers() {
        // 跳过特定的头部
        if key != "content-length" && key != "transfer-encoding" {
            client_resp.insert_header((key.clone(), value.clone()));
        }
    }

    // 6. 获取响应体
    let bytes = response.bytes().await.map_err(ProxyError::RequestError)?;

    // 7. 记录响应详情
    log::info!("=== 响应详情 ===");
    log::info!("响应状态码: {}", status);
    log::info!("响应体大小: {} bytes", bytes.len());

    // 8. 尝试将响应体转换为字符串并记录（仅用于调试）
    if let Ok(body_str) = String::from_utf8(bytes.to_vec()) {
        log::debug!("响应体: {}", body_str);
        Ok(client_resp.body(bytes)) // 返回响应
    } else {
        // 如果响应体不是有效的UTF-8文本（如二进制数据）
        log::warn!("响应体无法转换为 UTF-8 字符串");
        Err(ProxyError::ResponseBodyConversionError) // 返回错误
    }
}

// ==================== 主函数 ====================

// 程序入口点
#[actix_web::main] // 创建异步运行时环境
async fn main() -> std::io::Result<()> {
    // 1. 加载配置和初始化日志
    let (config, client) = init().map_err(|e| {
        eprintln!("初始化失败: {}", e);
        std::io::Error::new(std::io::ErrorKind::Other, e) // 转换为IO错误
    })?;

    // 2. 在闭包外部创建共享数据
    let client_data = web::Data::new(client); // 包装HTTP客户端
    let config_data = web::Data::new(config.clone()); // 包装配置

    // 3. 启动 Actix Web 服务器
    HttpServer::new(move || {
        // 配置CORS（跨源资源共享）
        let cors = Cors::default()
            .allow_any_origin() // 允许任何来源的请求
            .allow_any_method() // 允许任何HTTP方法（GET, POST等）
            .allow_any_header() // 允许任何请求头
            .supports_credentials(); // 允许携带认证信息（如cookies）

        // 创建应用程序
        App::new()
            .wrap(cors) // 添加CORS中间件
            .wrap(middleware::Logger::default()) // 添加日志中间件
            .app_data(client_data.clone()) // 注册HTTP客户端（克隆包装器而不是内容）
            .app_data(config_data.clone()) // 注册配置（克隆包装器而不是内容）
            .service(
                // 设置路由：使用配置的路径前缀
                web::scope(&config.proxy.path_prefix) // 创建一个带前缀的路由组
                    .default_service(web::route().to(proxy_handler)), // 所有请求都由proxy_handler处理
            )
    })
    .bind(format!("{}:{}", config.server.host, config.server.port))? // 绑定到配置的地址和端口
    .run() // 运行服务器
    .await // 等待服务器运行完成
}
