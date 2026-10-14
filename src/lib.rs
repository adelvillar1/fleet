//! Fleet's library face: the DeepSeek API client, the tool set, the agent
//! loop, session logs, command parsing, the MCP stdio client with its lane
//! registry, the git-tracked memory substrate with hybrid retrieval, the
//! self-maintenance loop's gated proposals, the three read-only boards
//! over the machine's own sources, the explanation beat over the blend1
//! lane, the TUI, and the doctor's checks.

pub mod agent;
pub mod api;
pub mod boards;
pub mod calib;
pub mod cli;
pub mod compaction;
pub mod cron;
pub mod doctor;
pub mod eval;
pub mod explain;
pub mod home;
pub mod improve;
pub mod install;
pub mod lanes;
pub mod mcp;
pub mod memory;
pub mod net;
pub mod route;
pub mod session;
pub mod skills;
pub mod subagents;
pub mod tools;
pub mod tui;
