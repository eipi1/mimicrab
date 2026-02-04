# Example: Dynamic Lua Mock

This example demonstrates how to use Lua scripting to process an incoming order and return a dynamic status based on the stock quantity.

## Scenario

We want to mock an inventory system. If the `quantity` in the request is greater than 10, we respond with "Insufficient Stock". Otherwise, we respond with "Success".

## Request

**Endpoint**: `POST /api/inventory/check`
**Body**:
```json
{
  "item_id": "item_123",
  "quantity": 5
}
```

## Lua Script

```lua
local body = request.body
local stock_status = "Success"
local status_code = 200

if body.quantity > 10 then
    stock_status = "Insufficient Stock"
    status_code = 400
end

return {
    status = status_code,
    headers = { ["Content-Type"] = "application/json" },
    body = {
        item_id = body.item_id,
        status = stock_status,
        timestamp = os.date("!%Y-%m-%dT%H:%M:%SZ")
    }
}
```

## Expected Response (Success)

```json
{
  "item_id": "item_123",
  "status": "Success",
  "timestamp": "2026-02-04T10:25:00Z"
}
```
