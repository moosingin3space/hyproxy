use std::io;
use std::rc::Rc;
use futures::{Future, Poll};
use hyper::Url;
use hyper::client::{Connect, HttpConnector};
use native_tls::TlsConnector;
use tokio_tls::{TlsStream, TlsConnectorExt};
use tokio_service::Service;
use tokio_core::net::TcpStream;

#[derive(Clone)]
pub struct HttpsConnector {
    http: HttpConnector,
    tls_connector: Rc<TlsConnector>,
}

impl HttpsConnector {

    pub fn new(http: HttpConnector, tls_connector: TlsConnector) -> HttpsConnector {
        HttpsConnector {
            http: http,
            tls_connector: Rc::new(tls_connector),
        }
    }
}

impl Service for HttpsConnector {
    type Request = Url;
    type Response = TlsStream<TcpStream>;
    type Error = io::Error;
    type Future = Box<Future<Item=Self::Response, Error=Self::Error>>;

    fn call(&self, url: Url) -> Self::Future {
        let http_connect = self.http.connect(url.clone());
        let tls_connector = self.tls_connector.clone();
        Box::new(
            http_connect.and_then(move |s| tls_connector.connect_async(url.domain().unwrap(), s).map_err(|e| io::Error::new(io::ErrorKind::Other, e)))
        )
    }

}
