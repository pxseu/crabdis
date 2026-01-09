use std::io::{Error as IoError, ErrorKind};

use crate::prelude::*;

/// Returns the RDB type byte for a value.
///
/// Note: `Expire` values should be unwrapped first by the caller.
#[must_use]
pub const fn type_byte(value: &Value) -> Option<u8> {
    match value {
        Value::String(_) | Value::Integer(_) => Some(super::symbols::TYPE_STRING),
        Value::Map(_) => Some(super::symbols::TYPE_HASH),
        _ => None,
    }
}

/// Saves a value's data to the async writer (without type byte).
///
/// The caller should write the type byte first using `type_byte()`.
///
/// # Errors
///
/// Returns an error if the value type is unsupported or I/O fails.
pub async fn save<W>(writer: &mut W, value: &Value) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    match value {
        Value::String(s) => {
            super::string::save(writer, s).await?;
        }
        Value::Integer(i) => {
            super::string::save(writer, &i.to_string()).await?;
        }
        Value::Map(map) => {
            // Write as plain hash (TYPE_HASH)
            super::length::save(writer, map.len() as u64).await?;
            for (k, v) in map {
                let key_str = value_to_string(k);
                let val_str = value_to_string(v);
                super::string::save(writer, &key_str).await?;
                super::string::save(writer, &val_str).await?;
            }
        }
        _ => {
            return Err(IoError::new(
                ErrorKind::Unsupported,
                format!("Cannot save value type to RDB: {value:?}"),
            )
            .into());
        }
    }
    Ok(())
}

/// Converts a Value to its string representation for RDB storage.
fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(s) => s.to_string(),
        Value::Integer(i) => i.to_string(),
        _ => String::new(),
    }
}

/// Loads an RDB object based on its type.
///
/// # Errors
///
/// Returns an error if the object type is invalid or I/O fails.
pub async fn load<R>(reader: &mut R, obj_type: u8) -> Result<Value>
where
    R: AsyncRead + Unpin,
{
    match obj_type {
        // String
        super::symbols::TYPE_STRING => {
            let s = super::string::load(reader).await?;
            Ok(Value::String(s))
        }

        // Hash (plain encoding)
        super::symbols::TYPE_HASH => {
            let len = super::length::load_plain(reader).await?;
            #[allow(clippy::cast_possible_truncation)]
            let mut map = HashMap::with_capacity(len as usize);

            for _ in 0..len {
                let key = super::string::load(reader).await?;
                let value = super::string::load(reader).await?;
                map.insert(Value::String(key), Value::String(value));
            }

            Ok(Value::Map(map))
        }

        // Hash (zipmap - very old format)
        super::symbols::TYPE_HASH_ZIPMAP => {
            let data = super::string::load_raw(reader).await?;
            let map = super::zipmap::parse(&data)?;
            Ok(Value::Map(map))
        }

        // Hash (ziplist - legacy format)
        super::symbols::TYPE_HASH_ZIPLIST => {
            let data = super::string::load_raw(reader).await?;
            let map = super::ziplist::parse_hash(&data)?;
            Ok(Value::Map(map))
        }

        // Hash (listpack - modern format)
        super::symbols::TYPE_HASH_LISTPACK => {
            let data = super::string::load_raw(reader).await?;
            let map = super::listpack::parse_hash(&data)?;
            Ok(Value::Map(map))
        }

        // Unsupported types - skip the data and return error
        super::symbols::TYPE_LIST
        | super::symbols::TYPE_LIST_ZIPLIST
        | super::symbols::TYPE_LIST_QUICKLIST
        | super::symbols::TYPE_LIST_QUICKLIST_2 => {
            skip_list(reader, obj_type).await?;
            Err(IoError::new(
                ErrorKind::Unsupported,
                "List type not supported (no list commands in crabdis)",
            )
            .into())
        }

        super::symbols::TYPE_SET
        | super::symbols::TYPE_SET_INTSET
        | super::symbols::TYPE_SET_LISTPACK => {
            skip_set(reader, obj_type).await?;
            Err(IoError::new(
                ErrorKind::Unsupported,
                "Set type not supported (no set commands in crabdis)",
            )
            .into())
        }

        super::symbols::TYPE_ZSET
        | super::symbols::TYPE_ZSET_2
        | super::symbols::TYPE_ZSET_ZIPLIST
        | super::symbols::TYPE_ZSET_LISTPACK => {
            skip_zset(reader, obj_type).await?;
            Err(IoError::new(
                ErrorKind::Unsupported,
                "Sorted set type not supported (no sorted set commands in crabdis)",
            )
            .into())
        }

        super::symbols::TYPE_MODULE | super::symbols::TYPE_MODULE_2 => {
            Err(IoError::new(ErrorKind::Unsupported, "Module types not supported").into())
        }

        super::symbols::TYPE_STREAM_LISTPACKS
        | super::symbols::TYPE_STREAM_LISTPACKS_2
        | super::symbols::TYPE_STREAM_LISTPACKS_3 => {
            Err(IoError::new(ErrorKind::Unsupported, "Stream types not supported").into())
        }

        _ => Err(IoError::new(
            ErrorKind::InvalidData,
            format!("Unknown RDB object type: {obj_type}"),
        )
        .into()),
    }
}

/// Skips a list object in the RDB stream.
async fn skip_list<R>(reader: &mut R, obj_type: u8) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    match obj_type {
        super::symbols::TYPE_LIST => {
            let len = super::length::load_plain(reader).await?;
            for _ in 0..len {
                let _ = super::string::load(reader).await?;
            }
        }
        super::symbols::TYPE_LIST_ZIPLIST => {
            let _ = super::string::load_raw(reader).await?;
        }
        super::symbols::TYPE_LIST_QUICKLIST | super::symbols::TYPE_LIST_QUICKLIST_2 => {
            let len = super::length::load_plain(reader).await?;
            for _ in 0..len {
                let _ = super::string::load_raw(reader).await?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Skips a set object in the RDB stream.
async fn skip_set<R>(reader: &mut R, obj_type: u8) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    match obj_type {
        super::symbols::TYPE_SET => {
            let len = super::length::load_plain(reader).await?;
            for _ in 0..len {
                let _ = super::string::load(reader).await?;
            }
        }
        super::symbols::TYPE_SET_INTSET | super::symbols::TYPE_SET_LISTPACK => {
            let _ = super::string::load_raw(reader).await?;
        }
        _ => {}
    }
    Ok(())
}

/// Skips a sorted set object in the RDB stream.
async fn skip_zset<R>(reader: &mut R, obj_type: u8) -> Result<()>
where
    R: AsyncRead + Unpin,
{
    match obj_type {
        super::symbols::TYPE_ZSET | super::symbols::TYPE_ZSET_2 => {
            let len = super::length::load_plain(reader).await?;
            for _ in 0..len {
                let _ = super::string::load(reader).await?;
                if obj_type == super::symbols::TYPE_ZSET_2 {
                    let mut bytes = [0u8; 8];
                    reader.read_exact(&mut bytes).await?;
                } else {
                    let _ = super::string::load(reader).await?;
                }
            }
        }
        super::symbols::TYPE_ZSET_ZIPLIST | super::symbols::TYPE_ZSET_LISTPACK => {
            let _ = super::string::load_raw(reader).await?;
        }
        _ => {}
    }
    Ok(())
}
