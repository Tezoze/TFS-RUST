use std::fs;
use std::path::PathBuf;

#[test]
fn test_no_sql_string_concatenation() {
    let mut search_paths = vec![PathBuf::from("src")];
    let mut files_checked = 0;

    while let Some(path) = search_paths.pop() {
        if path.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                let entry = entry.unwrap();
                search_paths.push(entry.path());
            }
        } else if path.extension().unwrap_or_default() == "rs" {
            let content = fs::read_to_string(&path).unwrap();
            files_checked += 1;

            for (i, line) in content.lines().enumerate() {
                if line.contains("sqlx::query") {
                    if line.contains("format!") {
                        panic!(
                            "SQL Injection vulnerability found in {:?} at line {}: \n`{}`\nDo not use format! inside sqlx::query(). Use .bind() instead.",
                            path, i + 1, line
                        );
                    }

                    if line.contains(" + ") && (line.contains("\"") || line.contains("'")) {
                        panic!(
                            "SQL Injection vulnerability found in {:?} at line {}: \n`{}`\nDo not concatenate strings for SQL queries. Use .bind() instead.",
                            path, i + 1, line
                        );
                    }
                }
            }
        }
    }

    assert!(
        files_checked > 0,
        "No rust files were checked for SQL safety!"
    );
}
