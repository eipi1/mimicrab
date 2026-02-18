# Request Matching

Mimicrab uses a powerful matching engine to identify which mock response to serve based on the incoming request.

## Matching Criteria

A request must satisfy all defined conditions in an expectation to match.

### HTTP Method
Matches the exact HTTP method (e.g., `GET`, `POST`).

### Path
Mimicrab supports flexible path matching, including static paths, parameterized segments, and wildcards.

- **Static Match**: Matches the exact path.
  - Example: `/api/v1/users`
- **Parameterized Match**: Use `:name` to capture path segments.
  - Example: `/books/:id/author` matches `/books/123/author` and `/books/abc/author`.
  - Captured segments can be used in [Templating](../advanced/templating.md) via `{{path[index]}}`.
- **Wildcard Match**: Use `*` to match any characters.
  - **Prefix Wildcard**: `*/books` matches `/path/to/books`.
  - **Suffix Wildcard**: `/api/*` matches `/api/v1/users` and `/api/v2/posts`.
  - **Middle/Segment Wildcard**: `/static/*/main.js` matches `/static/v1/main.js`.

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
