# Templating

Mimicrab supports simple templating in response bodies and headers. This allows you to echo back parts of the request dynamically.

## Path Segments

You can access segments of the request path using the `{{path[index]}}` syntax.

- `{{path[0]}}`: The first segment after the domain.
- `{{path[1]}}`: The second segment, and so on.

**Example**:
If a request is made to `/users/123`:
- `{{path[0]}}` resolves to `users`
- `{{path[1]}}` resolves to `123`

## Request Body

You can access values from the JSON request body using the `{{body.path}}` syntax.

- `{{body.name}}`: Access the `name` field in the root object.
- `{{body.user.id}}`: Access nested fields.
- `{{body.items[0].id}}`: Access items in an array.

**Example**:
If the request body is `{"user": {"name": "Alice"}}`:
- `{{body.user.name}}` resolves to `Alice`

## Typed Resolution

By default, Mimicrab attempts to maintain the data type of the resolved value when the template marker is the only content in a JSON field.

- `{"id": "{{path[1]}}"}`: If `path[1]` is `123`, it resolves to `{"id": 123}` (Number).
- `{"active": "{{body.flag}}"}`: If `flag` is `true`, it resolves to `{"active": true}` (Boolean).
- `{"data": "{{body.obj}}"}`: Resolves to the full JSON Object/Array.

## String Conversion Filter

If you want to explicitly force a typed value into a string, use the `:string` filter.

| Syntax | Description |
|--------|-------------|
| `{{path[n]:string}}` | Forces path segment to stay as a string |
| `{{body.field:string}}` | Converts number/boolean body field to string |
| `{{path[n]:int}}` | Parses path segment as an integer |
| `{{body.field:int}}` | Parses string body field as an integer |
| `{{path[n]:bool}}` | Parses path segment as a boolean (`true`/`false`) |
| `{{body.field:bool}}` | Parses string body field as a boolean |

**Example**:
`{"id_str": "{{path[1]:string}}"}` resolves to `{"id_str": "123"}`.

## Usage in UI

Simply enter the placeholders wrapped in double curly braces `{{ }}` in the "Response Body" or "Response Headers" section of the mock creation form. Partial templates (e.g., `Hello {{body.name}}`) always resolve to strings.
