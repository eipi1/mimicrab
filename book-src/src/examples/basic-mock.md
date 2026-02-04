# Example: Basic Mock

The simplest way to use Mimicrab is to create a static response for a specific endpoint. This is perfect for mocking health checks or simple welcome messages.

## Configuration

| Field | Value |
|-------|-------|
| **Method** | `GET` |
| **Path** | `/api/hello` |
| **Status** | `200` |
| **Headers** | `Content-Type: text/plain` |
| **Body** | `Hello from Mimicrab!` |

## Testing

Use `curl` to verify the mock:

```bash
curl -i http://localhost:3000/api/hello
```

### Expected Result

```http
HTTP/1.1 200 OK
content-type: text/plain
content-length: 22

Hello from Mimicrab!
```
