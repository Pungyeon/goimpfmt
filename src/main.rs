#![allow(dead_code)]

extern crate regex;
extern crate term;

use std::collections::{HashSet};
use difference::{Difference, Changeset};
use clap::{Arg, App};
use regex::Regex;
use std::cmp::Ordering;

fn main() {
  let matches = App::new("Go Import Format")
      .version("0.3.1")
      .author("Lasse Martin Jakobsen (Pungyeon)")
      .about("Formats Go imports enforcing the Vivino style guide, grouping and separating built-in, internal and external library imports")
      .arg(Arg::new("project")
          .short('p')
          .long("project")
          .value_name("PROJECT IMPORT")
          .about("determines the 'internal' import prefix")
          .takes_value(true)
          .required(true))
      .arg(Arg::new("input")
          .short('i')
          .long("input")
          .value_name("FILE/DIRECTORY")
          .about("specifies the directories and/or files to format. Directories are formatted recursively.")
          .takes_value(true)
          .required(true)
          .multiple(true))
      .arg(Arg::new("quiet")
          .short('q')
          .long("quiet")
          .about("will suppress diff output"))
      .arg(Arg::new("write")
          .short('w')
          .long("write")
          .about("specifies whether to write any eventual diff to the formatted file / directories"))
      .arg(Arg::new("ignore")
          .short('x')
          .long("ignore")
          .value_name("FILE/DIRECTORY")
          .about("specifies directories and/or files in which to ignore. This is meant to use with vendor modules and the like.")
          .takes_value(true)
          .multiple(true))
      .get_matches();


  if let Some(inputs) = matches.values_of("input") {
    let mut mds: Vec<Entry> = Vec::new();
    for input in inputs {
      match std::fs::metadata(input) {
        Ok(md) =>  mds.push(Entry{
          path: input,
          md,
        }),
        Err(e) => {
          println!("{}: {}", e, input);
          std::process::exit(1);
        },
      }
    }

    let mut ignored = HashSet::new();
    if let Some(ignored_paths) = matches.values_of("ignore") {
      for path in ignored_paths {
        ignored.insert(path);
      }
    }


    let mut formatter = Formatter::new(
      matches.value_of("project").unwrap(),
      ignored,
      matches.is_present("quiet"),
      matches.is_present("write"));
    for metadata in mds {
      formatter.format_md(metadata.path, metadata.md)
    }

    if formatter.found_difference {
      std::process::exit(1);
    }
  }
}

struct Entry<'a> {
  md: std::fs::Metadata,
  path: &'a str,
}

enum PackageType {
  External,
  Local,
  Other,
}

struct Matcher {
  external: Regex,
  local: Regex,
}

impl Matcher{
  fn new(local: &str) -> Self {
    let prefixes = local.split(",");
    let mut s : String = "(".to_string();
    for (i, prefix) in prefixes.enumerate() {
      if i > 0 {
        s.push_str("|");
      }
      s.push_str(prefix);
    }
    s.push_str(")");

    Matcher{
      external: Regex::new(r".+\..+/").unwrap(),
      local: Regex::new(&s).unwrap(),
    }
  }

  fn package(&self, package: &str) -> PackageType {
    if self.external.is_match(package) {
      if self.local.is_match(package) {
        return PackageType::Local;
      }
      return PackageType::External
    }
    PackageType::Other
  }
}

struct Formatter<'a> {
  matcher: Matcher,
  found_difference: bool,
  ignored: HashSet<&'a str>,
  quiet: bool,
  write: bool,
}

impl<'a> Formatter<'a> {
  fn new(project: &str, ignored: HashSet<&'a str>, quiet: bool, write: bool) -> Self {
    Formatter{
      matcher: Matcher::new(project),
      found_difference: false,
      ignored,
      write,
      quiet,
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

  fn format_md(&mut self, path: &str, md: std::fs::Metadata) {
    if self.ignore(path) {
      return;
    }
    if md.is_dir() {
      return self.format(path);
    }
    self.format_file(std::path::PathBuf::from(path));
  }

  fn format_entry(&mut self, entry: std::fs::DirEntry) {
    let path = entry.path();
    if self.ignore(path.to_str().unwrap()) {
      return;
    }
    if path.is_dir() {
      return self.format(path.to_str().unwrap());
    }
    self.format_file(path);
  }

  fn ignore(&self, path: &str) -> bool {
    self.ignored.get(path).is_some()
  }

fn format_file(&mut self, path: std::path::PathBuf) {
    if let Some(ext) = path.extension() {
      if ext != "go" {
        return;
      }
      self.format_go_file(path)
    }
  }

  fn format_go_file(&mut self, path: std::path::PathBuf) {
    if let Some(file) = GoFile::new(path.to_str().unwrap(), &self.matcher) {
      if file.diff.distance == 0 {
         return
      }
      self.found_difference = true;
      if !self.quiet {
        print_diff(path.to_str().unwrap(), &file.diff.diffs);
      }
      if self.write {
        match std::fs::write(path.to_str().unwrap(), file.output()) {
          Ok(_) => (), // println!("Processed: {:?}", path),
          Err(e) => println!("Error processing {:?}: {}", path, e),
        }
      }
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
        None
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

  fn from_line(lines: &[&str], i: usize, matcher: &Matcher) -> Option<Self> {
    let mut imports = Imports::new(&lines[i..], matcher);
    let output = imports.output();
    Some(GoFile{
      diff: Changeset::new(&lines[i..imports.lines()+i].join("\n"), &output, "\n"),
      output: format!("{}\n{}\n{}\n", lines[..i].join("\n"), imports.output(), lines[i+imports.lines()..].join("\n")),
    })
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
    let entries : Vec<&str> = line.trim().split(' ').collect();
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
      out.push(' ');
    }
    out.push_str(self.import.clone().as_str());

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

  fn parse_imports(&mut self, line: &str, matcher: &Matcher) {
    let starts_with : String = line.trim().chars().take(2).collect();
    if starts_with == "//" {
      return self.parse_comment(line);
    }
    self.parse_import(line, matcher);
  }

  fn parse_comment(&mut self, line: &str) {
    self.empty_lines += 1;
    if self.comment == None {
      return self.comment = Some(line.to_string());
    } 
    self.comment = Some(format!("{}\n{}", self.comment.clone().unwrap(), line));
  }

  fn parse_import(&mut self, line: &str, matcher: &Matcher) {
    let mut import = Import::new(line);
    if self.comment != None {
      import.with_comment(self.comment.clone());
      self.comment = None
    }

    match matcher.package(line) {
      PackageType::Other => self.handle_other(line, import),
      PackageType::Local => self.local.push(import),
      PackageType::External => self.external.push(import),
    }
  }
  fn handle_other(&mut self, line: &str, import: Import) {
    if line.is_empty() {
      return self.empty_lines += 1;
    } 
    self.builtin.push(import);
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
    if list.is_empty() { return self; }
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

fn print_diff(path: &str, diffs: &[Difference]) {
  match term::stdout() {
    Some(mut t) => {
      match print_diff_color(&mut t, path, diffs) {
        Ok(()) => (),
        Err(e) => {
          println!("issue printing diff in color terminal: {}", e);
          println!("falling back to plain terminal");
          print_diff_plain(path, diffs);
        }
      }
    },
    None => print_diff_plain(path, diffs),
  }
}

fn get_diff(diffs: &[Difference]) -> (usize, usize) {
  let (mut add, mut rem) = (0, 0);
  for diff in diffs {
    match diff {
      Difference::Same(_) => (),
      Difference::Add(_) => add += 1,
      Difference::Rem(_) => rem += 1,
    }
  }
  (add, rem)
}

fn print_diff_color(t: &mut Box<term::StdoutTerminal>, path: &str, diffs: &[Difference]) -> std::result::Result<(), term::Error> {
  writeln!(t, "diff --goimpfmt {}", path)?;
  let (add, rem) = get_diff(diffs);
  t.fg(term::color::CYAN)?;
  writeln!(t, "@@ +{}, -{} @@", add, rem)?;

  for diff in diffs {
    match diff { Difference::Same(ref x) => {
        t.reset()?;
        writeln!(t, " {}", x)?;
      }
      Difference::Add(ref x) => {
        t.fg(term::color::GREEN)?;
        writeln!(t, "+{}", x)?;
      }
      Difference::Rem(ref x) => {
        t.fg(term::color::RED)?;
        writeln!(t, "-{}", x)?;
      }
    }
  }
  t.reset()?;
  Ok(t.flush()?)
}

fn print_diff_plain(path: &str, diffs: &[Difference]) {
  println!("diff --goimpfmt {}", path);
  let (add, rem) = get_diff(diffs);
  println!("@@ +{}, -{} @@", add, rem);
  for diff in diffs {
    match diff {
      Difference::Same(ref x) => {
        println!(" {}", x);
      }
      Difference::Add(ref x) => {
        println!("+{}", x);
      }
      Difference::Rem(ref x) => {
        println!("-{}", x);
      }
    }
  }
}

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
  print_diff("test.go", &cs.diffs);
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
fn test_import_file_multi_projects() {
  let expected = "// * “other“ This is a comment
package main

import (
\t\"fmt\"
\t\"net/http\"

\t\"github.com/Vivi/go-api-services/d\"
\t\"github.com/Vivino/go-api/services/aerospike\"
\tkafka \"github.com/Vivino/go-api/services/kafka\"

\t\"github.com/Pungyeon/required\"
  \"github.com/mamamoo/hip\"
)

func main() {
    fmt.Println(\"something\")
}

";

  let file = GoFile::new("./test_files/multi.go.file", &Matcher::new("github.com/Vivino/go-api,github.com/Vivi"));

  let go_file = file.unwrap();
  let output = &go_file.output();
  println!("{}", &output);
  assert_eq!(output, expected);
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
  let output = &go_file.output();
  println!("{}", &output);
  assert_eq!(output, expected); 
}
