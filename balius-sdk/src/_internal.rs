use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

use crate::wit;

type ChannelId = u32;

pub trait Handler: Send + Sync + 'static {
    fn handle(
        &self,
        config: wit::Env,
        event: wit::Event,
    ) -> Result<wit::Response, wit::HandleError>;
}

pub(crate) struct Channel {
    pub(crate) handler: Box<dyn Handler>,
    pub(crate) pattern: wit::balius::app::driver::EventPattern,
}

type ChannelRegistry = HashMap<ChannelId, Channel>;

#[derive(Default)]
pub struct Worker {
    pub(crate) channels: ChannelRegistry,
    pub(crate) env: Option<wit::Env>,
}

static WORKER: LazyLock<RwLock<Worker>> = LazyLock::new(|| RwLock::new(Worker::default()));

pub fn global_init_worker(env: wit::Env, mut worker: Worker) {
    worker.init(env);

    for (id, handler) in worker.channels.iter() {
        wit::balius::app::driver::register_channel(*id, &handler.pattern);
    }

    let mut singelton = WORKER.write().unwrap();
    *singelton = worker;
}

pub fn global_handle_request(id: u32, evt: wit::Event) -> Result<wit::Response, wit::HandleError> {
    let worker = WORKER.read().unwrap();
    let channel = worker.channels.get(&id).ok_or(1u32)?;
    let env = match &worker.env {
        Some(e) => e.clone(),
        None => return Err(0),
    };

    channel.handler.handle(env, evt)
}
