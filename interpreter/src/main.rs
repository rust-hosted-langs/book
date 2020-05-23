extern crate blockalloc;
extern crate clap;
extern crate dirs;
extern crate fnv;
extern crate itertools;
extern crate num;
#[macro_use]
extern crate num_derive;
extern crate rustyline;
extern crate stickyimmix;

use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::process;

use clap::{App, Arg};

use rustyline::error::ReadlineError;
use rustyline::Editor;

mod arena;
mod array;
mod bytecode;
mod compiler;
mod containers;
mod dict;
mod error;
mod function;
mod hashable;
mod headers;
mod lexer;
mod list;
mod memory;
mod number;
mod pair;
mod parser;
mod pointerops;
mod printer;
mod rawarray;
mod repl;
mod safeptr;
mod symbol;
mod symbolmap;
mod taggedptr;
mod text;
mod vm;

use crate::error::RuntimeError;
use crate::memory::Memory;
use crate::repl::RepMaker;

/// Read a file into a String
fn load_file(filename: &str) -> Result<String, io::Error> {
    let mut contents = String::new();

    File::open(filename)?.read_to_string(&mut contents)?;

    Ok(contents)
}

/// Read and evaluate an entire file
fn read_file(filename: &str) -> Result<(), RuntimeError> {
    let _contents = load_file(&filename)?;

    // TODO

    Ok(())
}

/// Read a line at a time, printing the input back out
fn read_print_loop() -> Result<(), RuntimeError> {
    // establish a repl input history file path
    let history_file = match dirs::home_dir() {
        Some(mut path) => {
            path.push(".evalrus_history");
            Some(String::from(path.to_str().unwrap()))
        }
        None => None,
    };

    // () means no completion support (TODO)
    // Another TODO - find a more suitable alternative to rustyline
    let mut reader = Editor::<()>::new();

    // Try to load the repl history file
    if let Some(ref path) = history_file {
        if let Err(err) = reader.load_history(&path) {
            eprintln!("Could not read history: {}", err);
        }
    }

    let mem = Memory::new();
    let rep_maker = RepMaker {};
    let rep = mem.mutate(&rep_maker, ())?;

    // repl
    loop {
        let readline = reader.readline("> ");

        match readline {
            // valid input
            Ok(line) => {
                reader.add_history_entry(&line);
                mem.mutate(&rep, line)?;
            }

            // some kind of program termination condition
            Err(e) => {
                if let Some(ref path) = history_file {
                    reader.save_history(&path).unwrap_or_else(|err| {
                        eprintln!("could not save input history in {}: {}", path, err);
                    });
                }

                // EOF is fine
                if let ReadlineError::Eof = e {
                    return Ok(());
                } else {
                    return Err(RuntimeError::from(e));
                }
            }
        }
    }
}

fn main() {
    // parse command line argument, an optional filename
    let matches = App::new("Eval-R-Us")
        .about("Evaluate expressions")
        .arg(
            Arg::with_name("filename")
                .help("Optional filename to read in")
                .index(1),
        )
        .get_matches();

    if let Some(filename) = matches.value_of("filename") {
        // if a filename was specified, read it into a String
        read_file(filename).unwrap_or_else(|err| {
            eprintln!("Terminated: {}", err);
            process::exit(1);
        });
    } else {
        // otherwise begin a repl
        read_print_loop().unwrap_or_else(|err| {
            eprintln!("Terminated: {}", err);
            process::exit(1);
        });
    }
}
