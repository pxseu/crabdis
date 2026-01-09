pub mod checksum;
pub mod length;
pub mod listpack;
pub mod lzf;
pub mod object;
pub mod string;
pub mod symbols;
pub mod ziplist;
pub mod zipmap;

use std::io::{Error as IoError, ErrorKind};

use self::checksum::{Crc64Reader, Crc64Writer};
use crate::prelude::*;

/// RDB (Redis Database) file format parser and serializer.
///
/// This parser implements the Redis RDB file format specification.
/// It supports parsing and writing strings and hashes (the types supported by
/// crabdis).
pub struct Rdb;

impl Rdb {
    /// Serializes a `HashMap` of Values to RDB format.
    ///
    /// # Arguments
    ///
    /// * `writer` - An async writer to write the RDB data to.
    /// * `db` - The database contents to serialize.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - A value type is not supported for serialization
    /// - Any I/O error occurs during writing
    ///
    /// # Panics
    ///
    /// May panic if system time is before `UNIX_EPOCH` (should not happen in
    /// practice).
    pub async fn to<W>(writer: W, db: &HashMap<Value, Value>) -> Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        // Wrap writer to compute CRC64 as we write
        let mut writer = Crc64Writer::new(writer);

        // Write magic string "REDIS" + version "0011"
        writer.write_all(self::symbols::MAGIC).await?;
        writer
            .write_all(format!("{:04}", self::symbols::VERSION_WITH_CHECKSUM).as_bytes())
            .await?;

        // Count entries with expiry
        let expires_count = db
            .values()
            .filter(|v| matches!(v, Value::Expire(_)))
            .count();

        // Write SELECT DB 0
        writer.write_u8(self::symbols::OPCODE_SELECTDB).await?;
        self::length::save(&mut writer, 0).await?;

        // Write RESIZEDB hint
        writer.write_u8(self::symbols::OPCODE_RESIZEDB).await?;
        self::length::save(&mut writer, db.len() as u64).await?;
        self::length::save(&mut writer, expires_count as u64).await?;

        // Write each key-value pair
        for (key, value) in db {
            // Get the key string
            let key_str = match key {
                Value::String(s) => s.as_ref(),
                _ => continue, // Skip non-string keys
            };

            // Unwrap Expire to get the actual value
            let (actual_value, expire_at) = match value {
                Value::Expire((inner, expires_at)) => (inner.as_ref(), Some(expires_at)),
                other => (other, None),
            };

            // Get type byte, skip unsupported types
            let Some(type_byte) = self::object::type_byte(actual_value) else {
                continue;
            };

            // Write expiry if present
            #[allow(clippy::cast_possible_truncation)]
            if let Some(expires_at) = expire_at {
                let now_instant = tokio::time::Instant::now();
                let now_system = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("System time before UNIX EPOCH")
                    .as_millis() as i64;

                let duration_until_expiry = expires_at.saturating_duration_since(now_instant);
                let expire_ms = now_system + duration_until_expiry.as_millis() as i64;

                writer.write_u8(self::symbols::OPCODE_EXPIRETIME_MS).await?;
                writer.write_all(&expire_ms.to_le_bytes()).await?;
            }

            // Write type byte
            writer.write_u8(type_byte).await?;

            // Write key
            self::string::save(&mut writer, key_str).await?;

            // Write value data
            self::object::save(&mut writer, actual_value).await?;
        }

        // Write EOF opcode
        writer.write_u8(self::symbols::OPCODE_EOF).await?;

        // Get checksum and write it
        let (mut inner, crc) = writer.into_inner();
        inner.write_all(&crc.to_le_bytes()).await?;

        Ok(())
    }

    /// Parses RDB data from an async reader and returns a `HashMap` of Values.
    ///
    /// # Arguments
    ///
    /// * `reader` - An async reader containing the RDB data to be parsed.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The RDB magic string is invalid
    /// - The RDB version is not supported
    /// - The data is corrupted or malformed
    /// - The checksum doesn't match (for RDB version 5+)
    /// - Any I/O error occurs during parsing
    ///
    /// # Panics
    ///
    /// May panic if system time is before `UNIX_EPOCH` (should not happen in
    /// practice).
    pub async fn from<R>(reader: R) -> Result<HashMap<Value, Value>>
    where
        R: AsyncRead + Unpin,
    {
        // Wrap reader to compute CRC64 as we read
        let mut reader = Crc64Reader::new(reader);

        // Verify magic string "REDIS"
        let mut magic = [0u8; 5];
        reader.read_exact(&mut magic).await?;

        if &magic != self::symbols::MAGIC {
            return Err(IoError::new(ErrorKind::InvalidData, "Invalid RDB magic string").into());
        }

        // Read and verify version
        let mut version_bytes = [0u8; 4];
        reader.read_exact(&mut version_bytes).await?;

        let version = std::str::from_utf8(&version_bytes)
            .map_err(|_| IoError::new(ErrorKind::InvalidData, "Invalid RDB version"))?
            .parse::<u32>()
            .map_err(|_| IoError::new(ErrorKind::InvalidData, "Invalid RDB version number"))?;

        if version > self::symbols::VERSION {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                format!(
                    "RDB version {version} not supported (max {})",
                    self::symbols::VERSION
                ),
            )
            .into());
        }

        let mut db = HashMap::new();
        let mut expire_time_ms: Option<i64> = None;

        loop {
            let opcode = match reader.read_u8().await {
                Ok(b) => b,
                Err(e) if e.kind() == ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            };

            match opcode {
                self::symbols::OPCODE_EOF => {
                    // EOF reached - verify checksum if version >= 5
                    if version >= self::symbols::VERSION_WITH_CHECKSUM {
                        let (mut inner, computed) = reader.into_inner();

                        // Read stored checksum (8 bytes, little-endian)
                        let mut checksum_bytes = [0u8; 8];
                        inner.read_exact(&mut checksum_bytes).await?;
                        let stored = u64::from_le_bytes(checksum_bytes);

                        // Verify checksum (0 means disabled)
                        if stored != 0 {
                            self::checksum::verify(computed, stored)?;
                        }
                    }
                    break;
                }
                self::symbols::OPCODE_SELECTDB => {
                    // Select DB - read and ignore for now (we use single DB)
                    let _db_number = self::length::load_plain(&mut reader).await?;
                }
                self::symbols::OPCODE_EXPIRETIME => {
                    // Expiry time in seconds
                    let mut time_bytes = [0u8; 4];
                    reader.read_exact(&mut time_bytes).await?;
                    let time_sec = i64::from(u32::from_le_bytes(time_bytes));
                    expire_time_ms = Some(time_sec * 1000);
                }
                self::symbols::OPCODE_EXPIRETIME_MS => {
                    // Expiry time in milliseconds
                    let mut time_bytes = [0u8; 8];
                    reader.read_exact(&mut time_bytes).await?;
                    expire_time_ms = Some(i64::from_le_bytes(time_bytes));
                }
                self::symbols::OPCODE_RESIZEDB => {
                    // Resize DB hint
                    let _db_size = self::length::load_plain(&mut reader).await?;
                    let _expires_size = self::length::load_plain(&mut reader).await?;
                }
                self::symbols::OPCODE_AUX => {
                    // Auxiliary fields - read and ignore
                    let _key = self::string::load(&mut reader).await?;
                    let _value = self::string::load(&mut reader).await?;
                }
                value_type => {
                    // This is a key-value pair
                    let key = self::string::load(&mut reader).await?;
                    let value = self::object::load(&mut reader, value_type).await?;

                    #[allow(clippy::cast_possible_truncation)]
                    let final_value = if let Some(expire_ms) = expire_time_ms.take() {
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .expect("System time before UNIX EPOCH")
                            .as_millis() as i64;

                        if expire_ms > now {
                            // Not yet expired, calculate tokio::time::Instant
                            let duration = std::time::Duration::from_millis(
                                u64::try_from(expire_ms - now).unwrap_or(0),
                            );
                            let expires_at = tokio::time::Instant::now() + duration;
                            Value::Expire((Arc::new(value), expires_at))
                        } else {
                            // Already expired, skip this key
                            continue;
                        }
                    } else {
                        value
                    };

                    db.insert(Value::String(key), final_value);
                }
            }
        }

        Ok(db)
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[tokio::test]
    async fn test_rdb_invalid_magic() {
        let data = b"WRONG0009\xfe\x00\xff";
        let result = Rdb::from(Cursor::new(data)).await;
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid RDB magic string")
        );
    }

    #[tokio::test]
    async fn test_rdb_empty_database() {
        // Minimal valid RDB: REDIS + version + EOF + checksum
        let mut data = Vec::new();
        data.extend_from_slice(b"REDIS0011");
        data.push(self::symbols::OPCODE_EOF);
        // Compute CRC64 of everything before checksum
        let crc = self::checksum::crc64(&data);
        data.extend_from_slice(&crc.to_le_bytes());

        let result = Rdb::from(Cursor::new(&data)).await;
        assert!(result.is_ok());
        let db = result.unwrap();
        assert_eq!(db.len(), 0);
    }

    #[tokio::test]
    async fn test_rdb_with_string_value() {
        let mut data = Vec::new();
        // Header
        data.extend_from_slice(b"REDIS0011");
        // Select DB 0
        data.push(self::symbols::OPCODE_SELECTDB);
        data.push(0);
        // String type
        data.push(self::symbols::TYPE_STRING);
        // Key: "key" (3 bytes, 6-bit length encoding)
        data.push(3);
        data.extend_from_slice(b"key");
        // Value: "value" (5 bytes)
        data.push(5);
        data.extend_from_slice(b"value");
        // EOF
        data.push(self::symbols::OPCODE_EOF);
        // Checksum
        let crc = self::checksum::crc64(&data);
        data.extend_from_slice(&crc.to_le_bytes());

        let result = Rdb::from(Cursor::new(&data)).await;
        assert!(result.is_ok());
        let db = result.unwrap();
        assert_eq!(db.len(), 1);

        let key = Value::String("key".into());
        assert!(db.contains_key(&key));

        if let Some(Value::String(s)) = db.get(&key) {
            assert_eq!(s.as_ref(), "value");
        } else {
            panic!("Expected string value");
        }
    }

    #[tokio::test]
    async fn test_rdb_version_check() {
        let data = b"REDIS9999\xfe\x00\xff\x00\x00\x00\x00\x00\x00\x00\x00";
        let result = Rdb::from(Cursor::new(data)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not supported"));
    }

    #[tokio::test]
    async fn test_rdb_with_aux_field() {
        let mut data = Vec::new();
        data.extend_from_slice(b"REDIS0011");
        // AUX field
        data.push(self::symbols::OPCODE_AUX);
        // Key: "ver"
        data.push(3);
        data.extend_from_slice(b"ver");
        // Value: "7.0"
        data.push(3);
        data.extend_from_slice(b"7.0");
        // EOF
        data.push(self::symbols::OPCODE_EOF);
        // Checksum
        let crc = self::checksum::crc64(&data);
        data.extend_from_slice(&crc.to_le_bytes());

        let result = Rdb::from(Cursor::new(&data)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_rdb_checksum_mismatch() {
        let mut data = Vec::new();
        data.extend_from_slice(b"REDIS0011");
        data.push(self::symbols::OPCODE_EOF);
        // Wrong checksum
        data.extend_from_slice(&[0xFF; 8]);

        let result = Rdb::from(Cursor::new(&data)).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("checksum"));
    }

    #[tokio::test]
    async fn test_rdb_checksum_disabled() {
        let mut data = Vec::new();
        data.extend_from_slice(b"REDIS0011");
        data.push(self::symbols::OPCODE_EOF);
        // Checksum of 0 means disabled
        data.extend_from_slice(&[0u8; 8]);

        let result = Rdb::from(Cursor::new(&data)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_actual_dump() {
        // Real Redis dump with: hello=world, user={name: pxseu}
        const DUMP: &[u8] = &[
            0x52, 0x45, 0x44, 0x49, 0x53, 0x30, 0x30, 0x31, 0x32, 0xFA, 0x09, 0x72, 0x65, 0x64,
            0x69, 0x73, 0x2D, 0x76, 0x65, 0x72, 0x05, 0x38, 0x2E, 0x34, 0x2E, 0x30, 0xFA, 0x0A,
            0x72, 0x65, 0x64, 0x69, 0x73, 0x2D, 0x62, 0x69, 0x74, 0x73, 0xC0, 0x40, 0xFA, 0x05,
            0x63, 0x74, 0x69, 0x6D, 0x65, 0xC2, 0x59, 0xF1, 0x57, 0x69, 0xFA, 0x08, 0x75, 0x73,
            0x65, 0x64, 0x2D, 0x6D, 0x65, 0x6D, 0xC2, 0x80, 0x65, 0x0E, 0x00, 0xFA, 0x08, 0x61,
            0x6F, 0x66, 0x2D, 0x62, 0x61, 0x73, 0x65, 0xC0, 0x00, 0xFE, 0x00, 0xFB, 0x02, 0x00,
            0x10, 0x04, 0x75, 0x73, 0x65, 0x72, 0x14, 0x14, 0x00, 0x00, 0x00, 0x02, 0x00, 0x84,
            0x6E, 0x61, 0x6D, 0x65, 0x05, 0x85, 0x70, 0x78, 0x73, 0x65, 0x75, 0x06, 0xFF, 0x00,
            0x05, 0x68, 0x65, 0x6C, 0x6C, 0x6F, 0x05, 0x77, 0x6F, 0x72, 0x6C, 0x64, 0xFF, 0x26,
            0xFA, 0x18, 0x68, 0x11, 0x6A, 0x84, 0x50,
        ];

        let result = Rdb::from(Cursor::new(DUMP)).await;

        if let Err(e) = &result {
            eprintln!("Error parsing RDB: {e}");
        }

        assert!(result.is_ok(), "Failed to parse RDB: {:?}", result.err());
        let db = result.unwrap();

        assert_eq!(db.len(), 2);

        // Check string key
        let key1 = Value::String("hello".into());
        assert!(db.contains_key(&key1));
        if let Some(Value::String(s)) = db.get(&key1) {
            assert_eq!(s.as_ref(), "world");
        } else {
            panic!("Expected string value for 'hello'");
        }

        // Check hash key
        let key2 = Value::String("user".into());
        assert!(db.contains_key(&key2));
        if let Some(Value::Map(map)) = db.get(&key2) {
            assert_eq!(map.len(), 1);
            let field_key = Value::String("name".into());
            assert!(map.contains_key(&field_key));
            if let Some(Value::String(name)) = map.get(&field_key) {
                assert_eq!(name.as_ref(), "pxseu");
            } else {
                panic!("Expected string value for field 'name'");
            }
        } else {
            panic!("Expected hash value for 'user'");
        }
    }

    #[tokio::test]
    async fn test_round_trip_empty() {
        let original: HashMap<Value, Value> = HashMap::new();

        // Serialize
        let mut buffer = Vec::new();
        Rdb::to(&mut buffer, &original).await.unwrap();

        // Deserialize
        let parsed = Rdb::from(Cursor::new(&buffer)).await.unwrap();

        assert_eq!(parsed.len(), 0);
    }

    #[tokio::test]
    async fn test_round_trip_string() {
        let mut original = HashMap::new();
        original.insert(Value::String("hello".into()), Value::String("world".into()));

        // Serialize
        let mut buffer = Vec::new();
        Rdb::to(&mut buffer, &original).await.unwrap();

        // Deserialize
        let parsed = Rdb::from(Cursor::new(&buffer)).await.unwrap();

        assert_eq!(parsed.len(), 1);
        let key = Value::String("hello".into());
        assert!(parsed.contains_key(&key));
        assert_eq!(parsed.get(&key), Some(&Value::String("world".into())));
    }

    #[tokio::test]
    async fn test_round_trip_hash() {
        let mut hash = HashMap::new();
        hash.insert(
            Value::String("field1".into()),
            Value::String("value1".into()),
        );
        hash.insert(
            Value::String("field2".into()),
            Value::String("value2".into()),
        );

        let mut original = HashMap::new();
        original.insert(Value::String("myhash".into()), Value::Map(hash));

        // Serialize
        let mut buffer = Vec::new();
        Rdb::to(&mut buffer, &original).await.unwrap();

        // Deserialize
        let parsed = Rdb::from(Cursor::new(&buffer)).await.unwrap();

        assert_eq!(parsed.len(), 1);
        let key = Value::String("myhash".into());
        assert!(parsed.contains_key(&key));

        if let Some(Value::Map(map)) = parsed.get(&key) {
            assert_eq!(map.len(), 2);
            assert_eq!(
                map.get(&Value::String("field1".into())),
                Some(&Value::String("value1".into()))
            );
            assert_eq!(
                map.get(&Value::String("field2".into())),
                Some(&Value::String("value2".into()))
            );
        } else {
            panic!("Expected hash value");
        }
    }

    #[tokio::test]
    async fn test_round_trip_multiple() {
        let mut original = HashMap::new();
        original.insert(Value::String("key1".into()), Value::String("value1".into()));
        original.insert(Value::String("key2".into()), Value::String("value2".into()));
        original.insert(Value::String("counter".into()), Value::Integer(42));

        // Serialize
        let mut buffer = Vec::new();
        Rdb::to(&mut buffer, &original).await.unwrap();

        // Deserialize
        let parsed = Rdb::from(Cursor::new(&buffer)).await.unwrap();

        assert_eq!(parsed.len(), 3);
        assert_eq!(
            parsed.get(&Value::String("key1".into())),
            Some(&Value::String("value1".into()))
        );
        assert_eq!(
            parsed.get(&Value::String("key2".into())),
            Some(&Value::String("value2".into()))
        );
        // Integer is stored as string in RDB
        assert_eq!(
            parsed.get(&Value::String("counter".into())),
            Some(&Value::String("42".into()))
        );
    }
}
