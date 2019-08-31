
#[repr(C, packed)]
pub struct AllMemory {
    pub zone_id: ZoneID,
    pub target: Target,
    pub chat_log: ChatLog,
    pub mob_array: MobArray,
    pub party_list: PartyList,
    pub server_time: ServerTime,
    pub player: Player,
}

#[repr(C, packed)]
pub struct ZoneID {
    signature: [u8; 18],
    offset: [u8; 4],
    pub data: u32,
}

#[repr(C, packed)]
pub struct Target {
    signature: [u8; 20],
    offset: [u8; 4],
    pub target_data: TargetData
}

#[repr(C, packed)]
pub struct TargetData {
    _padding1: [u8; 192],
    pub target: u64,
    pub hovertarget: u64,
    _padding2: [u8; 72],
    pub focustarget: u64,
}

#[repr(C, packed)]
pub struct ChatLog {
    signature: [u8; 26],
    offset: [u8; 4],
    pub chat_log_data: ChatLogData
}

#[repr(C, packed)]
pub struct ChatLogData {
    _header_ptr_map_1: u64,
    _header_ptr_map_2: u64,
    _padding1: [u8; 80],
    _header_ptr_map_3: u64,
    _padding2: [u8; 36],
    _header_ptr_map_4: u64,
    _padding3: [u8; 352],
    _header_ptr_map_5: u64,
    _padding4: [u8; 32],
    pub header_data: ChatLogHeader,
}

#[repr(C, packed)]
pub struct ChatLogHeader {
    _padding: [u8; 952],
    pub length_array_start: u64,
    pub length_array_end: u64,
    _padding2: [u8; 8],
    pub message_array_start: u64,
    pub message_array_end: u64,
    _padding3: [u8; 32],
}

#[repr(C, packed)]
pub struct MobArray {
    signature: [u8; 20],
    offset: [u8; 4],
    // Array of 421 pointers to heap-allocated mobs
    pub data: [u64; 421],
}

#[repr(C, packed)]
pub struct PartyList {
    signature: [u8; 21],
    offset: [u8; 4],
    data: [u8; 25600],
}

#[repr(C, packed)]
pub struct ServerTime {
    signature: [u8; 9],
    offset: [u8; 4],
    pub ptr: u64,
}

#[repr(C, packed)]
pub struct ServerTimePart1 {
    _padding: [u8; 72],
    pub ptr2: u64
}

#[repr(C, packed)]
pub struct ServerTimePart2 {
    _padding: [u8; 8],
    pub ptr3: u64
}

#[repr(C, packed)]
pub struct ServerTimePart3 {
    _padding: [u8; 2116],
    pub data: u64
}

#[repr(C, packed)]
pub struct Player {
    signature: [u8; 14],
    offset: [u8; 4],
}


pub static SERVER_3: ServerTimePart3 = ServerTimePart3::create();
pub static mut SERVER_2: ServerTimePart2 = ServerTimePart2::create();
pub static mut SERVER_1: ServerTimePart1 = ServerTimePart1::create();

impl PartyList {
    pub const fn create() -> PartyList {
        PartyList {
            signature: [0x48,0x8D,0x7C,0x24,0x20,0x66,0x66,0x0F,0x1F,0x84,0x00,0x00,0x00,0x00,0x00,0x48,0x8B,0x17,0x48,0x8D,0x0D],
            offset: [0; 4],
            data: [0; 25600]
        }
    }
}

impl ServerTimePart1 {
    pub const fn create() -> ServerTimePart1 {
        ServerTimePart1 {
            _padding: [0; 72],
            ptr2: 0
        }
    }
}

impl ServerTimePart2 {
    pub const fn create() -> ServerTimePart2 {
        ServerTimePart2 {
            _padding: [0; 8],
            ptr3: 0
        }
    }
}
impl ServerTimePart3 {
    pub const fn create() -> ServerTimePart3 {
        ServerTimePart3 {
            _padding: [0; 2116],
            data: 0
        }
    }
}

impl ServerTime {
    pub const fn create() -> ServerTime {
        ServerTime {
            signature: [0x0f,0xb7,0xc0,0x89,0x47,0x10,0x48,0x8b,0x0d,],
            offset: [0; 4],
            ptr: 0
        }
    }
}

impl Player {
    pub const fn create() -> Player {
        Player {
            signature: [0x83,0xf9,0xff,0x74,0x12,0x44,0x8b,0x04,0x8e,0x8b,0xd3,0x48,0x8d,0x0d],
            offset: [0; 4]
        }
    }
}

impl AllMemory {
    pub const fn create() -> AllMemory {
        AllMemory {
            zone_id: ZoneID::create(),
            target: Target::create(),
            chat_log: ChatLog::create(),
            mob_array: MobArray::create(),
            party_list: PartyList::create(),
            server_time: ServerTime::create(),
            player: Player::create(),
        }
    }
}

impl ZoneID {
    pub const fn create() -> ZoneID {
        ZoneID {
            signature: [0xf3,0x0f,0x10,0x8d,0x08,0x04,0x00,0x00,0x4c,0x8d,0x85,0x58,0x06,0x00,0x00,0x0f,0xb7,0x05],
            offset: [0,0,0,0],
            data: 0
        }
    }
}

impl Target {
    pub const fn create() -> Target {
        Target {
            signature: [0x41,0xbc,0x00,0x00,0x00,0xe0,0x41,0xbd,0x01,0x00,0x00,0x00,0x49,0x3b,0xc4,0x75,0x55,0x48,0x8d,0x0d],
            offset: [0,0,0,0],
            target_data: TargetData::create()
        }
    }
}

impl TargetData {
    pub const fn create() -> TargetData {
        TargetData {
            _padding1: [0; 192],
            target: 0,
            hovertarget: 0,
            _padding2: [0; 72],
            focustarget: 0
        }
    }
}

impl ChatLog {
    pub const fn create() -> ChatLog {
        ChatLog {
            signature: [0xe8,0x00,0x00,0x00,0x00,0x85,0xc0,0x74,0x0e,0x48,0x8b,0x0d,0x00,0x00,0x00,0x00,0x33,0xD2,0xE8,0x00,0x00,0x00,0x00,0x48,0x8b,0x0d],
            offset: [0,0,0,0],
            chat_log_data: ChatLogData::create()
        }
    }
}

impl ChatLogData {
    pub const fn create() -> ChatLogData {
        ChatLogData {
            _header_ptr_map_1: 0x08,
            _header_ptr_map_2: 0x00,
            _padding1: [0; 80],
            _header_ptr_map_3: 0x00,
            _padding2: [0; 36],
            _header_ptr_map_4: 0x00,
            _padding3: [0; 352],
            _header_ptr_map_5: 0x00,
            _padding4: [0; 32],
            header_data: ChatLogHeader::create()
        }
    }
}

impl ChatLogHeader {
    pub const fn create() -> ChatLogHeader {
        ChatLogHeader {
            _padding: [0; 952],
            length_array_start: 0,
            length_array_end: 0,
            _padding2: [0; 8],
            message_array_start: 0,
            message_array_end: 0,
            _padding3: [0; 32]
        }
    }
}

impl MobArray {
    pub const fn create() -> MobArray {
        MobArray {
            signature: [0x48,0x8b,0x42,0x08,0x48,0xc1,0xe8,0x03,0x3d,0xa7,0x01,0x00,0x00,0x77,0x24,0x8b,0xc0,0x48,0x8d,0x0d],
            offset: [0; 4],
            data: [0; 421]
        }
    }
}