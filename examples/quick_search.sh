#!/bin/bash
# Quick search example - requires OSIT server running

QUERY="${1:-How does Rust ownership work?}"
DEPTH="${2:-quick}"

echo "=== Checking index stats ==="
curl -s http://localhost:8765/stats | jq '.'

echo ""
echo "=== Searching for: $QUERY ==="
curl -s -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d "{
    \"query\": \"$QUERY\",
    \"depth\": \"$DEPTH\",
    \"max_pages\": 10
  }" | jq '.'
