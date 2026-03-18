# Tauri 签名密码问题解决方案

## 🚨 错误信息

```
failed to decode secret key: incorrect updater private key password
```

---

## 🔍 原因分析

这个错误说明：
1. ✅ 私钥已找到（环境变量正确）
2. ❌ 密码错误或不需要密码

---

## ✅ 解决方案

### 情况 1: 密钥没有密码（最常见）

**症状：**
- 错误：`incorrect updater private key password`
- 您生成密钥时没有设置密码

**解决方案：**
- ✅ 已从 CI/CD 中移除 `TAURI_SIGNING_KEY_PASSWORD`
- ✅ 只保留 `TAURI_SIGNING_PRIVATE_KEY`

### 情况 2: 密钥有密码

**症状：**
- 您确信生成密钥时设置了密码
- 需要在 GitHub Secrets 中提供正确密码

**解决方案：**
1. 重新添加 `TAURI_SIGNING_KEY_PASSWORD` 到 build.yml
2. 确保密码正确
3. 提交并推送

```yaml
env:
  TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
  TAURI_SIGNING_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_KEY_PASSWORD }}  # ← 添加回来
```

---

## 🔑 如何确认密钥是否有密码

### 方法 1: 检查密钥文件

```bash
# macOS
cat ~/Library/Application\ Support/neomind.tauri/*.key

# Linux
cat ~/.tauri/*.key

# 如果文件开头包含 "ENCRYPTED"，则有密码
# 如果包含 "PRIVATE KEY"，通常没有密码
```

### 方法 2: 尝试无密码使用

```bash
# 尝试签名（不需要密码）
tauri signer sign --help

# 如果提示输入密码，说明密钥有密码保护
```

### 方法 3: 重新生成无密码密钥

```bash
# 使用辅助脚本生成
bash scripts/setup-tauri-key.sh neomind

# 或手动生成（不要设置密码）
tauri signer generate -w ~/.tauri/neomind.key
# 当提示输入密码时，直接按 Enter（留空）
```

---

## 📝 当前配置

### 已应用的修复（情况 1）

```yaml
# .github/workflows/build.yml
env:
  TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
  # TAURI_SIGNING_KEY_PASSWORD 已移除
```

### 如果您的密钥确实有密码

```bash
# 恢复密码配置
cd "/Users/shenmingming/CamThink Project/NeoMind"
git checkout HEAD~1 .github/workflows/build.yml

# 或者手动添加回去
```

---

## 🎯 推荐做法

### 生成无密码密钥（推荐）

**优点：**
- ✅ 配置简单
- ✅ 不需要管理密码
- ✅ CI/CD 更可靠

**步骤：**

```bash
# 1. 生成无密码密钥
bash scripts/setup-tauri-key.sh neomind
# 当提示输入密码时，直接按 Enter

# 2. 复制私钥到 GitHub Secrets
cat ~/.tauri/neomind.key

# 3. 只配置 TAURI_SIGNING_PRIVATE_KEY
# 不配置 TAURI_SIGNING_KEY_PASSWORD
```

---

## ⚠️ 安全考虑

### 无密码密钥安全吗？

**是安全的，因为：**
1. ✅ GitHub Secrets 已加密存储
2. ✅ 只有通过 CI/CD 才能访问
3. ✅ 私钥不会暴露在代码库中
4. ✅ 可以设置密钥过期时间

### 如果需要更高安全性

**可以使用密码保护的密钥：**
1. 生成时设置强密码
2. 将密码添加到 GitHub Secrets
3. 在 build.yml 中启用 `TAURI_SIGNING_KEY_PASSWORD`

---

## 📋 验证步骤

### 1. 检查 GitHub Secrets

访问：https://github.com/mingmingshen/NeoMind/settings/secrets/actions

**必需：**
- ✅ `TAURI_SIGNING_PRIVATE_KEY`

**可选：**
- ❌ `TAURI_SIGNING_KEY_PASSWORD`（仅当密钥有密码时）

### 2. 触发构建

```bash
# 做一个小改动
git commit --allow-empty -m "test: trigger build"
git push origin main
```

### 3. 查看构建日志

访问：https://github.com/mingmingshen/NeoMind/actions

**成功标志：**
- ✅ 没有 "incorrect password" 错误
- ✅ DMG/APP 打包成功
- ✅ app.tar.gz 生成成功

---

## 🔧 故障排除

### 问题 1: 仍然提示密码错误

**可能原因：**
- GitHub Secrets 中的密钥格式不对
- 复制时多了空格或换行

**解决：**
```bash
# 重新复制密钥（注意不要有多余空格）
cat ~/.tauri/neomind.key | pbcopy  # macOS
cat ~/.tauri/neomind.key | xclip   # Linux

# 粘贴到 GitHub Secrets 时，确保：
# 1. 没有前后空格
# 2. 没有多余换行
# 3. 完整复制整个文件
```

### 问题 2: 密钥格式错误

**错误信息：**
```
Invalid input for the given encoding
```

**可能原因：**
- 密钥文件损坏
- Base64 编码问题

**解决：**
```bash
# 重新生成密钥对
bash scripts/setup-tauri-key.sh neomind

# 更新 tauri.conf.json 中的 pubkey
# 更新 GitHub Secrets 中的私钥
```

### 问题 3: 想使用有密码的密钥

**步骤：**

1. **恢复密码配置**
   ```bash
   cd "/Users/shenmingming/CamThink Project/NeoMind"

   # 编辑 build.yml
   nano .github/workflows/build.yml
   ```

2. **添加密码环境变量**
   ```yaml
   env:
     TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
     TAURI_SIGNING_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_KEY_PASSWORD }}
   ```

3. **配置 GitHub Secrets**
   - `TAURI_SIGNING_PRIVATE_KEY`: 您的私钥
   - `TAURI_SIGNING_KEY_PASSWORD`: 您的密码

4. **提交并推送**
   ```bash
   git add .github/workflows/build.yml
   git commit -m "fix: add key password support"
   git push
   ```

---

## 📚 相关文档

- Tauri 签名文档: https://v2.tauri.app/distribute/sign-updater/
- 密钥生成脚本: `scripts/setup-tauri-key.sh`
- 之前的修复: `docs/TAURI_SIGNING_FIX.md`

---

**最后更新：** 2025-03-18
**当前配置：** 无密码密钥（推荐）
