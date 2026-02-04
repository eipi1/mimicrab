# Lua Scripting

Mimicrab allows you to generate dynamic responses using Lua scripting. This is useful for complex logic that cannot be achieved with simple templating.

## How it Works

When a mock matches a request and has a `script` defined, Mimicrab executes the script in a Lua environment. The script has access to a global `request` table and is expected to return a table representing the response.

## The `request` Object

The `request` global table contains the following fields:

- `method`: The HTTP method (e.g., "GET", "POST").
- `path`: The request path (e.g., "/api/v1/resource").
- `headers`: A table containing all request headers.
- `body`: The JSON request body (parsed as a Lua table).

## Example Script

```lua
local res = {
    status = 201,
    headers = {
        ["Content-Type"] = "application/json",
        ["X-Generated-By"] = "Lua"
    },
    body = {
        message = "Processed " .. request.method .. " request for " .. request.path,
        received_data = request.body
    }
}
return res
```

## Configuring in UI

1. Open the "Create Mock" or "Edit Mock" modal.
2. Expand "Advanced Options".
3. Toggle "Enable Lua Scripting".
4. Enter your script in the editor.
