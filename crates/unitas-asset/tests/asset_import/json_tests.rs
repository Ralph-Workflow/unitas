use unitas_asset::json::{parse, parse_bytes, write, JsonError, JsonValue};

#[test]
fn round_trip_preserves_the_value() -> Result<(), JsonError> {
    let text = r#"{"name":"Cube","count":2,"tags":["a","b"],"nested":{"x":1.5,"y":null},"ok":true}"#;
    let value = parse(text)?;
    let reparsed = parse(&write(&value))?;
    assert_eq!(reparsed, value);
    Ok(())
}

#[test]
fn escapes_are_decoded() -> Result<(), JsonError> {
    let value = parse(r#""\u0041\n\t\"\\\ud83d\ude00""#)?;
    assert_eq!(value.as_str(), Some("A\n\t\"\\\u{1f600}"));
    Ok(())
}

#[test]
fn numbers_parse_to_the_expected_values() -> Result<(), JsonError> {
    assert_eq!(parse("0")?.as_f64(), Some(0.0));
    assert_eq!(parse("-0")?.as_f64(), Some(-0.0));
    assert_eq!(parse("1.5")?.as_f64(), Some(1.5));
    assert_eq!(parse("1e3")?.as_f64(), Some(1000.0));
    assert_eq!(parse("-2.25e-3")?.as_f64(), Some(-0.00225));
    assert_eq!(parse("1.5")?.as_i64(), None);
    assert_eq!(parse("2")?.as_i64(), Some(2));
    assert_eq!(parse("-2")?.as_i64(), Some(-2));
    Ok(())
}

#[test]
fn object_key_order_survives_the_writer() -> Result<(), JsonError> {
    let value = parse(r#"{"z":1,"a":2,"m":3}"#)?;
    let entries = value.as_object().ok_or(JsonError::UnexpectedEnd)?;
    let keys: Vec<&str> = entries.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(keys, vec!["z", "a", "m"]);
    let written = write(&value);
    let z = written.find("\"z\"").ok_or(JsonError::UnexpectedEnd)?;
    let a = written.find("\"a\"").ok_or(JsonError::UnexpectedEnd)?;
    let m = written.find("\"m\"").ok_or(JsonError::UnexpectedEnd)?;
    assert!(z < a && a < m);
    Ok(())
}

#[test]
fn writer_output_matches_the_expected_text() -> Result<(), JsonError> {
    let value = parse(r#"{"name":"Cube","count":2,"tags":["a","b"],"nested":{"x":1.5,"y":null}}"#)?;
    let expected = concat!(
        "{\n",
        "  \"name\": \"Cube\",\n",
        "  \"count\": 2,\n",
        "  \"tags\": [\n",
        "    \"a\",\n",
        "    \"b\"\n",
        "  ],\n",
        "  \"nested\": {\n",
        "    \"x\": 1.5,\n",
        "    \"y\": null\n",
        "  }\n",
        "}\n"
    );
    assert_eq!(write(&value), expected);
    assert_eq!(value.to_json_string(), expected);
    Ok(())
}

#[test]
fn empty_containers_and_numbers_have_a_stable_spelling() -> Result<(), JsonError> {
    assert_eq!(write(&parse("{}")?), "{}\n");
    assert_eq!(write(&parse("[]")?), "[]\n");
    assert_eq!(write(&JsonValue::Number(2.0)), "2\n");
    assert_eq!(write(&JsonValue::Number(1.5)), "1.5\n");
    assert_eq!(write(&JsonValue::Number(-0.0)), "-0\n");
    assert_eq!(write(&JsonValue::Null), "null\n");
    assert_eq!(write(&JsonValue::Bool(true)), "true\n");
    Ok(())
}

#[test]
fn malformed_inputs_return_typed_errors() {
    assert!(matches!(parse("\"unterminated"), Err(JsonError::UnexpectedEnd)));
    assert!(matches!(parse("[1,2,]"), Err(JsonError::UnexpectedChar(']'))));
    assert!(matches!(parse(r#"{"a":1,}"#), Err(JsonError::UnexpectedChar('}'))));
    assert!(matches!(parse(r#""\q""#), Err(JsonError::InvalidEscape)));
    assert!(matches!(parse("bare"), Err(JsonError::UnexpectedChar('b'))));
    assert!(matches!(parse("{} {}"), Err(JsonError::TrailingData)));
    assert!(matches!(parse("1.2.3"), Err(JsonError::InvalidNumber)));
    assert!(matches!(parse("01"), Err(JsonError::InvalidNumber)));
    let deep = format!("{}1{}", "[".repeat(200), "]".repeat(200));
    assert!(matches!(parse(&deep), Err(JsonError::DepthExceeded)));
}

#[test]
fn unicode_escapes_and_raw_text_round_trip() -> Result<(), JsonError> {
    let text = "\"caf\u{e9} \u{4e2d}\u{6587} \\ud83d\\ude00\"";
    let value = parse(text)?;
    assert_eq!(value.as_str(), Some("caf\u{e9} \u{4e2d}\u{6587} \u{1f600}"));
    let reparsed = parse(&write(&value))?;
    assert_eq!(reparsed, value);
    Ok(())
}

#[test]
fn bytes_are_parsed_and_invalid_utf8_is_rejected() -> Result<(), JsonError> {
    let value = parse_bytes(br#"{"a":1}"#)?;
    assert_eq!(value.get("a").and_then(JsonValue::as_i64), Some(1));
    assert!(matches!(
        parse_bytes(b"\"\xff\xfe\""),
        Err(JsonError::InvalidUtf8)
    ));
    Ok(())
}

#[test]
fn accessors_expose_parsed_containers() -> Result<(), JsonError> {
    let value = parse(r#"{"a":{"b":[1,true,"x"]}}"#)?;
    let inner = value.get("a").ok_or(JsonError::UnexpectedEnd)?;
    let items = inner
        .get("b")
        .ok_or(JsonError::UnexpectedEnd)?
        .as_array()
        .ok_or(JsonError::UnexpectedEnd)?;
    assert_eq!(items.len(), 3);
    assert_eq!(items[0].as_i64(), Some(1));
    assert_eq!(items[1].as_bool(), Some(true));
    assert_eq!(items[2].as_str(), Some("x"));
    assert!(value.get("missing").is_none());
    assert!(value.as_array().is_none());
    assert!(value.as_object().is_some());
    Ok(())
}
