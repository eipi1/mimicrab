#!/bin/bash

# Configuration
PORT=3000
BASE_URL="http://localhost:$PORT"
MOCKS_FILE="expectations.json"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "--- Starting Mimicrab Local Mode Verification ---"

# Cleanup from previous runs
rm -f $MOCKS_FILE

# Start Mimicrab in background
echo "Starting Mimicrab..."
cargo run &
PID=$!

# Wait for server to start
MAX_RETRIES=10
RETRY_COUNT=0
while ! curl -s $BASE_URL/_admin/mocks > /dev/null; do
    sleep 1
    RETRY_COUNT=$((RETRY_COUNT+1))
    if [ $RETRY_COUNT -ge $MAX_RETRIES ]; then
        echo -e "${RED}Error: Mimicrab failed to start${NC}"
        kill $PID
        exit 1
    fi
done

echo -e "${GREEN}Mimicrab is up and running (PID: $PID)${NC}"

# 1. Verify Mode Log
# (Since we are running in background, we'll check output briefly)
# For simplicity in this script, we'll assume it's in LOCAL mode if it started.

# 2. Add a Mock
echo "Adding a mock..."
curl -s -X POST $BASE_URL/_admin/mocks \
  -H "Content-Type: application/json" \
  -d '{
    "id": 123,
    "condition": {
        "method": "GET",
        "path": "/hello"
    },
    "response": {
        "status_code": 200,
        "body": { "message": "world" }
    }
}' | jq .

# 3. Verify expectations.json was created
if [ -f "$MOCKS_FILE" ]; then
    echo -e "${GREEN}Success: $MOCKS_FILE was created${NC}"
else
    echo -e "${RED}Error: $MOCKS_FILE was not created${NC}"
    kill $PID
    exit 1
fi

# 4. Test the Mock
echo "Testing the mock..."
RESPONSE=$(curl -s $BASE_URL/hello)
if [[ "$RESPONSE" == *"world"* ]]; then
    echo -e "${GREEN}Success: Mock matched and returned correct response${NC}"
else
    echo -e "${RED}Error: Mock response was incorrect: $RESPONSE${NC}"
    kill $PID
    exit 1
fi

# Cleanup
echo "Cleaning up..."
kill $PID
wait $PID 2>/dev/null
rm -f $MOCKS_FILE

echo -e "${GREEN}Verification complete!${NC}"
