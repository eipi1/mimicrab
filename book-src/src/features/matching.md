# Request Matching

Mimicrab uses a powerful matching engine to identify which mock response to serve based on the incoming request.

## Matching Criteria

A request must satisfy all defined conditions in an expectation to match.

### HTTP Method
Matches the exact HTTP method (e.g., `GET`, `POST`).

### Path
Matches the exact request path (e.g., `/api/v1/users`).

### Headers
Matches if the request contains all specified headers with their corresponding values.

### Body (JSON)
Matches if the request body contains all key-value pairs specified in the condition. Mimicrab supports matching nested JSON structures.

**Example Condition**:
```json
{
  "user": {
    "role": "admin"
  }
}
```
This will match any request body that has a `user` object with a `role` set to `admin`, regardless of other fields.
