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

## Usage in UI

Simply enter the placeholders wrapped in double curly braces `{{ }}` in the "Response Body" or "Response Headers" section of the mock creation form.
