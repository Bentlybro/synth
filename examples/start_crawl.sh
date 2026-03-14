#!/bin/bash
# Start crawling with seed URLs

MAX_PAGES="${1:-100}"

echo "=== Starting crawl (max $MAX_PAGES pages) ==="
curl -s -X POST http://localhost:8765/crawl \
  -H "Content-Type: application/json" \
  -d "{
    \"max_pages\": $MAX_PAGES,
    \"seed_urls\": [
      \"https://doc.rust-lang.org/book/\",
      \"https://stackoverflow.com/questions/tagged/rust\",
      \"https://developer.mozilla.org/en-US/docs/Web/JavaScript\",
      \"https://en.wikipedia.org/wiki/Programming_language\"
    ]
  }" | jq '.'

echo ""
echo "Crawl started in background. Check stats with:"
echo "  curl http://localhost:8765/stats"
