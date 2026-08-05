#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${SCRIPT_DIR}"

echo "========================================================"
echo "✦ Building Optimized Release Binary for stars ✦"
echo "========================================================"

cargo build --release

BINARY_PATH="${SCRIPT_DIR}/target/release/stars"

if [ -f "${BINARY_PATH}" ]; then
    echo "========================================================"
    echo "✦ Release build complete successfully! ✦"
    echo "Binary location: ${BINARY_PATH}"
    echo "File size: $(du -h "${BINARY_PATH}" | cut -f1)"
    echo "========================================================"
else
    echo "Error: Binary build failed or executable not found at ${BINARY_PATH}" >&2
    exit 1
fi
