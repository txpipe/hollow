use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

use pallas::ledger::traverse::MultiEraOutput;

use crate::wit::balius::app::driver::EventPattern;

type WorkerId = String;
type ChannelId = u32;
type Method = String;
type AddressBytes = Vec<u8>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum MatchKey {
    RequestMethod(WorkerId, Method),
    UtxoAddress(AddressBytes),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Target {
    pub channel: ChannelId,
    pub worker: String,
}

fn infer_match_keys(worker: &str, pattern: &EventPattern) -> Vec<MatchKey> {
    match pattern {
        EventPattern::Request(x) => vec![MatchKey::RequestMethod(worker.to_owned(), x.to_owned())],
        EventPattern::Utxo(_) => todo!(),
        EventPattern::UtxoUndo(_) => todo!(),
        EventPattern::Timer(_) => todo!(),
        EventPattern::Message(_) => todo!(),
    }
}

type RouteMap = HashMap<MatchKey, HashSet<Target>>;

#[derive(Default, Clone)]
pub struct Router {
    routes: Arc<RwLock<RouteMap>>,
}

impl Router {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(RwLock::new(Default::default())),
        }
    }

    pub fn register_channel(&mut self, worker: &str, channel: u32, pattern: &EventPattern) {
        let keys = infer_match_keys(worker, pattern);
        let mut routes = self.routes.write().unwrap();

        for key in keys {
            let targets = routes.entry(key).or_default();

            targets.insert(Target {
                worker: worker.to_string(),
                channel,
            });
        }
    }

    pub fn find_utxo_targets(
        &self,
        utxo: &MultiEraOutput,
    ) -> Result<HashSet<Target>, super::Error> {
        let routes = self.routes.read().unwrap();

        let key = MatchKey::UtxoAddress(utxo.address()?.to_vec());
        let targets: HashSet<_> = routes
            .get(&key)
            .iter()
            .flat_map(|x| x.iter())
            .cloned()
            .collect();

        // TODO: match by policy / asset

        Ok(targets)
    }

    pub fn find_request_target(&self, worker: &str, method: &str) -> Result<Target, super::Error> {
        let key = MatchKey::RequestMethod(worker.to_owned(), method.to_owned());
        let routes = self.routes.read().unwrap();

        let targets = routes.get(&key).ok_or(super::Error::NoTarget)?;

        if targets.is_empty() {
            return Err(super::Error::NoTarget);
        }

        if targets.len() > 1 {
            return Err(super::Error::AmbiguousTarget);
        }

        let target = targets.iter().next().unwrap();

        Ok(target.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_channel() {
        let mut router = Router::new();
        let worker = "test_worker";
        let method = "test_method";
        let channel = 1;

        router.register_channel(worker, channel, &EventPattern::Request(method.to_string()));

        let target = router.find_request_target(worker, method).unwrap();
        assert_eq!(target.worker, worker);
        assert_eq!(target.channel, channel);
    }
}
