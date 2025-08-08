use config::{Config, ConfigError, Environment, File};
use quick_xml::events::{BytesText, Event};
use quick_xml::{Reader, Writer};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::io::Cursor;
use tokio;
use tracing::{info, error};
use warp::Filter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RssConfig {
    pub source_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversionConfig {
    pub default_priority: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub rss: RssConfig,
    pub conversion: ConversionConfig,
    pub logging: LoggingConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 3030,
            },
            rss: RssConfig {
                source_url: "https://example.com/rss.xml".to_string(),
            },
            conversion: ConversionConfig {
                default_priority: 100,
            },
            logging: LoggingConfig {
                level: "info".to_string(),
            },
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        // 先使用默认配置作为基础
        let default_config = AppConfig::default();
        
        let mut builder = Config::builder()
            .add_source(config::Config::try_from(&default_config)?)
            .add_source(File::with_name("config").required(false))
            .add_source(Environment::with_prefix("RSS_CONVERTER").separator("_"));

        // 支持直接的环境变量覆盖
        if let Ok(url) = std::env::var("RSS_SOURCE_URL") {
            builder = builder.set_override("rss.source_url", url)?;
        }
        if let Ok(host) = std::env::var("SERVER_HOST") {
            builder = builder.set_override("server.host", host)?;
        }
        if let Ok(port) = std::env::var("SERVER_PORT") {
            builder = builder.set_override("server.port", port)?;
        }
        if let Ok(priority) = std::env::var("CONVERSION_DEFAULT_PRIORITY") {
            builder = builder.set_override("conversion.default_priority", priority)?;
        }
        if let Ok(level) = std::env::var("LOGGING_LEVEL") {
            builder = builder.set_override("logging.level", level)?;
        }

        let config = builder.build()?;
        config.try_deserialize()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleRule {
    pub name: String,
    pub pattern: String,
    pub replacement: String,
    pub priority: u32,
}

#[derive(Debug, Clone)]
pub struct TitleConverter {
    rules: Vec<CompiledRule>,
}

#[derive(Debug, Clone)]
struct CompiledRule {
    name: String,
    regex: Regex,
    replacement: String,
    priority: u32,
}

impl TitleConverter {
    pub fn new() -> Self {
        let mut converter = TitleConverter { rules: Vec::new() };
        
        // 添加名侦探柯南的转换规则
        let conan_rule = TitleRule {
            name: "Detective Conan".to_string(),
            pattern: r"\[([^\]]+)\]\[名侦探柯南\]\[第(\d+)集\s+([^]]+)\]\[([^]]+)\]\[([^]]+)\](?:\[([^]]+)\])?\[([^]]+)\]".to_string(),
            replacement: " [$1] Detective Conan - $2 ($4 $7 $5) ".to_string(),
            priority: 1,
        };
        
        converter.add_rule(conan_rule);
        converter
    }
    
    pub fn add_rule(&mut self, rule: TitleRule) {
        match Regex::new(&rule.pattern) {
            Ok(regex) => {
                let compiled_rule = CompiledRule {
                    name: rule.name,
                    regex,
                    replacement: rule.replacement,
                    priority: rule.priority,
                };
                info!("Added conversion rule: {}", compiled_rule.name);
                self.rules.push(compiled_rule);
                // 按优先级排序
                self.rules.sort_by(|a, b| a.priority.cmp(&b.priority));
            }
            Err(e) => {
                error!("Invalid regex pattern for rule '{}': {}", rule.name, e);
            }
        }
    }
    
    pub fn convert_title(&self, original: &str) -> String {
        for rule in &self.rules {
            if let Some(captures) = rule.regex.captures(original) {
                let mut result = rule.replacement.clone();
                
                // 替换捕获组
                for i in 0..captures.len() {
                    let placeholder = format!("${}", i);
                    if let Some(capture) = captures.get(i) {
                        result = result.replace(&placeholder, capture.as_str());
                    }
                }
                
                info!("Title converted by rule '{}': {} -> {}", rule.name, original, result);
                return result;
            }
        }
        
        // 如果没有匹配的规则，返回原标题
        original.to_string()
    }
}

pub async fn fetch_and_convert_rss(url: &str, converter: &TitleConverter) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    info!("Fetching RSS from: {}", url);
    
    // 获取原始RSS内容
    let response = reqwest::get(url).await?;
    let rss_content = response.text().await?;
    
    // 使用quick-xml处理RSS，保留原始格式
    let mut reader = Reader::from_str(&rss_content);
    reader.trim_text(true);
    
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut buf = Vec::new();
    let mut inside_title = false;
    let mut inside_item = false;
    let mut current_title = String::new();
    
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name = e.name();
                if name.as_ref() == b"item" {
                    inside_item = true;
                    info!("Processing RSS item");
                }
                if name.as_ref() == b"title" && inside_item {
                    inside_title = true;
                    current_title.clear();
                }
                writer.write_event(Event::Start(e.clone()))?;
            }
            Ok(Event::End(ref e)) => {
                let name = e.name();
                if name.as_ref() == b"item" {
                    inside_item = false;
                }
                if name.as_ref() == b"title" && inside_title {
                    inside_title = false;
                    // 转换标题并写入
                    let converted_title = converter.convert_title(&current_title);
                    
                    // 直接写入转换后的标题作为文本
                    writer.write_event(Event::Text(BytesText::new(&converted_title)))?;
                }
                writer.write_event(Event::End(e.clone()))?;
            }
            Ok(Event::Text(ref e)) => {
                if inside_title && inside_item {
                    current_title.push_str(&e.unescape().unwrap_or_default());
                } else {
                    writer.write_event(Event::Text(e.clone()))?;
                }
            }
            Ok(Event::CData(ref e)) => {
                if inside_title && inside_item {
                    // CDATA内容，去掉CDATA标记获取实际内容
                    let cdata_content = String::from_utf8_lossy(e);
                    current_title.push_str(&cdata_content);
                } else {
                    writer.write_event(Event::CData(e.clone()))?;
                }
            }
            Ok(Event::Empty(ref e)) => {
                writer.write_event(Event::Empty(e.clone()))?;
            }
            Ok(Event::Comment(ref e)) => {
                writer.write_event(Event::Comment(e.clone()))?;
            }
            Ok(Event::Decl(ref e)) => {
                writer.write_event(Event::Decl(e.clone()))?;
            }
            Ok(Event::PI(ref e)) => {
                writer.write_event(Event::PI(e.clone()))?;
            }
            Ok(Event::DocType(ref e)) => {
                writer.write_event(Event::DocType(e.clone()))?;
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                error!("Error reading XML: {}", e);
                break;
            }
        }
        buf.clear();
    }

    let result = writer.into_inner().into_inner();
    let output = String::from_utf8(result)?;
    
    info!("RSS conversion completed");
    Ok(output)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载环境变量
    dotenvy::dotenv().ok();
    
    // 加载配置
    let config = AppConfig::load().unwrap_or_else(|err| {
        eprintln!("Error loading config: {}", err);
        eprintln!("Using default configuration");
        AppConfig::default()
    });
    
    // 初始化日志
    tracing_subscriber::fmt()
        .with_max_level(config.logging.level.parse().unwrap_or(tracing::Level::INFO))
        .init();
    
    info!("Starting RSS converter service");
    info!("Configuration loaded: RSS source = {}", config.rss.source_url);
    
    let converter = TitleConverter::new();
    
    // 测试转换功能
    let test_title = "[银色子弹字幕组][名侦探柯南][第1170集 食人教室的玄机（后篇）][WEBRIP][简繁日多语MKV][PGS][1080P]";
    let converted = converter.convert_title(test_title);
    info!("Test conversion - Original: {}", test_title);
    info!("Test conversion - Converted: {}", converted);
    
    // 创建共享配置和转换器
    let config_clone = config.clone();
    let app_config = warp::any().map(move || config_clone.clone());
    let converter_filter = warp::any().map(move || converter.clone());
    
    // 创建Web服务路由 - 不再需要URL参数
    let convert_route = warp::path("rss.xml")
        .and(warp::get())
        .and(app_config)
        .and(converter_filter)
        .and_then(handle_convert_request);
    
    // 健康检查端点
    let health_route = warp::path("health")
        .and(warp::get())
        .map(|| "OK");
    
    let routes = convert_route.or(health_route);
    
    let addr = (config.server.host.parse::<std::net::IpAddr>().unwrap_or([127, 0, 0, 1].into()), config.server.port);
    info!("RSS转换服务启动在 http://{}:{}", config.server.host, config.server.port);
    info!("使用方法: http://{}:{}/rss.xml", config.server.host, config.server.port);
    info!("健康检查: http://{}:{}/health", config.server.host, config.server.port);
    
    warp::serve(routes)
        .run(addr)
        .await;
    
    Ok(())
}

async fn handle_convert_request(
    config: AppConfig,
    converter: TitleConverter,
) -> Result<impl warp::Reply, warp::Rejection> {
    match fetch_and_convert_rss(&config.rss.source_url, &converter).await {
        Ok(rss_xml) => {
            info!("Successfully converted RSS feed");
            Ok(warp::reply::with_header(
                rss_xml,
                "content-type",
                "text/xml; charset=utf-8",
            ))
        }
        Err(e) => {
            error!("Error converting RSS: {}", e);
            Ok(warp::reply::with_header(
                format!("Error: {}", e),
                "content-type",
                "text/plain",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conan_title_conversion() {
        let converter = TitleConverter::new();
        let original = "[银色子弹字幕组][名侦探柯南][第1170集 食人教室的玄机（后篇）][WEBRIP][简繁日多语MKV][PGS][1080P]";
        let expected = " [银色子弹字幕组] Detective Conan - 1170 (WEBRIP 1080P 简繁日多语MKV) ";
        let result = converter.convert_title(original);
        assert_eq!(result, expected);
    }
    
    #[test]
    fn test_no_match_title() {
        let converter = TitleConverter::new();
        let original = "Some random title that doesn't match";
        let result = converter.convert_title(original);
        assert_eq!(result, original);
    }
    
    #[test]
    fn test_config_loading() {
        let config = AppConfig::default();
        assert_eq!(config.server.host, "127.0.0.1");
        assert_eq!(config.server.port, 3030);
    }
    
    #[test]
    fn test_conan_1167_title() {
        let converter = TitleConverter::new();
        let original = " [银色子弹字幕组][名侦探柯南][第1167集 17年前的真相 皇后的谋略][WEBRIP][简繁日多语MKV][1080P] ";
        let result = converter.convert_title(original);
        println!("Original: {}", original);
        println!("Result: {}", result);
        println!("Matched: {}", result != original);
    }
}