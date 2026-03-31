use crate::cli::OutputFormat;
use comfy_table::presets::UTF8_FULL;
use comfy_table::{ContentArrangement, Table};
use serde::Serialize;

const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

pub struct OutputWriter {
    format: OutputFormat,
}

impl OutputWriter {
    pub fn new(format: OutputFormat) -> Self {
        Self { format }
    }

    #[allow(dead_code)]
    pub fn format(&self) -> OutputFormat {
        self.format
    }

    pub fn write_table(&self, headers: &[&str], rows: &[Vec<String>]) {
        match self.format {
            OutputFormat::Human => self.print_table_human(headers, rows),
            OutputFormat::Json | OutputFormat::JsonCompact | OutputFormat::Ndjson => {
                let table = table_to_json(headers, rows);
                self.write_json_value(&table);
            }
        }
    }

    pub fn write_json<T: Serialize>(&self, data: &T) {
        match self.format {
            OutputFormat::Human => match serde_json::to_string_pretty(data) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("serialization error: {e}"),
            },
            OutputFormat::Json => match serde_json::to_string_pretty(&wrap_success(data)) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("serialization error: {e}"),
            },
            OutputFormat::JsonCompact => match serde_json::to_string(&wrap_success(data)) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("serialization error: {e}"),
            },
            OutputFormat::Ndjson => match serde_json::to_string(&wrap_success(data)) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("serialization error: {e}"),
            },
        }
    }

    #[allow(dead_code)]
    pub fn write_list<T: Serialize>(&self, items: &[T]) {
        match self.format {
            OutputFormat::Human => {
                if items.is_empty() {
                    println!("(no items)");
                    return;
                }
                match serde_json::to_string_pretty(&items) {
                    Ok(json) => println!("{json}"),
                    Err(e) => eprintln!("serialization error: {e}"),
                }
            }
            OutputFormat::Json => match serde_json::to_string_pretty(&wrap_success(&items)) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("serialization error: {e}"),
            },
            OutputFormat::JsonCompact | OutputFormat::Ndjson => {
                match serde_json::to_string(&wrap_success(&items)) {
                    Ok(json) => println!("{json}"),
                    Err(e) => eprintln!("serialization error: {e}"),
                }
            }
        }
    }

    pub fn write_status(&self, message: &str) {
        match self.format {
            OutputFormat::Human => println!("{GREEN}{BOLD}✓{RESET} {message}"),
            OutputFormat::Json | OutputFormat::JsonCompact | OutputFormat::Ndjson => {}
        }
    }

    pub fn write_error(&self, message: &str) {
        match self.format {
            OutputFormat::Human => eprintln!("{RED}{BOLD}✗{RESET} {message}"),
            OutputFormat::Json | OutputFormat::JsonCompact => {
                let err = serde_json::json!({"error": message, "success": false});
                match serde_json::to_string_pretty(&err) {
                    Ok(json) => eprintln!("{json}"),
                    Err(_) => {
                        eprintln!("{{\"error\": \"{}\", \"success\": false}}", escape_json(message))
                    }
                }
            }
            OutputFormat::Ndjson => {
                let err = serde_json::json!({"error": message, "success": false});
                match serde_json::to_string(&err) {
                    Ok(json) => eprintln!("{json}"),
                    Err(_) => {
                        eprintln!("{{\"error\": \"{}\", \"success\": false}}", escape_json(message))
                    }
                }
            }
        }
    }

    pub fn write_value(&self, key: &str, value: &str) {
        match self.format {
            OutputFormat::Human => {
                let padded = format!("{BOLD}{key:<20}{RESET}");
                println!("{padded} {value}");
            }
            OutputFormat::Json | OutputFormat::JsonCompact | OutputFormat::Ndjson => {
                let obj = serde_json::json!({"data": {key: value}, "success": true});
                self.write_json_value(&obj);
            }
        }
    }

    pub fn write_key_value_pairs(&self, pairs: &[(&str, &str)]) {
        match self.format {
            OutputFormat::Human => {
                for (key, value) in pairs {
                    println!("{BOLD}{key:<24}{RESET} {value}");
                }
            }
            OutputFormat::Json | OutputFormat::JsonCompact | OutputFormat::Ndjson => {
                let map: serde_json::Map<String, serde_json::Value> = pairs
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), serde_json::Value::String((*v).to_string())))
                    .collect();
                let obj = serde_json::json!({"data": map, "success": true});
                self.write_json_value(&obj);
            }
        }
    }

    pub fn write_warning(&self, message: &str) {
        match self.format {
            OutputFormat::Human => println!("{YELLOW}{BOLD}⚠{RESET} {message}"),
            OutputFormat::Json | OutputFormat::JsonCompact | OutputFormat::Ndjson => {}
        }
    }

    fn print_table_human(&self, headers: &[&str], rows: &[Vec<String>]) {
        if headers.is_empty() {
            return;
        }

        let mut table = Table::new();
        table.load_preset(UTF8_FULL).set_content_arrangement(ContentArrangement::Dynamic);

        let header_cells: Vec<comfy_table::Cell> =
            headers.iter().map(|h| comfy_table::Cell::new(*h)).collect();
        table.set_header(header_cells);

        for row in rows {
            let cells: Vec<comfy_table::Cell> =
                row.iter().map(|cell| comfy_table::Cell::new(cell)).collect();
            table.add_row(cells);
        }

        println!("{table}");
    }

    fn write_json_value(&self, value: &serde_json::Value) {
        match self.format {
            OutputFormat::Json => match serde_json::to_string_pretty(value) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("serialization error: {e}"),
            },
            OutputFormat::JsonCompact | OutputFormat::Ndjson => {
                match serde_json::to_string(value) {
                    Ok(json) => println!("{json}"),
                    Err(e) => eprintln!("serialization error: {e}"),
                }
            }
            OutputFormat::Human => match serde_json::to_string_pretty(value) {
                Ok(json) => println!("{json}"),
                Err(e) => eprintln!("serialization error: {e}"),
            },
        }
    }
}

fn wrap_success<T: Serialize>(data: &T) -> serde_json::Value {
    let data_val = serde_json::to_value(data).unwrap_or(serde_json::Value::Null);
    serde_json::json!({"data": data_val, "success": true})
}

fn table_to_json(headers: &[&str], rows: &[Vec<String>]) -> serde_json::Value {
    let objects: Vec<serde_json::Map<String, serde_json::Value>> = rows
        .iter()
        .map(|row| {
            let mut map = serde_json::Map::new();
            for (i, header) in headers.iter().enumerate() {
                let value = row.get(i).cloned().unwrap_or_default();
                map.insert((*header).to_string(), serde_json::Value::String(value));
            }
            map
        })
        .collect();
    serde_json::json!({"data": objects, "success": true})
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn writer(format: OutputFormat) -> OutputWriter {
        OutputWriter::new(format)
    }

    #[test]
    fn format_returns_correct_format() {
        assert!(matches!(writer(OutputFormat::Human).format(), OutputFormat::Human));
        assert!(matches!(writer(OutputFormat::Json).format(), OutputFormat::Json));
    }

    #[test]
    fn table_to_json_empty() {
        let result = table_to_json(&[], &[]);
        assert_eq!(result["data"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn table_to_json_with_data() {
        let headers = ["name", "value"];
        let rows = vec![vec!["foo".to_string(), "bar".to_string()]];
        let result = table_to_json(&headers, &rows);
        let arr = result["data"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "foo");
        assert_eq!(arr[0]["value"], "bar");
    }

    #[test]
    fn wrap_success_wraps_data() {
        let data = vec!["a", "b"];
        let wrapped = wrap_success(&data);
        assert_eq!(wrapped["success"], true);
        let arr = wrapped["data"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
    }

    #[test]
    fn escape_json_handles_special_chars() {
        assert_eq!(escape_json("hello \"world\""), "hello \\\"world\\\"");
        assert_eq!(escape_json("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_json("back\\slash"), "back\\\\slash");
    }

    #[test]
    fn table_to_json_uneven_columns() {
        let headers = ["a", "b", "c"];
        let rows = vec![vec!["1".to_string()]];
        let result = table_to_json(&headers, &rows);
        let obj = &result["data"].as_array().unwrap()[0];
        assert_eq!(obj["a"], "1");
        assert_eq!(obj["b"], "");
        assert_eq!(obj["c"], "");
    }

    #[test]
    fn write_status_human_does_not_panic() {
        writer(OutputFormat::Human).write_status("test message");
    }

    #[test]
    fn write_error_human_does_not_panic() {
        writer(OutputFormat::Human).write_error("test error");
    }

    #[test]
    fn write_error_json_does_not_panic() {
        writer(OutputFormat::Json).write_error("test error");
    }

    #[test]
    fn write_value_human_does_not_panic() {
        writer(OutputFormat::Human).write_value("key", "value");
    }

    #[test]
    fn write_value_json_does_not_panic() {
        writer(OutputFormat::Json).write_value("key", "value");
    }

    #[test]
    fn write_table_human_does_not_panic() {
        let headers = ["Name", "Value"];
        let rows = vec![vec!["test".to_string(), "123".to_string()]];
        writer(OutputFormat::Human).write_table(&headers, &rows);
    }

    #[test]
    fn write_table_json_does_not_panic() {
        let headers = ["Name", "Value"];
        let rows = vec![vec!["test".to_string(), "123".to_string()]];
        writer(OutputFormat::Json).write_table(&headers, &rows);
    }

    #[test]
    fn write_json_human_does_not_panic() {
        let data = serde_json::json!({"test": "value"});
        writer(OutputFormat::Human).write_json(&data);
    }

    #[test]
    fn write_json_compact_does_not_panic() {
        let data = serde_json::json!({"test": "value"});
        writer(OutputFormat::JsonCompact).write_json(&data);
    }

    #[test]
    fn write_list_empty_does_not_panic() {
        let items: Vec<String> = vec![];
        writer(OutputFormat::Human).write_list(&items);
    }

    #[test]
    fn write_list_with_data_does_not_panic() {
        let items = vec!["a".to_string(), "b".to_string()];
        writer(OutputFormat::Json).write_list(&items);
    }

    #[test]
    fn write_key_value_pairs_human_does_not_panic() {
        let pairs = [("name", "test"), ("version", "0.1.0")];
        writer(OutputFormat::Human).write_key_value_pairs(&pairs);
    }

    #[test]
    fn write_key_value_pairs_json_does_not_panic() {
        let pairs = [("name", "test"), ("version", "0.1.0")];
        writer(OutputFormat::Json).write_key_value_pairs(&pairs);
    }
}
