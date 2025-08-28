pub fn split_command(command: &str) -> Vec<String> {
    command.split_whitespace().map(String::from).collect()
}

pub fn parse_command_columns(command: Vec<String>) -> Vec<String> {
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
