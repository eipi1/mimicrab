#!/bin/bash

BASE_URL="http://localhost:3000"

echo "--- Testing Management API ---"

echo "Listing mocks..."
curl -s $BASE_URL/_admin/mocks | jq .

echo -e "\nAdding a new mock..."
curl -s -X POST -H "Content-Type: application/json" -d '{
  "id": 10,
  "condition": { "method": "GET", "path": "/hello" },
  "response": { "status_code": 200, "body": { "message": "hello world" } }
}' $BASE_URL/_admin/mocks | jq .

echo -e "\nTesting the new mock..."
curl -s $BASE_URL/hello | jq .

echo -e "\n--- Testing Parameterized Responses ---"

echo "Testing dynamic path and body replacement..."
curl -s -X POST -H "Content-Type: application/json" -d '{"user": {"name": "Alice"}}' $BASE_URL/echo/test | jq .

echo -e "\n--- Testing Log Streaming (Capture first 2 lines) ---"
# We'll run curl in background and take some output
(curl -s $BASE_URL/_admin/logs/stream | head -n 4) &
STREAM_PID=$!

sleep 2
curl -s $BASE_URL/status > /dev/null
curl -s $BASE_URL/non-existent > /dev/null

wait $STREAM_PID
echo "Log stream captured."
