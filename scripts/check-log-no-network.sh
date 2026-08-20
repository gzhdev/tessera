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
out="$(cargo tree -p tessera-core -p tracing -p tracing-subscriber --all-features 2>&1 \
  | grep -vE "(^warning|deprecated|config\.toml|^Downloading|^Downloaded|registry|^ *\||^ *=|^help:|^ *$)")"

# cargo tree 输出形如 "├── reqwest v0.12.1"；匹配包名出现在行首树字符之后
hits="$(printf '%s\n' "$out" | grep -E "[│ ├─└ ]+(${BANNED}) v[0-9]")"

if [ -n "$hits" ]; then
  echo "FAIL  日志子系统的依赖树中出现网络客户端："
  printf '        %s\n' "$hits"
  exit 1
fi
echo "PASS  日志子系统依赖树中无网络客户端"
