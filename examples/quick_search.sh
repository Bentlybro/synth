#!/bin/bash
# Quick search example - requires OSIT server running

QUERY="${1:-How does Rust ownership work?}"
DEPTH="${2:-quick}"

curl -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d "{
    \"query\": \"$QUERY\",
    \"depth\": \"$DEPTH\",
    \"max_pages\": 10
  }" | jq '.'
