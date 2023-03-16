use std::{
    io::{Cursor, Read as IoRead},
    mem::size_of,
};

use anyhow::{bail, Error};
use bytemuck::{bytes_of_mut, Zeroable};
use daicon::{ComponentEntry, ComponentTableHeader, SIGNATURE};
use ptero_io::{Read, ReadResult};
use stewart::{Actor, Addr, After, Id, Options, System};
use tracing::{event, instrument, Level};

use crate::manager::Message;

#[instrument("read-header", skip_all)]
pub fn start_read_header(
    system: &mut System,
    parent: Id,
    read: Addr<Read>,
    manager: Addr<Message>,
) -> Result<(), Error> {
    event!(Level::DEBUG, "reading header");

    let info = system.create(parent)?;

    let msg = Read {
        start: 0,
        length: (SIGNATURE.len() + size_of::<ComponentTableHeader>()) as u64,
        on_result: info.addr(),
    };
    system.send(read, msg);

    let actor = ReadHeaderActor { manager };
    system.start(info, Options::default(), actor)?;

    Ok(())
}

struct ReadHeaderActor {
    manager: Addr<Message>,
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
        system.send(self.manager, Message::Header(header));

        Ok(After::Stop)
    }
}

#[instrument("read-entries", skip_all)]
pub fn start_read_entries(
    system: &mut System,
    parent: Id,
    read: Addr<Read>,
    start: u64,
    length: usize,
    manager: Addr<Message>,
) -> Result<(), Error> {
    event!(Level::DEBUG, "reading entries");

    let info = system.create(parent)?;

    let msg = Read {
        start,
        length: (length * size_of::<ComponentEntry>()) as u64,
        on_result: info.addr(),
    };
    system.send(read, msg);

    let actor = ReadEntriesActor { manager, length };
    system.start(info, Options::high_priority(), actor)?;

    Ok(())
}

struct ReadEntriesActor {
    manager: Addr<Message>,
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
        system.send(self.manager, Message::Entries(entries));

        Ok(After::Stop)
    }
}
