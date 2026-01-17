#!/bin/bash

echo "Starting Mock Server tests..."

# Helper function to print test results
test_match() {
  local id=$1
  local cmd=$2
  echo "Test case: $id"
  response=$(eval $cmd)
  echo "Response: $response"
  echo "-----------------------------------"
}

# Wait for server to start if needed (not needed if run after server is up)

test_match "GET /status" "curl -s http://localhost:3000/status"
test_match "POST /users with matching body" "curl -s -X POST -H 'Content-Type: application/json' -d '{\"name\": \"John Doe\"}' http://localhost:3000/users"
test_match "GET /secure with matching header" "curl -s -H 'Authorization: Bearer secret-token' http://localhost:3000/secure"
test_match "No match (404 expected)" "curl -s http://localhost:3000/non-existent"
