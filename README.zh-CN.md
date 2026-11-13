<p align="center"><img src="docs/logo.svg" width="104" alt="ScribeWatch 标志"></p>
<h1 align="center">ScribeWatch</h1>
<p align="center"><strong>从录音到可用的文档，数据由您掌控。</strong><br>开源、隐私优先的录音、转录和结构化笔记工具。</p>
<p align="center"><a href="README.md">English</a> · <a href="README.fr.md">Français</a> · <strong>简体中文</strong></p>
<p align="center"><a href="https://github.com/GodsQuantum/ScribeWatch/releases/latest">下载最新版本</a> · <a href="https://github.com/GodsQuantum/ScribeWatch/issues">反馈问题</a> · <a href="https://arezkichougar.com">作者 Arezki Chougar</a></p>

<p align="center"><img src="docs/screenshots/zh-CN/home.png" width="100%" alt="ScribeWatch v0.6.4 中文首页"></p>

## ScribeWatch v0.6.2 下载

**源代码版本：v0.6.4。** 在 v0.6.4 发布验证完成前，下列安装包链接仍指向最后一个已正式发布并校验的 **v0.6.2**。界面截图采集自隔离运行的 v0.6.4，布局未改变。



| 系统 | 版本 | 官方安装包 | 说明 |
|---|---|---|---|
| **Windows 11 x64** | **完整独立版（推荐）** | [下载完整安装包](https://github.com/GodsQuantum/ScribeWatch/releases/download/v0.6.2/ScribeWatch-0.6.2-windows-amd64-standalone-setup.exe) | 包含本地 STT/LLM 引擎，首次启动自动下载模型 |
| Windows x64 | 轻量版 | [下载安装包](https://github.com/GodsQuantum/ScribeWatch/releases/download/v0.6.2/ScribeWatch-0.6.2-windows-amd64-setup.exe) | 需配置兼容的转录和可选 LLM 服务 |
| macOS Apple Silicon | 轻量版 | [下载 ARM64 DMG](https://github.com/GodsQuantum/ScribeWatch/releases/download/v0.6.2/ScribeWatch-0.6.2-darwin-arm64.dmg) | 需配置兼容引擎 |
| macOS Intel | 轻量版 | [下载 Intel DMG](https://github.com/GodsQuantum/ScribeWatch/releases/download/v0.6.2/ScribeWatch-0.6.2-darwin-amd64.dmg) | 需配置兼容引擎 |
| Linux x86-64 | AppImage | [下载 x86-64](https://github.com/GodsQuantum/ScribeWatch/releases/download/v0.6.2/ScribeWatch-0.6.2-x86_64.AppImage) | 需配置兼容引擎 |
| Linux ARM64 | AppImage | [下载 ARM64](https://github.com/GodsQuantum/ScribeWatch/releases/download/v0.6.2/ScribeWatch-0.6.2-aarch64.AppImage) | 需配置兼容引擎 |

**下一次正式安装包更新（当前可下载的 v0.6.2 尚未包含）：** Windows、macOS、Linux 桌面安装包均会内置 SHA-256 验证过的 Pandoc 3.14、Typst 0.15.1 和 Noto Sans CJK 字体。因此 MD、TXT、HTML、DOCX、ODT、PDF 导出无需在主机单独安装 Pandoc、WeasyPrint、LaTeX、Python 或 LibreOffice。浏览器 PDF 原本已独立运行，服务器端导出也将优先使用打包工具。首次下载 AI 模型仍需联网。

所有发布文件均提供对应的 SHA-256 校验文件。桌面安装包未必具备正式的代码签名或 Apple 公证，请核对官方发布说明。

## Windows 完整独立版：首次安装

安装完成后启动 ScribeWatch。应用会自动创建 Parakeet Ultra 转录服务、Qwen3.5-4B Q4_K_M 本地文档生成服务，并准备五个报告模板：**牙科会诊、普通医疗会诊、会议、头脑风暴、结构化笔记**。首次下载需要联网并预留数 GB 空间，后续在本机处理音频和文档。应用会根据受支持的显卡和内存调整运行参数。

等待 REC 显示模型就绪，填写文档信息、选择存储位置、授权麦克风，按 **REC** 开始，按 **Stop** 完成，随后可导出 PDF 或其他格式。生成的医疗报告仅供专业人员审核，不能代替诊断和病历复核。Windows 安装与引擎启动通过了 CI 自动测试，但尚未在所有实际 GPU、麦克风和医疗环境中完成端到端认证。其他系统不包含同样的完整自动 AI 安装包。

## v0.6.2 当前界面

**首页（Home）**：总览、快速转录、监控文件夹入口。**录音（REC）**：浏览器麦克风录制、断线恢复。**文档（Documents）**：历史任务、文档和导出。**设置（Settings）**：转录和 LLM 服务、模型、报告模板、高级参数。支持英语、法语、简体中文、五种主题及简单/高级模式。

<p align="center"><img src="docs/screenshots/zh-CN/record.png" width="49%" alt="简体中文 REC 录音界面"><img src="docs/screenshots/zh-CN/settings.png" width="49%" alt="简体中文设置界面"></p>
<p align="center"><img src="docs/screenshots/zh-CN/documents.png" width="80%" alt="简体中文文档列表界面"></p>

所有截图都来自隔离的真实应用演示环境，不含私人或患者信息。

## 功能与数据安全

- **录音：** 浏览器录制时通过 OPFS 分片保存，支持页面重载后恢复、继续和保存最终音频。
- **快速转录：** 上传音频、选择服务器上允许访问的文件或使用监控文件夹；FFprobe/FFmpeg 对输入音频进行解析和标准化。
- **原始转录：** 作为可靠来源保留；确定性段落格式化不会改写内容；支持 Obsidian 友好的 Markdown 和 YAML 属性。
- **可选结构化 AI：** 根据会议、会诊等模板整理笔记，不覆盖原始文字；长转录采用分段处理；LLM 失败不会删除已完成的原始转录。
- **导出：** MD、TXT、HTML、DOCX、ODT、PDF。工作流只会在 Markdown 写入成功后归档音频。
- **模型回退：** 兼容 OpenAI 风格的 STT 接口，可按顺序配置多个提供商和模型。
- **隐私：** 自托管安装可控制数据去向；使用外部 API 时由所选提供商处理传输的内容。请勿把患者数据、密钥或私有文档提交至公开 Issue。

## 使用 Docker 自托管

Docker 版本为轻量服务器，**不会自动安装 Windows 完整版自带的本地 Parakeet Ultra 和 Qwen 4B 模型**。需要自己提供兼容的 STT 服务，可选配置 LLM 服务。

前提：Docker Engine、Docker Compose 以及可访问的转录 API。

~~~bash
git clone https://github.com/GodsQuantum/ScribeWatch.git
cd ScribeWatch
cp .env.example .env
cp compose.example.yaml compose.yaml
mkdir -p config data watch notes archive
docker compose pull
docker compose up -d
~~~

浏览器打开 http://127.0.0.1:3000。对网络中的其他设备使用录音麦克风，通常需要 HTTPS 安全上下文；Docker 无法自动访问客户端设备的麦克风和文件夹。按实际需要设置允许访问的服务器根目录，并通过经过认证的代理或 VPN 提供远程访问。

## 硬件配置与高级选项

Docker 的只读调优脚本会读取 CPU 和内存，并提出可审查的并行度建议；它不会修改配置或磁盘。

~~~bash
python3 scripts/auto-tune.py
python3 scripts/auto-tune.py --env
~~~

推荐先测试真实音频，核对输出位置、录音恢复、转录提供商回退、原文保留和归档，再启用自动工作流。请阅读 [English technical guide](README.md)、[Français](README.fr.md)、[desktop packaging notes](packaging/desktop/README.md)、[roadmap](docs/ROADMAP_DESKTOP_PORTABILITY.md)。

## 开发与贡献

后端采用 Rust/Axum、SQLite；前端采用 SvelteKit，使用 FFmpeg/FFprobe 和文档导出工具。欢迎通过 [GitHub Issues](https://github.com/GodsQuantum/ScribeWatch/issues) 报告可重现的问题，也欢迎提交 Pull Request；请遵守 [CONTRIBUTING](.github/CONTRIBUTING.md) 与 [SECURITY](.github/SECURITY.md)。源码采用 [MIT 许可证](LICENSE)，作者 [Arezki Chougar](https://arezkichougar.com)。

**转录引擎来源：** Windows 完整安装包含 **[Speaches Reloaded](https://github.com/GodsQuantum/Speaches-Reloaded)** 原生可执行程序与完整依赖，不只是 `transcribe.cpp`；旧版可执行文件名仍为 `Cloud9-Speaches`。
