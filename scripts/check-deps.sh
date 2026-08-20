#!/usr/bin/env bash
# 依赖方向硬性检查（设计书 §17.1 约束 1/2，对应 specs/build-constraints 前两条 Requirement）
#
# 约束 1：五个内核 crate 的传递依赖树中不得出现 tauri
# 约束 2：tessera-core 的传递依赖树中不得出现 wasmtime
#
# 原理（design.md D2）：`cargo tree -i <被禁包> -p <crate>` 反查引入路径；
# 若输出 "did not match any packages" 说明依赖树中不存在该包（通过），
# 否则输出本身就是引入路径（失败并打印）。
set -u
cd "$(dirname "$0")/.."

fail=0

# check <被禁依赖> <crate> [<crate>...]
check() {
  local banned="$1"
  shift
  for crate in "$@"; do
    local out
    out="$(cargo tree -i "$banned" -p "$crate" --all-features 2>&1)"
    if printf '%s' "$out" | grep -q "did not match any packages"; then
      echo "  PASS  $crate 依赖树中无 $banned"
    else
      echo "  FAIL  $crate 依赖树中出现 $banned，引入路径："
      printf '%s\n' "$out" | grep -vE "(^warning|deprecated|config\.toml|^Downloading|^Downloaded|registry|^ *\||^ *=|^help:|^ *$)" | sed 's/^/        /'
      fail=1
    fi
  done
}

echo "== 约束 1：内核 crate 不得依赖 tauri =="
KERNEL_CRATES=(tessera-error tessera-manifest tessera-ui-schema tessera-store tessera-sandbox tessera-core)
check tauri "${KERNEL_CRATES[@]}"

echo "== 约束 2：tessera-core 不得依赖 wasmtime =="
check wasmtime tessera-core

if [ "$fail" -ne 0 ]; then
  echo "依赖方向检查未通过"
  exit 1
fi
echo "依赖方向检查全部通过"
