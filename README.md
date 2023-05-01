# async-rusqlite

A tiny, executor agnostic library for using `rusqlite` in async contexts. This essentially just spawns an `rusqlite` onto a long lived thread and sends messages to that thread to operate on it.

This library is inspired by `tokio-rusqlite`, but with the following design differences:
- Executor agnostic; can be used with `tokio`, `async-std` or whatever else.
- Bounded channels; `tokio-rusqlite` uses an unbounded channel to send messages to the `rusqlite` thread. This library uses bounded channels to allow backpressure to propagate back to the async task if the database is unable to keep up with the calls made to it.
- Fewer dependencies; aside from `rusqlite`, the tree of additional dependencies bought in is 1 (`asyncified`, which in itself is very small).

If you don't care about the above, prefer `tokio-rusqlite`, which is more heavily battle tested and relies on the venerable `crossbeam-channel` rather than my own fairly naive channel implementations in `asyncified`.