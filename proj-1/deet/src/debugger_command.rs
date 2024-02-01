pub enum DebuggerCommand {
    Quit,
    Run(Vec<String>),
    Contin,
    Backtrace,
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
            // Default case:
            _ => None,
        }
    }
}
