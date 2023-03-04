//! Pterodactil implementation of the "Dacti Package" format.

use std::{
    fs::OpenOptions,
    io::{Seek, SeekFrom, Write},
};

use anyhow::{Context as ContextExt, Error};
use dacti_index::{
    IndexEntry, IndexGroupEncoding, IndexGroupHeader, IndexHeader, INDEX_COMPONENT_UUID,
};
use daicon::{data::RegionData, ComponentEntry, ComponentTableHeader};
use ptero_daicon::{io::ReadWrite, start_find_component, FindComponent, FindComponentResult};
use stewart::{utils::Unreachable, Actor, ActorAddr, AfterProcess, AfterReduce, Start, System};
use tracing::{event, Level};
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
    package.write_all(&header)?;

    let mut entry = ComponentEntry::zeroed();
    entry.set_type_id(INDEX_COMPONENT_UUID);

    let region = RegionData::from_bytes_mut(entry.data_mut());
    region.set_relative_offset(indices_offset);
    region.set_size(IndexHeader::bytes_len() as u32);

    package.write_all(&entry)?;

    // Write an empty indices table
    package.seek(SeekFrom::Start(indices_offset as u64))?;
    let header = IndexHeader::zeroed();
    package.write_all(&header)?;

    Ok(())
}

pub fn start_add_data(system: &mut System, data: AddData) {
    system.start::<AddDataActor>(data);
}

pub struct AddData {
    pub package: ActorAddr<ReadWrite>,
    pub data: Vec<u8>,
    pub uuid: Uuid,
}

struct AddDataActor;

impl Start for AddDataActor {
    type Data = AddData;

    fn start(
        system: &mut System,
        _addr: ActorAddr<Unreachable>,
        data: AddData,
    ) -> Result<Self, Error> {
        event!(Level::DEBUG, "adding data to package");

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
            package: data.package.clone(),
            value: index_entry,
        };
        system.start::<AddIndexActor>(add_index);

        // Write the file to the package
        let write = ReadWrite::Write {
            start: data_start,
            data: data.data,
        };
        system.handle(data.package, write);

        Ok(AddDataActor)
    }
}

impl Actor for AddDataActor {
    type Protocol = Unreachable;

    fn reduce(&mut self, _message: Unreachable) -> Result<AfterReduce, Error> {
        unimplemented!()
    }

    fn process(&mut self, _system: &mut System) -> Result<AfterProcess, Error> {
        // TODO: Report success/failure back
        unimplemented!()
    }
}

struct AddIndex {
    package: ActorAddr<ReadWrite>,
    value: IndexEntry,
}

struct AddIndexActor {
    message: Option<FindComponentResult>,
    package: ActorAddr<ReadWrite>,
    value: IndexEntry,
}

impl Start for AddIndexActor {
    type Data = AddIndex;

    fn start(
        system: &mut System,
        addr: ActorAddr<FindComponentResult>,
        data: AddIndex,
    ) -> Result<Self, Error> {
        let find_component = FindComponent {
            target: INDEX_COMPONENT_UUID,
            package: data.package.clone(),
            reply: addr,
        };
        start_find_component(system, find_component);

        Ok(Self {
            message: None,
            package: data.package,
            value: data.value,
        })
    }
}

impl Actor for AddIndexActor {
    type Protocol = FindComponentResult;

    fn reduce(&mut self, message: FindComponentResult) -> Result<AfterReduce, Error> {
        self.message = Some(message);
        Ok(AfterReduce::Process)
    }

    fn process(&mut self, system: &mut System) -> Result<AfterProcess, Error> {
        let message = self.message.take().context("incorrect state")?;

        let region = RegionData::from_bytes(message.entry.data());
        let component_offset = region.offset(message.header.entries_offset());

        // TODO: Find a free slot rather than just assuming there's no groups and files yet
        // TODO: Update the component's size after adding the new index

        // Write the new table
        let data = create_table_data(&self.value)?;
        let msg = ReadWrite::Write {
            start: component_offset,
            data,
        };
        system.handle(self.package, msg);

        Ok(AfterProcess::Stop)
    }
}

fn create_table_data(entry: &IndexEntry) -> Result<Vec<u8>, Error> {
    let mut data = Vec::new();

    // Find the current location of the index component
    let mut header = IndexHeader::zeroed();
    header.set_groups(1);
    data.write_all(&header)?;

    let mut group = IndexGroupHeader::zeroed();
    group.set_encoding(IndexGroupEncoding::None);
    group.set_length(1);
    data.write_all(&group)?;

    data.write_all(&entry)?;

    Ok(data)
}
