use std::path::Path;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use agdb::{DbAny, QueryBuilder};

use crate::error::Error;
use crate::scope::ReadScope;
use crate::write::WriteScope;

/// Handle to the graph database. Cheap to clone; share one instance per
/// process. All access happens inside `read`/`write` closure scopes: write
/// scopes commit on `Ok` and roll back on `Err`. Scopes are synchronous and
/// short-lived; do not nest scope calls or hold their guards across awaits.
#[derive(Clone)]
pub struct Database {
    inner: Arc<RwLock<DbAny>>,
}

impl Database {
    /// Opens an in-memory database (tests and ephemeral data).
    ///
    /// # Errors
    /// Returns [`Error::Storage`] if the engine cannot start and
    /// [`Error::Storage`] if an index cannot be ensured.
    pub fn open_memory(name: &str, indexes: &[String]) -> Result<Self, Error> {
        let database = DbAny::new_memory(name)?;
        Self::start(database, indexes)
    }

    /// Opens a file-backed database with a write-ahead log. Missing parent
    /// directories are created.
    ///
    /// # Errors
    /// Returns [`Error::Invalid`] for non-utf8 paths, [`Error::Storage`] if
    /// the engine cannot start or an index cannot be ensured.
    pub fn open_mapped(path: &Path, indexes: &[String]) -> Result<Self, Error> {
        let filename = path
            .to_str()
            .ok_or_else(|| Error::Invalid(format!("non-utf8 database path {}", path.display())))?;
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .map_err(|error| Error::Storage(format!("create {}: {error}", parent.display())))?;
        }
        let database = DbAny::new_mapped(filename)?;
        Self::start(database, indexes)
    }

    fn start(mut database: DbAny, indexes: &[String]) -> Result<Self, Error> {
        ensure_indexes(&mut database, indexes)?;
        Ok(Self {
            inner: Arc::new(RwLock::new(database)),
        })
    }

    /// Runs a read-only scope. The closure must not outlive the call.
    ///
    /// # Errors
    /// Returns whatever the closure returns.
    pub fn read<T>(
        &self,
        f: impl FnOnce(&ReadScope<'_, '_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let guard = self.lock_read();
        guard.transaction(|txn| f(&ReadScope::new(txn)))
    }

    /// Runs a write scope: commits on `Ok`, rolls back on `Err`.
    ///
    /// # Errors
    /// Returns whatever the closure returns; the write is rolled back in
    /// that case.
    pub fn write<T>(
        &self,
        f: impl FnOnce(&mut WriteScope<'_, '_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut guard = self.lock_write();
        guard.transaction_mut(|txn| f(&mut WriteScope::new(txn)))
    }

    fn lock_read(&self) -> RwLockReadGuard<'_, DbAny> {
        match self.inner.read() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn lock_write(&self) -> RwLockWriteGuard<'_, DbAny> {
        match self.inner.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

fn ensure_indexes(database: &mut DbAny, indexes: &[String]) -> Result<(), Error> {
    let result = database.exec(QueryBuilder::select().indexes().query())?;
    let mut existing = std::collections::HashSet::new();
    if let Some(element) = result.elements.first() {
        for pair in &element.values {
            if let Ok(key) = pair.key.string() {
                existing.insert(key.clone());
            }
        }
    }
    for key in indexes {
        if existing.contains(key.as_str()) {
            continue;
        }
        database
            .exec_mut(QueryBuilder::insert().index(key).query())
            .map_err(|error| Error::Storage(format!("create index {key}: {error}")))?;
    }
    Ok(())
}
