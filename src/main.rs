#![allow(dead_code)]

extern crate regex;

use regex::Regex;
use std::cmp::Ordering;

fn main() {
  match Args::parse(std::env::args().collect()) {
    Ok(args) => {
      let mut formatter = Formatter::new(&args.project);
      if args.file_mode {
        for file in args.paths {
          formatter.format_file(std::path::PathBuf::from(file));
        }
      } else {
        for dir in args.paths {
          formatter.format(&dir);
        }
      }
      if formatter.found_difference {
        std::process::exit(1);
      }
    },
    Err(msg) => {
      println!("ERROR: {}", msg);
      println!();
      println!("USAGE: ./goimpft <project_import> (-f) <target_path(s)>");
      println!("\t-f (optional): use file_mode, in which you are feeding files, rather than directories");
      println!();
      println!("\tsample: ./goimpft github.com/Pungyeon/goimpfmt .");
      println!("\tsample: ./goimpft github.com/Pungyeon/goimpfmt -f main.go");
      println!("\tsample: ./goimpft github.com/Pungyeon/goimpfmt -f main.go main_test.go");
    }
  }
}


struct Args {
  file_mode: bool,
  paths: Vec<String>,
  project: String,
}

impl Args {
  fn default() -> Self {
    Args{
      file_mode: false,
      paths: Vec::new(),
      project: "".to_string(),
    }
  }
  fn parse(args: Vec<String>) -> Result<Self, String> {
    let mut a = Args::default();

    if args.len() < 3 {
      return Err(format!("not enough arguments, expected 3 or more: {:?}", args));
    }
    a.project = args[1].clone();

    if &args[2] == "-f" {
      a.file_mode = true;
      if args.len() < 4 {
        return Err(format!("not enough arguments with -f specified, expected 4 or more: {:?}", args));
      }
      a.paths = args[3..].to_vec();
    } else {
      a.paths = args[2..].to_vec();
    }

    Ok(a)
  }
}

enum PackageType {
  EXTERNAL,
  LOCAL,
  OTHER,
}

struct Matcher {
  external: Regex,
  local: Regex,
}

impl Matcher{
  fn new(local: &str) -> Self {
    Matcher{
      external: Regex::new(r".+\..+/").unwrap(),
      local: Regex::new(local).unwrap(),
    }
  }

  fn package(&self, package: &str) -> PackageType {
    if self.external.is_match(package) {
      if self.local.is_match(package) {
        return PackageType::LOCAL;
      }
      return PackageType::EXTERNAL
    }
    return PackageType::OTHER
  }
}

struct Formatter {
  found_difference: bool,
  matcher: Matcher,
}

impl Formatter {
  fn new(project: &str) -> Self {
    Formatter{
      found_difference: false,
      matcher: Matcher::new(project),
    }
  }

  fn format(&mut self, dir_path: &str) {
    match std::fs::read_dir(dir_path) {
      Ok(entries) => self.format_entries(entries),
      Err(err) => println!("{}", err),
    }
  }

  fn format_entries(&mut self, entries: std::fs::ReadDir) {
    for entry in entries {
      match entry {
        Ok(e) => self.format_entry(e),
        Err(e) => println!("{}", e),
      }
    }
  }

  fn format_entry(&mut self, entry: std::fs::DirEntry) {
    let path = entry.path();
    if path.is_dir() {
      return self.format(path.to_str().unwrap());
    }
    return self.format_file(path);
  }

  fn format_file(&mut self, path: std::path::PathBuf) {
    if let Some(ext) = path.extension() {
      if ext != "go" {
        return ();
      }
      return self.format_go_file(path);
    }
  }

  fn format_go_file(&mut self, path: std::path::PathBuf) {
    if let Some(file) = GoFile::new(path.to_str().unwrap(), &self.matcher) {
      // TODO diff logic here
      if file.diff.distance > 0 {
        self.found_difference = true;
        // TODO if quiet then don't print
        print_diff(path.to_str().unwrap(), file.diff.diffs);
      }
      // TODO if no-fix then don't write
      // match std::fs::write(path.to_str().unwrap(), file.output()) {
      //   Ok(_) => (), // println!("Processed: {:?}", path),
      //   Err(e) => println!("Error processing {:?}: {}", path, e),
      // }
    }
  }
}

struct GoFile {
  output: String,
  diff: Changeset,
}

impl GoFile {
  fn new(file_path: &str, matcher: &Matcher) -> Option<Self> {
    match std::fs::read_to_string(file_path) {
      Ok(file) => GoFile::from_file(file, matcher),
      Err(e) => {
        println!("error reading file {}: {}", file_path, e);
        return None;
      } 
    }
  }

  fn from_file(file: String, matcher: &Matcher) -> Option<Self> {
    let lines : Vec<&str> = file.lines().collect();
    for (i, line) in lines.iter().enumerate() {
      if line.len() > 6 {
        let starts_with : String = line.chars().take(6).collect();
        if starts_with == "import" {
          return GoFile::from_line(&lines, i, matcher);
        }
      }
    }
    None
  }

  fn from_line(lines: &Vec<&str>, i: usize, matcher: &Matcher) -> Option<Self> {
    let mut imports = Imports::new(&lines[i..], matcher);
    let output = imports.output();
    return Some(GoFile{
      diff: Changeset::new(&lines[i..imports.lines()+i].join("\n"), &output, "\n"),
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
    local: Vec<Import>,
    single: Option<String>,
    comment: Option<String>,
    empty_lines: usize,
}

struct Diff {
  changes: usize,
}

impl Imports {
  fn default() -> Self {
    Imports{
      builtin: Vec::new(),
      external: Vec::new(),
      local: Vec::new(),
      single: None,
      comment: None,
      empty_lines: 0,
    }
  }

  fn new(input: &[&str], matcher: &Matcher) -> Self {
    let mut imports = Imports::default();

    if input[0] != "import (" {
      imports.single = Some(input[0].to_string());
      return imports;
    }

    for line in input[1..].iter() {
      if *line == ")" { break; }
      imports.parse_imports(line, matcher);
    }

    imports
  }

  fn diff(old: &[&str], new: &str) -> Diff {
    Diff{ changes: 0 }
  }

  fn parse_imports(&mut self, line: &str, matcher: &Matcher) {
    let starts_with : String = line.trim().chars().take(2).collect();
    if starts_with == "//" {
      return self.parse_comment(line);
    }
    return self.parse_import(line, matcher);
  }

  fn parse_comment(&mut self, line: &str) {
    self.empty_lines += 1;
    if self.comment == None {
      return self.comment = Some(line.to_string());
    } 
    return self.comment = Some(format!("{}\n{}", self.comment.clone().unwrap(), line.to_string()));
  }

  fn parse_import(&mut self, line: &str, matcher: &Matcher) {
    let mut import = Import::new(line);
    if self.comment != None {
      import.with_comment(self.comment.clone());
      self.comment = None
    }

    match matcher.package(line) {
      PackageType::OTHER => self.handle_other(line, import),
      PackageType::LOCAL => self.local.push(import),
      PackageType::EXTERNAL => self.external.push(import),
    }
  }
  fn handle_other(&mut self, line: &str, import: Import) {
    if line == "" {
      return self.empty_lines += 1;
    } 
    return self.builtin.push(import);
  }

  fn output(&mut self) -> String {
    if self.single != None {
      return self.single.clone().unwrap();
    }

    ImportString::new("import (\n".to_string())
      .push(self.builtin.as_mut())
      .push(self.local.as_mut())
      .push(self.external.as_mut())
      .build()
  }

  fn lines(&self) -> usize {
    if self.single != None {
      return 1;
    }
    self.length() + self.empty_lines + IMPORT_WRAP_LEN
  }

  fn length(&self) -> usize {
    if self.single != None {
      return 1;
    }
    self.builtin.len() + self.external.len() + self.local.len()
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

fn print_diff(path: &str, diffs: Vec<Difference>) {
  let mut t = term::stdout().unwrap();
  writeln!(t, "diff --goimpfmt {}", path);
  t.fg(term::color::CYAN);
  for i in 0..diffs.len() {
    match diffs[i] {
      Difference::Same(ref x) => {
        t.reset().unwrap();
        writeln!(t, " {}", x);
      }
      Difference::Add(ref x) => {
        t.fg(term::color::GREEN).unwrap();
        writeln!(t, "+{}", x);
      }
      Difference::Rem(ref x) => {
        t.fg(term::color::RED).unwrap();
        writeln!(t, "-{}", x);
      }
    }
  }
  t.reset().unwrap();
  t.flush().unwrap();
}

use difference::{Difference, Changeset};
extern crate term;

#[test]
fn test_diff() {
  let left = "a
b
c
d
e
f
g";

  let right = "a
b
c
d
f
g
e";

  let cs = Changeset::new(left, right, "\n");
  print_diff("test.go", cs.diffs);
  assert_eq!(cs.distance, 2);
}

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
  let mut imports = Imports::new(&lines[..], &Matcher::new("github.com/Vivino/go-api"));
  let out = imports.output();
  println!("{}", out);

  let diff = Imports::diff(&lines[..], &out);
  assert_eq!(diff.changes, 4);
  assert_eq!(imports.length(), 4);
  assert_eq!(out, expected);
}

#[test]
fn test_simple_import() {
  let input = "import (
\t\"os\"
)";

  let lines : Vec<&str> = input.lines().collect();
  let mut imports = Imports::new(&lines[..], &Matcher::new("github.com/Vivino/go-api"));

  assert_eq!(imports.length(), 1);
  assert_eq!(imports.output(), input); // ensure there is no change
}

#[test]
fn test_one_liner_import() {
  let input = r#"import "os""#;

  let lines : Vec<&str> = input.lines().collect();
  let mut imports = Imports::new(&lines[..], &Matcher::new("github.com/Vivino/go-api"));
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

  let file = GoFile::new("./test_files/test.go.file", &Matcher::new("github.com/Vivino/go-api"));

  let go_file = file.unwrap();
  assert_eq!(go_file.diff.distance, 2);
  let output = &go_file.output();
  println!("{}", &output);
  assert_eq!(output, expected); 
}
