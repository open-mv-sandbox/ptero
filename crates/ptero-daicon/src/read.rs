use std::{
    io::{Cursor, Read},
    mem::size_of,
};

use anyhow::{bail, Error};
use bytemuck::{bytes_of_mut, Zeroable};
use daicon::{ComponentEntry, ComponentTableHeader, SIGNATURE};
use ptero_io::{ReadResult, ReadResultF, ReadWriteCmd};
use stewart::{
    handler::{Handler, Sender, SenderT},
    After, Id, System,
};
use tracing::{event, instrument, Level};

use crate::manager::FileManagerMsg;

#[instrument("read-header", skip_all)]
pub fn start_read_header(
    system: &mut System,
    parent: Option<Id>,
    read_write: SenderT<ReadWriteCmd>,
    manager: SenderT<FileManagerMsg>,
) -> Result<(), Error> {
    event!(Level::INFO, "reading header");

    let info = system.create_actor(parent)?;

    let msg = ReadWriteCmd::Read {
        start: 0,
        length: (SIGNATURE.len() + size_of::<ComponentTableHeader>()) as u64,
        on_result: Sender::new(info),
    };
    read_write.send(system, msg);

    let actor = ReadHeaderActor { manager };
    system.start_actor(info, actor)?;

    Ok(())
}

struct ReadHeaderActor {
    manager: SenderT<FileManagerMsg>,
}

impl Handler for ReadHeaderActor {
    type Family = ReadResultF;

    fn handle(&mut self, system: &mut System, message: ReadResult) -> Result<After, Error> {
        let data = message?;

        // Validate signature
        if &data[0..8] != SIGNATURE {
            bail!("invalid package signature");
        }

        // Copy the data
        let mut header = ComponentTableHeader::zeroed();
        bytes_of_mut(&mut header).copy_from_slice(&data[8..]);

        // Pass it to the manager
        self.manager.send(system, FileManagerMsg::Header(header));

        Ok(After::Stop)
    }
}

#[instrument("read-entries", skip_all)]
pub fn start_read_entries(
    system: &mut System,
    parent: Option<Id>,
    read_write: SenderT<ReadWriteCmd>,
    start: u64,
    length: usize,
    manager: SenderT<FileManagerMsg>,
) -> Result<(), Error> {
    event!(Level::INFO, "reading entries");

    let info = system.create_actor(parent)?;

    let msg = ReadWriteCmd::Read {
        start,
        length: (length * size_of::<ComponentEntry>()) as u64,
        on_result: Sender::new(info),
    };
    read_write.send(system, msg);

    let actor = ReadEntriesActor { manager, length };
    system.start_actor(info, actor)?;

    Ok(())
}

struct ReadEntriesActor {
    manager: SenderT<FileManagerMsg>,
    length: usize,
}

impl Handler for ReadEntriesActor {
    type Family = ReadResultF;

    fn handle(&mut self, system: &mut System, message: ReadResult) -> Result<After, Error> {
        let data = message?;

        let mut entries = Vec::new();
        let mut data = Cursor::new(data);

        for _ in 0..self.length {
            let mut entry = ComponentEntry::zeroed();
            data.read_exact(bytes_of_mut(&mut entry))?;
            entries.push(entry);
        }

        // Reply with the read data
        self.manager.send(system, FileManagerMsg::Entries(entries));

        Ok(After::Stop)
    }
}
