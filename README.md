# STS 3.0

Rust 重构的STS摄影表编辑器

## 功能特性

- **快速轻量**：优化后的可执行文件约 3MB，内存占用极小
- **原生 STS 格式**：完整支持读写 `.sts` 文件，支持 Shift-JIS 编码

## 使用方法

### 创建新时间表

1. 点击 **文件 → 新建** 或按 **Ctrl+N**
2. 配置参数：
   - **名称**：时间表名称
   - **图层数**：图层数量（1-26）
   - **帧率**：帧速率（24 或 30）
   - **每页帧数**：每页显示的帧数（12-288）
   - **时长**：总时长，格式为秒+帧（例如：6s + 0k）
3. 点击 **确定** 创建

### 编辑单元格

- **点击** 单元格开始编辑
- **输入** 数字并按 **Enter** 向下移动
- **方向键** 在编辑时导航
- **空输入** + **Enter** 复制上方单元格的值
- **Esc** 取消编辑

### 选择与剪贴板

- **拖动** 选择多个单元格
- **右键** 打开上下文菜单
- **复制/剪切/粘贴** 选中区域
- 支持系统剪贴板（与 Excel 兼容的 TSV 格式）

## 文件格式
ShiraheiTimeSheet 二进制格式

支持：
- 最多 **255 个图层**
- 最多 **65535 帧**

## 构建

```bash
# 调试
cargo build

# 发布
cargo build --release
```

发布版本的可执行文件将位于 `target/release/sts.exe`。

## 依赖项

- **egui** 0.29 - GUI 框架
- **eframe** 0.29 - 原生窗口包装器
- **encoding_rs** - Shift-JIS 编码/解码
- **rfd** - 原生文件对话框
- **anyhow** - 错误处理

## 系统要求

- **Windows**: Windows 7 或更高版本
- **macOS**: macOS 10.13 或更高版本
- **Linux**: 现代 Linux 发行版（需要 X11 或 Wayland）

## 许可证
TDB

### 编码处理

使用 `encoding_rs` 库处理 Shift-JIS 编码：
- 读取时自动解码为 UTF-8
- 保存时转换回 Shift-JIS
- 兼容日文图层名称

## 开发路线图

### 已完成 ✅
- [x] 基础表格编辑
- [x] STS 文件读写
- [x] 撤销/重做
- [x] 剪贴板操作
- [x] 键盘导航


## 贡献

欢迎提交问题和拉取请求！

### 开发环境设置

```bash
# 克隆仓库
git clone https://github.com/ChenxingM/sts-rust.git
cd sts-rust

# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 运行测试
cargo test

# 启动开发版本
cargo run
```

### 代码风格

- 使用 `rustfmt` 格式化代码
- 使用 `clippy` 检查代码质量
- 编写测试覆盖关键功能

```bash
cargo fmt
cargo clippy
cargo test
```


## 致谢

- 原始 ShiraheiTimeSheet 作者
- egui GUI 框架

## 联系方式

如有问题或建议，请提交 Issue 或 Pull Request。

---