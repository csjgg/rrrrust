use crate::dwarf_data::{DwarfData, Error as DwarfError};
use crate::inferior::Inferior;
use crate::{debugger_command::DebuggerCommand, inferior::Status};
use rustyline::error::ReadlineError;
use rustyline::Editor;
use Status::{Exited, Signaled, Stopped};

pub struct Debugger {
    target: String,
    history_path: String,
    readline: Editor<()>,
    inferior: Option<Inferior>,
    debug_data: DwarfData,
    break_point: Vec<usize>,
}

impl Debugger {
    /// Initializes the debugger.
    pub fn new(target: &str) -> Debugger {
        let debug_data = match DwarfData::from_file(target) {
            Ok(val) => val,
            Err(DwarfError::ErrorOpeningFile) => {
                println!("Could not open file {}", target);
                std::process::exit(1);
            }
            Err(DwarfError::DwarfFormatError(err)) => {
                println!("Could not debugging symbols from {}: {:?}", target, err);
                std::process::exit(1);
            }
        };
        let history_path = format!("{}/.deet_history", std::env::var("HOME").unwrap());
        let mut readline = Editor::<()>::new();
        // Attempt to load history from ~/.deet_history if it exists
        let _ = readline.load_history(&history_path);
        debug_data.print();
        Debugger {
            target: target.to_string(),
            history_path,
            readline,
            inferior: None,
            debug_data,
            break_point: Vec::new(),
        }
    }

    fn contin(&mut self) {
        let inf = match &mut self.inferior {
            Some(inf) => inf,
            None => {
                println!("No child process now");
                return;
            }
        };
        let re = inf.cont().expect("Error continuing inferior");
        match re {
            Stopped(signal, reg) => {
                println!("Child stopped (signal {})", signal);
                let func = match self.debug_data.get_function_from_addr(reg) {
                    Some(func) => func,
                    None => return,
                };
                let line = match self.debug_data.get_line_from_addr(reg) {
                    Some(line) => line,
                    None => return,
                };
                println!("Stop at {} ({}:{})", func, line.file, line.number);
            }
            Exited(code) => {
                println!("Child exited (status {})", code);
                self.inferior = None;
            }
            Signaled(signal) => {
                println!("Child exited (signal {})", signal);
                self.inferior = None;
            }
        }
    }

    fn parse_address(addr: &str) -> Option<usize> {
        let addr_without_0x = if addr.to_lowercase().starts_with("0x") {
            &addr[2..]
        } else {
            &addr
        };
        usize::from_str_radix(addr_without_0x, 16).ok()
    }

    fn insert_bp(&mut self, addr:usize) {
        self.break_point.push(addr);
        println!("Breakpoint at 0x{:x}", addr);
        let inf = match &mut self.inferior {
            Some(inf) => inf,
            None => {
                return;
            }
        };
        let _ = inf.insert_breakpoint(addr);
    }

    pub fn run(&mut self) {
        loop {
            match self.get_next_command() {
                DebuggerCommand::Run(args) => {
                    if let Some(inferior) = Inferior::new(&self.target, &args, &self.break_point) {
                        // Create the inferior
                        match &mut self.inferior {
                            Some(inf) => {
                                inf.kill();
                            }
                            None => {}
                        }
                        self.inferior = Some(inferior);
                        self.contin();
                    } else {
                        println!("Error starting subprocess");
                    }
                }
                DebuggerCommand::Contin => self.contin(),
                DebuggerCommand::Backtrace => match &self.inferior {
                    Some(inf) => {
                        let _ = inf.print_backtrace(&self.debug_data);
                    }
                    None => {
                        println!("No child process now");
                    }
                },
                DebuggerCommand::Quit => {
                    match self.inferior {
                        Some(ref mut inf) => {
                            inf.kill();
                        }
                        None => {}
                    }
                    return;
                }
                DebuggerCommand::Breakpoint(breakpoint) => {
                    let addr;
                    if breakpoint.starts_with('*') {
                        addr = Debugger::parse_address(&breakpoint[1..]);
                    }else {
                        let linenum:usize;
                        match breakpoint.parse::<usize>(){
                            Ok(num)=> {
                                linenum = num;
                                addr = self.debug_data.get_addr_for_line(None,linenum);

                            }
                            Err(_)=>{
                                addr = self.debug_data.get_addr_for_function(None,&breakpoint);
                            }
                        }
                    }
                    match addr {
                        Some(addr) => {
                            self.insert_bp(addr);
                        }
                        None => {
                            println!("Breakpoint on Invalid address");
                        }
                    }   
                }
            }
        }
    }

    /// This function prompts the user to enter a command, and continues re-prompting until the user
    /// enters a valid command. It uses DebuggerCommand::from_tokens to do the command parsing.
    ///
    /// You don't need to read, understand, or modify this function.
    fn get_next_command(&mut self) -> DebuggerCommand {
        loop {
            // Print prompt and get next line of user input
            match self.readline.readline("(deet) ") {
                Err(ReadlineError::Interrupted) => {
                    // User pressed ctrl+c. We're going to ignore it
                    println!("Type \"quit\" to exit");
                }
                Err(ReadlineError::Eof) => {
                    // User pressed ctrl+d, which is the equivalent of "quit" for our purposes
                    return DebuggerCommand::Quit;
                }
                Err(err) => {
                    panic!("Unexpected I/O error: {:?}", err);
                }
                Ok(line) => {
                    if line.trim().len() == 0 {
                        continue;
                    }
                    self.readline.add_history_entry(line.as_str());
                    if let Err(err) = self.readline.save_history(&self.history_path) {
                        println!(
                            "Warning: failed to save history file at {}: {}",
                            self.history_path, err
                        );
                    }
                    let tokens: Vec<&str> = line.split_whitespace().collect();
                    if let Some(cmd) = DebuggerCommand::from_tokens(&tokens) {
                        return cmd;
                    } else {
                        println!("Unrecognized command.");
                    }
                }
            }
        }
    }
}
