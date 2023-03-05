//! Pterodactil implementation of the "Daicon" format.

pub mod io;

use std::{
    io::{Cursor, Read},
    mem::size_of,
};

use anyhow::{bail, Context, Error};
use bytemuck::{bytes_of_mut, Zeroable};
use daicon::{ComponentEntry, ComponentTableHeader, SIGNATURE};
use stewart::{
    utils::{ActorAddrT, ActorT},
    Actor, ActorAddr, AfterProcess, AfterReduce, Start, System,
};
use uuid::Uuid;

use crate::io::{ReadResult, ReadResultF, ReadWrite};

pub fn start_find_component(system: &mut System, data: FindComponent) {
    system.start::<FindComponentActor>(data);
}

pub struct FindComponent {
    pub target: Uuid,
    pub package: ActorAddrT<ReadWrite>,
    pub reply: ActorAddrT<FindComponentResult>,
}

pub struct FindComponentResult {
    pub header: ComponentTableHeader,
    pub entry: ComponentEntry,
}

struct FindComponentActor {
    queue: Vec<FindComponentMessage>,
    address: ActorAddrT<FindComponentMessage>,
    data: FindComponent,
}

impl Start for FindComponentActor {
    type Data = FindComponent;

    fn start(
        system: &mut System,
        address: ActorAddrT<FindComponentMessage>,
        data: FindComponent,
    ) -> Result<FindComponentActor, Error> {
        // Start reading the header
        let read_header = ReadHeader {
            package: data.package,
            reply: address,
        };
        system.start::<ReadHeaderActor>(read_header);

        Ok(FindComponentActor {
            queue: Vec::new(),
            address,
            data,
        })
    }
}

impl ActorT for FindComponentActor {
    type Message = FindComponentMessage;

    fn reduce(&mut self, message: FindComponentMessage) -> Result<AfterReduce, Error> {
        self.queue.push(message);
        Ok(AfterReduce::Process)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        let mut next = AfterProcess::Nothing;

        while let Some(message) = self.queue.pop() {
            match message {
                FindComponentMessage::Header(location, header) => {
                    let read_entries = StartReadEntries {
                        package: self.data.package,
                        header_location: location,
                        header,
                        reply: self.address,
                    };
                    system.start::<ReadEntriesActor>(read_entries);

                    // TODO: Follow extensions
                }
                FindComponentMessage::Entries(header, entries) => {
                    if let Some(entry) = entries
                        .into_iter()
                        .find(|e| e.type_id() == self.data.target)
                    {
                        let result = FindComponentResult { header, entry };
                        system.handle(self.data.reply, result);
                    } else {
                        // TODO: Better error reporting
                        bail!("unable to find component");
                    }

                    next = AfterProcess::Stop;
                }
            }
        }

        Ok(next)
    }
}

enum FindComponentMessage {
    Header(u64, ComponentTableHeader),
    Entries(ComponentTableHeader, Vec<ComponentEntry>),
}

struct ReadHeader {
    package: ActorAddrT<ReadWrite>,
    reply: ActorAddrT<FindComponentMessage>,
}

struct ReadHeaderActor {
    header: ComponentTableHeader,
    reply: ActorAddrT<FindComponentMessage>,
}

impl Start for ReadHeaderActor {
    type Data = ReadHeader;

    fn start(
        system: &mut System,
        address: ActorAddr<ReadResultF>,
        data: ReadHeader,
    ) -> Result<Self, Error> {
        let msg = ReadWrite::Read {
            start: 0,
            length: (SIGNATURE.len() + size_of::<ComponentTableHeader>()) as u64,
            reply: address,
        };
        system.handle(data.package, msg);

        Ok(ReadHeaderActor {
            header: ComponentTableHeader::zeroed(),
            reply: data.reply,
        })
    }
}

impl Actor for ReadHeaderActor {
    type Family = ReadResultF;

    fn reduce(&mut self, message: ReadResult) -> Result<AfterReduce, Error> {
        let data = message?;

        // Validate signature
        if &data[0..8] != SIGNATURE {
            bail!("invalid package signature");
        }

        // Copy the data
        bytes_of_mut(&mut self.header).copy_from_slice(&data[8..]);

        Ok(AfterReduce::Process)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        // Read the header data
        let header_location = 8;

        let msg = FindComponentMessage::Header(header_location, self.header);
        system.handle(self.reply, msg);

        Ok(AfterProcess::Stop)
    }
}

struct StartReadEntries {
    package: ActorAddrT<ReadWrite>,
    header_location: u64,
    header: ComponentTableHeader,
    reply: ActorAddrT<FindComponentMessage>,
}

struct ReadEntriesActor {
    message: Option<Vec<u8>>,
    header: ComponentTableHeader,
    reply: ActorAddrT<FindComponentMessage>,
}

impl Start for ReadEntriesActor {
    type Data = StartReadEntries;

    fn start(
        system: &mut System,
        address: ActorAddr<ReadResultF>,
        data: StartReadEntries,
    ) -> Result<Self, Error> {
        let msg = ReadWrite::Read {
            start: data.header_location + size_of::<ComponentTableHeader>() as u64,
            length: (data.header.length() as usize * size_of::<ComponentEntry>()) as u64,
            reply: address,
        };
        system.handle(data.package, msg);

        Ok(ReadEntriesActor {
            message: None,
            header: data.header,
            reply: data.reply,
        })
    }
}

impl Actor for ReadEntriesActor {
    type Family = ReadResultF;

    fn reduce(&mut self, message: ReadResult) -> Result<AfterReduce, Error> {
        self.message = Some(message?.to_vec());
        Ok(AfterReduce::Process)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        let data = self.message.take().context("incorrect state")?;

        let mut entries = Vec::new();
        let mut data = Cursor::new(data);

        for _ in 0..self.header.length() {
            let mut entry = ComponentEntry::zeroed();
            data.read_exact(bytes_of_mut(&mut entry))?;
            entries.push(entry);
        }

        // Reply with the read data
        let msg = FindComponentMessage::Entries(self.header.clone(), entries);
        system.handle(self.reply, msg);

        Ok(AfterProcess::Stop)
    }
}
