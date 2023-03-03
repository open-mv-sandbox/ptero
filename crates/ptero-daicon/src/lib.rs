//! Pterodactil implementation of the "Daicon" format.

pub mod io;

use std::{
    io::{Cursor, Read},
    mem::size_of,
};

use anyhow::{bail, Error};
use daicon::{ComponentEntry, ComponentTableHeader, SIGNATURE};
use io::ReadResult;
use stewart::{Process, HandlerId, Context, Factory, Next};
use uuid::Uuid;

use crate::io::ReadWrite;

#[derive(Factory)]
#[factory(FindComponentActor::start)]
pub struct FindComponent {
    pub target: Uuid,
    pub package: HandlerId<ReadWrite>,
    pub reply: HandlerId<FindComponentResult>,
}

pub struct FindComponentResult {
    pub header: ComponentTableHeader,
    pub entry: ComponentEntry,
}

struct FindComponentActor {
    address: HandlerId<FindComponentMessage>,
    data: FindComponent,
}

impl FindComponentActor {
    fn start(
        ctx: &dyn Context,
        address: HandlerId<FindComponentMessage>,
        data: FindComponent,
    ) -> Result<FindComponentActor, Error> {
        // Start reading the header
        let read_header = ReadHeader {
            package: data.package.clone(),
            reply: address,
        };
        ctx.start(read_header);

        Ok(FindComponentActor { address, data })
    }
}

impl Process for FindComponentActor {
    type Message = FindComponentMessage;

    fn handle(&mut self, ctx: &dyn Context, message: FindComponentMessage) -> Result<Next, Error> {
        let next = match message {
            FindComponentMessage::Header(location, header) => {
                let read_entries = StartReadEntries {
                    package: self.data.package.clone(),
                    header_location: location,
                    header,
                    reply: self.address,
                };
                ctx.start(read_entries);

                // TODO: Follow extensions

                Next::Continue
            }
            FindComponentMessage::Entries(header, entries) => {
                if let Some(entry) = entries
                    .into_iter()
                    .find(|e| e.type_id() == self.data.target)
                {
                    let result = FindComponentResult { header, entry };
                    ctx.send(self.data.reply, result);
                } else {
                    // TODO: Better error reporting
                    bail!("unable to find component");
                }

                Next::Stop
            }
        };

        Ok(next)
    }
}

enum FindComponentMessage {
    Header(u64, ComponentTableHeader),
    Entries(ComponentTableHeader, Vec<ComponentEntry>),
}

#[derive(Factory)]
#[factory(ReadHeaderActor::start)]
struct ReadHeader {
    package: HandlerId<ReadWrite>,
    reply: HandlerId<FindComponentMessage>,
}

struct ReadHeaderActor {
    reply: HandlerId<FindComponentMessage>,
}

impl ReadHeaderActor {
    fn start(
        ctx: &dyn Context,
        address: HandlerId<ReadResult>,
        data: ReadHeader,
    ) -> Result<Self, Error> {
        let msg = ReadWrite::Read {
            start: 0,
            length: (SIGNATURE.len() + size_of::<ComponentTableHeader>()) as u64,
            reply: address,
        };
        ctx.send(data.package, msg);

        Ok(ReadHeaderActor { reply: data.reply })
    }
}

impl Process for ReadHeaderActor {
    type Message = ReadResult;

    fn handle(&mut self, ctx: &dyn Context, message: ReadResult) -> Result<Next, Error> {
        let data = message?;

        // Validate signature
        if &data[0..8] != SIGNATURE {
            bail!("invalid package signature");
        }

        // Read the header data
        let header_location = 8;
        let header = ComponentTableHeader::from_bytes(&data[8..]).clone();

        let msg = FindComponentMessage::Header(header_location, header);
        ctx.send(self.reply, msg);

        Ok(Next::Stop)
    }
}

#[derive(Factory)]
#[factory(ReadEntriesActor::start)]
struct StartReadEntries {
    package: HandlerId<ReadWrite>,
    header_location: u64,
    header: ComponentTableHeader,
    reply: HandlerId<FindComponentMessage>,
}

struct ReadEntriesActor {
    header: ComponentTableHeader,
    reply: HandlerId<FindComponentMessage>,
}

impl ReadEntriesActor {
    fn start(
        ctx: &dyn Context,
        address: HandlerId<ReadResult>,
        data: StartReadEntries,
    ) -> Result<Self, Error> {
        let msg = ReadWrite::Read {
            start: data.header_location + ComponentTableHeader::bytes_len() as u64,
            length: (data.header.length() as usize * size_of::<ComponentEntry>()) as u64,
            reply: address,
        };
        ctx.send(data.package, msg);

        Ok(ReadEntriesActor {
            header: data.header,
            reply: data.reply,
        })
    }
}

impl Process for ReadEntriesActor {
    type Message = ReadResult;

    fn handle(&mut self, ctx: &dyn Context, message: ReadResult) -> Result<Next, Error> {
        let data = message?;

        let mut entries = Vec::new();
        let mut data = Cursor::new(data);

        // TODO: Direct cast?
        for _ in 0..self.header.length() {
            let mut entry = ComponentEntry::zeroed();
            data.read_exact(&mut entry)?;
            entries.push(entry);
        }

        // Reply with the read data
        let msg = FindComponentMessage::Entries(self.header.clone(), entries);
        ctx.send(self.reply, msg);

        Ok(Next::Stop)
    }
}