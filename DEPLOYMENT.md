# ARRS RSS Converter 部署指南

## Docker Hub 自动构建

本项目配置了 GitHub Actions 自动构建和推送 Docker 镜像到 Docker Hub。

### 设置 GitHub Secrets

在 GitHub 仓库中设置以下 secrets：

```
DOCKERHUB_USERNAME: 你的 Docker Hub 用户名
DOCKERHUB_TOKEN: 你的 Docker Hub 访问令牌
```

### 镜像标签

- `latest`: 最新的 main 分支构建
- `v1.0.0`: 版本标签（当推送 git tag 时）
- `main`: main 分支构建

## Docker Compose 部署

### 1. 创建环境变量文件

创建 `.env` 文件：

```bash
# RSS源地址 (必需)
RSS_SOURCE_URL=https://your-rss-source.com/rss.xml

# 日志级别 (可选，默认: info)
RUST_LOG=info
```

### 2. 配置文件

确保 `config.toml` 文件存在并配置正确：

```toml
[server]
host = "0.0.0.0"
port = 3030

[rss]
source_url = "https://example.com/rss.xml"

[conversion]
default_priority = 100

[logging]
level = "info"
```

### 3. 启动服务

```bash
# 使用本地构建的镜像
docker-compose up -d

# 或使用 Docker Hub 镜像
# 修改 docker-compose.yml 中的 image 为: arrs-rss-converter:latest
docker-compose up -d
```

### 4. 验证服务

```bash
# 检查服务状态
docker-compose ps

# 查看日志
docker-compose logs -f

# 健康检查
curl http://localhost:3030/health

# 测试 RSS 转换
curl http://localhost:3030/rss.xml
```

## 环境变量配置

### 支持的环境变量

| 环境变量 | 描述 | 默认值 |
|---------|------|--------|
| `RSS_SOURCE_URL` | RSS源地址 | `https://example.com/rss.xml` |
| `SERVER_HOST` | 服务器监听地址 | `0.0.0.0` |
| `SERVER_PORT` | 服务器端口 | `3030` |
| `RUST_LOG` | 日志级别 | `info` |

### 配置优先级

环境变量 > 配置文件 > 默认值

## 端口说明

- **3030**: HTTP 服务端口
  - `/rss.xml` - 转换后的 RSS 输出
  - `/health` - 健康检查端点

## 数据持久化

### 配置文件持久化

Docker Compose 配置中已映射配置文件：

```yaml
volumes:
  - ./config.toml:/app/config/config.toml:ro
```

### 自定义配置

如需修改配置，编辑本地的 `config.toml` 文件，然后重启容器：

```bash
docker-compose restart
```

## 故障排除

### 查看日志

```bash
# 查看实时日志
docker-compose logs -f arrs-rss-converter

# 查看最近日志
docker-compose logs --tail=100 arrs-rss-converter
```

### 健康检查失败

```bash
# 检查容器状态
docker-compose ps

# 进入容器调试
docker-compose exec arrs-rss-converter /bin/bash

# 手动测试健康检查
curl http://localhost:3030/health
```

### 重新构建镜像

```bash
# 本地构建
docker-compose build --no-cache

# 清理并重新启动
docker-compose down
docker-compose up -d --build
```

## 生产环境建议

1. **资源限制**：在 docker-compose.yml 中添加资源限制
2. **日志轮转**：配置 Docker 日志轮转
3. **监控**：添加监控和告警
4. **备份**：备份配置文件
5. **安全**：使用非 root 用户运行（已配置）

## 示例配置

### 完整的 docker-compose.yml

```yaml
version: '3.8'

services:
  arrs-rss-converter:
    image: arrs-rss-converter:latest
    container_name: arrs-rss-converter
    ports:
      - "3030:3030"
    environment:
      - RSS_SOURCE_URL=${RSS_SOURCE_URL:-https://example.com/rss.xml}
      - SERVER_HOST=0.0.0.0
      - SERVER_PORT=3030
      - RUST_LOG=${RUST_LOG:-info}
    volumes:
      - ./config.toml:/app/config/config.toml:ro
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "--no-verbose", "--tries=1", "--spider", "http://localhost:3030/health"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 10s
    deploy:
      resources:
        limits:
          memory: 256M
          cpus: '0.5'
        reservations:
          memory: 128M
          cpus: '0.25'
```