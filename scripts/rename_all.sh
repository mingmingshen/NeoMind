#!/bin/bash
# Master script to rename all crates to NeoMind branding
#
# Execute in order from least dependent to most dependent
# This ensures cargo can still resolve dependencies during the process
#
# WARNING: This will modify your working directory!
# Make sure you have committed or stashed your changes before running.

set -e

echo "=========================================="
echo "NeoMind Rebranding - All Crates"
echo "=========================================="
echo ""
echo "This will rename all crates from edge-ai-* to neomind-*"
echo ""

# Verify we're in the project root
if [ ! -f "Cargo.toml" ] || [ ! -d "crates" ]; then
    echo "Error: Please run this script from the project root"
    exit 1
fi

# Ask for confirmation
read -p "Have you committed your changes? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Please commit your changes first, then run this script again."
    exit 1
fi

read -p "Continue with renaming? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RENAME_SCRIPT="$SCRIPT_DIR/rename_crate.sh"

# Make sure rename_crate.sh is executable
chmod +x "$RENAME_SCRIPT"

# Execute renames in dependency order
# (crates with no dependencies first)
echo "Starting rename process..."
echo ""

# Phase 1: No/low dependencies
echo "Phase 1: Core infrastructure crates..."
$RENAME_SCRIPT testing testing
$RENAME_SCRIPT storage storage
$RENAME_SCRIPT sandbox sandbox
$RENAME_SCRIPT commands commands
echo ""

# Phase 2: Core traits and interfaces
echo "Phase 2: Core crate..."
$RENAME_SCRIPT core core
echo ""

# Phase 3: Business logic crates
echo "Phase 3: Business logic crates..."
$RENAME_SCRIPT llm llm
$RENAME_SCRIPT tools tools
$RENAME_SCRIPT devices devices
$RENAME_SCRIPT rules rules
$RENAME_SCRIPT messages messages
$RENAME_SCRIPT memory memory
$RENAME_SCRIPT automation automation
$RENAME_SCRIPT integrations integrations
echo ""

# Phase 4: Higher-level crates
echo "Phase 4: Agent and API..."
$RENAME_SCRIPT agent agent
$RENAME_SCRIPT cli cli
$RENAME_SCRIPT api api
echo ""

# Phase 5: Plugin SDK
echo "Phase 5: Plugin SDK..."
$RENAME_SCRIPT plugin-sdk plugin-sdk
echo ""

echo "=========================================="
echo "All renames complete!"
echo "=========================================="
echo ""
echo "Next steps:"
echo "1. Check for any remaining edge_ai references:"
echo "   grep -r 'edge_ai' --include='*.rs' --include='*.toml' ."
echo ""
echo "2. Build the project:"
echo "   cargo build --all-targets"
echo ""
echo "3. Run tests:"
echo "   cargo test --all"
echo ""
echo "4. Commit changes:"
echo "   git add -A"
echo "   git commit -m 'refactor: rebrand to NeoMind'"
