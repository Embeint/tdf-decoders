#!/usr/bin/env python3
"""Generate decoder sources from the shared TDF definition schema."""

from __future__ import annotations

import argparse
import decimal
import json
import re
import subprocess
from xml.sax.saxutils import escape
from pathlib import Path

from numpy import format_float_positional as float_format
from jinja2 import Environment, FileSystemLoader, select_autoescape


REPO_ROOT = Path(__file__).resolve().parents[2]
COMMON_DIR = REPO_ROOT / "common"
TEMPLATE_DIR = COMMON_DIR / "scripts" / "templates"
CS_OUTPUT_DIR = REPO_ROOT / "csharp" / "generated"
RUST_OUTPUT_DIR = REPO_ROOT / "rust" / "src" / "generated"
DEFAULT_CS_NAMESPACE = "TdfDecoders"

CS_SCALAR_TYPES = {
    "char": "char",
    "float": "float",
    "int8_t": "sbyte",
    "uint8_t": "byte",
    "int16_t": "short",
    "uint16_t": "ushort",
    "int32_t": "int",
    "uint32_t": "uint",
    "uint64_t": "ulong",
}

TYPE_SIZES = {
    "char": 1,
    "float": 4,
    "int8_t": 1,
    "uint8_t": 1,
    "int16_t": 2,
    "uint16_t": 2,
    "int32_t": 4,
    "uint32_t": 4,
    "uint64_t": 8,
}

CS_SCALAR_READ_METHODS = {
    "float": "ReadSingle",
    "int8_t": "ReadSByte",
    "uint8_t": "ReadByte",
    "int16_t": "ReadInt16",
    "uint16_t": "ReadUInt16",
    "int32_t": "ReadInt32",
    "uint32_t": "ReadUInt32",
    "uint64_t": "ReadUInt64",
}

RUST_SCALAR_TYPES = {
    "char": ("u8", False),
    "int8_t": ("i8", False),
    "uint8_t": ("u8", False),
    "int16_t": ("i16", True),
    "uint16_t": ("u16", True),
    "int32_t": ("i32", True),
    "uint32_t": ("u32", True),
    "int64_t": ("i64", True),
    "uint64_t": ("u64", True),
    "float": ("f32", True),
    "float32_t": ("f32", True),
    "float64_t": ("f64", True),
}

RUST_VALUE_TYPES = {
    "char": "String",
    "int8_t": "i8",
    "uint8_t": "u8",
    "int16_t": "i16",
    "uint16_t": "u16",
    "int32_t": "i32",
    "uint32_t": "u32",
    "int64_t": "i64",
    "uint64_t": "u64",
    "float": "f32",
    "float32_t": "f32",
    "float64_t": "f64",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "extension",
        nargs="?",
        type=Path,
        help="Optional TDF definition extension JSON to merge with common/tdf.json.",
    )
    parser.add_argument(
        "--language",
        choices=("all", "csharp", "rust"),
        default="all",
        help="Which decoder sources to generate (default: all).",
    )
    parser.add_argument(
        "--csharp-output",
        type=Path,
        default=None,
        help=(
            "Path to write the generated C# source to. "
            f"Defaults to {CS_OUTPUT_DIR.relative_to(REPO_ROOT)}/TdfDecoders.cs."
        ),
    )
    parser.add_argument(
        "--csharp-namespace",
        default=DEFAULT_CS_NAMESPACE,
        help=f"Namespace for the generated C# source (default: {DEFAULT_CS_NAMESPACE}).",
    )
    parser.add_argument(
        "--rust-output",
        type=Path,
        default=None,
        help=(
            "Path to write the generated Rust source to. "
            f"Defaults to {RUST_OUTPUT_DIR.relative_to(REPO_ROOT)}/decoders.rs."
        ),
    )
    return parser.parse_args()


def display_path(path: Path) -> str:
    """Render an output path relative to the repository when it lives inside it.

    Generated sources can be written outside the repository, where
    ``Path.relative_to`` raises instead of returning a display string.
    """
    try:
        return str(path.relative_to(REPO_ROOT))
    except ValueError:
        return str(path)


def load_definitions(path: Path, *, parse_float=None) -> dict:
    kwargs = {}
    if parse_float is not None:
        kwargs["parse_float"] = parse_float

    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle, **kwargs)


def merge_definition_data(base: dict, extension: dict) -> dict:
    for key in ("structs", "definitions"):
        base.setdefault(key, {}).update(extension.get(key, {}))
    return base


def load_merged_definitions(extension_path: Path | None, *, parse_float=None) -> dict:
    data = load_definitions(COMMON_DIR / "tdf.json", parse_float=parse_float)
    if extension_path is not None:
        extension_data = load_definitions(extension_path, parse_float=parse_float)
        merge_definition_data(data, extension_data)
    return data


def has_hex_conversion(data: dict) -> bool:
    definitions = list(data.get("structs", {}).values()) + list(data.get("definitions", {}).values())
    return any(
        field.get("conversion", {}).get("hex") is True
        for definition in definitions
        for field in definition.get("fields", [])
    )


def pascal_case(value: str) -> str:
    return "".join(part.capitalize() for part in re.split(r"[^0-9A-Za-z]+", value) if part)


def class_name(value: str) -> str:
    if value.startswith("tdf_struct_"):
        value = value.removeprefix("tdf_struct_")
        return "TdfStruct" + pascal_case(value)
    return pascal_case(value)


def is_struct_type(type_name: str) -> bool:
    return type_name.startswith("struct ")


def struct_name(type_name: str) -> str:
    return type_name.removeprefix("struct ")


def field_size(field: dict, struct_sizes: dict[str, int]) -> int | None:
    type_name = field["type"]
    count = field.get("num")

    if count == 0:
        return None

    if is_struct_type(type_name):
        size = struct_sizes[struct_name(type_name)]
    else:
        size = TYPE_SIZES[type_name]

    return size * (count or 1)


def cs_type(field: dict) -> str:
    type_name = field["type"]
    count = field.get("num")
    conversion = field.get("conversion", {})

    if conversion.get("hex") is True:
        return "string"

    if "m" in conversion or "c" in conversion:
        return "double"

    if conversion.get("int") is not None:
        return "ulong"

    if is_struct_type(type_name):
        base_type = class_name(struct_name(type_name))
    elif type_name == "char" and count is not None:
        base_type = "string"
    else:
        base_type = CS_SCALAR_TYPES[type_name]

    if count is not None and type_name != "char":
        return f"IReadOnlyList<{base_type}>"

    return base_type


def read_method(field: dict) -> str:
    type_name = field["type"]
    count = field.get("num")
    conversion = field.get("conversion", {})

    if conversion.get("hex") is True:
        if type_name != "uint8_t":
            raise ValueError("Hex conversion is only supported for uint8_t fields")
        if count == 0:
            return "reader.ReadRemainingHex()"
        if count is not None:
            return f"reader.ReadHex({count})"
        return "reader.ReadHex(1)"

    if is_struct_type(type_name):
        base_type = class_name(struct_name(type_name))
        if count is None:
            return f"{base_type}.DecodeFrom(ref reader)"
        if count == 0:
            return f"Array.AsReadOnly(reader.ReadRemainingStructs({base_type}.DecodeFrom))"
        return f"Array.AsReadOnly(reader.ReadStructs({count}, {base_type}.DecodeFrom))"

    if type_name == "char":
        if count == 0:
            return "reader.ReadRemainingString()"
        if count is not None:
            return f"reader.ReadString({count})"
        return "reader.ReadChar()"

    if type_name == "uint8_t" and count is not None and not conversion:
        if count == 0:
            return "reader.ReadRemainingBytes()"
        return f"reader.ReadBytes({count})"

    if conversion.get("int") is not None:
        little_endian = "true" if conversion["int"] == "little" else "false"
        return f"reader.ReadUInt({count}, {little_endian})"

    method = CS_SCALAR_READ_METHODS[type_name]

    if count is None:
        expression = f"reader.{method}()"
        if "m" in conversion or "c" in conversion:
            expression = f"({expression} * {conversion.get('m', 1):g}) + {conversion.get('c', 0):g}"
        return expression
    if count == 0:
        return f"Array.AsReadOnly(reader.ReadRemaining{method[4:]}Values({TYPE_SIZES[type_name]}))"
    return f"Array.AsReadOnly(reader.Read{method[4:]}Values({count}))"


def prepare_fields(fields: list[dict], struct_sizes: dict[str, int]) -> list[dict]:
    prepared = []
    for field in fields:
        item = dict(field)
        item["property_name"] = pascal_case(field["name"])
        item["cs_type"] = cs_type(field)
        item["read_expression"] = read_method(field)
        item["size"] = field_size(field, struct_sizes)
        if item["cs_type"].startswith("IReadOnlyList<"):
            item["property_initializer"] = f" = Array.Empty<{item['cs_type'][14:-1]}>();"
        elif item["cs_type"] == "string":
            item["property_initializer"] = " = string.Empty;"
        elif item["cs_type"].startswith("TdfStruct"):
            item["property_initializer"] = " = default!;"
        else:
            item["property_initializer"] = ""
        prepared.append(item)
    return prepared


def rust_str(value: str) -> str:
    return json.dumps(value)


def rust_ident(name: str) -> str:
    name = re.sub(r"\W", "_", name)
    if not name or name[0].isdigit():
        name = f"_{name}"
    return name


def lower_camel(name: str) -> str:
    parts = [part for part in re.split(r"\W|_", name) if part]
    if not parts:
        return name
    return parts[0].lower() + "".join(part.capitalize() for part in parts[1:])


def rust_primitive_read_expr(field: dict) -> str:
    rust_type = RUST_SCALAR_TYPES[field["type"]]
    expression = f"cursor.read_{rust_type[0]}"
    if rust_type[1]:
        expression += "::<LittleEndian>"
    expression += "()?"

    if conversion := field.get("conversion"):
        if endian := conversion.get("int", None):
            byte_order = "LittleEndian" if endian == "little" else "BigEndian"
            if field["num"] == 3:
                type_name = "u24"
            elif field["num"] == 6:
                type_name = "u48"
            else:
                raise ValueError("Unknown integer length")
            expression = f"cursor.read_{type_name}::<{byte_order}>()?"

        if "m" in conversion or "c" in conversion:
            expression += " as f64"
            if "m" in conversion and conversion["m"] != 0:
                value = conversion["m"]
                inverse_ratio = (1 / value).as_integer_ratio()
                if inverse_ratio[1] == 1:
                    expression += f" / {inverse_ratio[0]}.0"
                else:
                    expression += f" * {float_format(conversion['m'])}"
            if "c" in conversion and conversion["c"] != 0:
                expression += f" + {float_format(conversion['c'])}"

    return expression


def rust_type_after_conversion(field: dict) -> str:
    conversion = field.get("conversion", {})
    if "m" in conversion or "c" in conversion:
        return "f64"
    if field["type"] == "char":
        return "String"
    if "int" in conversion:
        byte_len = field["num"]
        if byte_len <= 1:
            return "u8"
        if byte_len <= 2:
            return "u16"
        if byte_len <= 4:
            return "u32"
        if byte_len <= 8:
            return "u64"
    return RUST_VALUE_TYPES[field["type"]]


def rust_field_byte_size(field: dict, structs: dict) -> int:
    type_name = field["type"]
    conversion = field.get("conversion", {})
    if "int" in conversion:
        base_size = field["num"]
    elif is_struct_type(type_name):
        base_size = sum(
            rust_field_byte_size(child, structs)
            for child in structs[struct_name(type_name)]["fields"]
        )
    elif type_name == "char":
        base_size = field.get("num", 0)
    else:
        base_size = TYPE_SIZES[type_name]

    count = field.get("num", 1)
    if count == 0 or "int" in conversion or type_name == "char":
        return base_size
    return base_size * count


def rust_field_model(field: dict, path: list[str], structs: dict) -> dict:
    type_name = field["type"]
    count = field.get("num", None)
    conversion = field.get("conversion", {})

    if conversion.get("hex") is True:
        if type_name != "uint8_t":
            raise ValueError("Hex conversion is only supported for uint8_t fields")
        num = count if count is not None else 1
        return {
            "kind": "hex",
            "path": path,
            "read": f"tdf_field_read_hex(&mut cursor, cursor_start, {num}, size)?",
            "raw_field": field,
        }

    if is_struct_type(type_name):
        children = [
            rust_field_model(child, path + [rust_ident(child["name"])], structs)
            for child in structs[struct_name(type_name)]["fields"]
        ]
        model = {
            "kind": "struct",
            "path": path,
            "children": children,
            "raw_field": field,
        }
        if count == 0:
            item_field = {k: v for k, v in field.items() if k != "num"}
            return {
                "kind": "list",
                "path": path,
                "child": model,
                "item_size": rust_field_byte_size(item_field, structs),
                "raw_field": field,
            }
        if count is not None:
            item_field = {k: v for k, v in field.items() if k != "num"}
            return {
                "kind": "fixed_list",
                "path": path,
                "child": rust_field_model(item_field, path, structs),
                "num": count,
                "raw_field": field,
            }
        return model

    if type_name == "char":
        return {
            "kind": "string",
            "path": path,
            "read": f"tdf_field_read_string_to_string(&mut cursor, cursor_start, {count or 0}, size)?",
            "raw_field": field,
        }

    if count == 0 and type_name == "uint8_t" and "int" not in conversion:
        return {
            "kind": "binary",
            "path": path,
            "raw_field": field,
        }

    if count == 0:
        item_field = {k: v for k, v in field.items() if k != "num"}
        return {
            "kind": "list",
            "path": path,
            "child": rust_field_model(item_field, path, structs),
            "item_size": rust_field_byte_size(item_field, structs),
            "raw_field": field,
        }

    if count is not None and "int" not in conversion:
        item_field = {k: v for k, v in field.items() if k != "num"}
        return {
            "kind": "fixed_list",
            "path": path,
            "child": rust_field_model(item_field, path, structs),
            "num": count,
            "raw_field": field,
        }

    return {
        "kind": "primitive",
        "path": path,
        "rust_type": rust_type_after_conversion(field),
        "read": rust_primitive_read_expr(field),
        "raw_field": field,
    }


def rust_value_expr(model: dict, indent: int = 16) -> str:
    kind = model["kind"]
    pad = " " * indent
    child_pad = " " * (indent + 4)

    if kind == "primitive":
        read = model["read"]
        rust_type_name = model["rust_type"]
        if rust_type_name.startswith("u"):
            if rust_type_name == "u64":
                return f"TdfValue::UInt({read})"
            return f"TdfValue::UInt({read} as u64)"
        if rust_type_name.startswith("i"):
            if rust_type_name == "i64":
                return f"TdfValue::Int({read})"
            return f"TdfValue::Int({read} as i64)"
        if rust_type_name == "f64":
            return f"TdfValue::Float({read})"
        return f"TdfValue::Float({read} as f64)"

    if kind == "string":
        return f"TdfValue::String({model['read']})"

    if kind == "hex":
        return f"TdfValue::String({model['read']})"

    if kind == "binary":
        return "TdfValue::Bytes(tdf_field_read_vla(&mut cursor, cursor_start, size)?)"

    if kind == "fixed_list":
        child = rust_value_expr(model["child"], indent + 8)
        return (
            "{\n"
            f"{child_pad}let mut values = Vec::with_capacity({model['num']});\n"
            f"{child_pad}for _ in 0..{model['num']} {{\n"
            f"{child_pad}    values.push({child});\n"
            f"{child_pad}}}\n"
            f"{child_pad}TdfValue::List(values)\n"
            f"{pad}}}"
        )

    if kind == "list":
        child = rust_value_expr(model["child"], indent + 12)
        return (
            "{\n"
            f"{child_pad}let bytes_remaining = vla_bytes_remaining(&mut cursor, cursor_start, size)?;\n"
            f"{child_pad}if bytes_remaining % {model['item_size']} != 0 {{\n"
            f"{child_pad}    return Err(TdfDecodeError::InvalidData(\"Variable-length array does not align to element size\"));\n"
            f"{child_pad}}}\n"
            f"{child_pad}let item_count = bytes_remaining / {model['item_size']};\n"
            f"{child_pad}let mut values = Vec::with_capacity(item_count);\n"
            f"{child_pad}for _ in 0..item_count {{\n"
            f"{child_pad}    values.push({child});\n"
            f"{child_pad}}}\n"
            f"{child_pad}TdfValue::List(values)\n"
            f"{pad}}}"
        )

    if kind == "struct":
        fields = []
        for child in model["children"]:
            child_name = rust_str(lower_camel(child["raw_field"]["name"]))
            child_expr = rust_value_expr(child, indent + 12)
            fields.append(
                "TdfField {\n"
                f"{' ' * (indent + 12)}name: {child_name},\n"
                f"{' ' * (indent + 12)}value: {child_expr},\n"
                f"{' ' * (indent + 8)}}}"
            )
        joined = (",\n" + " " * (indent + 8)).join(fields)
        return (
            "TdfValue::Struct(vec![\n"
            f"{' ' * (indent + 8)}{joined}\n"
            f"{pad}])"
        )

    raise ValueError(f"Bad model kind {kind}")


def main() -> None:
    args = parse_args()

    cs_env = Environment(
        loader=FileSystemLoader(TEMPLATE_DIR),
        autoescape=select_autoescape(enabled_extensions=("html", "xml"), default_for_string=False),
        keep_trailing_newline=True,
        trim_blocks=True,
        lstrip_blocks=True,
    )
    cs_env.filters["xml_escape"] = escape

    rust_env = Environment(
        loader=FileSystemLoader(TEMPLATE_DIR),
        autoescape=select_autoescape(),
        keep_trailing_newline=True,
        trim_blocks=True,
        lstrip_blocks=True,
    )

    generate_csharp = args.language in ("all", "csharp")
    generate_rust = args.language in ("all", "rust")

    data = load_merged_definitions(args.extension)

    struct_sizes: dict[str, int] = {}
    for name, definition in data.get("structs", {}).items():
        size = 0
        for field in definition["fields"]:
            current_size = field_size(field, struct_sizes)
            if current_size is None:
                raise ValueError(f"Struct {name} cannot contain a variable-length field")
            size += current_size
        struct_sizes[name] = size

    structs = []
    for name, definition in sorted(data.get("structs", {}).items()):
        structs.append(
            {
                "schema_name": name,
                "class_name": class_name(name),
                "description": definition.get("description"),
                "size": struct_sizes[name],
                "fields": prepare_fields(definition["fields"], struct_sizes),
            }
        )

    definitions = sorted(data.get("definitions", {}).items(), key=lambda item: int(item[0]))
    readings = []
    for tdf_id, definition in definitions:
        readings.append(
            {
                "id": int(tdf_id),
                "schema_name": definition["name"],
                "class_name": class_name(definition["name"]),
                "description": definition.get("description"),
                "fields": prepare_fields(definition["fields"], struct_sizes),
            }
        )

    if generate_csharp:
        output_path = args.csharp_output or (CS_OUTPUT_DIR / "TdfDecoders.cs")
        output_path.parent.mkdir(parents=True, exist_ok=True)

        template = cs_env.get_template("decoder.cs.jinja2")
        output_path.write_text(
            template.render(
                definitions=definitions,
                has_hex_conversion=has_hex_conversion(data),
                namespace=args.csharp_namespace,
                readings=readings,
                structs=structs,
            ),
            encoding="utf-8",
        )
        print(f"Wrote {display_path(output_path)}")

    if not generate_rust:
        return

    rust_data = load_merged_definitions(args.extension, parse_float=decimal.Decimal)

    for _tdf_id, definition in rust_data["definitions"].items():
        definition["rust_payload_type_name"] = pascal_case(definition["name"])
        decode_fields = []
        for field in definition["fields"]:
            model = rust_field_model(field, [rust_ident(field["name"])], rust_data["structs"])
            field_name = rust_str(lower_camel(model["raw_field"]["name"]))
            value = rust_value_expr(model)
            decode_fields.append(
                "TdfField {\n"
                f"                    name: {field_name},\n"
                f"                    value: {value},\n"
                "                }"
            )
        definition["rust_decode_fields"] = ",\n                ".join(decode_fields)

    rust_output_path = args.rust_output or (RUST_OUTPUT_DIR / "decoders.rs")
    rust_output_path.parent.mkdir(parents=True, exist_ok=True)
    rust_template = rust_env.get_template("decoder.rs.jinja")
    rust_output_path.write_text(
        rust_template.render(
            definitions=rust_data["definitions"],
            has_hex_conversion=has_hex_conversion(rust_data),
            structs=rust_data["structs"],
        ),
        encoding="utf-8",
    )
    subprocess.run(["rustfmt", rust_output_path], check=True)
    print(f"Wrote {display_path(rust_output_path)}")


if __name__ == "__main__":
    main()
