use std::fs::File;
use std::io::{Read, Write};
use std::env;

fn main() {
    let current_dir = env::current_dir().expect("Failed to get current directory");
    println!("Current directory: {:?}", current_dir);
    let file_path = current_dir.join("items");
    let mut file = File::open(file_path).expect("Failed to open file");
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Failed to read file");
    let lines: Vec<String> = contents.lines().map(|line| line.trim_start_matches("// ").to_string()).collect();
    let capitalized_lines: Vec<String> = lines.iter().map(|line| line.to_uppercase()).collect();
    let modified_lines: Vec<String> = capitalized_lines
        .iter()
        .map(|line| {
            if line.starts_with("T1 ") || line.starts_with("T2 ") || line.starts_with("T3 ") || line.starts_with("T4 ") {
                line[3..].to_string()
            } else {
                line.to_string()
            }
        })
        .collect();

    let output_file_path = current_dir.join("modified_items");
    let mut output_file = File::create(output_file_path).expect("Failed to create output file");
    for line in modified_lines {
        output_file.write_all(line.as_bytes()).expect("Failed to write to output file");
        output_file.write_all(b"\n").expect("Failed to write newline to output file");
    }
}