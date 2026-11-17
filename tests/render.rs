//! The value of a line of the file `wb init` writes.

// A test is named after the requirement it covers, and the ID has capitals.
#![allow(non_snake_case)]

use watchbill::init::yaml_value;

#[test]
fn test_WB_INIT_22_a_value_is_written_plain_only_when_yaml_reads_it_back_as_the_same_text() {
    for value in [
        "0x1f", "0b101", "0o17", "017", "1_000", "1e3", "1.5", "2024-01-01", "2024-01-01 10:00:00", "true", "No",
        "off", "NULL", "y", "n", ".inf", "-1", "+1", "12:30", "1:20:30",
    ] {
        assert_eq!(yaml_value(value), format!("'{}'", value), "{}", value);
    }
    for value in ["brakes", "requirements/*.md", "brake-controller", "a.b_c", "x0", "0.0.1", "1.2.3-rc1"] {
        assert_eq!(yaml_value(value), value, "{}", value);
    }
}
