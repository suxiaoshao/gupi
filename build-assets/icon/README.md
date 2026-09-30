# 图标来源与派生

来源：Pi 官方 README 引用的 https://pi.dev/logo-auto.svg ，获取于 2026-09-07。
源 SVG SHA-256：`03d509c104b9570063fa268fd3235ed7e0e41dafd93124ca94cae3726f58f117`。

版权署名：`Copyright (c) 2026 Earendil Inc. and contributors`。官方 [pi-website 仓库中的 logo.svg](https://github.com/earendil-works/pi-website/blob/2f5e410b97474d0a34ec2500aa1aa58d6c3f992c/src/logo.svg) 与本地基础图标的两条 SVG 路径一致，该固定版本仓库采用 MIT License。彩色图标来自标注 MIT 的[官方 Press Kit](https://pi.dev/press-kit)；2026-09-30 核对时，本地 `assets/brand/logo-color.svg` 与官网下载文件完全一致，SHA-256 为 `abd66e7868b2d24f0f0895f9237ee8a6dcb22337583b0dc54aeb595acecb4d6b`。

完整来源、Gupi 的改动范围与上游 MIT 许可原文见根目录 [THIRD_PARTY_NOTICES.md](../../THIRD_PARTY_NOTICES.md)。该文件随 macOS、Windows 和 Linux 安装包分发。Gupi 是独立项目；版权许可记录不代表 Pi 或 Earendil 的官方背书或单独商标授权。

SVG 内容原样保留；PNG 由 `rsvg-convert -w 1024 -h 1024 pi-logo.svg -o app-icon.png` 生成。
Gupi.icon 使用 Assets/logo.svg 矢量图层：默认浅灰背景、黑色标记，Dark 深灰背景、白色标记；切换由 macOS 图标外观设置控制。已通过 actool 编译，并在 Icon Composer 查看 Default / Dark 预览。普通平台图标仍由 app-icon.png 派生。

应用内使用 assets/brand/logo-black.svg 与 logo-white.svg，按当前 GPUI Theme 的亮暗模式选择。两份仅将原始 SVG 的 CSS 填充改为明确的黑/白属性，路径和 viewBox 保持一致。当前 GPUI 的 usvg/simplecss 渲染链不支持原始 @media (prefers-color-scheme: dark) 规则。

## 运行时资源与主题验证

`assets/brand/tray-template.svg` 沿用经典标记，收紧 viewBox 至 `120 120 560 560`。macOS 托盘使用 36×36 透明 PNG，由 tray-icon 按 18pt 显示并标为 template；此 PNG 是启动时需要的运行资源，随 SVG 提交。修改 SVG 后从仓库根目录重新生成：

```sh
rsvg-convert -w 36 -h 36 assets/brand/tray-template.svg -o assets/brand/tray-template.png
```

`assets/brand/logo-color.svg` 为 2026-09-24 获取的 https://pi.dev/logo-auto.svg 原始内容，颜色为 `#F09082`、`#4D9ABF`、`#F1BE58`。用于正式 Pi 彩色主题与派生变体；默认仍保留经典。主题范围、打包与动态切换边界见[开发计划](../../docs/dev/issue-223/README.md)。

## 主题资源

运行 `python3 script/generate-icon-themes.py` 重新生成七种主题（需要 Xcode Icon Composer 的 ictool）。官方 SVG 是几何来源，脚本只更换配色；经典 `Gupi.icon` 为材质/外观模板。生成的各 `.icon/icon.json`、SVG 图层和运行时 PNG 需要同步提交；Assets.car 与 icns/iconset 由 xtask 在临时目录派生，不提交。`default-icon` 明确指定 Gupi，xtask 会一次编译目录下全部主题。

彩虹主题的十格按行使用红橙黄、绿蓝、紫红橙、绿蓝，复用红、橙、黄、绿、蓝、紫六种纯色；不提供彩虹渐变。彩虹与乌克兰主题均用官方路径裁切外轮廓。彩虹按方格赋色；乌克兰主题继续使用上下蓝黄双色。

经典保持系统明暗背景。其余主题在 Default / Dark 中使用相同的有色基底：经典渐变为淡蓝紫 `#B9BEDD`，Pi 彩色与 Pi 渐变为蓝灰 `#40516B`，彩虹为淡紫 `#C6B9DA`，乌克兰两款为明紫 `#A67AD8`。这让派生图标在亮、暗桌面上都保留配色，不依赖运行时重选外观。

Pi 渐变按官方颜色位置做三向融合：珊瑚色在上方，蓝色在左下，金色在右下。SVG 使用水平蓝金渐变叠加从上方淡出的珊瑚色，保留矢量路径，不依赖网格渐变或滤镜。乌克兰渐变保留 `#0057B7` / `#FFDD00` 两端，中间加入亮青色 `#83C7DB`，避免直接 sRGB 混色产生偏暗的橄榄色。中间色方法参考 [Make Beautiful Gradients](https://www.joshwcomeau.com/css/make-beautiful-gradients/)，具体配色是本应用的视觉选择。
