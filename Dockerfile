FROM rust:slim as builder

WORKDIR /usr/src/app

# 安装构建依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制构建文件
COPY Cargo.toml Cargo.lock ./
COPY src/ src/

# 构建应用
RUN cargo build --release

# 运行时镜像
FROM debian:bookworm-slim

# 安装运行时依赖
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# 创建非root用户
RUN groupadd -r rssconverter && useradd -r -g rssconverter rssconverter

# 创建工作目录和配置目录
WORKDIR /app
RUN mkdir -p /app/config && chown -R rssconverter:rssconverter /app

# 复制构建的二进制文件
COPY --from=builder /usr/src/app/target/release/arrs-rss-converter /app/

# 复制默认配置文件
COPY config.toml /app/config/

# 切换到非root用户
USER rssconverter

# 暴露端口
EXPOSE 3030

# 设置环境变量
ENV SERVER_HOST=0.0.0.0
ENV SERVER_PORT=3030
ENV RSS_SOURCE_URL=https://example.com/rss.xml
ENV RUST_LOG=info

# 启动应用
CMD ["./arrs-rss-converter"]