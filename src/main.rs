use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::Path;
use std::process;

#[derive(Debug, Clone)]
struct FileSummary {
    file_type: FileType,
    line_count: usize,
    word_count: usize,
    char_count: usize,
    key_insights: Vec<String>,
    statistics: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
enum FileType {
    Text, Markdown, Log, RustCode, Unknown
}

impl FileType {
    fn from_extension(path: &Path) -> Self {
        match path.extension().and_then(|s| s.to_str()) {
            Some("txt") => FileType::Text,
            Some("md") => FileType::Markdown,
            Some("log") => FileType::Log,
            Some("rs") => FileType::RustCode,
            _ => FileType::Unknown
        }   
    }
}

struct TextAnalyzer {
    content: String,
    lines: Vec<String>,
}

impl TextAnalyzer {
    fn new(content: String) -> Self {
        let lines = content.lines().map(|s| s.to_string()).collect();
        Self { content, lines }
    }

    fn basic_stats(&self) -> (usize, usize, usize) {
        let line_count = self.lines.len();
        let word_count = self.content.split_whitespace().count();
        let char_count = self.content.chars().count();
        (line_count, word_count, char_count)
    }

    fn analyze_text(&self) -> FileSummary {
        let (line_count, word_count, char_count) = self.basic_stats();
        let mut insights = Vec::new();
        let mut statistics = HashMap::new();

        let word_freq = self.get_word_frequency();
        let top_words: Vec<_> = word_freq.iter().filter(|(word, _)| word.len() > 3) .take(5).map(|(word, count)| format!("{} ({})", word, count)).collect();
        if !top_words.is_empty() {
            insights.push(format!("Most frequent words: {}", top_words.join(", ")));
        }

        let avg_word_len = if word_count > 0 {
            self.content.split_whitespace().map(|w| w.chars().count()).sum::<usize>() as f64 / word_count as f64
        } else {
            0.0
        };

        statistics.insert("avg_word_length".to_string(), format!("{:.1}", avg_word_len));
        statistics.insert("avg_line_length".to_string(), format!("{:.1}", if line_count > 0 { char_count as f64 / line_count as f64 } else { 0.0 }));

        FileSummary {
            file_type: FileType::Text,
            line_count,
            word_count,
            char_count,
            key_insights: insights,
            statistics,
        }
    }

    fn analyze_markdown(&self) -> FileSummary {
        let (line_count, word_count, char_count) = self.basic_stats();
        let mut insights = Vec::new();
        let mut statistics = HashMap::new();

        let mut headers = Vec::new();
        let mut links = 0;
        let mut images = 0;
        let mut code_blocks = 0;

        for line in &self.lines {
            let trimmed = line.trim();
            
            if trimmed.starts_with('#') {
                let level = trimmed.chars().take_while(|&c| c == '#').count();
                headers.push((level, trimmed.trim_start_matches('#').trim().to_string()));
            }
            
            links += line.matches("](").count();
            images += line.matches("![").count();
            if trimmed.starts_with("```") {
                code_blocks += 1;
            }
        }

        if !headers.is_empty() {
            let header_summary: Vec<_> = headers.iter().take(5).map(|(level, text)| format!("H{}: {}", level, text)).collect();
            insights.push(format!("Document structure: {}", header_summary.join(", ")));
        }

        statistics.insert("headers".to_string(), headers.len().to_string());
        statistics.insert("links".to_string(), links.to_string());
        statistics.insert("images".to_string(), images.to_string());
        statistics.insert("code_blocks".to_string(), (code_blocks / 2).to_string());

        FileSummary {
            file_type: FileType::Markdown,
            line_count,
            word_count,
            char_count,
            key_insights: insights,
            statistics,
        }
    }

    fn analyze_log(&self) -> FileSummary {
        let (line_count, word_count, char_count) = self.basic_stats();
        let mut insights = Vec::new();
        let mut statistics = HashMap::new();

        let mut log_levels = HashMap::new();
        let mut timestamps = Vec::new();
        let mut errors = Vec::new();

        for line in &self.lines {
            let upper_line = line.to_uppercase();
            for level in &["ERROR", "WARN", "INFO", "DEBUG", "TRACE"] {
                if upper_line.contains(level) {
                    *log_levels.entry(level.to_string()).or_insert(0) += 1;
                }
            }

            if upper_line.contains("ERROR") || upper_line.contains("EXCEPTION") || upper_line.contains("FAIL") {
                errors.push(line.clone());
            }

            if line.len() > 10 && (line.contains(":") || line.contains("-") || line.contains("/")) {
                let parts: Vec<_> = line.split_whitespace().collect();
                if !parts.is_empty() && (parts[0].contains(":") || parts[0].contains("-")) {
                    timestamps.push(parts[0].to_string());
                }
            }
        }

        if !log_levels.is_empty() {
            let level_summary: Vec<_> = log_levels.iter().map(|(level, count)| format!("{}: {}", level, count)).collect();
            insights.push(format!("Log levels: {}", level_summary.join(", ")));
        }

        if timestamps.len() > 1 {
            insights.push(format!("Time range: {} to {}", timestamps.first().unwrap_or(&"N/A".to_string()), timestamps.last().unwrap_or(&"N/A".to_string())));
        }

        if !errors.is_empty() {
            let error_sample = if errors.len() > 3 { &errors[0..3] } else { &errors };
            insights.push(format!("Sample errors found: {} total", errors.len()));
            for (i, error) in error_sample.iter().enumerate() {
                if error.len() > 100 {
                    insights.push(format!("  {}: {}...", i + 1, &error[0..100]));
                } else {
                    insights.push(format!("  {}: {}", i + 1, error));
                }
            }
        }

        statistics.insert("unique_timestamps".to_string(), 
            timestamps.into_iter().collect::<HashSet<_>>().len().to_string());

        FileSummary {
            file_type: FileType::Log,
            line_count,
            word_count,
            char_count,
            key_insights: insights,
            statistics,
        }
    }

    fn analyze_rust_code(&self) -> FileSummary {
        let (line_count, word_count, char_count) = self.basic_stats();
        let mut insights = Vec::new();
        let mut statistics = HashMap::new();

        let mut functions = Vec::new();
        let mut structs = Vec::new();
        let mut enums = Vec::new();
        let mut imports = Vec::new();
        let mut comments = 0;
        let mut todo_count = 0;

        for line in &self.lines {
            let trimmed = line.trim();
            if trimmed.starts_with("fn ") || trimmed.contains(" fn ") {
                if let Some(name_start) = trimmed.find("fn ") {
                    let name_part = &trimmed[name_start + 3..];
                    if let Some(paren_pos) = name_part.find('(') {
                        functions.push(name_part[..paren_pos].trim().to_string());
                    }
                }
            }
            
            if trimmed.starts_with("struct ") {
                if let Some(name) = trimmed.split_whitespace().nth(1) {
                    structs.push(name.to_string());
                }
            }
            if trimmed.starts_with("enum ") {
                if let Some(name) = trimmed.split_whitespace().nth(1) {
                    enums.push(name.to_string());
                }
            }
            
            if trimmed.starts_with("use ") {
                imports.push(trimmed.to_string());
            }
            
            if trimmed.starts_with("//") || trimmed.starts_with("/*") {
                comments += 1;
            }
            if trimmed.to_uppercase().contains("TODO") || trimmed.to_uppercase().contains("FIXME") {
                todo_count += 1;
            }
        }

        if !functions.is_empty() {
            let func_sample: Vec<_> = functions.iter().take(5).collect();
            insights.push(format!("Functions ({}): {}", functions.len(), func_sample.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
        }

        if !structs.is_empty() {
            insights.push(format!("Structs: {}", structs.join(", ")));
        }

        if !enums.is_empty() {
            insights.push(format!("Enums: {}", enums.join(", ")));
        }

        if todo_count > 0 {
            insights.push(format!("TODOs/FIXMEs found: {}", todo_count));
        }

        let code_lines = line_count - comments;
        let comment_ratio = if line_count > 0 { 
            comments as f64 / line_count as f64 * 100.0 
        } else { 0.0 };

        statistics.insert("functions".to_string(), functions.len().to_string());
        statistics.insert("structs".to_string(), structs.len().to_string());
        statistics.insert("enums".to_string(), enums.len().to_string());
        statistics.insert("imports".to_string(), imports.len().to_string());
        statistics.insert("comment_ratio".to_string(), format!("{:.1}%", comment_ratio));

        FileSummary {
            file_type: FileType::RustCode,
            line_count,
            word_count,
            char_count,
            key_insights: insights,
            statistics,
        }
    }

    fn get_word_frequency(&self) -> Vec<(String, usize)> {
        let mut word_count = HashMap::new();
        
        for word in self.content.split_whitespace() {
            let clean_word = word.to_lowercase().chars().filter(|c| c.is_alphabetic()).collect::<String>();
            if !clean_word.is_empty() && clean_word.len() > 2 {
                *word_count.entry(clean_word).or_insert(0) += 1;
            }
        }

        let mut sorted_words: Vec<_> = word_count.into_iter().collect();
        sorted_words.sort_by(|a, b| b.1.cmp(&a.1));
        sorted_words
    }
}

fn print_summary(summary: &FileSummary, filename: &str) {
    println!("File Summary: {}", filename);
    
    match summary.file_type {
        FileType::Text => println!("Type: Plain Text"),
        FileType::Markdown => println!("Type: Markdown"),
        FileType::Log => println!("Type: Log File"),
        FileType::RustCode => println!("Type: Rust Source Code"),
        FileType::Unknown => println!("Type: Unknown"),
    }

    println!("\nBasic Statistics:");
    println!("Lines: {}", summary.line_count);
    println!("Words: {}", summary.word_count);
    println!("Characters: {}", summary.char_count);

    if !summary.statistics.is_empty() {
        println!("\nDetailed Statistics:");
        for (key, value) in &summary.statistics {
            let display_key = key.replace("_", " ").replace("avg", "Average");
            println!("{}: {}", display_key.chars().next().unwrap().to_uppercase().to_string() + &display_key[1..], value);
        }
    }
    if !summary.key_insights.is_empty() {
        println!("\nKey Insights:");
        for insight in &summary.key_insights {
            println!("   • {}", insight);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        eprintln!("Supported file types: .txt, .md, .log, .rs");
        process::exit(1);
    }

    let file_path = &args[1];
    let path = Path::new(file_path);

    if !path.exists() {
        eprintln!("Error: File '{}' does not exist.", file_path);
        process::exit(1);
    }

    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", file_path, e);
            process::exit(1);
        }
    };

    let analyzer = TextAnalyzer::new(content);
    let file_type = FileType::from_extension(path);

    let summary = match file_type {
        FileType::Text => analyzer.analyze_text(),
        FileType::Markdown => analyzer.analyze_markdown(),
        FileType::Log => analyzer.analyze_log(),
        FileType::RustCode => analyzer.analyze_rust_code(),
        FileType::Unknown => {
            println!("Unknown file type, analyzing as plain text...");
            analyzer.analyze_text()
        }
    };

    print_summary(&summary, file_path);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_basic_text_analysis() {
        let content = "Hello world! This is a test file with some words.".to_string();
        let analyzer = TextAnalyzer::new(content);
        let summary = analyzer.analyze_text();
        
        assert_eq!(summary.word_count, 10);
        assert_eq!(summary.line_count, 1);
    }

    #[test]
    fn test_markdown_detection() {
        let content = "# Header\n\nSome content with [link](url) and ![image](img.jpg)\n\n```code```".to_string();
        let analyzer = TextAnalyzer::new(content);
        let summary = analyzer.analyze_markdown();
        assert_eq!(summary.file_type, FileType::Markdown);
        assert!(summary.statistics.contains_key("headers"));
        assert!(summary.statistics.contains_key("links"));
    }

    #[test]
    fn test_file_type_detection() {
        assert_eq!(FileType::from_extension(Path::new("test.txt")), FileType::Text);
        assert_eq!(FileType::from_extension(Path::new("README.md")), FileType::Markdown);
        assert_eq!(FileType::from_extension(Path::new("app.log")), FileType::Log);
        assert_eq!(FileType::from_extension(Path::new("main.rs")), FileType::RustCode);
    }
}