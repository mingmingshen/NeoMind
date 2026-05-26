# 安装部署

NeoMind 支持桌面应用和无头服务端两种运行模式。请根据实际场景选择：

- **桌面端**——个人使用、本地开发、带显示屏的边缘设备
- **服务端**——树莓派、边缘网关、云服务器及其他无显示环境

---

## 系统要求

| 组件 | 最低要求 | 推荐配置 |
|------|----------|----------|
| **操作系统** | macOS 12+、Windows 10+、Ubuntu 20.04+ | macOS 14+、Windows 11、Ubuntu 22.04+ |
| **内存** | 1 GB（仅服务端） | 2 GB（桌面端）；运行本地 LLM 建议 8 GB 以上 |
| **磁盘** | 200 MB（程序）+ 500 MB（数据） | 2 GB（含数据保留） |
| **网络** | 任意（云端 LLM 需要互联网） | 稳定连接 |
| **端口** | 9375（API）、1883（MQTT） | 同左 |

> NeoMind 非常轻量——典型的服务端仅占用约 50 MB 内存。通过 Ollama 运行本地 LLM 需要额外内存/显存（一个 4B 模型约需 3 GB）。使用云端 LLM 则无本地资源开销。

---

## 桌面端安装

桌面端将后端、前端和 Tauri 2.x 运行时打包为单一安装包——这是最快的上手方式。

### 第一步——下载

前往 [GitHub Releases](https://github.com/camthink-ai/NeoMind/releases/latest) 下载对应平台的安装包：

| 平台 | 文件 |
|------|------|
| macOS（Apple Silicon） | `NeoMind_{version}_aarch64.dmg` |
| macOS（Intel） | `NeoMind_{version}_x64.dmg` |
| Windows | `NeoMind_{version}_x64-setup.exe` 或 `.msi` |
| Linux（通用） | `NeoMind_{version}_amd64.AppImage` |
| Linux（Debian/Ubuntu） | `neomind_{version}_amd64.deb` |

### 第二步——安装

**macOS：** 双击 `.dmg` 文件，将 **NeoMind** 拖入 **应用程序** 文件夹。首次启动时，macOS 可能提示"无法验证开发者"——点击 **打开** 即可。

**Windows：** 运行 `.exe` 或 `.msi` 安装程序。若出现 SmartScreen 提示，点击 **更多信息** 然后选择 **仍要运行**。

**Linux（AppImage）：**

```bash
chmod +x NeoMind_*.AppImage
./NeoMind_*.AppImage
```

**Linux（deb）：**

```bash
sudo dpkg -i neomind_${version}_amd64.deb
sudo apt-get install -f
```

### 第三步——首次启动与登录

启动 NeoMind。应用会自动启动内置服务端并打开 Web 界面。

如果是全新安装，系统将跳转到 **初始化向导**（`/setup`）引导你创建管理员账户。如果已有账户，则会显示登录页面：

![登录页面——输入用户名 ① 和密码 ②，然后点击登录 ③](../../img/login.png)

使用右上角的 **语言** 切换器可更改界面语言。

![登录页面（已填写凭据）——用户名和密码已输入](../../img/login-filled.png)

**初始化向导——创建管理员账户：**

| 字段 | 要求 |
|------|------|
| **用户名** | 至少 3 个字符。创建后不可修改。 |
| **邮箱** | 可选。用于通知和密码找回。 |
| **密码** | 至少 8 个字符，须同时包含字母和数字。 |
| **时区** | 由浏览器自动检测。影响定时任务和时间显示。 |

创建管理员账户后，系统会引导你进入 **设置 > LLM 后端** 添加第一个 AI 模型。各提供商的配置说明请参阅 [系统设置](./02-settings.md)。

> **快速上手提示**：安装 [Ollama](https://ollama.com)，执行 `ollama pull qwen3:8b`，然后在设置中添加一个 Ollama 后端，地址为 `http://localhost:11434`，模型填写 `qwen3:8b`。

---

## 服务端安装

服务端模式将 NeoMind 作为无头后台服务运行。基本流程是：安装、启动、打开 Web 界面、创建管理员账户。

### 方式 A——一行命令安装（推荐）

```bash
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | sh
```

此脚本会自动检测操作系统和架构，将最新二进制文件下载到 `/usr/local/bin`（无 sudo 权限时为 `~/.local/bin`），并将 Web 界面部署到 `/var/www/neomind`。

**环境变量配置：**

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `VERSION` | 最新版 | 指定版本号，如 `0.8.0` |
| `INSTALL_DIR` | `/usr/local/bin` | 二进制文件安装目录 |
| `DATA_DIR` | `/var/lib/neomind` | 数据存储目录 |
| `USE_NGINX` | `false` | 是否安装并配置 Nginx 反向代理 |

```bash
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh \
  | VERSION=0.8.0 INSTALL_DIR=~/.local/bin DATA_DIR=~/.neomind sh
```

### 方式 B——手动安装二进制文件

```bash
VERSION=0.8.0

# 下载并解压
wget https://github.com/camthink-ai/NeoMind/releases/download/v${VERSION}/neomind-server-linux-amd64.tar.gz
tar xzf neomind-server-linux-amd64.tar.gz
sudo install -m 755 neomind /usr/local/bin/
sudo install -m 755 neomind-extension-runner /usr/local/bin/

# Web 界面
wget https://github.com/camthink-ai/NeoMind/releases/download/v${VERSION}/neomind-web-${VERSION}.tar.gz
sudo mkdir -p /var/www/neomind
sudo tar xzf neomind-web-${VERSION}.tar.gz -C /var/www/neomind

# 数据目录
sudo mkdir -p /var/lib/neomind
```

### 方式 C——从源码编译

前置条件：Rust 1.85+、Node.js 20+

```bash
git clone https://github.com/camthink-ai/NeoMind.git
cd NeoMind
cargo build --release
cd web && npm install && npm run build && cd ..
cargo run --release -p neomind-cli -- serve
```

编译产物位于 `target/release/`，Web 资源位于 `web/dist/`。

### 使用 systemd 管理服务（Linux）

生产环境中，建议通过 systemd 运行 NeoMind，以实现开机自启和故障自动恢复。

**1. 创建服务文件：**

```bash
sudo tee /etc/systemd/system/neomind.service > /dev/null <<EOF
[Unit]
Description=NeoMind Edge AI Platform
After=network.target

[Service]
Type=simple
User=neomind
Group=neomind
WorkingDirectory=/var/lib/neomind
ExecStart=/usr/local/bin/neomind serve
Restart=on-failure
RestartSec=10
Environment=RUST_LOG=info
Environment=NEOMIND_DATA_DIR=/var/lib/neomind
Environment=NEOMIND_BIND_ADDR=0.0.0.0:9375

[Install]
WantedBy=multi-user.target
EOF
```

**2. 创建服务用户并启动：**

```bash
sudo useradd --system --home-dir /var/lib/neomind --shell /usr/sbin/nologin neomind
sudo chown -R neomind:neomind /var/lib/neomind
sudo systemctl daemon-reload
sudo systemctl enable neomind
sudo systemctl start neomind
sudo systemctl status neomind
```

### 首次访问（服务端）

启动服务后，在浏览器中打开 `http://your-server:9375`。首次访问将看到初始化向导——与桌面端首次启动流程相同。

---

## 网络配置

### 默认端口

| 端口 | 协议 | 服务 | 用途 |
|------|------|------|------|
| 9375 | HTTP | API 服务端 | REST API、WebSocket、SSE |
| 1883 | MQTT | 内置 Broker | 设备遥测数据（启用时） |
| 11434 | HTTP | Ollama（外部） | 本地 LLM 后端（独立程序） |

### 防火墙

**桌面端 / 本地访问**——无需更改。

**服务端 / 远程访问：**

```bash
# UFW（Ubuntu）
sudo ufw allow 9375/tcp    # API 服务端
sudo ufw allow 1883/tcp    # MQTT Broker

# firewalld（CentOS/RHEL）
sudo firewall-cmd --permanent --add-port=9375/tcp
sudo firewall-cmd --permanent --add-port=1883/tcp
sudo firewall-cmd --reload
```

### Nginx 反向代理与 SSL

生产环境中，建议在 NeoMind 前面部署 Nginx 并启用 HTTPS：

```nginx
server {
    listen 443 ssl http2;
    server_name neomind.example.com;

    ssl_certificate     /etc/ssl/certs/neomind.crt;
    ssl_certificate_key /etc/ssl/private/neomind.key;

    root /var/www/neomind;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }

    location /api/ {
        proxy_pass http://127.0.0.1:9375/api/;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_read_timeout 86400;
    }
}

server {
    listen 80;
    server_name neomind.example.com;
    return 301 https://$host$request_uri;
}
```

使用 `sudo nginx -t && sudo systemctl reload nginx` 应用配置。

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `RUST_LOG` | `info` | 日志级别：trace、debug、info、warn、error |
| `NEOMIND_DATA_DIR` | `./data` | 数据存储目录 |
| `NEOMIND_BIND_ADDR` | `0.0.0.0:9375` | 服务端绑定地址 |
| `SERVER_PORT` | `9375` | API 服务端口 |

---

## 验证安装

安装完成后，执行以下命令确认一切正常：

```bash
neomind health              # 预期输出：Status: healthy
curl http://localhost:9375/api/health
mosquitto_pub -h localhost -p 1883 -t "test" -m "hello"  # 检查 MQTT
```

然后在浏览器中打开 `http://localhost:9375`（桌面端）或 `http://your-server:9375`（服务端）。

---

## 升级

### 桌面端

桌面端会自动检查更新。收到更新通知后，从 [GitHub Releases](https://github.com/camthink-ai/NeoMind/releases/latest) 下载新版安装包并覆盖安装即可。数据和配置会自动保留。

### 服务端（一行命令安装）

```bash
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | VERSION=0.8.0 sh
sudo systemctl restart neomind
```

### 服务端（手动安装）

```bash
VERSION=0.8.0
sudo systemctl stop neomind
sudo cp -r /var/lib/neomind /var/lib/neomind.bak   # 备份

wget https://github.com/camthink-ai/NeoMind/releases/download/v${VERSION}/neomind-server-linux-amd64.tar.gz
tar xzf neomind-server-linux-amd64.tar.gz
sudo install -m 755 neomind /usr/local/bin/
sudo install -m 755 neomind-extension-runner /usr/local/bin/

wget https://github.com/camthink-ai/NeoMind/releases/download/v${VERSION}/neomind-web-${VERSION}.tar.gz
sudo rm -rf /var/www/neomind/*
sudo tar xzf neomind-web-${VERSION}.tar.gz -C /var/www/neomind

sudo systemctl start neomind
```

> 升级前请务必备份数据目录。NeoMind 使用 redb 数据库，支持向前兼容但不保证向后兼容。

---

[下一篇：系统设置 >](./02-settings.md)
