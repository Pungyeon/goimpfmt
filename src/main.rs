#![allow(dead_code)]

extern crate regex;

use regex::Regex;
use std::cmp::Ordering;

fn main() {
  let args : Vec<String> = std::env::args().collect();

  if args.len() != 2 {
    println!("unexpected argument format, expected: ./goimpft <project_directory> <project_root_package>");
    println!("\tsample: ./goimpft ~/projects/import-fix github.com/Pungyeon/import-fix");
    return
  }

  let directory = &args[1];
  let project = &args[2];

  let external = Regex::new(r".+\..+/").unwrap();
  let local = Regex::new(project).unwrap();
  
  Formatter::new(&local, &external).format(directory);
  println!("Done");
}

struct Formatter<'a> {
  external: &'a Regex,
  local: &'a Regex,
}

impl<'a> Formatter<'a> {
  fn new(local: &'a Regex, external: &'a Regex) -> Self {
    Formatter{
      external: external,
      local: local,
    } 
  }

  fn format(&self, dir_path: &str) {
    match std::fs::read_dir(dir_path) {
      Ok(entries) => self.format_entries(entries),
      Err(err) => println!("{}", err),
    }
  }

  fn format_entries(&self, entries: std::fs::ReadDir) {
    for entry in entries {
      match entry {
        Ok(e) => self.format_entry(e),
        Err(e) => println!("{}", e),
      }
    }
  }

  fn format_entry(&self, entry: std::fs::DirEntry) {
    let path = entry.path();
    if path.is_dir() {
      return self.format(path.to_str().unwrap());
    }
    return self.format_file(path);
  }

  fn format_file(&self, path: std::path::PathBuf) {
    if let Some(ext) = path.extension() {
      if ext != "go" {
        return ();
      }
      return self.format_go_file(path);
    }
  }

  fn format_go_file(&self, path: std::path::PathBuf) {
    if let Some(file) = GoFile::new(path.to_str().unwrap(), self.local, self.external) {
      match std::fs::write(path.to_str().unwrap(), file.output()) {
        Ok(_) => (), // println!("Processed: {:?}", path),
        Err(e) => println!("Error processing {:?}: {}", path, e),
      }
    }
  }
}

struct GoFile {
  output: String,
}

impl GoFile {
  fn new(file_path: &str, local: &Regex, external: &Regex) -> Option<Self> {
    match std::fs::read_to_string(file_path) {
      Ok(file) => GoFile::from_file(file, local, external),
      Err(e) => {
        println!("error reading file {}: {}", file_path, e);
        return None;
      } 
    }
  }

  fn from_file(file: String, local: &Regex, external: &Regex) -> Option<Self> {
    let lines : Vec<&str> = file.lines().collect();
    for (i, line) in lines.iter().enumerate() {
      if line.len() > 6 {
        let starts_with : String = line.chars().take(6).collect();
        if starts_with == "import" {
          return GoFile::from_line(&lines, i, local, external);
        }
      }
    }
    None
  }

  fn from_line(lines: &Vec<&str>, i: usize, local: &Regex, external: &Regex) -> Option<Self> {
    let mut imports = Imports::new(&lines[i..], local, external);
    return Some(GoFile{
      output: format!("{}\n{}\n{}\n", lines[..i].join("\n"), imports.output(), lines[i+imports.lines()..].join("\n")),
    });
  }

  fn output(self) -> String {
    self.output
  }
}

#[derive(Eq)]
struct Import {
  prefix: String,
  alias: Option<String>,
  import: String,
  comment: Option<String>,
}

impl Import {
  fn new(line: &str) -> Self {
    let entries : Vec<&str> = line.trim().split(" ").collect();
    if entries.len() > 1 {
      Import{
        prefix: whitespace_prefix(line),
        alias: Some(entries[0].to_string()),
        import: entries[1..].join(" "),
        comment: None,
      }
    } else {
      Import{
        prefix: whitespace_prefix(line),
        alias: None,
        import: entries[0].to_string(),
        comment: None,
      }
    }
  }

  fn with_comment(&mut self, comment: Option<String>) {
    self.comment = comment;
  }

  fn to_str(&self) -> String {
    let mut out = self.prefix.clone();
    if self.alias != None {
      out.push_str(&self.alias.clone().unwrap());
      out.push_str(" ");
    }
    out.push_str(&format!("{}", self.import.clone()));

    if self.comment != None {
      return format!("{}\n{}", self.comment.clone().unwrap(), out);
    }
    out
  }
}

fn whitespace_prefix(line: &str) -> String {
    for (i, c) in line.chars().enumerate() {
      if c != ' ' && c != '\t' {
        return line[..i].to_string();
      }
    }
    "".to_string()
}

impl Ord for Import {
  fn cmp(&self, other: &Self) -> Ordering {
    self.import.cmp(&other.import)
  }
}

impl PartialOrd for Import {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl PartialEq for Import {
  fn eq(&self, other: &Self) -> bool {
    self.import == other.import
  }
}

impl std::fmt::Display for Import {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.to_str())
  }
}

struct Imports {
    builtin: Vec<Import>,
    external: Vec<Import>,
    project: Vec<Import>,
    single: Option<String>,
    comment: Option<String>,
    empty: usize,
}

impl Imports {
  fn default() -> Self {
    Imports{
      builtin: Vec::new(),
      external: Vec::new(),
      project: Vec::new(),
      single: None,
      comment: None,
      empty: 0,
    }
  }

  fn new(input: &[&str], local: &Regex, external: &Regex) -> Self {
    let mut imports = Imports::default();

    if input[0] != "import (" {
      imports.single = Some(input[0].to_string());
      return imports;
    }

    for line in input[1..].iter() {
      if *line == ")" { break; } 
      imports.parse_imports(line, local, external);
    }

    imports
  }

  fn parse_imports(&mut self, line: &str, local: &Regex, external: &Regex) {
    let starts_with : String = line.trim().chars().take(2).collect();
    if starts_with == "//" {
      return self.parse_comment(line);
    }
    return self.parse_import(line, local, external);
  }

  fn parse_comment(&mut self, line: &str) {
    self.empty += 1;
    if self.comment == None {
      return self.comment = Some(line.to_string());
    } 
    return self.comment = Some(format!("{}\n{}", self.comment.clone().unwrap(), line.to_string()));
  }

  fn parse_import(&mut self, line: &str, local: &Regex, external: &Regex) {
    let mut import = Import::new(line);
    if self.comment != None {
      import.with_comment(self.comment.clone());
      self.comment = None
    }

    if external.is_match(line) {
      return self.handle_external(line, import, local);
    } 
    return self.handle_other(line, import);
  }

  fn handle_external(&mut self, line: &str, import: Import, local: &Regex) {
    if local.is_match(line) {
      return self.project.push(import);
    }
    return self.external.push(import);
  }

  fn handle_other(&mut self, line: &str, import: Import) {
    if line == "" {
      return self.empty += 1;
    } 
    return self.builtin.push(import);
  }

  fn output(&mut self) -> String {
    if self.single != None {
      return self.single.clone().unwrap();
    }

    ImportString::new("import (\n".to_string())
      .push(self.builtin.as_mut())
      .push(self.project.as_mut())
      .push(self.external.as_mut())
      .build()
  }

  fn lines(&self) -> usize {
    if self.single != None {
      return 1;
    }
    self.length() + self.empty + IMPORT_WRAP_LEN
  }

  fn length(&self) -> usize {
    if self.single != None {
      return 1;
    }
    self.builtin.len() + self.external.len() + self.project.len()
  }
}

struct ImportString {
  out: String,
  previous: bool,
}

impl ImportString {
  fn new(header: String) -> Self {
    ImportString{
      out: header,
      previous: false,
    } 
  }

  fn push(&mut self, list: &mut Vec<Import>) -> &mut Self {
    if list.len() == 0 { return self; }
    if self.previous { self.out.push('\n'); }

    list.sort();
    for imp in list { 
      self.out.push_str(&imp.to_str());
      self.out.push('\n');
    }

    self.previous = true;
    self
  }

  fn build(&self) -> String {
    format!("{})", self.out)
  }
}

const IMPORT_WRAP_LEN : usize = 2;

#[test]
fn test_import() {
  let input = r#"json "github.com/Pungyeon/required/pkg/json""#;
  let aliased = Import::new(input);
  assert_eq!(aliased.alias, Some("json".to_string()));
  assert_eq!(&aliased.import, "\"github.com/Pungyeon/required/pkg/json\"");
  assert_eq!(aliased.to_str(), input);

  let normal = Import::new(r#""github.com/Pungyeon/required/pkg/json""#);
  assert_eq!(normal.alias, None);
  assert_eq!(&normal.import, "\"github.com/Pungyeon/required/pkg/json\"");

  let prefixed = Import::new("\t\"os\"");
  assert_eq!(prefixed.prefix, "\t");
}

#[test]
fn test_import_fix() {
  let input = "import (
  \"github.com/Vivino/go-api/something\"
  \"os\"
  // This is something
  \"github.com/Vivino/go-tools/something\"
  \"github.com/Pungyeon/import-fix\"
)";

  let expected = "import (
  \"os\"

  \"github.com/Vivino/go-api/something\"

  \"github.com/Pungyeon/import-fix\"
  // This is something
  \"github.com/Vivino/go-tools/something\"
)";

  let lines : Vec<&str> = input.lines().collect();
  let external = Regex::new(r".+\..+/").unwrap();
  let local = Regex::new("github.com/Vivino/go-api").unwrap();
  let mut imports = Imports::new(&lines[..], &local, &external);
  let out = imports.output();
  println!("{}", out);
  assert_eq!(imports.length(), 4);
  assert_eq!(out, expected);
}

#[test]
fn test_simple_import() {
  let input = "import (
\t\"os\"
)";

  let lines : Vec<&str> = input.lines().collect();
  let external = Regex::new(r".+\..+/").unwrap();
  let local = Regex::new("github.com/Vivino/go-api").unwrap();
  let mut imports = Imports::new(&lines[..], &local, &external);
  assert_eq!(imports.length(), 1);
  assert_eq!(imports.output(), input); // ensure there is no change
}

#[test]
fn test_one_liner_import() {
  let input = r#"import "os""#;

  let lines : Vec<&str> = input.lines().collect();
  let external = Regex::new(r".+\..+/").unwrap();
  let local = Regex::new("github.com/Vivino/go-api").unwrap();
  let mut imports = Imports::new(&lines[..], &local, &external);
  assert_eq!(imports.length(), 1);
  assert_eq!(imports.output(), input); // ensure there is no change
}


#[test]
fn test_import_file_fix() {
  let expected = "// * “other“ This is a comment
package main

import (
\t\"fmt\"
\t\"net/http\"

\t\"github.com/Vivino/go-api/services/aerospike\"
\tkafka \"github.com/Vivino/go-api/services/kafka\"

\t\"github.com/Pungyeon/required\"
  \"github.com/mamamoo/hip\"
)

func main() {
    fmt.Println(\"something\")
}

";

  let external = Regex::new(r".+\..+/").unwrap();
  let local = Regex::new("github.com/Vivino/go-api").unwrap();
  let file = GoFile::new("./test_files/test.go.file", &local, &external);

  let output = file.unwrap().output();
  println!("{}", &output);
  assert_eq!(output, expected); 
}
