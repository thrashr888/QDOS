#!/bin/bash
# Migrate a plugin from src/plugins/$name to crates/qdos-plugin-$name
# Usage: ./scripts/migrate-plugin.sh <plugin-name>

set -e

NAME="$1"
if [ -z "$NAME" ]; then
    echo "Usage: $0 <plugin-name>"
    exit 1
fi

SRC="src/plugins/$NAME"
DST="crates/qdos-plugin-$NAME"

if [ ! -d "$SRC" ]; then
    echo "Error: $SRC does not exist"
    exit 1
fi

if [ -d "$DST" ]; then
    echo "Error: $DST already exists"
    exit 1
fi

echo "Migrating $NAME..."

# 1. Create crate structure
mkdir -p "$DST/src"

# 2. Create Cargo.toml
cat > "$DST/Cargo.toml" << EOF
[package]
name = "qdos-plugin-$NAME"
version = "0.1.0"
edition = "2021"
description = "$(grep -m1 '//!' "$SRC/mod.rs" 2>/dev/null | sed 's/^\/\/! //' || echo "$NAME plugin for QDOS")"
authors = ["thrashr888"]
license = "MIT"

[dependencies]
qdos-plugin-api = { path = "../qdos-plugin-api" }
ratatui = "0.29"
EOF

# 3. Move files
for f in "$SRC"/*.rs; do
    if [ -f "$f" ]; then
        base=$(basename "$f")
        if [ "$base" = "mod.rs" ]; then
            cp "$f" "$DST/src/lib.rs"
        else
            cp "$f" "$DST/src/$base"
        fi
    fi
done

# 4. Transform lib.rs imports
LIB="$DST/src/lib.rs"

# Add #![allow(clippy::ptr_arg)] after doc comments
sed -i '' '/^\/\/!/a\
\
#![allow(clippy::ptr_arg)]
' "$LIB"

# Remove old imports and add prelude
sed -i '' 's/use super::{$/use qdos_plugin_api::prelude::*;/' "$LIB"
sed -i '' '/^    AppEntry,/d' "$LIB"
sed -i '' '/^    KeyHandleResult,/d' "$LIB"
sed -i '' '/^    Plugin,/d' "$LIB"
sed -i '' '/^    PluginCapabilities,/d' "$LIB"
sed -i '' '/^    PluginCategory,/d' "$LIB"
sed -i '' '/^    PluginMenuItem,/d' "$LIB"
sed -i '' '/^    PluginStatusInfo,/d' "$LIB"
sed -i '' '/^    SoundEvent,/d' "$LIB"
sed -i '' '/^};$/d' "$LIB"

# Remove crate::ui::components imports (now in prelude)
sed -i '' '/use crate::ui::components::/d' "$LIB"

# Remove crate::app::ThemeColors (now in prelude)
sed -i '' '/use crate::app::ThemeColors/d' "$LIB"

# Fix ThemeColors references
sed -i '' 's/&crate::app::ThemeColors/\&ThemeColors/g' "$LIB"

# Get the struct name (PascalCase of plugin name)
STRUCT=$(echo "$NAME" | sed -r 's/(^|_)([a-z])/\U\2/g')Plugin

# Add inventory registration at the end if not present
if ! grep -q "inventory::submit!" "$LIB"; then
    cat >> "$LIB" << EOF

// Self-registration
inventory::submit! {
    PluginRegistration::new("$NAME", || Box::new(${STRUCT}::new()))
}
EOF
fi

echo "Created $DST"
echo "  - Remember to:"
echo "    1. Add to workspace Cargo.toml members"
echo "    2. Add to main Cargo.toml dependencies"
echo "    3. Update src/plugins/mod.rs"
echo "    4. Remove $SRC after verifying"
echo "    5. Run: cargo fmt && cargo clippy -- -D warnings"
