# Quickstart

Get your first mock running in less than a minute.

## 1. Open the Dashboard

By default, the Mimicrab dashboard is available at `http://localhost:3000`.

## 2. Create a Mock

1. Click the **+ Create Mock** button.
2. Enter the path: `/hello`.
3. Select the method: `GET`.
4. Enter the response status: `200`.
5. Click **Save Mock**.

## 3. Test it Out

You can test your mock directly from the dashboard using the **Test** button, or use `curl`:

```bash
curl http://localhost:3000/hello
```

**Expected Response**:
```text
(Empty body with 200 OK)
```
