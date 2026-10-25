# Observability

This directory holds the OpenTelemetry / metrics / tracing configuration for a
self-hosted Aevio deployment. The server binaries already emit structured
logs with request and job IDs; wire them to your collector via the standard
`OTEL_EXPORTER_OTLP_ENDPOINT` environment variable and the reverse proxy in
front of the server.

No collector configuration is shipped here yet; add one when a concrete
deployment requires it.
