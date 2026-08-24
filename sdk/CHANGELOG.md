# Changelog

## Unreleased

### Breaking changes

- Removed `CliTransport` and the `cli` client option. The SDK now requires an
  `http.baseUrl` for a running `agnetd serve` daemon (or the in-memory `mock`
  option in tests).
- Feed subscriptions and inference result streaming now connect directly to
  the daemon over WebSocket.
