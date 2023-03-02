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
use ptero_daicon::{io::ReadWrite, FindComponent, FindComponentResult};
use stewart::{Process, HandlerId, Context, Factory, Next};
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

#[derive(Factory)]
#[factory(AddDataActor::start)]
pub struct AddData {
    pub package: HandlerId<ReadWrite>,
    pub data: Vec<u8>,
    pub uuid: Uuid,
}

struct AddDataActor;

impl AddDataActor {
    pub fn start(ctx: &dyn Context, _address: HandlerId<()>, data: AddData) -> Result<Self, Error> {
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
        ctx.start(add_index);

        // Write the file to the package
        let write = ReadWrite::Write {
            start: data_start,
            data: data.data,
        };
        ctx.send(data.package, write);

        Ok(AddDataActor)
    }
}

impl Process for AddDataActor {
    type Message = ();

    fn handle(&mut self, _ctx: &dyn Context, _message: ()) -> Result<Next, Error> {
        // TODO: Report success/failure back
        unimplemented!()
    }
}

#[derive(Factory)]
#[factory(AddIndexActor::start)]
struct AddIndex {
    package: HandlerId<ReadWrite>,
    value: IndexEntry,
}

struct AddIndexActor {
    package: HandlerId<ReadWrite>,
    value: IndexEntry,
}

impl AddIndexActor {
    pub fn start(
        ctx: &dyn Context,
        address: HandlerId<FindComponentResult>,
        data: AddIndex,
    ) -> Result<Self, Error> {
        let find_component = FindComponent {
            target: INDEX_COMPONENT_UUID,
            package: data.package.clone(),
            reply: address,
        };
        ctx.start(find_component);

        Ok(Self {
            package: data.package,
            value: data.value,
        })
    }
}

impl Process for AddIndexActor {
    type Message = FindComponentResult;

    fn handle(&mut self, ctx: &dyn Context, message: FindComponentResult) -> Result<Next, Error> {
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
        ctx.send(self.package, msg);

        Ok(Next::Stop)
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
