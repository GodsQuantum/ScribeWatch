# ScribeWatch — 把录音自动整理为 Markdown

[English](README.md) · [Français](README.fr.md) · **简体中文**

**声音输入，Markdown 输出。** ScribeWatch 支持快速音频转录、浏览器录音、文件夹监听以及 Obsidian 兼容的 Markdown 工作流。可以连接自托管 Speaches 或其他 OpenAI 兼容的语音 API，结构化 LLM 输出不应替代原始转录。

## 开始使用

1. 下载 [Linux 发行版](https://github.com/GodsQuantum/ScribeWatch/releases) 或 [Windows x64/macOS Intel、Apple Silicon 候选安装包](https://github.com/GodsQuantum/ScribeWatch/releases/tag/desktop-v0.5.3-rc.2)。后者**还没有正式签名或 Apple 公证**，请核对 SHA256。
2. Docker 用户复制 .env.example 为 .env，然后运行：

~~~bash
docker compose -f compose.example.yaml up -d
~~~

3. 在设置中填写实际运行的 Speaches /v1 地址，完成连接测试，先转录一段真实音频，确认笔记保存路径与原文后再启用自动监听。

## 自动资源检测和配置建议

~~~bash
python3 scripts/auto-tune.py
python3 scripts/auto-tune.py --env
~~~

脚本只读取 CPU 亲和性与容器内存，**不会修改配置、数据、服务或磁盘挂载**。2–4 线程或不足 8 GiB 时先用 1 个转录任务；Ryzen 7 8845HS（16 线程、32 GiB 共享容器）从 2 个并行任务、每任务 1 个 FFmpeg 解码线程、5 秒扫描周期和 2 秒文件稳定性检查开始。网络磁盘比较慢时增加稳定时间。

远程 NAS 上的 Docker 不能自动读取客户端电脑的麦克风或本地文件夹；应明确配置授权路径。测试模型回退、原始转录保存、Markdown 文件和归档后再扩大规模。远程访问建议通过 HTTPS 及身份验证。

[英文详细文档](README.md) · [法语文档](README.fr.md) · [发布版本](https://github.com/GodsQuantum/ScribeWatch/releases) · [报告问题](https://github.com/GodsQuantum/ScribeWatch/issues)。MIT 许可证。
