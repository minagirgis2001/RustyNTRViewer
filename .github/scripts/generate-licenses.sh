#!/bin/bash
set -euo pipefail

OUTPUT="${1:-THIRD_PARTY_LICENSES.html}"
TEMP_FILE="$(mktemp)"
trap 'rm -f "$TEMP_FILE"' EXIT

cargo about generate --workspace --frozen --fail about.hbs -o "$TEMP_FILE"

# Preserve the license wording while normalizing line endings and incidental
# trailing whitespace from upstream license files.
perl -pe 's/\r$//; s/[ \t]+$//' "$TEMP_FILE" > "$OUTPUT"
