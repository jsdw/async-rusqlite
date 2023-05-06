# async-rusqlite

A tiny, executor agnostic library for using `rusqlite` in async contexts. This essentially just spawns an `rusqlite` onto a long lived thread and sends closures to that thread to operate on it.

This library is inspired by `tokio-rusqlite`, but with the following design differences:
- Executor agnostic; can be used with `tokio`, `async-std` or whatever else.
- Bounded channels; `tokio-rusqlite` uses an unbounded channel to send messages to the `rusqlite` thread. This library uses bounded channels to allow backpressure to propagate back to the async task if the database is unable to keep up with the calls made to it.
- Fewer dependencies; aside from `rusqlite`, the tree of additional dependencies bought in is 1 (`asyncified`, which in itself is very small).

If you don't care about the above, prefer `tokio-rusqlite`, which is more heavily battle tested and relies on the venerable `crossbeam-channel` rather than my own fairly naive channel implementations in `asyncified`.

```rust
use async_rusqlite::Connection;

#[derive(Debug)]
struct Person {
    id: i32,
    name: String,
    data: Option<Vec<u8>>,
}

let conn = Connection::open_in_memory().await?;

conn.call(|conn| {
    conn.execute(
        "CREATE TABLE person (
            id   INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            data BLOB
        )",
        (),
    )
}).await?;

let me = Person {
    id: 0,
    name: "Steven".to_string(),
    data: None,
};

conn.call(move |conn| {
    conn.execute(
        "INSERT INTO person (name, data) VALUES (?1, ?2)",
        (&me.name, &me.data),
    )
}).await?;
```