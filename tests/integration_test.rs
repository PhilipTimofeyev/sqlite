use std::process::Command;

#[test]
fn test_db_info() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlite"))
        .args(["tests/fixtures/sample.db", ".dbinfo"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_eq!(stdout, "number of tables: 3\ndatabase page size: 4096\n");
}

#[test]
fn test_tables() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlite"))
        .args(["tests/fixtures/sample.db", ".tables"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_eq!(stdout, "apples\nsqlite_sequence\noranges\n");
}

#[test]
fn test_row_count() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlite"))
        .args(["tests/fixtures/sample.db", "SELECT COUNT(*) FROM apples"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_eq!(stdout, "4\n");
}

#[test]
fn test_reading_single_column() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlite"))
        .args(["tests/fixtures/sample.db", "SELECT name FROM apples"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_eq!(stdout, "Granny Smith\nFuji\nHoneycrisp\nGolden Delicious\n");
}

#[test]
fn test_reading_multiple_columns() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlite"))
        .args(["tests/fixtures/sample.db", "SELECT name, color FROM apples"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_eq!(
        stdout,
        "Granny Smith|Light Green\nFuji|Red\nHoneycrisp|Blush Red\nGolden Delicious|Yellow\n"
    );
}

#[test]
fn test_filter_with_where() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlite"))
        .args([
            "tests/fixtures/sample.db",
            "SELECT name, color FROM apples WHERE color = 'Yellow'",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_eq!(stdout, "Golden Delicious|Yellow\n");
}

#[test]
fn test_get_data_full_table_scan() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlite"))
        .args([
            "tests/fixtures/superheroes.db",
            "SELECT id, name FROM superheroes WHERE eye_color = 'Pink Eyes'",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_eq!(
        stdout,
        "297|Stealth (New Earth)\n790|Tobias Whale (New Earth)\n1085|Felicity (New Earth)\n2729|Thrust (New Earth)\n3289|Angora Lapin (New Earth)\n3913|Matris Ater Clementia (New Earth)\n"
    );
}

#[test]
fn test_get_data_using_index() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlite"))
        .args([
            "tests/fixtures/companies.db",
            "SELECT id, name FROM companies WHERE country = 'eritrea'",
        ])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_eq!(
        stdout,
        "6634629|asmara rental\n2102438|orange asmara it solutions\n121311|unilink s.c.\n5729848|zara mining share company\n"
    );
}
