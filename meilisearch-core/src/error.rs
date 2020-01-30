use crate::serde::{DeserializerError, SerializerError};
use serde_json::Error as SerdeJsonError;
use std::{error, fmt, io};

pub type MResult<T> = Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    IndexAlreadyExists,
    MissingIdentifier,
    SchemaMissing,
    WordIndexMissing,
    MissingDocumentId,
    MaxFieldsLimitExceeded,
    Schema(meilisearch_schema::Error),
    Zlmdb(heed::Error),
    Fst(fst::Error),
    SerdeJson(SerdeJsonError),
    Bincode(bincode::Error),
    Serializer(SerializerError),
    Deserializer(DeserializerError),
    UnsupportedOperation(UnsupportedOperation),
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::Io(error)
    }
}

impl From<meilisearch_schema::Error> for Error {
    fn from(error: meilisearch_schema::Error) -> Error {
        Error::Schema(error)
    }
}

impl From<heed::Error> for Error {
    fn from(error: heed::Error) -> Error {
        Error::Zlmdb(error)
    }
}

impl From<fst::Error> for Error {
    fn from(error: fst::Error) -> Error {
        Error::Fst(error)
    }
}

impl From<SerdeJsonError> for Error {
    fn from(error: SerdeJsonError) -> Error {
        Error::SerdeJson(error)
    }
}

impl From<bincode::Error> for Error {
    fn from(error: bincode::Error) -> Error {
        Error::Bincode(error)
    }
}

impl From<SerializerError> for Error {
    fn from(error: SerializerError) -> Error {
        Error::Serializer(error)
    }
}

impl From<DeserializerError> for Error {
    fn from(error: DeserializerError) -> Error {
        Error::Deserializer(error)
    }
}

impl From<UnsupportedOperation> for Error {
    fn from(op: UnsupportedOperation) -> Error {
        Error::UnsupportedOperation(op)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::Error::*;
        match self {
            Io(e) => write!(f, "{}", e),
            IndexAlreadyExists => write!(f, "index already exists"),
            MissingIdentifier => write!(f, "schema cannot be build without identifier"),
            SchemaMissing => write!(f, "this index does not have a schema"),
            WordIndexMissing => write!(f, "this index does not have a word index"),
            MissingDocumentId => write!(f, "document id is missing"),
            MaxFieldsLimitExceeded => write!(f, "maximum field in a document is exceeded"),
            Schema(e) => write!(f, "schemas error; {}", e),
            Zlmdb(e) => write!(f, "heed error; {}", e),
            Fst(e) => write!(f, "fst error; {}", e),
            SerdeJson(e) => write!(f, "serde json error; {}", e),
            Bincode(e) => write!(f, "bincode error; {}", e),
            Serializer(e) => write!(f, "serializer error; {}", e),
            Deserializer(e) => write!(f, "deserializer error; {}", e),
            UnsupportedOperation(op) => write!(f, "unsupported operation; {}", op),
        }
    }
}

impl error::Error for Error {}

#[derive(Debug)]
pub enum UnsupportedOperation {
    SchemaAlreadyExists,
    CannotUpdateSchemaIdentifier,
    CannotReorderSchemaAttribute,
    CanOnlyIntroduceNewSchemaAttributesAtEnd,
    CannotRemoveSchemaAttribute,
}

impl fmt::Display for UnsupportedOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::UnsupportedOperation::*;
        match self {
            SchemaAlreadyExists => write!(f, "Cannot update index which already have a schema"),
            CannotUpdateSchemaIdentifier => write!(f, "Cannot update the identifier of a schema"),
            CannotReorderSchemaAttribute => write!(f, "Cannot reorder the attributes of a schema"),
            CanOnlyIntroduceNewSchemaAttributesAtEnd => {
                write!(f, "Can only introduce new attributes at end of a schema")
            }
            CannotRemoveSchemaAttribute => write!(f, "Cannot remove attributes from a schema"),
        }
    }
}
