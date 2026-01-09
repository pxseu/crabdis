// RDB magic string
pub const MAGIC: &[u8; 5] = b"REDIS";

// Maximum supported RDB version
pub const VERSION: u32 = 12;

// Minimum RDB version that includes checksum
pub const VERSION_WITH_CHECKSUM: u32 = 5;

// RDB opcodes
pub const OPCODE_AUX: u8 = 0xFA;
pub const OPCODE_RESIZEDB: u8 = 0xFB;
pub const OPCODE_EXPIRETIME_MS: u8 = 0xFC;
pub const OPCODE_EXPIRETIME: u8 = 0xFD;
pub const OPCODE_SELECTDB: u8 = 0xFE;
pub const OPCODE_EOF: u8 = 0xFF;

// RDB value types
pub const TYPE_STRING: u8 = 0;
pub const TYPE_LIST: u8 = 1;
pub const TYPE_SET: u8 = 2;
pub const TYPE_ZSET: u8 = 3;
pub const TYPE_HASH: u8 = 4;
pub const TYPE_ZSET_2: u8 = 5;
pub const TYPE_MODULE: u8 = 6;
pub const TYPE_MODULE_2: u8 = 7;
// 8 is reserved
pub const TYPE_HASH_ZIPMAP: u8 = 9;
pub const TYPE_LIST_ZIPLIST: u8 = 10;
pub const TYPE_SET_INTSET: u8 = 11;
pub const TYPE_ZSET_ZIPLIST: u8 = 12;
pub const TYPE_HASH_ZIPLIST: u8 = 13;
pub const TYPE_LIST_QUICKLIST: u8 = 14;
pub const TYPE_STREAM_LISTPACKS: u8 = 15;
pub const TYPE_HASH_LISTPACK: u8 = 16;
pub const TYPE_ZSET_LISTPACK: u8 = 17;
pub const TYPE_LIST_QUICKLIST_2: u8 = 18;
pub const TYPE_STREAM_LISTPACKS_2: u8 = 19;
pub const TYPE_SET_LISTPACK: u8 = 20;
pub const TYPE_STREAM_LISTPACKS_3: u8 = 21;

// Length encoding types (top 2 bits of first byte)
pub const LEN_6BIT: u8 = 0;
pub const LEN_14BIT: u8 = 1;
pub const LEN_32BIT: u8 = 2;
pub const LEN_ENCVAL: u8 = 3;

// String encoding types (when LEN_ENCVAL is set)
pub const ENC_INT8: u8 = 0;
pub const ENC_INT16: u8 = 1;
pub const ENC_INT32: u8 = 2;
pub const ENC_LZF: u8 = 3;

// Ziplist encoding types
pub const ZIPLIST_END: u8 = 0xFF;
pub const ZIPLIST_BIGLEN: u8 = 0xFE;

// Ziplist entry encoding (top 2 bits)
pub const ZIPLIST_STR_6BIT: u8 = 0x00; // 00xxxxxx - 6-bit length string
pub const ZIPLIST_STR_14BIT: u8 = 0x40; // 01xxxxxx - 14-bit length string
pub const ZIPLIST_STR_32BIT: u8 = 0x80; // 10000000 - 32-bit length string

// Ziplist integer encodings (when top 2 bits are 11)
pub const ZIPLIST_INT_16: u8 = 0xC0; // 11000000 - int16
pub const ZIPLIST_INT_32: u8 = 0xD0; // 11010000 - int32
pub const ZIPLIST_INT_64: u8 = 0xE0; // 11100000 - int64
pub const ZIPLIST_INT_24: u8 = 0xF0; // 11110000 - 24-bit signed int
pub const ZIPLIST_INT_8: u8 = 0xFE; // 11111110 - 8-bit signed int
// 1111xxxx (0xF1-0xFD) - 4-bit unsigned int (0-12, stored as 1-13)

// Listpack encoding types
pub const LISTPACK_END: u8 = 0xFF;

// Zipmap constants
pub const ZIPMAP_END: u8 = 0xFF;
pub const ZIPMAP_BIGLEN: u8 = 0xFE;
