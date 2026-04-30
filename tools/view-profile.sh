#!/bin/bash
# Convenience script to run profile-viewer from anywhere in the project

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROFILE_VIEWER="$SCRIPT_DIR/profile-viewer/target/release/profile-viewer"

# Build if not exists
if [ ! -f "$PROFILE_VIEWER" ]; then
    echo "Building profile-viewer..."
    (cd "$SCRIPT_DIR/profile-viewer" && cargo build --release)
    echo ""
fi

# Run profile-viewer with all arguments
"$PROFILE_VIEWER" "$@"
