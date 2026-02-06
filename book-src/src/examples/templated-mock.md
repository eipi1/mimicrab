# Example: Templated Mock

Templating allows you to echo back information from the request, making your mocks feel more dynamic without writing any code.

## Scenario

We want to create a personalized login message that uses the username from the request body and the API version from the path.

## Configuration

| Field | Value |
|-------|-------|
| **Method** | `POST` |
| **Path** | `/v1/login` |
| **Status** | `200` |
| **Body** | `{"message": "Welcome, {{body.username}}! You are using API {{path[0]}}."}` |

## Testing

Send a `POST` request with a JSON body:

```bash
curl -X POST http://localhost:3000/v1/login \
     -H "Content-Type: application/json" \
     -d '{"username": "MimicrabUser"}'
```

### Expected Result

```json
{
  "message": "Welcome, MimicrabUser! You are using API v1."
}
```

> \[!TIP]
> You can also use indices for path segments, such as `{{path[0]}}` for the first segment, `{{path[1]}}` for the second, and so on.
