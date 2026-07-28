# vt - visual terminal

**用 Rust 打造的终端媒体播放器，让你在命令行里看视频！**

> 暂时不作二进制分发，请自行构建！

## 特性

- **视频播放** - 基于 FFmpeg 解码，支持常见视频格式
- **音频播放** - 可选音频输出（--audio 参数）
- **多协议支持** - 自动检测 Kitty/Sixel 终端图形协议
- **丰富色彩** - Sixel 模式支持 2-256 色和多种抖动算法
- **自由缩放** - 通过 --scale 调整视频尺寸
- **状态显示** - verbose 模式实时显示 FPS 和帧数
- **ratatui中图片渲染** - 参考examples使用

**注意sixel解码器有两种实现，`pure rust` or `sixel-rs`**

## cli构建
```bash
cargo build -r --features sixel-rs # 依赖sixel-rs

#or

cargo build -r # rust实现

```

## ratatui 使用
```bash
 cargo run --example ratatui --features ratatui -- /path/to/image
```



## 安装

```bash

# archlinux
paru -S ffmpeg

# ffmpeg version n8.1.1 Copyright (c) 2000-2026 the FFmpeg developers
#   built with gcc 16.1.1 (GCC) 20260430

cargo install --git https://github.com/akirco/vt

```

## 使用

```bash
# 基本播放
vt video.mp4

# 启用音频
vt video.mp4 -a

# 缩放播放
vt video.mp4 -s 0.5

# Sixel 模式调优
vt video.mp4 -c 128 -d fs -q high

# 指定渲染协议
vt video.mp4 -p ascii # halfblock, braille
```

## 同类项目

| 项目                                               | 语言   | 特点                                   |
| -------------------------------------------------- | ------ | -------------------------------------- |
| **vt**                                             | Rust   | 多协议、音频支持、高性能               |
| [timg](https://github.com/hzeller/timg)            | C++    | 功能最全，支持摄像头、PDF、多线程解码  |
| [chafa](https://github.com/hpjansson/chafa)        | C      | 图像转换神器，支持丰富符号集和色彩空间 |
| [viu](https://github.com/atanunq/viu)              | Rust   | 轻量图像查看器，支持 iTerm/Kitty       |
| [buddy](https://github.com/JVSCHANDRADITHYA/buddy) | Python | 灵感来源，3种渲染模式，区域平均降采样  |
| [see](https://github.com/svanichkin/see/)          | Go     | 支持音频播放                           |

## 存在的问题

1. **无交互控制** - 不支持暂停、快进、快退、音量调节
2. **缺少循环播放** - 视频播放完即退出，无法循环
3. **终端变化无响应** - 播放时改变终端大小不会自适应
4. **cli 参数分组**

---

**灵感来源**：[buddy](https://github.com/JVSCHANDRADITHYA/buddy)

---
