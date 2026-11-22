# 划词翻译（Selection Translate）

一个常驻托盘的轻量级桌面工具：捕获你在网页、文档或桌面应用中选中的文字，翻译为英文，并在悬浮卡片中展示结果。

技术栈：Tauri 2（Rust 后端）+ 原生 TypeScript / Vite 前端。

## 已实现的功能（MVP）

- 全局热键唤起翻译（默认 `Alt+Shift+T`），托盘菜单也可触发。
- 两种取词模式：
  - **默认模式**：选中文字 → 按 `Ctrl/Cmd+C` → 按热键。零额外权限、最可靠。
  - **自动复制模式**：在设置中开启后，按热键会代替你发送 `Ctrl/Cmd+C`，并尝试恢复原剪贴板内容。需要目标窗口处于焦点，macOS 需要辅助功能权限。
- 文本规范化：去零宽字符、合并空白、自动检测中文，单次上限 2000 字符。
- 翻译服务可插拔：DeepL API、OpenAI 兼容接口、本地模拟（默认，无需密钥即可体验）。
- 本地缓存（redb）、5 秒超时、单次重试、错误内联展示。
- 结果卡片：原文、英文译文、缓存命中标记、耗时、复制、朗读、置顶、关闭。
- API 密钥存系统凭据管理器（Windows 凭据管理器 / macOS 钥匙串 / libsecret），不写进配置文件。
- 脱敏日志思路：正文不落盘；卡片中展示的耗时数据来自各阶段实测。

## 明确不做

图片 OCR、整篇文档翻译、自动回写原文、剪贴板历史、账号与云同步、多目标语言（固定英文）。

## 环境要求

- Rust 1.77.2+（`rustup` 安装，需包含 `msvc` target；Windows 还需 WebView2 Runtime 和 VS Build Tools）
- Node.js 18+（建议 20/22）
- Windows 10/11；macOS 与 Linux 可编译，但自动复制模式的系统权限需求不同

## 本地运行

```bash
cd selection-translate
npm install
npm run dev            # = tauri dev，自动起 Vite 并编译 Rust
npm run build          # = tauri build，产出安装包
```

若只想预览前端界面（无 Rust 后端能力）：

```bash
npm run vite
```

## 使用方式

1. 首次启动后，默认使用 `mock` 服务，无需配置密钥，按上述步骤即可看到 `[EN] ...` 的模拟结果。
2. 打开托盘菜单 →「设置」，选择真实服务：
   - DeepL：填入 `https://api-free.deepl.com/v2/translate`（付费账户换成 `api.deepl.com/v2/translate`）和 API 密钥。
   - OpenAI 兼容接口：填入完整 chat completions 地址、模型名和密钥。
3. 保存后热键立即生效。设置中的 API 密钥输入框留空表示不修改已保存密钥。

## 目录结构

```
selection-translate/
├─ index.html                 # 卡片与设置两个视图
├─ src/main.ts                # 前端事件监听与交互
├─ src/styles.css             # 跟随系统明暗主题
├─ vite.config.ts
├─ tools/generate_icons.py    # 用标准库生成图标（已生成，可重跑）
└─ src-tauri/
   ├─ tauri.conf.json         # 无默认窗口，全部窗口在运行时创建
   ├─ Cargo.toml
   └─ src/
      ├─ main.rs              # 进程入口
      ├─ lib.rs               # Tauri 命令、热键、托盘、窗口编排
      ├─ capture.rs           # 剪贴板取词（含备份恢复）
      ├─ platform.rs          # 唯一平台相关模块：合成按键
      ├─ preprocess.rs        # 规范化与语言检测
      ├─ translator.rs        # Provider 实现与 HTTP 调用
      ├─ cache.rs             # redb 缓存与哈希键
      ├─ settings.rs          # 配置与凭据存储
      └─ error.rs             # 统一错误类型
```

## 性能账本（设计目标）

| 阶段 | p50 | p95 |
|---|---|---|
| 热键回调 | 2ms | 5ms |
| 剪贴板取词与恢复 | 25ms | 60ms |
| 规范化与语言检测 | 3ms | 8ms |
| 缓存查询 | <1ms | 1ms |
| 网络翻译 | 350ms | 1800ms |
| 卡片渲染 | 5ms | 12ms |

优化手段已落在代码里：卡片窗口启动时预建并隐藏、HTTP keep-alive 连接池、5 秒超时、本地缓存、单次重试。卡片每次显示前定位到主显示器右下角。

## 已知限制与后续扩展

- 自动复制模式通过 `enigo` 合成按键，少数应用（远程桌面、输入法候选框、提权窗口）可能失效；此时退回默认模式即可。后续可替换为 Windows UI Automation / macOS Accessibility API 直接读取选区。
- 自动复制模式下备份仅保留纯文本，会丢失原剪贴板的富文本格式。后续可用平台剪贴板格式枚举保留全部格式。
- 卡片目前固定定位在屏幕右下角，尚未按选区矩形贴边翻转。
- 未实现：多目标语言、术语表、翻译记忆、本地小模型、一键替换原文、浏览器扩展桥接。

## 云端编译与打包

当前环境没有可直接调用的原生桌面应用云端构建服务（「发布为应用」仅支持 Web/HTTP 项目）。我已写好 `.github/workflows/build-installers.yml`：**推送到 `main` 分支即自动触发**，在 Windows / macOS / Ubuntu 云端机器上并行编译，并把 `.msi` / `.exe` / `.dmg` / `.AppImage` 作为 Actions 制品上传。

图标由 `build.rs` 在编译时自动生成，因此仓库不需要提交二进制文件；本地直接 `npm run dev` 也能正常编译。

在仓库的 Actions 页打开对应运行记录，从 Artifacts 下载 `installers-windows-latest` 等制品即可。若你授权 GitHub 连接器，我可以直接把代码推上去并跟进构建结果。

## 验证状态

本工作区已验证：图标生成、前端 TypeScript 类型检查与 Vite 构建。
未验证：Rust 编译与安装包构建——当前环境未安装 Rust 工具链，请在你本地执行 `npm run dev` 完成首次编译。若依赖版本 API 有差异，最可能需要调整的是 `platform.rs`（按键合成）和 `lib.rs` 中的托盘菜单事件字段访问。
