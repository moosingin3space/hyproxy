use futures;
use futures::{Future, BoxFuture};
use futures::future::FutureResult;

use hyper;
use hyper::{Client, StatusCode, Body};
use hyper::client::HttpConnector;
use hyper::server::{Service, Request, Response};

use chrono::prelude::*;

use regex;

use errors;

#[derive(Clone)]
pub struct Routes {
    pub routes: Vec<(regex::Regex, String)>,
    pub regexes: regex::RegexSet,
}

pub struct Proxy {
    pub routes: Routes,
    pub client: Client<HttpConnector, Body>,
}

impl Service for Proxy {
    type Request = Request;
    type Response = Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item=Response, Error=Self::Error>>;

    fn call(&self, req: Request) -> Self::Future {
        let uri = req.uri();
        let matches = self.routes.regexes.matches(uri.path());
        let fut = {
            if !matches.matched_any() {
                futures::future::ok(Response::new().with_status(StatusCode::NotFound)).boxed()
            } else {
                // Find the most specific match (unwrap called here because of the above check)
                let index = matches.iter().next().unwrap();
                let (ref regex, ref other_site) = self.routes.routes[index];
                let url = hyper::Url::parse(other_site).expect("configuration problem, other site not valid URL");
                if let Some(caps) = regex.captures(uri.path()) {
                    let site_url = match caps.name("site_url") {
                        Some(m) => m.as_str(),
                        None => {
                            error!("no site_url present");
                            return futures::future::ok(
                                Response::new().with_status(StatusCode::InternalServerError)).boxed()
                        },
                    };
                    let url = url.join(site_url).unwrap();
                    println!("forward request to {}", url);
                    let proxied_request = hyper::client::Request::new(req.method().clone(), url);
                    Box::new(self.client.request(proxied_request).then(|res| {
                        println!("got response back!");
                        if let Ok(res) = res {
                        futures::future::ok(
                            Response::new()
                                .with_status(res.status().clone())
                                .with_headers(res.headers().clone())
                                .with_body(res.body()))
                        } else {
                            futures::future::ok(
                                Response::new()
                                    .with_status(StatusCode::ServiceUnavailable))
                        }
                    })) as Self::Future
                } else {
                    futures::future::ok(Response::new().with_status(StatusCode::BadGateway)).boxed()
                }
            }
        };
        fut
    }
}
