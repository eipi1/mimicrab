# Response Configuration

Mimicrab allows you to define exactly what the mock server should return when a request matches.

## Response Components

### Status Code
Any valid HTTP status code (e.g., 200, 404, 500).

### Headers
Custom HTTP headers to include in the response.

### Body
The payload of the response. Mimicrab supports multiple body types:
- **JSON**: Automatically sets `Content-Type: application/json`.
- **Text / HTML**: Sets `Content-Type` based on your selection.
- **BSON**: Encodes the response as BSON if the `Accept` header matches.

## Configuration in UI

The "Response Status", "Response Headers", and "Response Body" fields in the mock form allow you to specify these components easily.
