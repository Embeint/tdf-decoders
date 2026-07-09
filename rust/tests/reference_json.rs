use std::{
    fs,
    path::{Path, PathBuf},
};

use tdf::{decoders::TdfValue, time, TdfBlockDecodeResult, TdfDecodeError, TdfReading};

const STORAGE_BLOCK_SIZE: usize = 512;

#[test]
fn output_matches_reference_json_for_common_test_data() {
    let test_data_dir = find_repo_root().join("common/test_data");
    let include_extensions = tdf::decoders::tdf_known_name(1025).is_some();
    let mut fixtures: Vec<_> = fs::read_dir(&test_data_dir)
        .expect("read common test data directory")
        .map(|entry| entry.expect("read test data entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "bin"))
        .filter(|path| include_extensions || !is_extension_fixture(path))
        .collect();
    fixtures.sort();

    assert!(!fixtures.is_empty());

    for fixture in fixtures {
        let reference_path = fixture.with_extension("reference.json");
        let expected_json = fs::read_to_string(&reference_path).expect("read reference json");
        let actual_json = generate_reference_json(&fixture);

        assert_reference_json_matches(&fixture, &expected_json, &actual_json);
    }
}

fn is_extension_fixture(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("extension_"))
}

fn assert_reference_json_matches(fixture: &Path, expected: &str, actual: &str) {
    if expected == actual {
        return;
    }

    let expected_lines: Vec<_> = expected.lines().collect();
    let actual_lines: Vec<_> = actual.lines().collect();
    let line_count = expected_lines.len().max(actual_lines.len());

    for index in 0..line_count {
        match (expected_lines.get(index), actual_lines.get(index)) {
            (Some(expected_line), Some(actual_line)) if expected_line == actual_line => {}
            (Some(expected_line), Some(actual_line)) => {
                panic!(
                    "reference JSON mismatch for {}\nfirst difference at line {}\nexpected: {:?}\nactual:   {:?}\nexpected line count: {}\nactual line count: {}",
                    fixture.display(),
                    index + 1,
                    expected_line,
                    actual_line,
                    expected_lines.len(),
                    actual_lines.len(),
                );
            }
            (Some(expected_line), None) => {
                panic!(
                    "reference JSON mismatch for {}\nactual output ended before expected line {}\nexpected: {:?}\nexpected line count: {}\nactual line count: {}",
                    fixture.display(),
                    index + 1,
                    expected_line,
                    expected_lines.len(),
                    actual_lines.len(),
                );
            }
            (None, Some(actual_line)) => {
                panic!(
                    "reference JSON mismatch for {}\nactual output has an extra line at {}\nactual: {:?}\nexpected line count: {}\nactual line count: {}",
                    fixture.display(),
                    index + 1,
                    actual_line,
                    expected_lines.len(),
                    actual_lines.len(),
                );
            }
            (None, None) => break,
        }
    }

    panic!(
        "reference JSON mismatch for {}\ncontent differs outside line text, likely trailing newline or encoding differences\nexpected bytes: {}\nactual bytes: {}",
        fixture.display(),
        expected.len(),
        actual.len(),
    );
}

fn generate_reference_json(fixture: &Path) -> String {
    let data = fs::read(fixture).expect("read fixture");
    let fixture_name = fixture
        .file_name()
        .and_then(|name| name.to_str())
        .expect("fixture file name");
    let result = decode_storage_fixture(&data);
    let success = result.success();

    let mut out = String::new();
    out.push_str("{\n");
    push_json_field(&mut out, 1, "format", "\"tdf-decoder-reference-v1\"", true);
    push_json_field(&mut out, 1, "fixture", &json_string(fixture_name), true);
    push_json_field(&mut out, 1, "byteLength", &data.len().to_string(), true);
    push_json_field(
        &mut out,
        1,
        "sha256",
        &json_string(&sha256_hex(&data)),
        true,
    );
    push_json_field(
        &mut out,
        1,
        "success",
        if success { "true" } else { "false" },
        true,
    );
    push_readings(&mut out, &result);
    out.push_str(",\n  \"error\": ");
    if let Some(error) = &result.error {
        push_error(&mut out, error, 1);
        out.push('\n');
    } else {
        out.push_str("null\n");
    }
    out.push_str("}\n");
    out
}

fn decode_storage_fixture(data: &[u8]) -> TdfBlockDecodeResult {
    let mut readings = Vec::new();

    for chunk in data.chunks(STORAGE_BLOCK_SIZE) {
        let result = tdf::block_decode(None, tdf_payload_from_storage_fixture(chunk));
        readings.extend(result.readings);

        if result.error.is_some() {
            return TdfBlockDecodeResult {
                readings,
                error: result.error,
            };
        }
    }

    TdfBlockDecodeResult {
        readings,
        error: None,
    }
}

fn tdf_payload_from_storage_fixture(data: &[u8]) -> &[u8] {
    &data[2..]
}

fn push_error(out: &mut String, error: &TdfDecodeError, indent: usize) {
    match error {
        TdfDecodeError::UnexpectedEof(
            "Block ended before a complete TDF header could be read",
        ) => {
            out.push_str("{\n");
            push_json_field(out, indent + 1, "kind", "\"UnexpectedEof\"", true);
            push_json_field(out, indent + 1, "offset", "null", true);
            push_json_field(out, indent + 1, "id", "null", true);
            push_json_field(
                out,
                indent + 1,
                "message",
                "\"Block ended before a complete TDF header could be read\"",
                true,
            );
            push_json_field(out, indent + 1, "exceptionType", "null", false);
            push_indent(out, indent);
            out.push('}');
        }
        _ => panic!("unsupported reference fixture decode error: {error}"),
    }
}

fn push_readings(out: &mut String, result: &TdfBlockDecodeResult) {
    out.push_str("  \"readings\": [\n");
    for (index, reading) in result.readings.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        push_reading(out, reading, 2);
    }
    out.push_str("\n  ]");
}

fn push_reading(out: &mut String, reading: &TdfReading, indent: usize) {
    push_indent(out, indent);
    out.push_str("{\n");
    push_json_field(out, indent + 1, "id", &reading.id.to_string(), true);
    let name = reading
        .name
        .as_ref()
        .map(|name| json_string(name))
        .unwrap_or_else(|| "null".to_string());
    push_json_field(out, indent + 1, "name", &name, true);
    push_json_field(
        out,
        indent + 1,
        "rawDataHex",
        &json_string(&hex_string(&reading.raw_data)),
        true,
    );
    push_json_field(
        out,
        indent + 1,
        "timestamp",
        &timestamp_json(reading.timestamp, indent + 1),
        true,
    );
    let period = reading
        .period
        .map(json_float)
        .unwrap_or_else(|| "null".to_string());
    push_json_field(out, indent + 1, "period", &period, true);
    let index = reading
        .index
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string());
    push_json_field(out, indent + 1, "index", &index, true);
    push_indent(out, indent + 1);
    out.push_str("\"decodedPayload\": [");
    if !reading.decoded_payload.is_empty() {
        out.push('\n');
        for (payload_index, payload) in reading.decoded_payload.iter().enumerate() {
            if payload_index > 0 {
                out.push_str(",\n");
            }
            push_indent(out, indent + 2);
            out.push_str("{\n");
            push_json_field(
                out,
                indent + 3,
                "type",
                &json_string(payload.type_name),
                true,
            );
            push_indent(out, indent + 3);
            out.push_str("\"fields\": ");
            push_fields_object(out, &payload.fields, indent + 3);
            out.push('\n');
            push_indent(out, indent + 2);
            out.push('}');
        }
        out.push('\n');
        push_indent(out, indent + 1);
    }
    out.push_str("]\n");
    push_indent(out, indent);
    out.push('}');
}

fn timestamp_json(timestamp: Option<i64>, indent: usize) -> String {
    let Some(epoch_time) = timestamp else {
        return "null".to_string();
    };
    let (unix_seconds, unix_nanoseconds) = time::tdf_time_to_unix(epoch_time);
    format!(
        "{{\n{0}\"epochTime\": {1},\n{0}\"gpsSeconds\": {2},\n{0}\"fractionalTicks\": {3},\n{0}\"unixSeconds\": {4},\n{0}\"unixNanoseconds\": {5}\n{6}}}",
        "  ".repeat(indent + 1),
        epoch_time,
        epoch_time >> 16,
        epoch_time & 0xFFFF,
        unix_seconds,
        unix_nanoseconds,
        "  ".repeat(indent),
    )
}

fn push_fields_object(out: &mut String, fields: &[tdf::decoders::TdfField], indent: usize) {
    out.push_str("{\n");
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        push_indent(out, indent + 1);
        out.push('"');
        out.push_str(field.name);
        out.push_str("\": ");
        push_value(out, &field.value, indent + 1);
    }
    out.push('\n');
    push_indent(out, indent);
    out.push('}');
}

fn push_value(out: &mut String, value: &TdfValue, indent: usize) {
    match value {
        TdfValue::Int(value) => out.push_str(&value.to_string()),
        TdfValue::UInt(value) => out.push_str(&value.to_string()),
        TdfValue::Float(value) => out.push_str(&json_float(*value)),
        TdfValue::String(value) => out.push_str(&json_string(value)),
        TdfValue::Bytes(values) => push_bytes_array(out, values, indent),
        TdfValue::List(values) => push_list(out, values, indent),
        TdfValue::Struct(fields) => push_fields_object(out, fields, indent),
    }
}

fn push_list(out: &mut String, values: &[TdfValue], indent: usize) {
    out.push('[');
    if !values.is_empty() {
        out.push('\n');
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                out.push_str(",\n");
            }
            push_indent(out, indent + 1);
            push_value(out, value, indent + 1);
        }
        out.push('\n');
        push_indent(out, indent);
    }
    out.push(']');
}

fn push_bytes_array(out: &mut String, values: &[u8], indent: usize) {
    out.push('[');
    if !values.is_empty() {
        out.push('\n');
        for (index, value) in values.iter().enumerate() {
            if index > 0 {
                out.push_str(",\n");
            }
            push_indent(out, indent + 1);
            out.push_str(&value.to_string());
        }
        out.push('\n');
        push_indent(out, indent);
    }
    out.push(']');
}

fn push_json_field(out: &mut String, indent: usize, name: &str, value: &str, trailing_comma: bool) {
    push_indent(out, indent);
    out.push('"');
    out.push_str(name);
    out.push_str("\": ");
    out.push_str(value);
    if trailing_comma {
        out.push(',');
    }
    out.push('\n');
}

fn push_indent(out: &mut String, indent: usize) {
    out.push_str(&"  ".repeat(indent));
}

fn json_string(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => out.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn json_float(value: f64) -> String {
    if value == 0.0 {
        "0".to_string()
    } else {
        value.to_string()
    }
}

fn hex_string(bytes: &[u8]) -> String {
    bytes.iter().map(|value| format!("{value:02x}")).collect()
}

fn find_repo_root() -> PathBuf {
    let mut directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if directory.join("common/test_data").is_dir() && directory.join("csharp").is_dir() {
            return directory;
        }

        if !directory.pop() {
            panic!("could not find repository root");
        }
    }
}

fn sha256_hex(data: &[u8]) -> String {
    hex_string(&sha256(data))
}

fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64) * 8;
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in message.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (index, bytes) in chunk.chunks_exact(4).enumerate().take(16) {
            w[index] = u32::from_be_bytes(bytes.try_into().expect("sha256 word"));
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7)
                ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17)
                ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0)
                .wrapping_add(w[index - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[index])
                .wrapping_add(w[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        for (state, value) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *state = state.wrapping_add(value);
        }
    }

    let mut out = [0u8; 32];
    for (index, value) in h.iter().enumerate() {
        out[index * 4..][..4].copy_from_slice(&value.to_be_bytes());
    }
    out
}
