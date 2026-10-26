# Changelog

## 0.6.0

- Rename the small `IntoName` trait to `AsName` to better reflect what it is doing.
- Update to latest `rusqlite` (0.41.0) and `asyncified` (0.7.0).
- Move to the 2024 edition.

## 0.5.0

- Update to latest `rusqlite` (0.37.0).

## 0.4.0

- Add ConnectionBuilder and move less common open calls to that, as well as exposing some other configuration from the underlying `Asyncified` instance (notably an `on_close` handler, to allow reacting to the db being closed).

## 0.3.0

- More precise error types:
  - return `rusqlite::Error` whenevr possible, and `Error` only when connection is closed (because we have to represent the possibility that it's been closed already).
  - Allow user errors to be returned from `.call()`, so long as they implement `From<AlreadyClosed>` to capture that possibility.

## 0.2.0

- Improve the docs and bump to asyncified 0.5

## 0.1.0

- Initial release