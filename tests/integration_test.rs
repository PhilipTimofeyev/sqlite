use std::process::Command;

#[test]
fn test_db_info() {
    let output = Command::new(env!("CARGO_BIN_EXE_sqlite"))
        .args(["tests/fixtures/sample.db", ".dbinfo"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    assert_eq!(stdout, "number of tables: 3\ndatabase page size: 4096");
}
