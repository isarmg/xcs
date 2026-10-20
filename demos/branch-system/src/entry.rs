pub fn html(free: bool, base: &str) -> String {
    let mode = if free { "free" } else { "guided" };
    let title = if free { "自由操作" } else { "过程演示" };
    format!(
        r#"<!doctype html><html lang="zh-CN" data-zxs-demo="{mode}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="color-scheme" content="light dark"><title>ZXS · 分支{title}</title><link rel="stylesheet" href="{base}styles/startup.css"><link rel="stylesheet" href="{base}zxs.css"><link rel="stylesheet" href="{base}demo.css"><link rel="modulepreload" href="{base}pkg/zxs.js"></head><body><div id="root"><p role="status">正在加载 Rust 分支演示。请通过 HTTP 服务打开此页面。</p></div><noscript>此页面需要启用 JavaScript 来加载 Rust / WebAssembly。[静态图示保存在 reference 文件夹。]</noscript><script type="module" src="{base}boot.js"></script></body></html>"#
    )
}
