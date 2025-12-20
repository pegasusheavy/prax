//! Connection wrapper for SQLx.

use crate::config::DatabaseBackend;
use crate::error::SqlxResult;
use crate::pool::SqlxPool;

/// A database connection acquired from the pool.
pub enum SqlxConnection {
    /// PostgreSQL connection
    #[cfg(feature = "postgres")]
    Postgres(sqlx::pool::PoolConnection<sqlx::Postgres>),
    /// MySQL connection
    #[cfg(feature = "mysql")]
    MySql(sqlx::pool::PoolConnection<sqlx::MySql>),
    /// SQLite connection
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::pool::PoolConnection<sqlx::Sqlite>),
}

impl SqlxConnection {
    /// Acquire a connection from the pool.
    pub async fn acquire(pool: &SqlxPool) -> SqlxResult<Self> {
        match pool {
            #[cfg(feature = "postgres")]
            SqlxPool::Postgres(p) => {
                let conn = p.acquire().await?;
                Ok(Self::Postgres(conn))
            }
            #[cfg(feature = "mysql")]
            SqlxPool::MySql(p) => {
                let conn = p.acquire().await?;
                Ok(Self::MySql(conn))
            }
            #[cfg(feature = "sqlite")]
            SqlxPool::Sqlite(p) => {
                let conn = p.acquire().await?;
                Ok(Self::Sqlite(conn))
            }
        }
    }

    /// Get the database backend type.
    pub fn backend(&self) -> DatabaseBackend {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(_) => DatabaseBackend::Postgres,
            #[cfg(feature = "mysql")]
            Self::MySql(_) => DatabaseBackend::MySql,
            #[cfg(feature = "sqlite")]
            Self::Sqlite(_) => DatabaseBackend::Sqlite,
        }
    }
}

/// A transaction handle.
pub enum SqlxTransaction<'c> {
    /// PostgreSQL transaction
    #[cfg(feature = "postgres")]
    Postgres(sqlx::Transaction<'c, sqlx::Postgres>),
    /// MySQL transaction
    #[cfg(feature = "mysql")]
    MySql(sqlx::Transaction<'c, sqlx::MySql>),
    /// SQLite transaction
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::Transaction<'c, sqlx::Sqlite>),
}

impl<'c> SqlxTransaction<'c> {
    /// Commit the transaction.
    pub async fn commit(self) -> SqlxResult<()> {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(tx) => tx.commit().await?,
            #[cfg(feature = "mysql")]
            Self::MySql(tx) => tx.commit().await?,
            #[cfg(feature = "sqlite")]
            Self::Sqlite(tx) => tx.commit().await?,
        }
        Ok(())
    }

    /// Rollback the transaction.
    pub async fn rollback(self) -> SqlxResult<()> {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(tx) => tx.rollback().await?,
            #[cfg(feature = "mysql")]
            Self::MySql(tx) => tx.rollback().await?,
            #[cfg(feature = "sqlite")]
            Self::Sqlite(tx) => tx.rollback().await?,
        }
        Ok(())
    }

    /// Get the database backend type.
    pub fn backend(&self) -> DatabaseBackend {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(_) => DatabaseBackend::Postgres,
            #[cfg(feature = "mysql")]
            Self::MySql(_) => DatabaseBackend::MySql,
            #[cfg(feature = "sqlite")]
            Self::Sqlite(_) => DatabaseBackend::Sqlite,
        }
    }
}

impl SqlxPool {
    /// Begin a transaction.
    pub async fn begin(&self) -> SqlxResult<SqlxTransaction<'_>> {
        match self {
            #[cfg(feature = "postgres")]
            Self::Postgres(pool) => {
                let tx = pool.begin().await?;
                Ok(SqlxTransaction::Postgres(tx))
            }
            #[cfg(feature = "mysql")]
            Self::MySql(pool) => {
                let tx = pool.begin().await?;
                Ok(SqlxTransaction::MySql(tx))
            }
            #[cfg(feature = "sqlite")]
            Self::Sqlite(pool) => {
                let tx = pool.begin().await?;
                Ok(SqlxTransaction::Sqlite(tx))
            }
        }
    }
}

