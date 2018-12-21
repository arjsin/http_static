mod file_serving;
mod in_memory_serving;

use self::{file_serving::FileServing, in_memory_serving::InMemoryServing};
use clap::{clap_app, crate_version};
use futures::Future;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_web::ServiceBuilder;

fn main() {
    let matches = clap_app!(http_static =>
        (version: crate_version!())
        (about: "A lightwight static file server for HTTP")
        (@arg in_mem: -m --in_memory "Sets in memory file server")
        (@arg listen: -l --listen [ADDRESS] "Sets the address to listen on, default: [::1]:8080")
        (@arg root: -r --root [PATH] "Sets path of a directory for serving files, default: .")
        (@arg index: -i --index [FILENAME] "Sets file name inside each directory to be served at path of directory, default: index.html")
        (@arg default: -d --default [PATH] "Sets path of a file which is served when the file requested in not available, default: index.html")
    )
    .get_matches();
    let addr = matches.value_of("listen").unwrap_or("[::1]:8080");
    let addr = addr.parse().expect("Invalid address");
    let root = matches.value_of("root").unwrap_or(".");
    let index = matches.value_of("index").unwrap_or("index.html");
    let default = matches.value_of("default").unwrap_or("index.html");

    if matches.is_present("in_mem") {
        serve_in_memory(&addr, root, index, default);
    } else {
        serve(&addr, root, index, default);
    }
}

fn serve_in_memory(addr: &SocketAddr, root: &str, index: &str, default: &str) {
    println!("Listening in memory on http://{}", addr);

    let incoming = tokio::net::TcpListener::bind(addr).unwrap().incoming();
    let fut = InMemoryServing::new(
        PathBuf::from(root),
        PathBuf::from(index),
        PathBuf::from(default),
    )
    .map_err(|e| println!("Error while initializing: {:?}", e))
    .and_then(move |in_mem| ServiceBuilder::new().resource(in_mem).serve(incoming));
    tokio::run(fut);
}

fn serve(addr: &SocketAddr, root: &str, index: &str, default: &str) {
    println!("Listening on http://{}", addr);

    ServiceBuilder::new()
        .resource(FileServing::new(root, index, default))
        .run(addr)
        .unwrap();
}
