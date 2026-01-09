//! Command parsing module for Q-MIND
//!
//! Parses natural language commands into structured file operations
//! using LLM chat completions.

mod executor;
mod parser;

pub use executor::{CommandExecutor, ExecutionResult};
pub use parser::{CommandAction, CommandParser, ParsedCommand};
