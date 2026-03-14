#!/bin/bash
# OSIT search example - DuckDuckGo + AI analysis

QUERY="${1:-How does Rust ownership work?}"
DEPTH="${2:-quick}"

echo "=== Checking cache stats ==="
curl -s http://localhost:8765/stats | jq '.'

echo ""
echo "=== Searching DuckDuckGo for: $QUERY ==="
echo "This will:"
echo "  1. Search DuckDuckGo for top URLs"
echo "  2. Check cache (skip recently scraped pages)"
echo "  3. Scrape fresh content in parallel"
echo "  4. Analyze with Claude AI"
echo "  5. Synthesize comprehensive answer"
echo ""

curl -s -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d "{
    \"query\": \"$QUERY\",
    \"depth\": \"$DEPTH\"
  }" | jq '.'
