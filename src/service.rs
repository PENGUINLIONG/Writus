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
        use std::time::Instant;
        let from = Instant::now();
        // When the write lock is not released, the option can never be `None`.
        let future = self.0.read().unwrap().as_ref().unwrap().route(req);
        let delta = from.elapsed();
        let delta = (delta.as_secs() as f64) * 1000.0 + (delta.subsec_nanos() as f64) / 1_000_000.0;
        info!("Request processed. (time = {}ms)", delta);
        future
    }
}
