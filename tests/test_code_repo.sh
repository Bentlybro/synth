#!/bin/bash
# Test code repository extractor

set -e

API="http://localhost:8765"

echo "Testing Code Repository Extractor..."
echo ""

# Test 1: Extract a small repo (basic mode)
echo "Test 1: Extract Synth repo (basic)"
curl -s -X POST "$API/extract" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://github.com/Bentlybro/synth"
  }' | jq '{
    url,
    title,
    content_type,
    metadata,
    content_preview: .content[:500]
  }'

echo ""
echo "================================"
echo ""

# Test 2: Extract with query
echo "Test 2: Extract with analysis query"
curl -s -X POST "$API/extract" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://github.com/Bentlybro/synth",
    "query": "What does this project do and what is its architecture?"
  }' | jq '{
    url,
    title,
    content_type,
    metadata,
    analysis: .analysis | {key_facts, confidence}
  }'

echo ""
echo "================================"
echo ""

# Test 3: Try a larger repo (Tokio)
echo "Test 3: Extract Tokio repo (basic)"
curl -s -X POST "$API/extract" \
  -H "Content-Type: application/json" \
  -d '{
    "url": "https://github.com/tokio-rs/tokio",
    "query": "Explain how Tokio handles async I/O"
  }' | jq '{
    url,
    title,
    content_type,
    metadata,
    analysis: .analysis | {key_facts: .key_facts[:3], confidence}
  }'

echo ""
echo "Done!"
