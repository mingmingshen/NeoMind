# README 安装说明分析

## 🔍 检查结果

### ✅ 正确的部分

1. **安装脚本路径** ✅
   - `https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh`
   - 文件存在且正确

2. **下载文件名格式** ✅
   - `neomind-server-${PLATFORM}.tar.gz`
   - 与 CI/CD 构建产物一致
   - 脚本 line 109, 182 确认

3. **平台标识** ✅
   - `linux-amd64`, `linux-arm64`
   - `darwin-amd64`, `darwin-arm64`
   - 与 install.sh 中的命名一致

4. **Web UI 嵌入描述** ✅
   - "Web UI embedded" 描述准确
   - 使用 rust-embed 技术
   - 无需额外部署前端

5. **服务文件** ✅
   - `scripts/neomind.service` 存在
   - 适用于 systemd (Linux)

### ❌ 需要修正的部分

#### 1. **版本号过旧** (必须修复)

**当前 README:**
```bash
VERSION=0.5.11
```

**实际当前版本:**
```bash
VERSION=0.6.0  # 从 Cargo.toml 确认
```

**需要修改的地方:**
```bash
# 第 1 处: One-line installation (specific version)
curl -fsSL ... | VERSION=0.6.0 sh  # 原 0.5.11

# 第 2 处: Manual installation
wget .../v0.6.0/neomind-server-linux-amd64.tar.gz  # 原 v0.5.11
```

#### 2. **缺少扩展进程说明** (建议添加)

**当前状态：**
- 只提到 `neomind` 二进制
- 没有说明 `neomind-extension-runner`

**应该补充：**
```bash
tar xzf neomind-server-linux-amd64.tar.gz
# 解压后包含:
#   - neomind                    # 主程序
#   - neomind-extension-runner   # 扩展进程
```

#### 3. **systemd 服务文件路径问题** (需要确认)

**README 中的命令:**
```bash
sudo cp scripts/neomind.service /etc/systemd/system/
```

**问题：**
- 用户下载的是 tar.gz，里面没有 `scripts/` 目录
- 应该使用 install.sh 自动生成，或者从 GitHub 单独下载

**建议修改为:**
```bash
# 方法 1: 使用 install.sh (推荐，自动安装服务)
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | sh

# 方法 2: 手动创建服务
sudo tee /etc/systemd/system/neomind.service >/dev/null <<EOF
[Unit]
Description=NeoMind Edge AI Platform
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=neomind
Group=neomind
WorkingDirectory=/var/lib/neomind
ExecStart=/usr/local/bin/neomind
Restart=always
RestartSec=3

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable neomind
sudo systemctl start neomind
```

---

## 📝 建议的完整修正

### 修正前

```markdown
### 🖥️ Server Binary Deployment

> **✨ Out of the Box**: The server binary has the Web UI embedded.

**One-line installation (Linux & macOS):**

\`\`\`bash
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | sh
\`\`\`

**Install specific version:**

\`\`\`bash
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | VERSION=0.5.11 sh
\`\`\`

**Manual installation:**

\`\`\`bash
# Download binary (replace VERSION and PLATFORM)
# PLATFORM: linux-amd64, linux-arm64, darwin-amd64, darwin-arm64
wget https://github.com/camthink-ai/NeoMind/releases/download/v0.5.11/neomind-server-linux-amd64.tar.gz
tar xzf neomind-server-linux-amd64.tar.gz

# Run directly - Web UI included!
./neomind

# Or install system-wide
sudo install -m 755 neomind /usr/local/bin/

# Create systemd service (Linux)
sudo cp scripts/neomind.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable neomind
sudo systemctl start neomind
\`\`\`
```

### 修正后

```markdown
### 🖥️ Server Binary Deployment

> **✨ Out of the Box**: The server binary has the Web UI embedded. Just download and run - no additional frontend deployment needed!

The server package includes:
- **neomind** - Main server binary with embedded Web UI (~50 MB)
- **neomind-extension-runner** - Extension process for sandboxed extensions (~8 MB)

**One-line installation (Linux & macOS):**

\`\`\`bash
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | sh
\`\`\`

**Install specific version:**

\`\`\`bash
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | VERSION=0.6.0 sh
\`\`\`

**Custom installation:**

\`\`\`bash
# Custom directories
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | INSTALL_DIR=~/.local/bin DATA_DIR=~/.neomind sh

# Skip service installation
curl -fsSL https://raw.githubusercontent.com/camthink-ai/NeoMind/main/scripts/install.sh | NO_SERVICE=true sh
\`\`\`

**Supported platforms:**
- Linux (x86_64/amd64, aarch64/arm64)
- macOS (Intel, Apple Silicon)

**What the script does:**
1. Detects your OS and architecture automatically
2. Downloads the correct binary with embedded Web UI
3. Installs to `/usr/local/bin` (or custom directory)
4. Creates systemd service (Linux) or launchd service (macOS)
5. Starts the service automatically

---

**Manual installation:**

\`\`\`bash
# 1. Download binary (replace VERSION and PLATFORM)
# PLATFORM: linux-amd64, linux-arm64, darwin-amd64, darwin-arm64
wget https://github.com/camthink-ai/NeoMind/releases/download/v0.6.0/neomind-server-linux-amd64.tar.gz

# 2. Extract (contains neomind + neomind-extension-runner)
tar xzf neomind-server-linux-amd64.tar.gz

# 3. Run directly - Web UI included!
./neomind serve

# Or install system-wide
sudo install -m 755 neomind /usr/local/bin/
sudo install -m 755 neomind-extension-runner /usr/local/bin/

# 4. (Optional) Create systemd service (Linux)
sudo useradd -r -s /bin/false -d /var/lib/neomind neomind 2>/dev/null || true
sudo mkdir -p /var/lib/neomind
sudo chown -R neomind:neomind /var/lib/neomind

sudo tee /etc/systemd/system/neomind.service >/dev/null <<'EOF'
[Unit]
Description=NeoMind Edge AI Platform
Documentation=https://github.com/camthink-ai/NeoMind
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=neomind
Group=neomind
WorkingDirectory=/var/lib/neomind
ExecStart=/usr/local/bin/neomind serve
Restart=always
RestartSec=3
Environment=RUST_LOG=info
Environment=NEOMIND_DATA_DIR=/var/lib/neomind

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable neomind
sudo systemctl start neomind
\`\`\`

**Access the application:**
- Web UI: http://localhost:9375
- API: http://localhost:9375/api
- API Docs: http://localhost:9375/api/docs
```

---

## 🔧 需要立即修复的问题

### 1. 版本号更新 (必须)

```bash
# 查找并替换所有 0.5.11 为 0.6.0
sed -i 's/0\.5\.11/0.6.0/g' README.md
```

### 2. 添加扩展进程说明 (建议)

在 "Manual installation" 部分添加：
```markdown
# Extract (contains neomind + neomind-extension-runner)
tar xzf neomind-server-linux-amd64.tar.gz

# Verify contents
ls -lh
# neomind                        # Main server (~50 MB)
# neomind-extension-runner       # Extension process (~8 MB)
```

### 3. 修正 systemd 服务安装 (推荐)

将 `sudo cp scripts/neomind.service` 改为使用 `tee` 命令直接写入。

---

## ✅ 总结

| 项目 | 状态 | 优先级 |
|------|------|--------|
| **版本号更新** | ❌ 0.5.11 → 0.6.0 | 🔴 必须修复 |
| **扩展进程说明** | ⚠️ 缺失 | 🟡 建议添加 |
| **systemd 服务路径** | ⚠️ 不准确 | 🟡 建议修正 |
| **脚本路径** | ✅ 正确 | - |
| **文件名格式** | ✅ 正确 | - |
| **Web UI 描述** | ✅ 正确 | - |

**核心问题：版本号过旧，必须立即更新到 0.6.0！**

---

**生成时间:** 2025-03-18
**检查版本:** v0.6.0
