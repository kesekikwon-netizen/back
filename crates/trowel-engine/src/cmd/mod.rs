//! Trowel's commands, as the shell hands them to the engine at startup.
//!
//! A command here is a [`vectorcraft_engine::cmd::CommandSpec`] like any built-in one — same
//! shape, same enablement, same journal and undo guard, reachable from the control channel and
//! the MCP. What differs is only *where the list lives*: the engine's registry is a static list
//! of VectorCraft interface strings that every language catalog has to translate, and Trowel's
//! archaeological vocabulary is not VectorCraft vocabulary. So `apps/trowel` calls
//! [`Session::add_commands`] once at launch.
//!
//! See [`legend`] for the rule these exist to carry: **one legend line is one layer**.

pub mod legend;
