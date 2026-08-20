#!/usr/bin/env bash
# 日志子系统无网络出口检查（specs/observability Requirement 4）
#
# 检查 tessera-core（日志设施所在地）及 tracing 系列 crate 的传递依赖树中
# 不含任何 HTTP/网络客户端。与 tessera-core 单元测试
# `no_network_client_in_direct_deps`（直接依赖断言）互补。
set -u
cd "$(dirname "$0")/.."

# 常见的 Rust HTTP/网络客户端（类型库如 http 不在此列——它不是客户端）
BANNED="reqwest|ureq|curl|isahc|surf|hyper|attohttpc|minreq|wget-rs|emhttp"

fail=0

# 先捕获原始输出并显式检查退出码：cargo tree 失败（依赖解析错误等）时若继续走
# 文本匹配，错误输出过滤后不含被禁包名，会误判 PASS——安全检查必须 fail-closed。
# （check-deps.sh 靠 "did not match any packages" 特征串天然 fail-closed，此处对齐。）
raw="$(cargo tree -p tessera-core -p tracing -p tracing-subscriber --all-features 2>&1)"
if [ $? -ne 0 ]; then
  echo "FAIL  cargo tree 执行失败，无法验证依赖树："
  printf '%s\n' "$raw" | grep -vE "(^warning|deprecated|config\.toml|^Downloading|^Downloaded|registry|^ *\||^ *=|^help:|^ *$)" | sed 's/^/        /'
  exit 1
fi
out="$(printf '%s\n' "$raw" | grep -vE "(^warning|deprecated|config\.toml|^Downloading|^Downloaded|registry|^ *\||^ *=|^help:|^ *$)")"

# cargo tree 输出形如 "├── reqwest v0.12.1"；匹配包名出现在行首树字符之后
hits="$(printf '%s\n' "$out" | grep -E "[│ ├─└ ]+(${BANNED}) v[0-9]")"

if [ -n "$hits" ]; then
  echo "FAIL  日志子系统的依赖树中出现网络客户端："
  printf '        %s\n' "$hits"
  exit 1
fi
echo "PASS  日志子系统依赖树中无网络客户端"
