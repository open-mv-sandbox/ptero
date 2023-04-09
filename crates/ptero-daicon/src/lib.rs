//! Pterodactil implementation of the "daicon" format.

mod file;
mod set;

use std::{fs::OpenOptions, io::Write};

use anyhow::{Context, Error};
use bytemuck::{bytes_of, Zeroable};
use daicon::{Entry, Header};
use ptero_file::ReadResult;
use stewart::Addr;
use uuid::Uuid;

pub use self::file::start_file_source_service;

pub struct SourceMessage {
    pub id: Uuid,
    pub action: SourceAction,
}

pub enum SourceAction {
    /// Get the data associated with a UUID.
    Get {
        id: Uuid,
        /// TODO: Reply with an inner file actor Addr instead.
        on_result: Addr<ReadResult>,
    },
    /// Set the data associated with a UUID.
    Set {
        id: Uuid,
        data: Vec<u8>,
        on_result: Addr<()>,
    },
}

/// TODO: Restructure to use actors
pub fn create_package(path: &str) -> Result<(), Error> {
    // Open the target file, overwriting anything already there
    let mut package = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)
        .context("failed to open target package for writing")?;

    // Write the component table
    let mut header = Header::default();
    header.set_capacity(256);
    package.write_all(bytes_of(&header))?;

    // Write an empty entries table
    for _ in 0..256 {
        let entry = Entry::zeroed();
        package.write_all(bytes_of(&entry))?;
    }

    Ok(())
}
