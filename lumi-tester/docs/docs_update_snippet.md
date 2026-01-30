
### Lark Integration
Send notifications to Lark/Feishu.

```yaml
- sendLarkMessage:
    webhook: "${LARK_WEBHOOK}"
    secret: "${LARK_SECRET}"
    title: "Test Report ${date}"
    content: "All tests passed at ${time}"
    status: "success" # success, failure, info, warning
    files:
      - "./output/report.json"
```

Automatic variables:
- `${time}`: Current time (HH:MM:SS)
- `${date}`: Current date (YYYY-MM-DD)
- `${timestamp}`: Unix timestamp

### File Upload
Since standard Webhooks cannot upload files directly, `files` content will be read and embedded into the message card.
