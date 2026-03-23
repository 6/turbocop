#!/usr/bin/env bash
set -euo pipefail

# Guardrail: production Rust source must not include compile-time paths under vendor/.
# Submodules are optional for crate consumers, so vendor include macros are packaging hazards.

if rg --pcre2 -n -U 'include_(str|bytes)!\s*\((?s:.*?)vendor/' src -g'*.rs'; then
  echo
  echo "error: found include_str!/include_bytes! macros referencing vendor/ in src code"
  echo "Use bundled data under src/resources/ instead."
  exit 1
fi

echo "ok: no vendor include macros found in src/**/*.rs"
