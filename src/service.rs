use std::sync::{Arc, RwLock};
use writium::Writium;
use writium::proto::{HyperRequest, HyperResponse};
use tokio_service::Service;

pub struct WritiumService(Arc<RwLock<Option<Writium>>>);
impl WritiumService {
    pub fn new(w: Writium) -> WritiumService {
        WritiumService(Arc::new(RwLock::new(Some(w))))
    }
    pub fn writium(&self) -> Arc<RwLock<Option<Writium>>> {
        self.0.clone()
    }
}
impl Service for WritiumService {
    type Request = HyperRequest;
    type Response = HyperResponse;
    type Error = ::hyper::Error;
    type Future = Box<::futures::future::Future<Item=Self::Response, Error=Self::Error>>;
    fn call(&self, req: HyperRequest) -> Self::Future {
        // When the write lock is not released, the option can never be `None`.
        self.0.read().unwrap().as_ref().unwrap().route(req)
    }
}
