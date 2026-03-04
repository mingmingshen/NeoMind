#!/bin/bash
# NeoMind launcher script for systems with limited GPU support

# Detect if running on ARM with software renderer
if [ "$(uname -m)" = "aarch64" ] || [ "$(uname -m)" = "arm64" ]; then
    if glxinfo 2>/dev/null | grep -q "llvmpipe"; then
        echo "Detected software renderer (llvmpipe), disabling GPU acceleration..."
        export WEBKIT_DISABLE_COMPOSITING_MODE=1
        export LIBGL_ALWAYS_SOFTWARE=1
        export WEBKIT_DISABLE_DMABUF_RENDERER=1
    fi
fi

# Launch the actual application
exec "$(dirname "$0")/neomind" "$@"
