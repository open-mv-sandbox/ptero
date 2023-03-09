use std::{
    io::{Cursor, Read},
    mem::size_of,
};

use anyhow::{bail, Error};
use bytemuck::{bytes_of_mut, Zeroable};
use daicon::{ComponentEntry, ComponentTableHeader, SIGNATURE};
use ptero_io::{ReadResult, ReadResultF, ReadWriteCmd};
use stewart::{Actor, AddrT, After, Id, System};
use tracing::{event, instrument, Level};

use crate::manager::FileManagerMsg;

#[instrument("read-header", skip_all)]
pub fn start_read_header(
    system: &mut System,
    parent: Id,
    read_write: AddrT<ReadWriteCmd>,
    manager: AddrT<FileManagerMsg>,
) -> Result<(), Error> {
    event!(Level::INFO, "reading header");

    let info = system.create_actor(parent)?;

    let msg = ReadWriteCmd::Read {
        start: 0,
        length: (SIGNATURE.len() + size_of::<ComponentTableHeader>()) as u64,
        reply: info.addr(),
    };
    system.handle(read_write, msg);

    let actor = ReadHeaderActor { manager };
    system.start_actor(info, actor)?;

    Ok(())
}

struct ReadHeaderActor {
    manager: AddrT<FileManagerMsg>,
}

impl Actor for ReadHeaderActor {
    type Family = ReadResultF;

    fn reduce(&mut self, system: &mut System, message: ReadResult) -> Result<After, Error> {
        let data = message?;

        // Validate signature
        if &data[0..8] != SIGNATURE {
            bail!("invalid package signature");
        }

        // Copy the data
        let mut header = ComponentTableHeader::zeroed();
        bytes_of_mut(&mut header).copy_from_slice(&data[8..]);

        // Pass it to the manager
        system.handle(self.manager, FileManagerMsg::Header(header));

        Ok(After::Process)
    }

    fn process(&mut self, _system: &mut System) -> Result<After, Error> {
        Ok(After::Stop)
    }
}

#[instrument("read-entries", skip_all)]
pub fn start_read_entries(
    system: &mut System,
    parent: Id,
    read_write: AddrT<ReadWriteCmd>,
    start: u64,
    length: usize,
    manager: AddrT<FileManagerMsg>,
) -> Result<(), Error> {
    event!(Level::INFO, "reading entries");

    let info = system.create_actor(parent)?;

    let msg = ReadWriteCmd::Read {
        start,
        length: (length * size_of::<ComponentEntry>()) as u64,
        reply: info.addr(),
    };
    system.handle(read_write, msg);

    let actor = ReadEntriesActor { manager, length };
    system.start_actor(info, actor)?;

    Ok(())
}

struct ReadEntriesActor {
    manager: AddrT<FileManagerMsg>,
    length: usize,
}

impl Actor for ReadEntriesActor {
    type Family = ReadResultF;

    fn reduce(&mut self, system: &mut System, message: ReadResult) -> Result<After, Error> {
        let data = message?;

        let mut entries = Vec::new();
        let mut data = Cursor::new(data);

        for _ in 0..self.length {
            let mut entry = ComponentEntry::zeroed();
            data.read_exact(bytes_of_mut(&mut entry))?;
            entries.push(entry);
        }

        // Reply with the read data
        system.handle(self.manager, FileManagerMsg::Entries(entries));

        Ok(After::Process)
    }

    fn process(&mut self, _system: &mut System) -> Result<After, Error> {
        Ok(After::Stop)
    }
}
