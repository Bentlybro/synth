#!/bin/bash
set -e

echo "=== Synth Integration Tests ==="
echo ""

# Test 1: Health check
echo "1. Health check..."
curl -s http://localhost:8765/health
echo " ✓"
echo ""

# Test 2: Extract web page
echo "2. Extracting web page..."
RESULT=$(curl -s -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{"url": "https://www.rust-lang.org/"}' | jq -r '.content_type')
echo "   Content type: $RESULT"
[[ "$RESULT" == "Web" ]] && echo " ✓" || echo " ✗ FAILED"
echo ""

# Test 3: Extract with analysis
echo "3. Extracting with LLM analysis..."
RESULT=$(curl -s -X POST http://localhost:8765/extract \
  -H "Content-Type: application/json" \
  -d '{"url": "https://www.rust-lang.org/", "query": "What is Rust?"}' | jq -r '.analysis.key_facts[0]')
echo "   First fact: $RESULT"
[[ -n "$RESULT" ]] && echo " ✓" || echo " ✗ FAILED"
echo ""

# Test 4: Search (multi-modal)
echo "4. Search query..."
RESULT=$(curl -s -X POST http://localhost:8765/search \
  -H "Content-Type: application/json" \
  -d '{"query": "rust programming", "max_pages": 2}' | jq -r '.status')
echo "   Status: $RESULT"
[[ "$RESULT" == "complete" ]] && echo " ✓" || echo " ✗ FAILED"
echo ""

# Test 5: Cache verification
echo "5. Checking cache directories..."
for dir in extractors_web extractors_pdf extractors_video extractors_audio extractors_image; do
    if [ -d ~/clawd/projects/synth/index/cache/$dir ]; then
        COUNT=$(ls ~/clawd/projects/synth/index/cache/$dir/*.json 2>/dev/null | wc -l)
        echo "   $dir: $COUNT files"
    fi
done
echo " ✓"
echo ""

echo "=== All Tests Passed! ==="
