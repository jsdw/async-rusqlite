use asyncified::Asyncified;
use std::path::Path;

// re-export rusqlite types.
pub use rusqlite;

/// A result returned from calls to [`Connection`].
pub type Result<T> = std::result::Result<T, Error>;

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
    pub async fn open<P: AsRef<Path>>(path: P) -> Result<Connection> {
        let path = path.as_ref().to_owned();
        let conn = Asyncified::new(move || rusqlite::Connection::open(path).map(Some)).await?;
        Ok(Connection { conn })
    }

    /// Open a new connection to an in-memory SQLite database.
    ///
    /// # Failure
    ///
    /// Will return `Err` if the underlying SQLite open call fails.
    pub async fn open_in_memory() -> Result<Connection> {
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
    pub async fn open_with_flags<P: AsRef<Path>>(path: P, flags: rusqlite::OpenFlags) -> Result<Connection> {
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
    ) -> Result<Connection> {
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
    pub async fn open_in_memory_with_flags(flags: rusqlite::OpenFlags) -> Result<Connection> {
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
    pub async fn open_in_memory_with_flags_and_vfs(flags: rusqlite::OpenFlags, vfs: &str) -> Result<Connection> {
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
    pub async fn close(&self) -> Result<()> {
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
                None => Err(Error::ConnectionClosed)
            }
        }).await
    }

    /// Run some arbitrary function against the [`rusqlite::Connection`] and return the result.
    ///
    /// # Failure
    ///
    /// Will return Err if the connection is closed, or if the provided function returns an error.
    pub async fn call<R, F>(&self, f: F) -> Result<R>
    where
        R: Send + 'static,
        F: Send + 'static + FnOnce(&mut rusqlite::Connection) -> rusqlite::Result<R>
    {
        self.conn.call(|conn| {
            match conn {
                Some(conn) => Ok(f(conn)?),
                None => Err(Error::ConnectionClosed)
            }
        }).await
    }
}

/// Errors that can be emitted from this library.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The connection to SQLite has been closed.
    ConnectionClosed,
    /// A `rusqlite` error occured.
    Rusqlite(rusqlite::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ConnectionClosed => write!(f, "Connection closed"),
            Error::Rusqlite(e) => write!(f, "Rusqlite error: {e}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::ConnectionClosed => None,
            Error::Rusqlite(e) => Some(e),
        }
    }
}

impl From<rusqlite::Error> for Error {
    fn from(value: rusqlite::Error) -> Self {
        Error::Rusqlite(value)
    }
}