pub enum DebuggerCommand {
    Quit,
    Run(Vec<String>),
    Contin,
    Backtrace,
    Breakpoint(String),
}

impl DebuggerCommand {
    pub fn from_tokens(tokens: &Vec<&str>) -> Option<DebuggerCommand> {
        match tokens[0] {
            "q" | "quit" => Some(DebuggerCommand::Quit),
            "r" | "run" => {
                let args = tokens[1..].to_vec();
                Some(DebuggerCommand::Run(
                    args.iter().map(|s| s.to_string()).collect(),
                ))
            }
            "c" | "continue" => Some(DebuggerCommand::Contin),
            "bt" | "backtrace" | "back" => Some(DebuggerCommand::Backtrace),
            "b" | "break" => {
                if tokens.len() < 2 {
                    println!("No breakpoint specified");
                    None
                } else {
                    Some(DebuggerCommand::Breakpoint(tokens[1].to_string()))
                }
            }
            // Default case:
            _ => None,
        }
    }
}
