mod file_serving;
mod in_memory_serving;

use self::{file_serving::FileServing, in_memory_serving::InMemoryServing};
use clap::{clap_app, crate_version};
use futures::Future;
use std::net::SocketAddr;
use tower_web::ServiceBuilder;

fn main() {
    let matches = clap_app!(http_static =>
        (version: crate_version!())
        (about: "A lightwight static file server for HTTP")
        (@arg in_mem: -m --in_memory "Sets in memory file server")
        (@arg listen: -l --listen [LISTEN] "Sets the address to listen on")
    )
    .get_matches();
    let addr = matches
        .value_of("listen")
        .unwrap_or("[::1]:8080")
        .parse()
        .expect("Invalid address");

    if matches.is_present("in_mem") {
        serve_in_memory(&addr);
    } else {
        serve(&addr);
    }
}

fn serve_in_memory(addr: &SocketAddr) {
    println!("Listening in memory on http://{}", addr);

    let incoming = tokio::net::TcpListener::bind(addr).unwrap().incoming();
    let fut = InMemoryServing::new(".", "index.html", "./index.html")
        .map_err(|e| println!("Error while initializing: {:?}", e))
        .and_then(move |in_mem| ServiceBuilder::new().resource(in_mem).serve(incoming));
    tokio::run(fut);
}

fn serve(addr: &SocketAddr) {
    println!("Listening on http://{}", addr);

    ServiceBuilder::new()
        .resource(FileServing::new(".", "index.html", "./index.html"))
        .run(addr)
        .unwrap();
}
