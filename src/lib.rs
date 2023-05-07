//! # Async-rusqlite
//!
//! A tiny async wrapper around [`rusqlite`]. Use [`crate::Connection`]
//! to open a connection, and then [`crate::Connection::call()`] to
//! execute commands against it.
//!
//! ```rust
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use async_rusqlite::Connection;
//!
//! #[derive(Debug)]
//! struct Person {
//!     id: i32,
//!     name: String,
//!     data: Option<Vec<u8>>,
//! }
//!
//! let conn = Connection::open_in_memory().await?;
//!
//! conn.call(|conn| {
//!     conn.execute(
//!         "CREATE TABLE person (
//!             id   INTEGER PRIMARY KEY,
//!             name TEXT NOT NULL,
//!             data BLOB
//!         )",
//!         (),
//!     )
//! }).await?;
//!
//! let me = Person {
//!     id: 0,
//!     name: "Steven".to_string(),
//!     data: None,
//! };
//!
//! conn.call(move |conn| {
//!     conn.execute(
//!         "INSERT INTO person (name, data) VALUES (?1, ?2)",
//!         (&me.name, &me.data),
//!     )
//! }).await?;
//!
//! # Ok(())
//! # }
//! ```

use asyncified::Asyncified;
use std::path::Path;

// re-export rusqlite types.
pub use rusqlite;

/// A handle which allows access to the underlying [`rusqlite::Connection`]
/// via [`Connection::call()`].
#[derive(Debug, Clone)]
pub struct Connection {
    // None if connection is closed, else Some(connection).
    conn: Asyncified<Option<rusqlite::Connection>>
}

impl Connection {
    /// Open a new connection to an SQLite database. If a database does not exist at the
    /// path, one is created.
    ///
    /// # Failure
    ///
    /// Will return `Err` if `path` cannot be converted to a C-compatible string
    /// or if the underlying SQLite open call fails.
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Connection,rusqlite::Error> {
        let path = path.as_ref().to_owned();
        let conn = Asyncified::new(move || rusqlite::Connection::open(path).map(Some)).await?;
        Ok(Connection { conn })
    }

    /// Open a new connection to an in-memory SQLite database.
    ///
    /// # Failure
    ///
    /// Will return `Err` if the underlying SQLite open call fails.
    pub async fn open_in_memory() -> Result<Connection,rusqlite::Error> {
        let conn = Asyncified::new(|| rusqlite::Connection::open_in_memory().map(Some)).await?;
        Ok(Connection { conn })
    }

    /// Open a new connection to a SQLite database.
    ///
    /// [Database Connection](http://www.sqlite.org/c3ref/open.html) for a description of valid
    /// flag combinations.
    ///
    /// # Failure
    ///
    /// Will return `Err` if `path` cannot be converted to a C-compatible
    /// string or if the underlying SQLite open call fails.
    pub async fn open_with_flags<P: AsRef<Path>>(path: P, flags: rusqlite::OpenFlags) -> Result<Connection,rusqlite::Error> {
        let path = path.as_ref().to_owned();
        let conn = Asyncified::new(move || rusqlite::Connection::open_with_flags(path, flags).map(Some)).await?;
        Ok(Connection { conn })
    }

    /// Open a new connection to a SQLite database using the specific flags and
    /// vfs name.
    ///
    /// [Database Connection](http://www.sqlite.org/c3ref/open.html) for a description of valid
    /// flag combinations.
    ///
    /// # Failure
    ///
    /// Will return `Err` if either `path` or `vfs` cannot be converted to a
    /// C-compatible string or if the underlying SQLite open call fails.
    pub async fn open_with_flags_and_vfs<P: AsRef<Path>>(
        path: P,
        flags: rusqlite::OpenFlags,
        vfs: &str,
    ) -> Result<Connection,rusqlite::Error> {
        let path = path.as_ref().to_owned();
        let vfs = vfs.to_owned();
        let conn = Asyncified::new(move || rusqlite::Connection::open_with_flags_and_vfs(path, flags, &vfs).map(Some)).await?;
        Ok(Connection { conn })
    }

    /// Open a new connection to an in-memory SQLite database.
    ///
    /// [Database Connection](http://www.sqlite.org/c3ref/open.html) for a description of valid
    /// flag combinations.
    ///
    /// # Failure
    ///
    /// Will return `Err` if the underlying SQLite open call fails.
    pub async fn open_in_memory_with_flags(flags: rusqlite::OpenFlags) -> Result<Connection,rusqlite::Error> {
        Connection::open_with_flags(":memory:", flags).await
    }

    /// Open a new connection to an in-memory SQLite database using the specific
    /// flags and vfs name.
    ///
    /// [Database Connection](http://www.sqlite.org/c3ref/open.html) for a description of valid
    /// flag combinations.
    ///
    /// # Failure
    ///
    /// Will return `Err` if `vfs` cannot be converted to a C-compatible
    /// string or if the underlying SQLite open call fails.
    pub async fn open_in_memory_with_flags_and_vfs(flags: rusqlite::OpenFlags, vfs: &str) -> Result<Connection,rusqlite::Error> {
        Connection::open_with_flags_and_vfs(":memory:", flags, vfs).await
    }

    /// Close the SQLite connection.
    ///
    /// This is functionally equivalent to the `Drop` implementation for
    /// [`Connection`] except that on failure, it returns the error. Unlike
    /// the [`rusqlite`] version of this method, it does not need to consume
    /// `self`.
    ///
    /// # Failure
    ///
    /// Will return `Err` if the underlying SQLite call fails.
    pub async fn close(&self) -> Result<(),Error> {
        self.conn.call(|conn| {
            match conn.take() {
                Some(c) => {
                    match c.close() {
                        Ok(_) => Ok(()),
                        Err((c, err)) => {
                            // close failed; replace the connection and
                            // return the error.
                            *conn = Some(c);
                            Err(Error::Rusqlite(err))
                        }
                    }
                },
                // Already closed!
                None => Err(Error::AlreadyClosed)
            }
        }).await
    }

    /// Run some arbitrary function against the [`rusqlite::Connection`] and return the result.
    ///
    /// # Failure
    ///
    /// Will return Err if the connection is closed, or if the provided function returns an error.
    /// The error type must impl [`From<AlreadyClosed>`] to handle this possibility being emitted.
    pub async fn call<R, E, F>(&self, f: F) -> Result<R,E>
    where
        R: Send + 'static,
        E: Send + 'static + From<AlreadyClosed>,
        F: Send + 'static + FnOnce(&mut rusqlite::Connection) -> Result<R, E>
    {
        self.conn.call(|conn| {
            match conn {
                Some(conn) => Ok(f(conn)?),
                None => Err(AlreadyClosed.into())
            }
        }).await
    }
}

/// If the connection is already closed, this will be returned
/// for the user to convert into their own error type. This can be
/// converted into [`Error`] and [`rusqlite::Error`] so that either
/// can be returned in the [`Connection::call()`] function.
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
pub struct AlreadyClosed;

impl From<AlreadyClosed> for rusqlite::Error {
    fn from(_: AlreadyClosed) -> Self {
        // There's not an ideal match for this error, so
        // just output something that is sortof sensible:
        let e = rusqlite::ffi::Error {
            code: rusqlite::ffi::ErrorCode::CannotOpen,
            extended_code: rusqlite::ffi::SQLITE_CANTOPEN
        };
        rusqlite::Error::SqliteFailure(e, None)
    }
}

/// An error emitted if closing the connection fails.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// The connection to SQLite has already been closed.
    AlreadyClosed,
    /// A `rusqlite` error occured trying to close the connection.
    Rusqlite(rusqlite::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::AlreadyClosed => write!(f, "The connection has already been closed"),
            Error::Rusqlite(e) => write!(f, "Rusqlite error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::AlreadyClosed => None,
            Error::Rusqlite(e) => Some(e),
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(value: rusqlite::Error) -> Self {
        Error::Rusqlite(value)
    }
}

impl From<AlreadyClosed> for Error {
    fn from(_: AlreadyClosed) -> Self {
        Error::AlreadyClosed
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_many_calls() -> Result<(), Error> {
        let conn = Connection::open_in_memory().await?;

        conn.call(|conn| {
            conn.execute(
                "CREATE TABLE numbers (
                    id   INTEGER PRIMARY KEY,
                    num  INTEGER NOT NULL
                )",
                (),
            )
        }).await?;

        for n in 0..10000 {
            conn.call(move |conn| {
                conn.execute(
                    "INSERT INTO numbers (num) VALUES (?1)",
                    (n,)
                )
            }).await?;
        }

        let count: usize = conn.call(|conn| {
            conn.query_row(
                "SELECT count(num) FROM numbers",
                (),
                |r| r.get(0)
            )
        }).await?;

        assert_eq!(count, 10000);
        Ok(())
    }

    #[tokio::test]
    async fn closes_once() {
        let conn = Connection::open_in_memory().await.unwrap();

        conn.close().await.expect("should close ok first time");
        let err = conn.close().await.expect_err("should error second time");

        assert_eq!(err, Error::AlreadyClosed);
    }

    #[tokio::test]
    async fn cant_call_after_close() {
        let conn = Connection::open_in_memory().await.unwrap();

        conn.close().await.expect("should close ok");
        let err = conn
            .call(|_conn| Ok::<_,Error>(()))
            .await
            .expect_err("should error second time");

        assert_eq!(err, Error::AlreadyClosed);
    }

    #[tokio::test]
    async fn custom_call_error() {
        // Custom error type that can capture possibility
        // of connection being closed.
        #[derive(Debug,PartialEq)]
        pub enum MyErr { AlreadyClosed, Other(&'static str) }
        impl From<AlreadyClosed> for MyErr {
            fn from(_: AlreadyClosed) -> MyErr {
                MyErr::AlreadyClosed
            }
        }

        let conn = Connection::open_in_memory().await.unwrap();

        let err = conn
            .call(|_conn| Err::<(),_>(MyErr::Other("foo")))
            .await
            .expect_err("should error");

        assert_eq!(err, MyErr::Other("foo"));

        conn.close().await.unwrap();

        let err = conn
            .call(|_conn| Ok::<_,MyErr>(()))
            .await
            .expect_err("should error");

        assert_eq!(err, MyErr::AlreadyClosed);
    }
}