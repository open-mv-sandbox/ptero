use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

use anyhow::{Context as _, Error};
use js_sys::{ArrayBuffer, Uint8Array};
use ptero_file::{FileAction, FileMessage, ReadResult};
use stewart::{ActorId, Addr, State, System, SystemOptions, World};
use stewart_utils::map;
use tracing::{event, instrument, Level};
use uuid::Uuid;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Request, RequestInit, RequestMode, Response};

#[instrument("fetch-file", skip_all)]
pub fn open_fetch_file(
    world: &mut World,
    parent: Option<ActorId>,
    url: String,
    hnd: SystemH,
) -> Result<Addr<FileMessage>, Error> {
    let system = world.register(SystemOptions::default(), FetchFileSystem);

    let actor = world.create(parent)?;
    let addr = Addr::new(actor);

    let service = FetchFile {
        data: None,
        pending: Vec::new(),
    };
    world.start(actor, system, service)?;

    spawn_local(do_fetch(hnd, addr, url));

    Ok(map(world, Some(actor), addr, Message::File)?)
}

struct FetchFileSystem;

impl System for FetchFileSystem {
    type Instance = FetchFile;
    type Message = Message;

    #[instrument("fetch-file", skip_all)]
    fn process(&mut self, world: &mut World, state: &mut State<Self>) -> Result<(), Error> {
        let mut has_update = BTreeSet::new();

        while let Some((actor, message)) = state.next() {
            let instance = state.get_mut(actor).context("failed to get instance")?;

            match message {
                Message::File(message) => {
                    match message.action {
                        FileAction::Read {
                            offset,
                            size,
                            on_result,
                        } => {
                            let pending = Pending {
                                id: message.id,
                                offset,
                                size,
                                on_result,
                            };
                            instance.pending.push(pending);
                            has_update.insert(actor);
                        }
                        FileAction::Write { .. } => {
                            // TODO: Report back invalid operation
                        }
                    }
                }
                Message::FetchResult(data) => {
                    event!(Level::INFO, "fetch response received");
                    instance.data = Some(data);
                    has_update.insert(actor);
                }
            }
        }

        // Check if we can respond to accumulated requests
        for actor in has_update {
            let instance = state.get_mut(actor).context("instance missing")?;

            if let Some(data) = &instance.data {
                event!(
                    Level::INFO,
                    count = instance.pending.len(),
                    "resolving entries"
                );

                for pending in instance.pending.drain(..) {
                    let offset = pending.offset as usize;
                    let mut reply_data = vec![0u8; pending.size as usize];

                    let available = data.len() - offset;
                    let slice_len = usize::min(reply_data.len(), available);

                    let src = &data[offset..offset + slice_len];
                    let dst = &mut reply_data[0..slice_len];

                    dst.copy_from_slice(src);

                    let message = ReadResult {
                        id: pending.id,
                        offset: pending.offset,
                        data: reply_data,
                    };
                    world.send(pending.on_result, message);
                }
            }
        }

        Ok(())
    }
}

struct FetchFile {
    data: Option<Vec<u8>>,
    pending: Vec<Pending>,
}

enum Message {
    File(FileMessage),
    FetchResult(Vec<u8>),
}

struct Pending {
    id: Uuid,
    offset: u64,
    size: u64,
    on_result: Addr<ReadResult>,
}

async fn do_fetch(hnd: SystemH, addr: Addr<Message>, url: String) {
    event!(Level::INFO, "fetching data");

    let mut opts = RequestInit::new();
    opts.method("GET");
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts).unwrap();

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .unwrap();

    let resp: Response = resp_value.dyn_into().unwrap();
    let data = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
    let data: ArrayBuffer = data.dyn_into().unwrap();
    let data = Uint8Array::new(&data).to_vec();

    // Send the data back
    let mut system = hnd.borrow_mut();
    system.send(addr, Message::FetchResult(data));
    system.run_until_idle().unwrap();
}

/// TODO: Replace this with a more thought out executor abstraction.
pub type SystemH = Rc<RefCell<World>>;
