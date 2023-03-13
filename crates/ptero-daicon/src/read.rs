use std::{
    io::{Cursor, Read},
    mem::size_of,
};

use anyhow::{bail, Error};
use bytemuck::{bytes_of_mut, Zeroable};
use daicon::{ComponentEntry, ComponentTableHeader, SIGNATURE};
use ptero_io::{ReadResult, ReadWriteCmd};
use stewart::{Actor, Addr, After, Id, System};
use tracing::{event, instrument, Level};

use crate::manager::FileManagerMessage;

#[instrument("read-header", skip_all)]
pub fn start_read_header(
    system: &mut System,
    parent: Id,
    read_write: Addr<ReadWriteCmd>,
    manager: Addr<FileManagerMessage>,
) -> Result<(), Error> {
    event!(Level::DEBUG, "reading header");

    let info = system.create_actor(parent)?;

    let msg = ReadWriteCmd::Read {
        start: 0,
        length: (SIGNATURE.len() + size_of::<ComponentTableHeader>()) as u64,
        on_result: info.addr(),
    };
    system.send(read_write, msg);

    let actor = ReadHeaderActor { manager };
    system.start_actor(info, actor)?;

    Ok(())
}

struct ReadHeaderActor {
    manager: Addr<FileManagerMessage>,
}

impl Actor for ReadHeaderActor {
    type Message = ReadResult;

    fn handle(&mut self, system: &mut System, message: ReadResult) -> Result<After, Error> {
        let data = message;

        // Validate signature
        if &data[0..8] != SIGNATURE {
            bail!("invalid package signature");
        }

        // Copy the data
        let mut header = ComponentTableHeader::zeroed();
        bytes_of_mut(&mut header).copy_from_slice(&data[8..]);

        // Pass it to the manager
        system.send(self.manager, FileManagerMessage::Header(header));

        Ok(After::Stop)
    }
}

#[instrument("read-entries", skip_all)]
pub fn start_read_entries(
    system: &mut System,
    parent: Id,
    read_write: Addr<ReadWriteCmd>,
    start: u64,
    length: usize,
    manager: Addr<FileManagerMessage>,
) -> Result<(), Error> {
    event!(Level::DEBUG, "reading entries");

    let info = system.create_actor(parent)?;

    let msg = ReadWriteCmd::Read {
        start,
        length: (length * size_of::<ComponentEntry>()) as u64,
        on_result: info.addr(),
    };
    system.send(read_write, msg);

    let actor = ReadEntriesActor { manager, length };
    system.start_actor(info, actor)?;

    Ok(())
}

struct ReadEntriesActor {
    manager: Addr<FileManagerMessage>,
    length: usize,
}

impl Actor for ReadEntriesActor {
    type Message = ReadResult;

    fn handle(&mut self, system: &mut System, message: ReadResult) -> Result<After, Error> {
        let data = message;

        let mut entries = Vec::new();
        let mut data = Cursor::new(data);

        for _ in 0..self.length {
            let mut entry = ComponentEntry::zeroed();
            data.read_exact(bytes_of_mut(&mut entry))?;
            entries.push(entry);
        }

        // Reply with the read data
        system.send(self.manager, FileManagerMessage::Entries(entries));

        Ok(After::Stop)
    }
}
