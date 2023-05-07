# Changelog

## 0.3.0

- More precise error types:
  - return `rusqlite::Error` whenevr possible, and `Error` only when connection is closed (because we have to represent the possibility that it's been closed already).
  - Allow user errors to be returned from `.call()`, so long as they implement `From<AlreadyClosed>` to capture that possibility.

## 0.2.0

- Improve the docs and bump to asyncified 0.5

## 0.1.0

- Initial release