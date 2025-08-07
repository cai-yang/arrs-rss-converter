# ARRS RSS 转换器

一个用于转换RSS标题格式的Web服务，专为媒体归集服务器（*arrs）设计。

## 功能特性

- 🔄 **智能标题转换** - 使用正则表达式规则转换RSS标题
- ⚙️ **灵活配置** - 支持配置文件和环境变量配置
- 🎯 **格式保留** - 保持原始RSS的XML格式，包括CDATA等特殊格式
- 🚀 **高性能** - 基于Rust和Tokio的异步Web服务
- 📝 **日志记录** - 详细的转换过程日志

## 配置方式

### 1. 配置文件 (config.toml)

```toml
[server]
host = "127.0.0.1"
port = 3030

[rss]
source_url = "https://your-rss-source.com/rss.xml"

[conversion]
default_priority = 100

[logging]
level = "info"
```

### 2. 环境变量

创建 `.env` 文件：

```bash
RSS_SOURCE_URL=https://your-rss-source.com/rss.xml
SERVER_HOST=127.0.0.1
SERVER_PORT=3030
RUST_LOG=info
```

或直接设置环境变量：

```bash
export RSS_SOURCE_URL="https://your-rss-source.com/rss.xml"
export SERVER_HOST="127.0.0.1"
export SERVER_PORT=3030
```

### 配置优先级

环境变量 > 配置文件 > 默认值

## 使用方法

### 1. 启动服务

```bash
# 使用cargo运行
cargo run

# 或编译后运行
cargo build --release
./target/release/arrs-rss-converter
```

### 2. 访问转换后的RSS

```
GET http://localhost:3030/rss
```

### 3. 健康检查

```
GET http://localhost:3030/health
```

## 转换规则

### 名侦探柯南规则

**原标题格式：**
```
[银色子弹字幕组][名侦探柯南][第1170集 食人教室的玄机（后篇）][WEBRIP][简繁日多语MKV][PGS][1080P]
```

**转换后格式：**
```
 [银色子弹字幕组] 名侦探柯南 / Detective Conan - 1170 (WEBRIP 1080P 简繁日多语MKV) 
```

## 添加新的转换规则

在 `src/main.rs` 中的 `TitleConverter::new()` 方法里添加新规则：

```rust
let new_rule = TitleRule {
    name: "规则名称".to_string(),
    pattern: r"正则表达式模式".to_string(),
    replacement: "替换模板，使用 $1, $2 引用捕获组".to_string(),
    priority: 1, // 数字越小优先级越高
};
converter.add_rule(new_rule);
```

## 开发和测试

```bash
# 运行测试
cargo test

# 运行测试并显示输出
cargo test -- --nocapture

# 检查代码格式
cargo fmt

# 代码静态检查
cargo clippy
```

## Docker 支持（可选）

创建 `Dockerfile`：

```dockerfile
FROM rust:1.70 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bullseye-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/arrs-rss-converter /usr/local/bin/
EXPOSE 3030
CMD ["arrs-rss-converter"]
```

构建和运行：

```bash
docker build -t arrs-rss-converter .
docker run -p 3030:3030 -e RSS_SOURCE_URL="https://example.com/rss.xml" arrs-rss-converter
```

## 许可证

MIT License