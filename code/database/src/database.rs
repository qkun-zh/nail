use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use agdb::{DbAny, QueryBuilder};

use crate::error::Error;
use crate::scope::ReadScope;
use crate::write::WriteScope;

#[derive(Clone)]
pub struct Database {
    inner: Arc<RwLock<DbAny>>,
}

impl Database {
    pub fn open_memory(name: &str, indexes: &[String]) -> Result<Self, Error> {
        let database = DbAny::new_memory(name)?;
        Self::start(database, indexes)
    }

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

    pub fn read<T>(
        &self,
        f: impl FnOnce(&ReadScope<'_, '_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let guard = self.lock_read()?;
        guard.transaction(|txn| {
            catch_unwind(AssertUnwindSafe(|| f(&ReadScope::new(txn))))
                .unwrap_or_else(|payload| Err(panic_error(&payload)))
        })
    }

    pub fn write<T>(
        &self,
        f: impl FnOnce(&mut WriteScope<'_, '_>) -> Result<T, Error>,
    ) -> Result<T, Error> {
        let mut guard = self.lock_write()?;
        guard.transaction_mut(|txn| {
            catch_unwind(AssertUnwindSafe(|| f(&mut WriteScope::new(txn))))
                .unwrap_or_else(|payload| Err(panic_error(&payload)))
        })
    }

    fn lock_read(&self) -> Result<RwLockReadGuard<'_, DbAny>, Error> {
        self.inner
            .read()
            .map_err(|_| Error::Storage("database lock poisoned by an escaped panic".into()))
    }

    fn lock_write(&self) -> Result<RwLockWriteGuard<'_, DbAny>, Error> {
        self.inner
            .write()
            .map_err(|_| Error::Storage("database lock poisoned by an escaped panic".into()))
    }
}

fn panic_error(payload: &Box<dyn Any + Send>) -> Error {
    let message = if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_string()
    };
    Error::Panic(format!(
        "database access panicked and was rolled back: {message}"
    ))
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
