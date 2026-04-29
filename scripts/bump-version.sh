#!/bin/bash
# Bump version across all packages in the mono-repo.
# Usage: scripts/bump-version.sh <new-version>
#
# Updates:
#   Cargo.toml
#   pyproject.toml
#   python/string_space_client/pyproject.toml
#   python/string_space_completer/pyproject.toml
#   typescript/package.json

set -euo pipefail

if [ $# -ne 1 ]; then
    echo "Usage: $0 <new-version>"
    echo "  e.g. $0 0.7.0"
    exit 1
fi

VERSION="$1"

# Validate semver-ish
if ! echo "$VERSION" | rg -q '^\d+\.\d+\.\d+$'; then
    echo "Error: version must be semver (e.g. 0.7.0), got '$VERSION'"
    exit 1
fi

# Collect current versions
CARGO=$(rg '^version = "' Cargo.toml | head -1 | rg -o '"[^"]+"' | tr -d '"')
PYROOT=$(rg '^version = "' pyproject.toml | head -1 | rg -o '"[^"]+"' | tr -d '"')
PYCLIENT=$(rg '^version = "' python/string_space_client/pyproject.toml | head -1 | rg -o '"[^"]+"' | tr -d '"')
PYCOMPLETER=$(rg '^version = "' python/string_space_completer/pyproject.toml | head -1 | rg -o '"[^"]+"' | tr -d '"')
TS=$(rg '"version":' typescript/package.json | head -1 | rg -o '"[^"]+"' | tail -1 | tr -d '"')

echo "Current versions:"
echo "  Cargo.toml:                          $CARGO"
echo "  pyproject.toml:                      $PYROOT"
echo "  python/string_space_client/:         $PYCLIENT"
echo "  python/string_space_completer/:      $PYCOMPLETER"
echo "  typescript/package.json:             $TS"
echo ""
echo "Bumping all to: $VERSION"
echo ""

# Update each file
sed -i '' "s/^version = \"$CARGO\"/version = \"$VERSION\"/" Cargo.toml
sed -i '' "s/^version = \"$PYROOT\"/version = \"$VERSION\"/" pyproject.toml
sed -i '' "s/^version = \"$PYCLIENT\"/version = \"$VERSION\"/" python/string_space_client/pyproject.toml
sed -i '' "s/^version = \"$PYCOMPLETER\"/version = \"$VERSION\"/" python/string_space_completer/pyproject.toml
sed -i '' "s/\"version\": \"$TS\"/\"version\": \"$VERSION\"/" typescript/package.json

# Verify
echo "Updated versions:"
echo "  Cargo.toml:      $(rg '^version = "' Cargo.toml | head -1 | rg -o '"[^"]+"' | tr -d '"')"
echo "  pyproject.toml:  $(rg '^version = "' pyproject.toml | head -1 | rg -o '"[^"]+"' | tr -d '"')"
echo "  pyclient:        $(rg '^version = "' python/string_space_client/pyproject.toml | head -1 | rg -o '"[^"]+"' | tr -d '"')"
echo "  pycompleter:     $(rg '^version = "' python/string_space_completer/pyproject.toml | head -1 | rg -o '"[^"]+"' | tr -d '"')"
echo "  typescript:      $(rg '"version":' typescript/package.json | head -1 | rg -o '"[^"]+"' | tail -1 | tr -d '"')"
echo ""
echo "Done. Don't forget to git commit and tag (e.g. git tag v$VERSION)."
