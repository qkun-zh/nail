use agdb::DbKeyValue;
use agdb::DbValue;

use crate::error::Error;
use crate::kinds::NodeKind;

pub trait Row: Sized {
    const KIND: NodeKind;

    fn business_id(&self) -> &str;

    fn to_row(&self) -> Vec<(String, DbValue)>;

    fn from_lookup(lookup: &dyn ValueLookup) -> Result<Self, Error>;
}

pub trait ValueLookup {
    fn get(&self, key: &str) -> Option<DbValue>;

    fn required_text(&self, key: &str) -> Result<String, Error> {
        match self.get(key) {
            Some(value) => value
                .string()
                .cloned()
                .map_err(|_| Error::Invalid(format!("key {key} is not text"))),
            None => Err(Error::Invalid(format!("missing key {key}"))),
        }
    }

    fn optional_text(&self, key: &str) -> Result<Option<String>, Error> {
        match self.get(key) {
            Some(value) => value
                .string()
                .cloned()
                .map(Some)
                .map_err(|_| Error::Invalid(format!("key {key} is not text"))),
            None => Ok(None),
        }
    }

    fn required_int(&self, key: &str) -> Result<i64, Error> {
        match self.get(key) {
            Some(value) => value
                .to_i64()
                .map_err(|_| Error::Invalid(format!("key {key} is not an int"))),
            None => Err(Error::Invalid(format!("missing key {key}"))),
        }
    }
}

pub(crate) struct ElementLookup<'a> {
    values: &'a [DbKeyValue],
}

impl<'a> ElementLookup<'a> {
    pub(crate) fn new(values: &'a [DbKeyValue]) -> Self {
        Self { values }
    }
}

impl ValueLookup for ElementLookup<'_> {
    fn get(&self, key: &str) -> Option<DbValue> {
        self.values
            .iter()
            .find(|pair| pair.key.string().is_ok_and(|name| name == key))
            .map(|pair| pair.value.clone())
    }
}
