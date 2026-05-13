# API Reference

## Sidecar HTTP Endpoints

| Endpoint | Purpose |
|----------|--------|
| `/` | Serves overlay HTML (SimHub overlay) |
| `/src/{path}` | Serves overlay JS modules |
| `/styles/{path}` | Serves overlay CSS |
| `/health` | Health check |
| `/ws` | WebSocket telemetry/recommendations |
| `/test/telemetry` | Inject telemetry (dev/test) |
| `/test/recommendation` | Inject recommendation (dev/test) |

**Minimum version:** Sidecar v0.2.0+ (overlay serving)

## Overlay Serving

The SimHub overlay is now served directly from the sidecar HTTP endpoint (`http://127.0.0.1:7778/`). The `.djson` file points SimHub to this address. No separate static file server or overlay bundle is required.

## WebSocket API

See [ADR-0002](../adr/0002-ws-api-contract.md) for schema and contract details.

## Test Injection Endpoints

For development/testing, you can inject telemetry and recommendations:

```bash
curl -s -X POST http://127.0.0.1:7778/test/telemetry \
  -H 'Content-Type: application/json' \
  -d '{"data":{"speed_kph":120,"gear":3}}'

curl -s -X POST http://127.0.0.1:7778/test/recommendation \
  -H 'Content-Type: application/json' \
  -d '{"data":{"id":"01test","session_id":"01sess","lap_number":2,"category":"springs","title":"Front bottoming out","detected":"Front suspension >95% travel on corners.","confidence":"high","adjustment":{"summary":"Front spring rate 85 → 92 N/mm"}}}'
```
