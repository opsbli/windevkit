# windevkit 🪟🔧

**Windows Development Environment Toolkit** — 一键安装开发工具、版本切换、App 迁移。

> 重装 Windows 后，一个命令恢复全部开发环境和常用软件。

[![CI](https://github.com/opsbli/windevkit/actions/workflows/ci.yml/badge.svg)](https://github.com/opsbli/windevkit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## 功能

| 命令 | 说明 |
|------|------|
| `windevkit init` | 初始化 `~/.windevkit/` 目录结构 |
| `windevkit install node 22.11.0` | 安装指定版本 Node.js |
| `windevkit install java 21.0.3` | 安装指定版本 JDK (Temurin) |
| `windevkit install maven 3.9.16` | 安装指定版本 Maven |
| `windevkit install node 22.11.0 --from D:\backup\node.zip` | 从本地文件安装 |
| `windevkit use node 18.20.0` | 切换 Node.js 版本 |
| `windevkit list node` | 列出已安装版本 |
| `windevkit uninstall node 18.20.0` | 卸载指定版本 |
| `windevkit app scan` | 扫描已安装的应用 |
| `windevkit app export` | 导出离线工具箱到 U 盘 |
| `windevkit app import D:\my-toolbox` | 在新机恢复全部环境 |
| `windevkit app add-path D:\tools\everything` | 添加便携应用到清单 |
| `windevkit status` | 查看环境状态 |
| `windevkit doctor --fix` | 诊断并修复环境问题 |
| `windevkit self-update` | 更新 windevkit 自身 |

## 快速开始

### 1. 安装

从 [GitHub Releases](https://github.com/opsbli/windevkit/releases) 下载 `windevkit-x86_64-pc-windows-msvc.zip`，解压到任意目录。

或者用 PowerShell 一键安装：
```powershell
# TODO: get-windevkit.ps1
```

### 2. 初始化

```bash
windevkit init --mirror aliyun
```

推荐启用 Windows 开发者模式（无需管理员权限即可创建 symlink）：
```powershell
# PowerShell 管理员
reg add HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock /t REG_DWORD /v AllowDevelopmentWithoutDevLicense /d 1 /f
```

### 3. 安装运行时

```bash
# 安装 Node.js 22
windevkit install node 22.11.0

# 安装 JDK 21
windevkit install java 21.0.3

# 安装 Maven 3.9
windevkit install maven 3.9.16

# 查看状态
windevkit status
```

### 4. 导出 App 工具箱

```bash
# 扫描已安装的应用
windevkit app scan

# 交互式选择 → 导出到 U 盘
windevkit app export --output D:\my-toolbox
```

### 5. 在新系统恢复

```bash
# 装好 windevkit → 从 U 盘导入
windevkit app import D:\my-toolbox
```

## 镜像站支持

中国用户推荐使用国内镜像加速下载：

| 镜像 | Node.js | Maven | Java |
|------|---------|-------|------|
| `aliyun` | ✅ npmmirror | ✅ mirrors.aliyun.com | ❌ GitHub |
| `huawei` | ✅ repo.huaweicloud.com | ✅ repo.huaweicloud.com | ❌ GitHub |
| `npmmirror` | ✅ npmmirror.com | ✅ mirrors.aliyun.com | ❌ GitHub |
| `direct` | nodejs.org | dlcdn.apache.org | GitHub |

```bash
# 安装时指定镜像
windevkit install node 22.11.0 --mirror huawei

# 或在 init 时设置默认镜像
windevkit init --mirror aliyun
```

## 目录结构

```
%USERPROFILE%\.windevkit\
├── config.toml              # 全局配置
├── versions\                # 运行时版本存储
│   ├── node\v22.11.0\
│   ├── java\jdk21.0.3\
│   └── maven\3.9.16\
├── active\                  # Symlink 激活目录
│   ├── node → ..\versions\node\v22.11.0\
│   ├── java → ..\versions\java\jdk21.0.3\
│   └── maven → ..\versions\maven\3.9.16\
├── export\                  # 离线工具箱输出
│   ├── manifest.toml
│   ├── installers\
│   ├── portables\
│   └── runtimes\
├── backups\                 # PATH 快照备份
└── logs\windevkit.log       # 日志
```

## 版本切换原理

windevkit 使用 **Symlink** 实现版本切换：

1. 所有版本解压到 `~/.windevkit/versions/<tool>/<version>/`
2. `~/.windevkit/active/<tool>` 是一个 symlink，指向当前激活版本
3. `%USERPROFILE%\.windevkit\active\bin` 添加到 PATH
4. `windevkit use node 18` = 重新指向 symlink，即刻生效

## 开发

```bash
# 构建
cargo build

# 测试
cargo test

# 运行
cargo run -- status
```

## License

MIT
