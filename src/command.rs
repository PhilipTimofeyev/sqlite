use anyhow::{bail, Result};

pub fn split_command(command: &str) -> Vec<String> {
    command.split_whitespace().map(String::from).collect()
}

pub fn parse_command_columns(command: &[String]) -> Vec<String> {
    let from_idx = command
        .iter()
        .position(|word| word.to_lowercase() == "from")
        .unwrap();

    let columns = &command[1..from_idx];

    columns
        .iter()
        .map(|column| keep_ascii_alphabet_chars(column.to_string()))
        .collect()
}

fn keep_ascii_alphabet_chars(word: String) -> String {
    word.chars()
        .filter(|char| char.is_ascii_alphabetic())
        .collect()
}

pub fn parse_command_table_name(command: &[String]) -> Result<String, anyhow::Error> {
    let from_idx = command
        .iter()
        .position(|word| word.to_lowercase() == "from")
        .expect("FROM keyword is missing");

    let table = &command[from_idx + 1..from_idx + 2];

    if let Some(table_name) = table.first() {
        Ok(table_name.to_string())
    } else {
        bail!("Table name not found")
    }
}

pub fn parse_command_where(command: &[String]) -> Result<(String, String), anyhow::Error> {
    let where_idx = command
        .iter()
        .position(|word| word.to_lowercase() == "where")
        .unwrap();

    let where_cmd = &command[where_idx + 1..];

    let column = where_cmd
        .first()
        .ok_or_else(|| anyhow::anyhow!("Column not found"))?
        .to_string();
    let column_value = where_cmd
        .last()
        .ok_or_else(|| anyhow::anyhow!("Column value not found"))?;

    let column_value = column_value.replace("'", "");

    Ok((column, column_value))
}
