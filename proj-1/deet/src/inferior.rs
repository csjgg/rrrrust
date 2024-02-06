use nix::sys::ptrace;
use nix::sys::signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use nix::unistd::Pid;
use std::os::unix::process::CommandExt;
use std::process::Child;
use std::process::Command;
use crate::dwarf_data::DwarfData;
use std::mem::size_of;


pub enum Status {
    /// Indicates inferior stopped. Contains the signal that stopped the process, as well as the
    /// current instruction pointer that it is stopped at.
    Stopped(signal::Signal, usize),

    /// Indicates inferior exited normally. Contains the exit status code.
    Exited(i32),

    /// Indicates the inferior exited due to a signal. Contains the signal that killed the
    /// process.
    Signaled(signal::Signal),
}

/// This function calls ptrace with PTRACE_TRACEME to enable debugging on a process. You should use
/// pre_exec with Command to call this in the child process.
fn child_traceme() -> Result<(), std::io::Error> {
    ptrace::traceme().or(Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "ptrace TRACEME failed",
    )))
}

/// This function is used to wirte memory in the breakpoint command 
fn align_addr_to_word(addr: usize) -> usize {
    addr & (-(size_of::<usize>() as isize) as usize)
}

pub struct Inferior {
    child: Child,
}

impl Inferior {
    /// This function can wirte a byte in the memory of the inferior process
    fn write_byte(&mut self, addr: usize, val: u8) -> Result<u8, nix::Error> {
        let aligned_addr = align_addr_to_word(addr);
        let byte_offset = addr - aligned_addr;
        let word = ptrace::read(self.pid(), aligned_addr as ptrace::AddressType)? as u64;
        let orig_byte = (word >> 8 * byte_offset) & 0xff;
        let masked_word = word & !(0xff << 8 * byte_offset);
        let updated_word = masked_word | ((val as u64) << 8 * byte_offset);
        ptrace::write(
            self.pid(),
            aligned_addr as ptrace::AddressType,
            updated_word as *mut std::ffi::c_void,
        )?;
        Ok(orig_byte as u8)
    }

    /// Attempts to start a new inferior process. Returns Some(Inferior) if successful, or None if
    /// an error is encountered.
    pub fn new(target: &str, args: &Vec<String>, breakpoints: &Vec<usize>) -> Option<Inferior> {
        let mut binding = Command::new(target);
        let cmd = binding.args(args);
        unsafe {
            cmd.pre_exec(child_traceme);
        }
        let child = cmd.spawn().ok()?;
        let mut inferior = Inferior { child };
        let result = inferior.wait(None).ok()?;
        match result {
            Status::Stopped(signal, _) => {
                match signal {
                    signal::SIGTRAP => {
                        for bp in breakpoints{
                            inferior.write_byte(*bp,0xcc).ok()?;
                        }
                        Some(inferior)
                    }
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Kill the inferior process.
    pub fn kill(&mut self) {
        self.child.kill().expect("Error killing inferior");
        self.child.wait().expect("Error waiting for inferior");
    }

    /// Returns the pid of this inferior.
    pub fn pid(&self) -> Pid {
        nix::unistd::Pid::from_raw(self.child.id() as i32)
    }

    /// Make process continue
    pub fn cont(&self) -> Result<Status, nix::Error> {
        ptrace::cont(self.pid(), None)?;
        self.wait(None)
    }
    
    /// Calls waitpid on this inferior and returns a Status to indicate the state of the process
    /// after the waitpid call.
    pub fn wait(&self, options: Option<WaitPidFlag>) -> Result<Status, nix::Error> {
        Ok(match waitpid(self.pid(), options)? {
            WaitStatus::Exited(_pid, exit_code) => Status::Exited(exit_code),
            WaitStatus::Signaled(_pid, signal, _core_dumped) => Status::Signaled(signal),
            WaitStatus::Stopped(_pid, signal) => {
                let regs = ptrace::getregs(self.pid())?;
                Status::Stopped(signal, regs.rip as usize)
            }
            other => panic!("waitpid returned unexpected status: {:?}", other),
        })
    }
    pub fn print_backtrace(&self,debug_data: &DwarfData) -> Result<(), nix::Error>{
        let regs = ptrace::getregs(self.pid())?;
        println!("%rip register: {:#x}", regs.rip);
        let mut base_ptr = regs.rbp as usize;
        let mut instruction_ptr = regs.rip as usize;
        loop{
            let line = match debug_data.get_line_from_addr(instruction_ptr){
                Some(line) => line,
                None => {
                    println!("No line information found");
                    return Ok(());
                }
            };
            let func = match debug_data.get_function_from_addr(instruction_ptr){
                Some(func) => func,
                None => {
                    println!("No function information found");
                    return Ok(());
                }
            };
            println!("{} ({}:{})",func,line.file,line.number);
            if func == "main"{
                break;
            }
            instruction_ptr = ptrace::read(self.pid(),(base_ptr+8) as ptrace::AddressType)? as usize;
            base_ptr = ptrace::read(self.pid(),base_ptr as ptrace::AddressType)? as usize;
        }
        Ok(())
    }
}
