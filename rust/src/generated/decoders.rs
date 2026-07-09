use std::io::{Cursor, Read};

use byteorder::{BigEndian, LittleEndian, ReadBytesExt};

use crate::{TdfDecodeError, TdfDecodeResult};

#[derive(Clone, Debug, PartialEq)]
pub enum TdfValue {
    Int(i64),
    UInt(u64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    List(Vec<TdfValue>),
    Struct(Vec<TdfField>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct TdfField {
    pub name: &'static str,
    pub value: TdfValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TdfDecodedPayload {
    pub id: u16,
    pub name: &'static str,
    pub type_name: &'static str,
    pub fields: Vec<TdfField>,
}

pub fn tdf_known_name(tdf_id: u16) -> Option<&'static str> {
    match tdf_id {
        1 => Some("ANNOUNCE"),
        2 => Some("BATTERY_STATE"),
        3 => Some("AMBIENT_TEMP_PRES_HUM"),
        4 => Some("AMBIENT_TEMPERATURE"),
        5 => Some("TIME_SYNC"),
        6 => Some("REBOOT_INFO"),
        7 => Some("ANNOUNCE_V2"),
        8 => Some("SOC_TEMPERATURE"),
        10 => Some("ACC_2G"),
        11 => Some("ACC_4G"),
        12 => Some("ACC_8G"),
        13 => Some("ACC_16G"),
        14 => Some("GYR_125DPS"),
        15 => Some("GYR_250DPS"),
        16 => Some("GYR_500DPS"),
        17 => Some("GYR_1000DPS"),
        18 => Some("GYR_2000DPS"),
        19 => Some("GCS_WGS84_LLHA"),
        20 => Some("UBX_NAV_PVT"),
        21 => Some("LTE_CONN_STATUS"),
        22 => Some("GLOBALSTAR_PKT"),
        23 => Some("ACC_MAGNITUDE_STD_DEV"),
        24 => Some("ACTIVITY_METRIC"),
        25 => Some("ALGORITHM_OUTPUT"),
        26 => Some("RUNTIME_ERROR"),
        27 => Some("CHARGER_EN_CONTROL"),
        28 => Some("GNSS_FIX_INFO"),
        29 => Some("BLUETOOTH_CONNECTION"),
        30 => Some("BLUETOOTH_RSSI"),
        31 => Some("BLUETOOTH_DATA_THROUGHPUT"),
        32 => Some("ALGORITHM_CLASS_HISTOGRAM"),
        33 => Some("ALGORITHM_CLASS_TIME_SERIES"),
        34 => Some("LTE_TAC_CELLS"),
        35 => Some("WIFI_AP_INFO"),
        36 => Some("DEVICE_TILT"),
        37 => Some("NRF9X_GNSS_PVT"),
        38 => Some("BATTERY_CHARGE_ACCUMULATED"),
        39 => Some("INFUSE_BLUETOOTH_RSSI"),
        40 => Some("ADC_RAW_8"),
        41 => Some("ADC_RAW_16"),
        42 => Some("ADC_RAW_32"),
        43 => Some("ANNOTATION"),
        44 => Some("LORA_RX"),
        45 => Some("LORA_TX"),
        46 => Some("IDX_ARRAY_FREQ"),
        47 => Some("IDX_ARRAY_PERIOD"),
        48 => Some("WIFI_CONNECTED"),
        49 => Some("WIFI_CONNECTION_FAILED"),
        50 => Some("WIFI_DISCONNECTED"),
        51 => Some("NETWORK_SCAN_COUNT"),
        52 => Some("EXCEPTION_STACK_FRAME"),
        53 => Some("BATTERY_VOLTAGE"),
        54 => Some("BATTERY_SOC"),
        55 => Some("STATE_EVENT_SET"),
        56 => Some("STATE_EVENT_CLEARED"),
        57 => Some("STATE_DURATION"),
        58 => Some("PCM_16BIT_CHAN_LEFT"),
        59 => Some("PCM_16BIT_CHAN_RIGHT"),
        60 => Some("PCM_16BIT_CHAN_DUAL"),
        61 => Some("KVS_VALUE_CHANGED"),
        62 => Some("AMBIENT_PRESSURE"),
        _ => None,
    }
}

fn tdf_field_read_string_to_string(
    cursor: &mut Cursor<&[u8]>,
    cursor_start: u64,
    num: u8,
    size: u8,
) -> TdfDecodeResult<String> {
    let buf = tdf_field_read_string(cursor, cursor_start, num, size)?;

    match String::from_utf8(buf) {
        Ok(val) => Ok(val.trim_matches(char::from(0)).to_string()),
        Err(..) => Ok(String::new()),
    }
}

pub fn tdf_decode_payload(
    tdf_id: u16,
    size: u8,
    data: &[u8],
) -> TdfDecodeResult<Option<TdfDecodedPayload>> {
    let mut cursor = Cursor::new(data);
    let cursor_start = cursor.position();

    let decoded = match tdf_id {
        1 => Some(TdfDecodedPayload {
            id: 1,
            name: "ANNOUNCE",
            type_name: "Announce",
            fields: vec![
                TdfField {
                    name: "application",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "version",
                    value: TdfValue::Struct(vec![
                        TdfField {
                            name: "major",
                            value: TdfValue::UInt(cursor.read_u8()? as u64),
                        },
                        TdfField {
                            name: "minor",
                            value: TdfValue::UInt(cursor.read_u8()? as u64),
                        },
                        TdfField {
                            name: "revision",
                            value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                        },
                        TdfField {
                            name: "buildNum",
                            value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                        },
                    ]),
                },
                TdfField {
                    name: "kvCrc",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "blocks",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "uptime",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "reboots",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "flags",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
            ],
        }),
        2 => Some(TdfDecodedPayload {
            id: 2,
            name: "BATTERY_STATE",
            type_name: "BatteryState",
            fields: vec![
                TdfField {
                    name: "voltageMv",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "currentUa",
                    value: TdfValue::Int(cursor.read_i32::<LittleEndian>()? as i64),
                },
                TdfField {
                    name: "soc",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
            ],
        }),
        3 => Some(TdfDecodedPayload {
            id: 3,
            name: "AMBIENT_TEMP_PRES_HUM",
            type_name: "AmbientTempPresHum",
            fields: vec![
                TdfField {
                    name: "temperature",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "pressure",
                    value: TdfValue::Float(cursor.read_u32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "humidity",
                    value: TdfValue::Float(cursor.read_u16::<LittleEndian>()? as f64 / 100.0),
                },
            ],
        }),
        4 => Some(TdfDecodedPayload {
            id: 4,
            name: "AMBIENT_TEMPERATURE",
            type_name: "AmbientTemperature",
            fields: vec![TdfField {
                name: "temperature",
                value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 1000.0),
            }],
        }),
        5 => Some(TdfDecodedPayload {
            id: 5,
            name: "TIME_SYNC",
            type_name: "TimeSync",
            fields: vec![
                TdfField {
                    name: "source",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "shift",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 1000000.0),
                },
            ],
        }),
        6 => Some(TdfDecodedPayload {
            id: 6,
            name: "REBOOT_INFO",
            type_name: "RebootInfo",
            fields: vec![
                TdfField {
                    name: "reason",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "hardwareFlags",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "count",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "uptime",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "param1",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "param2",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "thread",
                    value: TdfValue::String(tdf_field_read_string_to_string(
                        &mut cursor,
                        cursor_start,
                        8,
                        size,
                    )?),
                },
            ],
        }),
        7 => Some(TdfDecodedPayload {
            id: 7,
            name: "ANNOUNCE_V2",
            type_name: "AnnounceV2",
            fields: vec![
                TdfField {
                    name: "application",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "version",
                    value: TdfValue::Struct(vec![
                        TdfField {
                            name: "major",
                            value: TdfValue::UInt(cursor.read_u8()? as u64),
                        },
                        TdfField {
                            name: "minor",
                            value: TdfValue::UInt(cursor.read_u8()? as u64),
                        },
                        TdfField {
                            name: "revision",
                            value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                        },
                        TdfField {
                            name: "buildNum",
                            value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                        },
                    ]),
                },
                TdfField {
                    name: "boardCrc",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "kvCrc",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "blocks",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "uptime",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "reboots",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "flags",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
            ],
        }),
        8 => Some(TdfDecodedPayload {
            id: 8,
            name: "SOC_TEMPERATURE",
            type_name: "SocTemperature",
            fields: vec![TdfField {
                name: "temperature",
                value: TdfValue::Float(cursor.read_i16::<LittleEndian>()? as f64 / 100.0),
            }],
        }),
        10 => Some(TdfDecodedPayload {
            id: 10,
            name: "ACC_2G",
            type_name: "Acc2g",
            fields: vec![TdfField {
                name: "sample",
                value: TdfValue::Struct(vec![
                    TdfField {
                        name: "x",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "y",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "z",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                ]),
            }],
        }),
        11 => Some(TdfDecodedPayload {
            id: 11,
            name: "ACC_4G",
            type_name: "Acc4g",
            fields: vec![TdfField {
                name: "sample",
                value: TdfValue::Struct(vec![
                    TdfField {
                        name: "x",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "y",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "z",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                ]),
            }],
        }),
        12 => Some(TdfDecodedPayload {
            id: 12,
            name: "ACC_8G",
            type_name: "Acc8g",
            fields: vec![TdfField {
                name: "sample",
                value: TdfValue::Struct(vec![
                    TdfField {
                        name: "x",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "y",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "z",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                ]),
            }],
        }),
        13 => Some(TdfDecodedPayload {
            id: 13,
            name: "ACC_16G",
            type_name: "Acc16g",
            fields: vec![TdfField {
                name: "sample",
                value: TdfValue::Struct(vec![
                    TdfField {
                        name: "x",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "y",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "z",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                ]),
            }],
        }),
        14 => Some(TdfDecodedPayload {
            id: 14,
            name: "GYR_125DPS",
            type_name: "Gyr125dps",
            fields: vec![TdfField {
                name: "sample",
                value: TdfValue::Struct(vec![
                    TdfField {
                        name: "x",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "y",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "z",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                ]),
            }],
        }),
        15 => Some(TdfDecodedPayload {
            id: 15,
            name: "GYR_250DPS",
            type_name: "Gyr250dps",
            fields: vec![TdfField {
                name: "sample",
                value: TdfValue::Struct(vec![
                    TdfField {
                        name: "x",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "y",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "z",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                ]),
            }],
        }),
        16 => Some(TdfDecodedPayload {
            id: 16,
            name: "GYR_500DPS",
            type_name: "Gyr500dps",
            fields: vec![TdfField {
                name: "sample",
                value: TdfValue::Struct(vec![
                    TdfField {
                        name: "x",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "y",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "z",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                ]),
            }],
        }),
        17 => Some(TdfDecodedPayload {
            id: 17,
            name: "GYR_1000DPS",
            type_name: "Gyr1000dps",
            fields: vec![TdfField {
                name: "sample",
                value: TdfValue::Struct(vec![
                    TdfField {
                        name: "x",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "y",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "z",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                ]),
            }],
        }),
        18 => Some(TdfDecodedPayload {
            id: 18,
            name: "GYR_2000DPS",
            type_name: "Gyr2000dps",
            fields: vec![TdfField {
                name: "sample",
                value: TdfValue::Struct(vec![
                    TdfField {
                        name: "x",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "y",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                    TdfField {
                        name: "z",
                        value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                    },
                ]),
            }],
        }),
        19 => Some(TdfDecodedPayload {
            id: 19,
            name: "GCS_WGS84_LLHA",
            type_name: "GcsWgs84Llha",
            fields: vec![
                TdfField {
                    name: "location",
                    value: TdfValue::Struct(vec![
                        TdfField {
                            name: "latitude",
                            value: TdfValue::Float(
                                cursor.read_i32::<LittleEndian>()? as f64 / 10000000.0,
                            ),
                        },
                        TdfField {
                            name: "longitude",
                            value: TdfValue::Float(
                                cursor.read_i32::<LittleEndian>()? as f64 / 10000000.0,
                            ),
                        },
                        TdfField {
                            name: "height",
                            value: TdfValue::Float(
                                cursor.read_i32::<LittleEndian>()? as f64 / 1000.0,
                            ),
                        },
                    ]),
                },
                TdfField {
                    name: "hAcc",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "vAcc",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 1000.0),
                },
            ],
        }),
        20 => Some(TdfDecodedPayload {
            id: 20,
            name: "UBX_NAV_PVT",
            type_name: "UbxNavPvt",
            fields: vec![
                TdfField {
                    name: "itow",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "year",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "month",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "day",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "hour",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "min",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "sec",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "valid",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "tAcc",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "nano",
                    value: TdfValue::Int(cursor.read_i32::<LittleEndian>()? as i64),
                },
                TdfField {
                    name: "fixType",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "flags",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "flags2",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "numSv",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "lon",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 10000000.0),
                },
                TdfField {
                    name: "lat",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 10000000.0),
                },
                TdfField {
                    name: "height",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "hMsl",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "hAcc",
                    value: TdfValue::Float(cursor.read_u32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "vAcc",
                    value: TdfValue::Float(cursor.read_u32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "velN",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "velE",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "velD",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "gSpeed",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "headMot",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 100000.0),
                },
                TdfField {
                    name: "sAcc",
                    value: TdfValue::Float(cursor.read_u32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "headAcc",
                    value: TdfValue::Float(cursor.read_u32::<LittleEndian>()? as f64 / 100000.0),
                },
                TdfField {
                    name: "pDop",
                    value: TdfValue::Float(cursor.read_u16::<LittleEndian>()? as f64 / 100.0),
                },
                TdfField {
                    name: "flags3",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "reserved0",
                    value: {
                        let mut values = Vec::with_capacity(4);
                        for _ in 0..4 {
                            values.push(TdfValue::UInt(cursor.read_u8()? as u64));
                        }
                        TdfValue::List(values)
                    },
                },
                TdfField {
                    name: "headVeh",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 100000.0),
                },
                TdfField {
                    name: "magDec",
                    value: TdfValue::Float(cursor.read_i16::<LittleEndian>()? as f64 / 100.0),
                },
                TdfField {
                    name: "magAcc",
                    value: TdfValue::Float(cursor.read_u16::<LittleEndian>()? as f64 / 100.0),
                },
            ],
        }),
        21 => Some(TdfDecodedPayload {
            id: 21,
            name: "LTE_CONN_STATUS",
            type_name: "LteConnStatus",
            fields: vec![
                TdfField {
                    name: "cell",
                    value: TdfValue::Struct(vec![
                        TdfField {
                            name: "mcc",
                            value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                        },
                        TdfField {
                            name: "mnc",
                            value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                        },
                        TdfField {
                            name: "eci",
                            value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                        },
                        TdfField {
                            name: "tac",
                            value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                        },
                    ]),
                },
                TdfField {
                    name: "earfcn",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "status",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "tech",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "rsrp",
                    value: TdfValue::Float(cursor.read_u8()? as f64 / -1.0),
                },
                TdfField {
                    name: "rsrq",
                    value: TdfValue::Int(cursor.read_i8()? as i64),
                },
            ],
        }),
        22 => Some(TdfDecodedPayload {
            id: 22,
            name: "GLOBALSTAR_PKT",
            type_name: "GlobalstarPkt",
            fields: vec![TdfField {
                name: "payload",
                value: {
                    let mut values = Vec::with_capacity(9);
                    for _ in 0..9 {
                        values.push(TdfValue::UInt(cursor.read_u8()? as u64));
                    }
                    TdfValue::List(values)
                },
            }],
        }),
        23 => Some(TdfDecodedPayload {
            id: 23,
            name: "ACC_MAGNITUDE_STD_DEV",
            type_name: "AccMagnitudeStdDev",
            fields: vec![
                TdfField {
                    name: "count",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "stdDev",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
            ],
        }),
        24 => Some(TdfDecodedPayload {
            id: 24,
            name: "ACTIVITY_METRIC",
            type_name: "ActivityMetric",
            fields: vec![TdfField {
                name: "value",
                value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
            }],
        }),
        25 => Some(TdfDecodedPayload {
            id: 25,
            name: "ALGORITHM_OUTPUT",
            type_name: "AlgorithmOutput",
            fields: vec![
                TdfField {
                    name: "algorithmId",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "algorithmVersion",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "output",
                    value: TdfValue::Bytes(tdf_field_read_vla(&mut cursor, cursor_start, size)?),
                },
            ],
        }),
        26 => Some(TdfDecodedPayload {
            id: 26,
            name: "RUNTIME_ERROR",
            type_name: "RuntimeError",
            fields: vec![
                TdfField {
                    name: "errorId",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "errorCtx",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
            ],
        }),
        27 => Some(TdfDecodedPayload {
            id: 27,
            name: "CHARGER_EN_CONTROL",
            type_name: "ChargerEnControl",
            fields: vec![TdfField {
                name: "enabled",
                value: TdfValue::UInt(cursor.read_u8()? as u64),
            }],
        }),
        28 => Some(TdfDecodedPayload {
            id: 28,
            name: "GNSS_FIX_INFO",
            type_name: "GnssFixInfo",
            fields: vec![
                TdfField {
                    name: "timeFix",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "locationFix",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "numSv",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
            ],
        }),
        29 => Some(TdfDecodedPayload {
            id: 29,
            name: "BLUETOOTH_CONNECTION",
            type_name: "BluetoothConnection",
            fields: vec![
                TdfField {
                    name: "address",
                    value: TdfValue::Struct(vec![
                        TdfField {
                            name: "type",
                            value: TdfValue::UInt(cursor.read_u8()? as u64),
                        },
                        TdfField {
                            name: "val",
                            value: TdfValue::UInt(cursor.read_u48::<LittleEndian>()?),
                        },
                    ]),
                },
                TdfField {
                    name: "connected",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
            ],
        }),
        30 => Some(TdfDecodedPayload {
            id: 30,
            name: "BLUETOOTH_RSSI",
            type_name: "BluetoothRssi",
            fields: vec![
                TdfField {
                    name: "address",
                    value: TdfValue::Struct(vec![
                        TdfField {
                            name: "type",
                            value: TdfValue::UInt(cursor.read_u8()? as u64),
                        },
                        TdfField {
                            name: "val",
                            value: TdfValue::UInt(cursor.read_u48::<LittleEndian>()?),
                        },
                    ]),
                },
                TdfField {
                    name: "rssi",
                    value: TdfValue::Int(cursor.read_i8()? as i64),
                },
            ],
        }),
        31 => Some(TdfDecodedPayload {
            id: 31,
            name: "BLUETOOTH_DATA_THROUGHPUT",
            type_name: "BluetoothDataThroughput",
            fields: vec![
                TdfField {
                    name: "address",
                    value: TdfValue::Struct(vec![
                        TdfField {
                            name: "type",
                            value: TdfValue::UInt(cursor.read_u8()? as u64),
                        },
                        TdfField {
                            name: "val",
                            value: TdfValue::UInt(cursor.read_u48::<LittleEndian>()?),
                        },
                    ]),
                },
                TdfField {
                    name: "throughput",
                    value: TdfValue::Int(cursor.read_i32::<LittleEndian>()? as i64),
                },
            ],
        }),
        32 => Some(TdfDecodedPayload {
            id: 32,
            name: "ALGORITHM_CLASS_HISTOGRAM",
            type_name: "AlgorithmClassHistogram",
            fields: vec![
                TdfField {
                    name: "algorithmId",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "algorithmVersion",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "classes",
                    value: TdfValue::Bytes(tdf_field_read_vla(&mut cursor, cursor_start, size)?),
                },
            ],
        }),
        33 => Some(TdfDecodedPayload {
            id: 33,
            name: "ALGORITHM_CLASS_TIME_SERIES",
            type_name: "AlgorithmClassTimeSeries",
            fields: vec![
                TdfField {
                    name: "algorithmId",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "algorithmVersion",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "values",
                    value: TdfValue::Bytes(tdf_field_read_vla(&mut cursor, cursor_start, size)?),
                },
            ],
        }),
        34 => Some(TdfDecodedPayload {
            id: 34,
            name: "LTE_TAC_CELLS",
            type_name: "LteTacCells",
            fields: vec![
                TdfField {
                    name: "cell",
                    value: TdfValue::Struct(vec![
                        TdfField {
                            name: "mcc",
                            value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                        },
                        TdfField {
                            name: "mnc",
                            value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                        },
                        TdfField {
                            name: "eci",
                            value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                        },
                        TdfField {
                            name: "tac",
                            value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                        },
                    ]),
                },
                TdfField {
                    name: "earfcn",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "rsrp",
                    value: TdfValue::Float(cursor.read_u8()? as f64 / -1.0),
                },
                TdfField {
                    name: "rsrq",
                    value: TdfValue::Int(cursor.read_i8()? as i64),
                },
                TdfField {
                    name: "neighbours",
                    value: {
                        let bytes_remaining = vla_bytes_remaining(&mut cursor, cursor_start, size)?;
                        if bytes_remaining % 10 != 0 {
                            return Err(TdfDecodeError::InvalidData(
                                "Variable-length array does not align to element size",
                            ));
                        }
                        let item_count = bytes_remaining / 10;
                        let mut values = Vec::with_capacity(item_count);
                        for _ in 0..item_count {
                            values.push(TdfValue::Struct(vec![
                                    TdfField {
                                        name: "earfcn",
                                        value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                                    },
                                    TdfField {
                                        name: "pci",
                                        value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                                    },
                                    TdfField {
                                        name: "timeDiff",
                                        value: TdfValue::Float(cursor.read_u16::<LittleEndian>()? as f64 / 1000.0),
                                    },
                                    TdfField {
                                        name: "rsrp",
                                        value: TdfValue::Float(cursor.read_u8()? as f64 / -1.0),
                                    },
                                    TdfField {
                                        name: "rsrq",
                                        value: TdfValue::Int(cursor.read_i8()? as i64),
                                    }
                            ]));
                        }
                        TdfValue::List(values)
                    },
                },
            ],
        }),
        35 => Some(TdfDecodedPayload {
            id: 35,
            name: "WIFI_AP_INFO",
            type_name: "WifiApInfo",
            fields: vec![
                TdfField {
                    name: "bssid",
                    value: TdfValue::Struct(vec![TdfField {
                        name: "val",
                        value: TdfValue::UInt(cursor.read_u48::<BigEndian>()?),
                    }]),
                },
                TdfField {
                    name: "channel",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "rsrp",
                    value: TdfValue::Int(cursor.read_i8()? as i64),
                },
            ],
        }),
        36 => Some(TdfDecodedPayload {
            id: 36,
            name: "DEVICE_TILT",
            type_name: "DeviceTilt",
            fields: vec![TdfField {
                name: "cosine",
                value: TdfValue::Float(cursor.read_f32::<LittleEndian>()? as f64),
            }],
        }),
        37 => Some(TdfDecodedPayload {
            id: 37,
            name: "NRF9X_GNSS_PVT",
            type_name: "Nrf9xGnssPvt",
            fields: vec![
                TdfField {
                    name: "lat",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 10000000.0),
                },
                TdfField {
                    name: "lon",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 10000000.0),
                },
                TdfField {
                    name: "height",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "hAcc",
                    value: TdfValue::Float(cursor.read_u32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "vAcc",
                    value: TdfValue::Float(cursor.read_u32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "hSpeed",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "hSpeedAcc",
                    value: TdfValue::Float(cursor.read_u32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "vSpeed",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "vSpeedAcc",
                    value: TdfValue::Float(cursor.read_u32::<LittleEndian>()? as f64 / 1000.0),
                },
                TdfField {
                    name: "headMot",
                    value: TdfValue::Float(cursor.read_i32::<LittleEndian>()? as f64 / 100000.0),
                },
                TdfField {
                    name: "headAcc",
                    value: TdfValue::Float(cursor.read_u32::<LittleEndian>()? as f64 / 100000.0),
                },
                TdfField {
                    name: "year",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "month",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "day",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "hour",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "min",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "sec",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "ms",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "pDop",
                    value: TdfValue::Float(cursor.read_u16::<LittleEndian>()? as f64 / 100.0),
                },
                TdfField {
                    name: "hDop",
                    value: TdfValue::Float(cursor.read_u16::<LittleEndian>()? as f64 / 100.0),
                },
                TdfField {
                    name: "vDop",
                    value: TdfValue::Float(cursor.read_u16::<LittleEndian>()? as f64 / 100.0),
                },
                TdfField {
                    name: "tDop",
                    value: TdfValue::Float(cursor.read_u16::<LittleEndian>()? as f64 / 100.0),
                },
                TdfField {
                    name: "flags",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "numSv",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
            ],
        }),
        38 => Some(TdfDecodedPayload {
            id: 38,
            name: "BATTERY_CHARGE_ACCUMULATED",
            type_name: "BatteryChargeAccumulated",
            fields: vec![TdfField {
                name: "charge",
                value: TdfValue::Int(cursor.read_i32::<LittleEndian>()? as i64),
            }],
        }),
        39 => Some(TdfDecodedPayload {
            id: 39,
            name: "INFUSE_BLUETOOTH_RSSI",
            type_name: "InfuseBluetoothRssi",
            fields: vec![
                TdfField {
                    name: "infuseId",
                    value: TdfValue::UInt(cursor.read_u64::<LittleEndian>()?),
                },
                TdfField {
                    name: "rssi",
                    value: TdfValue::Int(cursor.read_i8()? as i64),
                },
            ],
        }),
        40 => Some(TdfDecodedPayload {
            id: 40,
            name: "ADC_RAW_8",
            type_name: "AdcRaw8",
            fields: vec![TdfField {
                name: "val",
                value: TdfValue::Int(cursor.read_i8()? as i64),
            }],
        }),
        41 => Some(TdfDecodedPayload {
            id: 41,
            name: "ADC_RAW_16",
            type_name: "AdcRaw16",
            fields: vec![TdfField {
                name: "val",
                value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
            }],
        }),
        42 => Some(TdfDecodedPayload {
            id: 42,
            name: "ADC_RAW_32",
            type_name: "AdcRaw32",
            fields: vec![TdfField {
                name: "val",
                value: TdfValue::Int(cursor.read_i32::<LittleEndian>()? as i64),
            }],
        }),
        43 => Some(TdfDecodedPayload {
            id: 43,
            name: "ANNOTATION",
            type_name: "Annotation",
            fields: vec![
                TdfField {
                    name: "timestamp",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "event",
                    value: TdfValue::String(tdf_field_read_string_to_string(
                        &mut cursor,
                        cursor_start,
                        0,
                        size,
                    )?),
                },
            ],
        }),
        44 => Some(TdfDecodedPayload {
            id: 44,
            name: "LORA_RX",
            type_name: "LoraRx",
            fields: vec![
                TdfField {
                    name: "snr",
                    value: TdfValue::Int(cursor.read_i8()? as i64),
                },
                TdfField {
                    name: "rssi",
                    value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                },
                TdfField {
                    name: "payload",
                    value: TdfValue::Bytes(tdf_field_read_vla(&mut cursor, cursor_start, size)?),
                },
            ],
        }),
        45 => Some(TdfDecodedPayload {
            id: 45,
            name: "LORA_TX",
            type_name: "LoraTx",
            fields: vec![TdfField {
                name: "payload",
                value: TdfValue::Bytes(tdf_field_read_vla(&mut cursor, cursor_start, size)?),
            }],
        }),
        46 => Some(TdfDecodedPayload {
            id: 46,
            name: "IDX_ARRAY_FREQ",
            type_name: "IdxArrayFreq",
            fields: vec![
                TdfField {
                    name: "tdfId",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "frequency",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
            ],
        }),
        47 => Some(TdfDecodedPayload {
            id: 47,
            name: "IDX_ARRAY_PERIOD",
            type_name: "IdxArrayPeriod",
            fields: vec![
                TdfField {
                    name: "tdfId",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "period",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
            ],
        }),
        48 => Some(TdfDecodedPayload {
            id: 48,
            name: "WIFI_CONNECTED",
            type_name: "WifiConnected",
            fields: vec![TdfField {
                name: "network",
                value: TdfValue::Struct(vec![
                    TdfField {
                        name: "bssid",
                        value: TdfValue::UInt(cursor.read_u48::<BigEndian>()?),
                    },
                    TdfField {
                        name: "band",
                        value: TdfValue::UInt(cursor.read_u8()? as u64),
                    },
                    TdfField {
                        name: "channel",
                        value: TdfValue::UInt(cursor.read_u8()? as u64),
                    },
                    TdfField {
                        name: "ifaceMode",
                        value: TdfValue::UInt(cursor.read_u8()? as u64),
                    },
                    TdfField {
                        name: "linkMode",
                        value: TdfValue::UInt(cursor.read_u8()? as u64),
                    },
                    TdfField {
                        name: "security",
                        value: TdfValue::UInt(cursor.read_u8()? as u64),
                    },
                    TdfField {
                        name: "rssi",
                        value: TdfValue::Int(cursor.read_i8()? as i64),
                    },
                    TdfField {
                        name: "beaconInterval",
                        value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                    },
                    TdfField {
                        name: "twtCapable",
                        value: TdfValue::UInt(cursor.read_u8()? as u64),
                    },
                ]),
            }],
        }),
        49 => Some(TdfDecodedPayload {
            id: 49,
            name: "WIFI_CONNECTION_FAILED",
            type_name: "WifiConnectionFailed",
            fields: vec![TdfField {
                name: "reason",
                value: TdfValue::UInt(cursor.read_u8()? as u64),
            }],
        }),
        50 => Some(TdfDecodedPayload {
            id: 50,
            name: "WIFI_DISCONNECTED",
            type_name: "WifiDisconnected",
            fields: vec![TdfField {
                name: "reason",
                value: TdfValue::UInt(cursor.read_u8()? as u64),
            }],
        }),
        51 => Some(TdfDecodedPayload {
            id: 51,
            name: "NETWORK_SCAN_COUNT",
            type_name: "NetworkScanCount",
            fields: vec![
                TdfField {
                    name: "numWifi",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "numLte",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
            ],
        }),
        52 => Some(TdfDecodedPayload {
            id: 52,
            name: "EXCEPTION_STACK_FRAME",
            type_name: "ExceptionStackFrame",
            fields: vec![TdfField {
                name: "frame",
                value: {
                    let bytes_remaining = vla_bytes_remaining(&mut cursor, cursor_start, size)?;
                    if bytes_remaining % 4 != 0 {
                        return Err(TdfDecodeError::InvalidData(
                            "Variable-length array does not align to element size",
                        ));
                    }
                    let item_count = bytes_remaining / 4;
                    let mut values = Vec::with_capacity(item_count);
                    for _ in 0..item_count {
                        values.push(TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64));
                    }
                    TdfValue::List(values)
                },
            }],
        }),
        53 => Some(TdfDecodedPayload {
            id: 53,
            name: "BATTERY_VOLTAGE",
            type_name: "BatteryVoltage",
            fields: vec![TdfField {
                name: "voltage",
                value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
            }],
        }),
        54 => Some(TdfDecodedPayload {
            id: 54,
            name: "BATTERY_SOC",
            type_name: "BatterySoc",
            fields: vec![TdfField {
                name: "soc",
                value: TdfValue::UInt(cursor.read_u8()? as u64),
            }],
        }),
        55 => Some(TdfDecodedPayload {
            id: 55,
            name: "STATE_EVENT_SET",
            type_name: "StateEventSet",
            fields: vec![TdfField {
                name: "state",
                value: TdfValue::UInt(cursor.read_u8()? as u64),
            }],
        }),
        56 => Some(TdfDecodedPayload {
            id: 56,
            name: "STATE_EVENT_CLEARED",
            type_name: "StateEventCleared",
            fields: vec![TdfField {
                name: "state",
                value: TdfValue::UInt(cursor.read_u8()? as u64),
            }],
        }),
        57 => Some(TdfDecodedPayload {
            id: 57,
            name: "STATE_DURATION",
            type_name: "StateDuration",
            fields: vec![
                TdfField {
                    name: "state",
                    value: TdfValue::UInt(cursor.read_u8()? as u64),
                },
                TdfField {
                    name: "duration",
                    value: TdfValue::UInt(cursor.read_u32::<LittleEndian>()? as u64),
                },
            ],
        }),
        58 => Some(TdfDecodedPayload {
            id: 58,
            name: "PCM_16BIT_CHAN_LEFT",
            type_name: "Pcm16bitChanLeft",
            fields: vec![TdfField {
                name: "val",
                value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
            }],
        }),
        59 => Some(TdfDecodedPayload {
            id: 59,
            name: "PCM_16BIT_CHAN_RIGHT",
            type_name: "Pcm16bitChanRight",
            fields: vec![TdfField {
                name: "val",
                value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
            }],
        }),
        60 => Some(TdfDecodedPayload {
            id: 60,
            name: "PCM_16BIT_CHAN_DUAL",
            type_name: "Pcm16bitChanDual",
            fields: vec![
                TdfField {
                    name: "left",
                    value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                },
                TdfField {
                    name: "right",
                    value: TdfValue::Int(cursor.read_i16::<LittleEndian>()? as i64),
                },
            ],
        }),
        61 => Some(TdfDecodedPayload {
            id: 61,
            name: "KVS_VALUE_CHANGED",
            type_name: "KvsValueChanged",
            fields: vec![
                TdfField {
                    name: "key",
                    value: TdfValue::UInt(cursor.read_u16::<LittleEndian>()? as u64),
                },
                TdfField {
                    name: "value",
                    value: TdfValue::Bytes(tdf_field_read_vla(&mut cursor, cursor_start, size)?),
                },
            ],
        }),
        62 => Some(TdfDecodedPayload {
            id: 62,
            name: "AMBIENT_PRESSURE",
            type_name: "AmbientPressure",
            fields: vec![TdfField {
                name: "pressure",
                value: TdfValue::Float(cursor.read_u32::<LittleEndian>()? as f64 / 1000.0),
            }],
        }),
        _ => None,
    };

    if decoded.is_some() && cursor.position() != size as u64 {
        return Err(TdfDecodeError::InvalidData(
            "Decoded payload did not consume exactly its element size",
        ));
    }

    Ok(decoded)
}

fn vla_bytes_remaining(
    cursor: &mut Cursor<&[u8]>,
    cursor_start: u64,
    size: u8,
) -> TdfDecodeResult<usize> {
    let cursor_current = cursor.position();
    let cursor_read = cursor_current - cursor_start;
    if cursor_read > size as u64 {
        return Err(TdfDecodeError::InvalidData("Insufficient data remaining"));
    }
    let bytes_remaining = size as u64 - cursor_read;

    Ok(bytes_remaining as usize)
}

fn tdf_field_read_string(
    cursor: &mut Cursor<&[u8]>,
    cursor_start: u64,
    num: u8,
    size: u8,
) -> TdfDecodeResult<Vec<u8>> {
    let string_length = match num {
        0 => vla_bytes_remaining(cursor, cursor_start, size)?,
        _ => num as usize,
    };

    let mut buf = vec![0u8; string_length];
    cursor.read_exact(&mut buf)?;

    Ok(buf)
}

fn tdf_field_read_vla(
    cursor: &mut Cursor<&[u8]>,
    cursor_start: u64,
    size: u8,
) -> TdfDecodeResult<Vec<u8>> {
    let bytes_remaining = vla_bytes_remaining(cursor, cursor_start, size)?;
    let mut buf = vec![0u8; bytes_remaining];

    cursor.read_exact(&mut buf)?;
    Ok(buf)
}
