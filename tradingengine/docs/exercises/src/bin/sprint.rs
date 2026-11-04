// Rustlings-style runner for the sprint exercises.
//
//   cargo run              watch mode: runs the current exercise again on every save
//   cargo run -- list      every exercise and whether it is done
//   cargo run -- hint      the hint for the current exercise
//   cargo run -- reset     forget the progress (your files are not touched)
//
// In watch mode, type a letter and press Enter:
//   n next exercise (once the current one passes)   h hint   l list   r run again   q quit
//
// Progress is kept in .sprint-progress next to Cargo.toml.

use std::collections::BTreeSet;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime};
use std::{env, fs};

const PROGRESS: &str = ".sprint-progress";
const CLEAR: &str = "\x1b[2J\x1b[H";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

// ex00_contract_quiz, ex01_wire_enums, ... in order
fn exercises() -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root().join("exercises"))
        .expect("exercises/ is missing")
        .filter_map(|entry| {
            let name = entry.ok()?.file_name().into_string().ok()?;
            let stem = name.strip_suffix(".rs")?;
            stem.starts_with("ex").then(|| stem.to_string())
        })
        .collect();
    names.sort();
    names
}

fn file_of(name: &str) -> PathBuf {
    root().join("exercises").join(format!("{name}.rs"))
}

fn load_done() -> BTreeSet<String> {
    fs::read_to_string(root().join(PROGRESS))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

fn save_done(done: &BTreeSet<String>) {
    let text: String = done.iter().map(|name| format!("{name}\n")).collect();
    fs::write(root().join(PROGRESS), text).expect("cannot write .sprint-progress");
}

fn hint(name: &str) -> String {
    let hints = fs::read_to_string(root().join("HINTS.md")).unwrap_or_default();
    let heading = format!("## {name}");
    let Some(start) = hints.find(&heading) else {
        return format!("no hint for {name} in HINTS.md");
    };
    let body = &hints[start + heading.len()..];
    let end = body.find("\n## ").unwrap_or(body.len());
    body[..end].trim().to_string()
}

// (passed, cargo's output)
fn run(name: &str) -> (bool, String) {
    let output = Command::new("cargo")
        .args(["test", "--quiet", "--color", "always", "--test", name])
        .current_dir(root())
        .output()
        .expect("cannot start cargo");
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), text)
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn progress_bar(done: usize, total: usize) -> String {
    let width = 30;
    let filled = done * width / total.max(1);
    format!(
        "[{GREEN}{}{RESET}{}] {done}/{total}",
        "#".repeat(filled),
        "-".repeat(width - filled)
    )
}

fn print_list(all: &[String], done: &BTreeSet<String>, current: Option<&str>) {
    for name in all {
        let mark = if done.contains(name) {
            format!("{GREEN}done{RESET}")
        } else {
            format!("{RED}todo{RESET}")
        };
        let arrow = if Some(name.as_str()) == current { ">" } else { " " };
        println!("{arrow} {mark}  {name}");
    }
}

fn render(name: &str, passed: bool, output: &str, done: usize, total: usize) {
    print!("{CLEAR}{output}\n");
    if passed {
        println!("{GREEN}{BOLD}{name} passes.{RESET} Type n and Enter for the next exercise.");
    } else {
        println!("{RED}{BOLD}{name} is not done yet.{RESET} Save the file to run it again.");
    }
    println!();
    println!("{}  {}", progress_bar(done, total), file_of(name).display());
    println!("n next   h hint   l list   r run again   q quit");
    io::stdout().flush().ok();
}

fn watch() {
    let all = exercises();
    let mut done = load_done();

    // keystrokes come in on their own thread, so saving a file is noticed meanwhile
    let (keys, input) = mpsc::channel::<String>();
    thread::spawn(move || {
        for line in io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            if keys.send(line.trim().to_string()).is_err() {
                break;
            }
        }
    });

    'exercises: loop {
        let Some(name) = all.iter().find(|name| !done.contains(*name)).cloned() else {
            println!("{CLEAR}{GREEN}{BOLD}All {} exercises are done.{RESET}", all.len());
            println!("Now the real thing: PR A in ../SPRINT.md.");
            return;
        };
        let file = file_of(&name);

        println!("{CLEAR}running {name} ...");
        let (mut passed, mut output) = run(&name);
        render(&name, passed, &output, done.len(), all.len());
        let mut seen = modified(&file);

        loop {
            match input.recv_timeout(Duration::from_millis(300)) {
                Ok(key) => match key.as_str() {
                    "n" if passed => {
                        done.insert(name.clone());
                        save_done(&done);
                        continue 'exercises;
                    }
                    "n" => println!("{RED}{name} doesn't pass yet.{RESET}"),
                    "h" => println!("\n{BOLD}Hint for {name}{RESET}\n{}\n", hint(&name)),
                    "l" => {
                        println!();
                        print_list(&all, &done, Some(&name));
                    }
                    "r" => {
                        (passed, output) = run(&name);
                        render(&name, passed, &output, done.len(), all.len());
                    }
                    "q" => return,
                    "" => {}
                    other => println!("unknown command {other:?}: n, h, l, r or q"),
                },
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                // stdin closed: keep watching, but nothing can be typed any more
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    thread::sleep(Duration::from_millis(300));
                }
            }

            let now = modified(&file);
            if now != seen {
                seen = now;
                println!("{CLEAR}running {name} ...");
                (passed, output) = run(&name);
                render(&name, passed, &output, done.len(), all.len());
            }
        }
    }
}

fn main() {
    let all = exercises();
    let done = load_done();
    let current = all.iter().find(|name| !done.contains(*name)).cloned();

    match env::args().nth(1).as_deref() {
        None | Some("watch") => watch(),
        Some("list") => print_list(&all, &done, current.as_deref()),
        Some("hint") => match current {
            Some(name) => println!("{BOLD}Hint for {name}{RESET}\n{}", hint(&name)),
            None => println!("all exercises are done"),
        },
        Some("reset") => {
            fs::remove_file(root().join(PROGRESS)).ok();
            println!("progress cleared, your exercise files are unchanged");
        }
        Some(other) => {
            eprintln!("unknown command {other:?}: use watch, list, hint or reset");
            std::process::exit(2);
        }
    }
}
