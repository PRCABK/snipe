# Snipe

> 面向 Windows 10/11 的 Rust 截图工具，目前处于开发预览阶段。

Snipe 使用 Rust、Slint 与 Win32 构建。当前仓库包含 GDI 截图、区域选择、复制/保存、多钉图基础窗口、AI OCR/翻译调用、标注栅格化和长图拼接等基础实现，但尚未完成 `PLAN.md` 定义的端到端 MVP，不应作为稳定正式版发布。

当前完成度和发布阻塞项见 [`docs/implementation-status.md`](docs/implementation-status.md)。缺口闭合顺序与验收标准见 [`docs/repair-plan.md`](docs/repair-plan.md)。

---

## 当前可验证能力

- 单实例、托盘后台常驻和全局快捷键基础流程。
- GDI 虚拟桌面截图、矩形选区、复制和保存。
- 多个基础钉图窗口，以及复制、保存和关闭操作。
- OpenAI Chat Completions 风格的图片 OCR/翻译调用。
- Windows Credential Manager 中的 API Key 基础读写。
- 标注数据模型、撤销/重做栈和 CPU 栅格合成库。
- 连续等尺寸帧的垂直重叠估算和拼接库。

长截图采集、完整标注编辑器、钉图高级交互、完整取色器、WGC/DXGI 捕获、设置即时生效以及发布验收仍在开发中。

---

## 快捷键 (默认)

- **F1**：常规截图 (区域 / 全屏 / 快速复制保存)
- **F2**：钉图 (多窗口独立置顶)
- **F3**：屏幕取色 (放大镜与颜色格式复制)
- **Ctrl + Alt + S**：滚动长截图

可在“偏好设置”中根据习惯自由配置。

---

## 构建与打包

本项目配备 Windows GitHub Actions CI/CD。推送与 `Cargo.toml` 版本一致的 Tag（例如 `v0.1.0`）会执行锁定依赖测试，并使用 Inno Setup 生成单文件安装程序 `Snipe-Setup-*-x64.exe`。发布前必须提交 `Cargo.lock`；当前不生成便携包。

### 本地编译 (需已安装 Rust 及 MSVC C++ 工具链)

```powershell
# 编译 Release 二进制
cargo build --workspace --release --locked

# 打包安装程序 (需预先安装 Inno Setup 6)
./scripts/package.ps1
```

---

## 开源协议

本项目遵循 MIT 或 Apache-2.0 双重开源协议。
