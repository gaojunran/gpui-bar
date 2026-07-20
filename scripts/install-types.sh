#!/bin/sh
# 安装 gpui-bar 类型定义到用户配置目录
# 用法: sh scripts/install-types.sh

set -e

CONFIG_DIR="${HOME}/.config/gpui-bar"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TYPES_DIR="${SCRIPT_DIR}/../types"

mkdir -p "${CONFIG_DIR}"

cp "${TYPES_DIR}/gpui-dashboard.d.ts" "${CONFIG_DIR}/gpui-dashboard.d.ts"
cp "${TYPES_DIR}/tsconfig.json" "${CONFIG_DIR}/tsconfig.json"

echo "已安装类型定义到 ${CONFIG_DIR}/"
echo "  - gpui-dashboard.d.ts  (类型定义)"
echo "  - tsconfig.json        (TypeScript 配置)"
echo ""
echo "在 ${CONFIG_DIR}/bar.config.ts 写配置时，编辑器会自动提供类型提示。"
