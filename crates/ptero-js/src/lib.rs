use std::{cell::RefCell, rc::Rc};

use anyhow::Error;
use js_sys::{ArrayBuffer, Uint8Array};
use ptero_file::{FileAction, FileMessage, ReadResult};
use stewart::{Actor, ActorData, Addr, After, Context, Id, Options, System};
use stewart_utils::MapExt;
use tracing::{event, instrument, Level};
use uuid::Uuid;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{Request, RequestInit, RequestMode, Response};

#[instrument("fetch-file", skip_all)]
pub fn open_fetch_file(
    ctx: &mut Context,
    url: String,
    hnd: SystemH,
) -> Result<Addr<FileMessage>, Error> {
    let (id, mut ctx) = ctx.create()?;
    let addr = Addr::new(id);

    let service = FetchFileService {
        data: None,
        pending: Vec::new(),
    };
    ctx.start(id, Options::default(), service)?;

    spawn_local(do_fetch(hnd, addr, url));

    let addr = ctx.map(addr, Message::File)?;
    Ok(addr)
}

struct FetchFileService {
    data: Option<Vec<u8>>,
    pending: Vec<Pending>,
}

impl Actor for FetchFileService {
    type Message = Message;

    fn process(
        &mut self,
        system: &mut System,
        _id: Id,
        data: &mut ActorData<Message>,
    ) -> Result<After, Error> {
        while let Some(message) = data.next() {
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
                            self.pending.push(pending);
                        }
                        FileAction::Write { .. } => {
                            // TODO: Report back invalid operation
                        }
                    }
                }
                Message::FetchResult(data) => {
                    event!(Level::INFO, "fetch response received");
                    self.data = Some(data);
                }
            }
        }

        // Check if we can respond to requests
        if let Some(data) = &self.data {
            event!(Level::INFO, count = self.pending.len(), "resolving entries");

            for pending in self.pending.drain(..) {
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
                system.send(pending.on_result, message);
            }
        }

        Ok(After::Continue)
    }
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
pub type SystemH = Rc<RefCell<System>>;
