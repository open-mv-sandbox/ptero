//! Pterodactil implementation of the "Dacti Package" format.

use std::{
    fs::OpenOptions,
    io::{Seek, SeekFrom, Write},
    mem::size_of,
};

use anyhow::{Context as ContextExt, Error};
use bytemuck::{bytes_of, from_bytes, from_bytes_mut, Zeroable};
use dacti_index::{
    IndexEntry, IndexGroupEncoding, IndexGroupHeader, IndexHeader, INDEX_COMPONENT_UUID,
};
use daicon::{data::RegionData, ComponentEntry, ComponentTableHeader};
use ptero_daicon::{FileManagerCmd, GetComponentCmd, GetComponentResult};
use ptero_io::ReadWriteCmd;
use stewart::{ActorT, AddrT, After, Id, System};
use tracing::{event, instrument, Level};
use uuid::Uuid;

/// TODO: Change to actor
pub fn create_package(path: &str) -> Result<(), Error> {
    // Reserve 1kb for header and component table
    let indices_offset: u32 = 1024;

    // Open the target file, overwriting anything already there
    let mut package = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)
        .context("failed to open target package for writing")?;

    // Write the signature
    package.write_all(daicon::SIGNATURE)?;

    // Write the component table
    let mut header = ComponentTableHeader::zeroed();
    header.set_length(1);
    package.write_all(bytes_of(&header))?;

    let mut entry = ComponentEntry::zeroed();
    entry.set_type_id(INDEX_COMPONENT_UUID);

    let region = from_bytes_mut::<RegionData>(entry.data_mut());
    region.set_relative_offset(indices_offset);
    region.set_size(size_of::<IndexHeader>() as u32);

    package.write_all(bytes_of(&entry))?;

    // Write an empty indices table
    package.seek(SeekFrom::Start(indices_offset as u64))?;
    let header = IndexHeader::zeroed();
    package.write_all(bytes_of(&header))?;

    Ok(())
}

#[instrument("add-data", skip_all)]
pub fn start_add_data(system: &mut System, parent: Id, data: AddData) -> Result<(), Error> {
    event!(Level::DEBUG, "adding data to package");

    let info = system.create_actor(parent)?;
    system.start_actor(info, AddDataActor)?;

    // The first 64kb is reserved for components and indices
    // TODO: Actually find a free spot
    let data_start = 1024 * 64;
    let data_len = data.data.len() as u32;

    // Add the index for the file to the package
    let mut index_entry = IndexEntry::zeroed();
    index_entry.set_region_id(data.uuid);
    index_entry.set_offset(data_start as u32);
    index_entry.set_size(data_len);
    let add_index = AddIndex {
        file: data.file,
        file_manager: data.file_manager,
        value: index_entry,
    };
    AddIndexActor::start(system, info.id(), add_index)?;

    // Write the file to the package
    event!(Level::DEBUG, "writing file data to package");
    let write = ReadWriteCmd::Write {
        start: data_start,
        data: data.data,
    };
    system.handle(data.file, write);

    Ok(())
}

pub struct AddData {
    pub file: AddrT<ReadWriteCmd>,
    pub file_manager: AddrT<FileManagerCmd>,
    pub data: Vec<u8>,
    pub uuid: Uuid,
}

struct AddDataActor;

impl ActorT for AddDataActor {
    type Message = ();

    fn reduce(&mut self, _system: &mut System, _message: ()) -> Result<After, Error> {
        unimplemented!()
    }

    fn process(&mut self, _system: &mut System) -> Result<After, Error> {
        // TODO: Report success/failure back
        unimplemented!()
    }
}

struct AddIndex {
    file: AddrT<ReadWriteCmd>,
    file_manager: AddrT<FileManagerCmd>,
    value: IndexEntry,
}

struct AddIndexActor {
    message: Option<GetComponentResult>,
    file: AddrT<ReadWriteCmd>,
    value: IndexEntry,
}

impl AddIndexActor {
    fn start(system: &mut System, parent: Id, data: AddIndex) -> Result<(), Error> {
        let info = system.create_actor(parent)?;

        let cmd = GetComponentCmd {
            id: INDEX_COMPONENT_UUID,
            on_result: info.addr(),
        };
        let cmd = FileManagerCmd::GetComponent(cmd);
        system.handle(data.file_manager, cmd);

        let actor = Self {
            message: None,
            file: data.file,
            value: data.value,
        };
        system.start_actor(info, actor)?;

        Ok(())
    }
}

impl ActorT for AddIndexActor {
    type Message = GetComponentResult;

    fn reduce(
        &mut self,
        _system: &mut System,
        message: GetComponentResult,
    ) -> Result<After, Error> {
        self.message = Some(message);
        Ok(After::Process)
    }

    fn process(&mut self, system: &mut System) -> Result<After, Error> {
        let message = self.message.take().context("incorrect state")?;

        let region = from_bytes::<RegionData>(message.entry.data());
        let component_offset = region.offset(message.header.entries_offset());

        // TODO: Find a free slot rather than just assuming there's no groups and files yet
        // TODO: Update the component's size after adding the new index

        // Write the new table
        event!(Level::DEBUG, "writing index to package");
        let data = create_table_data(&self.value)?;
        let msg = ReadWriteCmd::Write {
            start: component_offset,
            data,
        };
        system.handle(self.file, msg);

        Ok(After::Stop)
    }
}

fn create_table_data(entry: &IndexEntry) -> Result<Vec<u8>, Error> {
    let mut data = Vec::new();

    // Find the current location of the index component
    let mut header = IndexHeader::zeroed();
    header.set_groups(1);
    data.write_all(bytes_of(&header))?;

    let mut group = IndexGroupHeader::zeroed();
    group.set_encoding(IndexGroupEncoding::None);
    group.set_length(1);
    data.write_all(bytes_of(&group))?;

    data.write_all(bytes_of(entry))?;

    Ok(data)
}
