//! Pterodactil implementation of the "Daicon" format.

mod manager;

use std::{
    io::{Cursor, Read},
    mem::size_of,
};

use anyhow::{bail, Context, Error};
use bytemuck::{bytes_of_mut, Zeroable};
use daicon::{ComponentEntry, ComponentTableHeader, SIGNATURE};
use ptero_io::{ReadResult, ReadResultF, ReadWriteCmd};
use stewart::{
    utils::{ActorT, AddrT},
    Actor, AfterProcess, AfterReduce, Id, Info, System,
};
use tracing::instrument;
use uuid::Uuid;

pub use self::manager::{start_file_manager, FileManagerCmd, FindComponentResult};

#[instrument("find-component", skip_all)]
fn start_find_component(
    system: &mut System,
    parent: Id,
    data: FindComponentData,
) -> Result<(), Error> {
    let info = system.create_actor(parent)?;

    // Start reading the header
    let read_header = ReadHeader {
        package: data.package,
        reply: info.addr(),
    };
    ReadHeaderActor::start(system, info.id(), read_header)?;

    let actor = FindComponentActor {
        info,
        queue: Vec::new(),
        data,
    };
    system.start_actor(info, actor)?;

    Ok(())
}

struct FindComponentData {
    pub target: Uuid,
    pub package: AddrT<ReadWriteCmd>,
    pub reply: AddrT<FindComponentResult>,
}

struct FindComponentActor {
    info: Info<Self>,
    queue: Vec<FindComponentMessage>,
    data: FindComponentData,
}

impl ActorT for FindComponentActor {
    type Message = FindComponentMessage;

    fn reduce(
        &mut self,
        _system: &mut System,
        message: FindComponentMessage,
    ) -> Result<AfterReduce, Error> {
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
                        reply: self.info.addr(),
                    };
                    ReadEntriesActor::start(system, self.info.id(), read_entries)?;

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
    package: AddrT<ReadWriteCmd>,
    reply: AddrT<FindComponentMessage>,
}

struct ReadHeaderActor {
    header: ComponentTableHeader,
    reply: AddrT<FindComponentMessage>,
}

impl ReadHeaderActor {
    fn start(system: &mut System, parent: Id, data: ReadHeader) -> Result<(), Error> {
        let info = system.create_actor(parent)?;

        let msg = ReadWriteCmd::Read {
            start: 0,
            length: (SIGNATURE.len() + size_of::<ComponentTableHeader>()) as u64,
            reply: info.addr(),
        };
        system.handle(data.package, msg);

        let actor = ReadHeaderActor {
            header: ComponentTableHeader::zeroed(),
            reply: data.reply,
        };
        system.start_actor(info, actor)?;

        Ok(())
    }
}

impl Actor for ReadHeaderActor {
    type Family = ReadResultF;

    fn reduce(&mut self, _system: &mut System, message: ReadResult) -> Result<AfterReduce, Error> {
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
    package: AddrT<ReadWriteCmd>,
    header_location: u64,
    header: ComponentTableHeader,
    reply: AddrT<FindComponentMessage>,
}

struct ReadEntriesActor {
    message: Option<Vec<u8>>,
    header: ComponentTableHeader,
    reply: AddrT<FindComponentMessage>,
}

impl ReadEntriesActor {
    fn start(system: &mut System, parent: Id, data: StartReadEntries) -> Result<(), Error> {
        let info = system.create_actor(parent)?;

        let msg = ReadWriteCmd::Read {
            start: data.header_location + size_of::<ComponentTableHeader>() as u64,
            length: (data.header.length() as usize * size_of::<ComponentEntry>()) as u64,
            reply: info.addr(),
        };
        system.handle(data.package, msg);

        let actor = ReadEntriesActor {
            message: None,
            header: data.header,
            reply: data.reply,
        };
        system.start_actor(info, actor)?;

        Ok(())
    }
}

impl Actor for ReadEntriesActor {
    type Family = ReadResultF;

    fn reduce(&mut self, _system: &mut System, message: ReadResult) -> Result<AfterReduce, Error> {
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
        let msg = FindComponentMessage::Entries(self.header, entries);
        system.handle(self.reply, msg);

        Ok(AfterProcess::Stop)
    }
}
