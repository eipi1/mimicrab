# Admin API

Mimicrab provides a management API under the `/_admin` prefix to manage mocks, view logs, and export/import configurations.

## Mocks Management

### List All Mocks
Returns a list of all currently active mock expectations.

- **URL**: `/_admin/mocks`
- **Method**: `GET`
- **Response**: `200 OK` (JSON array of expectations)

### Add a Mock
Creates a new mock expectation.

- **URL**: `/_admin/mocks`
- **Method**: `POST`
- **Body**: [Expectation Object](#expectation-object)
- **Response**: `201 Created`

### Update a Mock
Updates an existing mock by its ID.

- **URL**: `/_admin/mocks/{id}`
- **Method**: `PUT`
- **Body**: [Expectation Object](#expectation-object)
- **Response**: `200 OK` or `404 Not Found`

### Delete a Mock
Removes a mock by its ID.

- **URL**: `/_admin/mocks/{id}`
- **Method**: `DELETE`
- **Response**: `204 No Content` or `404 Not Found`

## Configuration & Logs

### Export Configuration
Exports all expectations as a JSON array.

- **URL**: `/_admin/export`
- **Method**: `GET`
- **Response**: `200 OK` (JSON array)

### Import Configuration
Overwrites the current expectations with a new list.

- **URL**: `/_admin/import`
- **Method**: `POST`
- **Body**: JSON array of expectations
- **Response**: `200 OK`

### Stream Logs
Streams incoming request logs via Server-Sent Events (SSE).

- **URL**: `/_admin/logs/stream`
- **Method**: `GET`
- **Response**: `200 OK` (Content-Type: `text/event-stream`)

### Metrics
Exposes Prometheus-compatible metrics.

- **URL**: `/_admin/metrics`
- **Method**: `GET`
- **Response**: `200 OK` (Text format)

## Data Structures

### Expectation Object
```json
{
  "id": 1,
  "condition": {
    "method": "GET",
    "path": "/api/test",
    "headers": {
      "Accept": "application/json"
    },
    "body": {
      "key": "value"
    }
  },
  "response": {
    "status_code": 200,
    "headers": {
      "Content-Type": "application/json"
    },
    "body": {
      "status": "ok"
    },
    "latency": 0
  }
}
```
