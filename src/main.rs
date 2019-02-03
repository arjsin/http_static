mod file_serving;
mod in_memory_serving;

use self::{file_serving::FileServing, in_memory_serving::InMemoryServing};
use clap::{clap_app, crate_version};
use futures::prelude::*;
use std::{
    fs::File,
    io::{self, BufReader},
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{
    rustls::{
        internal::pemfile::{certs, rsa_private_keys},
        Certificate, NoClientAuth, PrivateKey, ServerConfig, ServerSession,
    },
    TlsAcceptor, TlsStream,
};

use tower_web::{net::ConnectionStream, ServiceBuilder};

fn main() {
    let matches = clap_app!(http_static =>
        (version: crate_version!())
        (about: "A lightweight static file server for HTTP")
        (@arg in_mem: -m --in_memory "Sets in memory file server")
        (@arg listen: -l --listen [ADDRESS] "Sets the address to listen on, default: [::1]:8080")
        (@arg root: -r --root [PATH] "Sets path of a directory for serving files, default: .")
        (@arg index: -i --index [FILENAME] "Sets file name inside each directory to be served at path of directory, default: index.html")
        (@arg default: -d --default [PATH] "Sets path of a file which is served when the file requested in not available, default: index.html")
        (@arg tls_key: --tls_key [FILENAME] "Sets file name for TLS private key, must be present with TLS certificate")
        (@arg tls_cert: --tls_cert [FILENAME] "Sets file name for TLS certificate, must be present with TLS private key")
    )
    .get_matches();
    let addr = matches.value_of("listen").unwrap_or("[::1]:8080");
    let addr = addr.parse().expect("Invalid address");
    let root = matches.value_of("root").unwrap_or(".");
    let index = matches.value_of("index").unwrap_or("index.html");
    let default = matches.value_of("default").unwrap_or("index.html");
    enum EitherIncoming<TLS, TCP> {
        Tls(TLS),
        Tcp(TCP),
    }
    let either_socket = if let Some(keys) = matches.value_of("tls_key") {
        let certs = matches
            .value_of("tls_cert")
            .expect("TLS cert not provided with key");
        EitherIncoming::Tls(tls_incoming(certs, keys, &addr))
    } else {
        EitherIncoming::Tcp(tokio::net::TcpListener::bind(&addr).unwrap().incoming())
    };

    if matches.is_present("in_mem") {
        match either_socket {
            EitherIncoming::Tls(incoming) => {
                println!("Listening in memory on https://{}", addr);
                serve_in_memory(incoming, root, index, default);
            }
            EitherIncoming::Tcp(incoming) => {
                println!("Listening in memory on http://{}", addr);
                serve_in_memory(incoming, root, index, default);
            }
        }
    } else {
        match either_socket {
            EitherIncoming::Tls(incoming) => {
                println!("Listening on https://{}", addr);
                serve(incoming, root, index, default);
            }
            EitherIncoming::Tcp(incoming) => {
                println!("Listening on http://{}", addr);
                serve(incoming, root, index, default);
            }
        }
    }
}

fn load_certs(path: &str) -> Vec<Certificate> {
    certs(&mut BufReader::new(File::open(path).unwrap())).unwrap()
}

fn load_keys(path: &str) -> Vec<PrivateKey> {
    rsa_private_keys(&mut BufReader::new(File::open(path).unwrap())).unwrap()
}

fn tls_incoming(
    certs: &str,
    keys: &str,
    addr: &SocketAddr,
) -> impl Stream<Item = TlsStream<TcpStream, ServerSession>, Error = io::Error> {
    let tls_config = {
        let mut config = ServerConfig::new(NoClientAuth::new());
        config
            .set_single_cert(load_certs(certs), load_keys(keys).remove(0))
            .expect("invalid key or certificate");
        TlsAcceptor::from(Arc::new(config))
    };
    TcpListener::bind(addr)
        .unwrap()
        .incoming()
        .and_then(move |tcp_stream| tls_config.accept(tcp_stream))
        .then(|r| match r {
            Ok(x) => Ok::<_, io::Error>(Some(x)),
            Err(_) => Ok(None), // TODO: log TLS errors here
        })
        .filter_map(|x| x)
}

fn serve_in_memory<CS>(incoming: CS, root: &str, index: &str, default: &str)
where
    CS: ConnectionStream + Send + 'static,
    CS::Item: Send + 'static,
{
    let fut = InMemoryServing::new(
        PathBuf::from(root),
        PathBuf::from(index),
        PathBuf::from(default),
    )
    .map_err(|e| println!("Error while initializing: {:?}", e))
    .and_then(move |in_mem| ServiceBuilder::new().resource(in_mem).serve(incoming));
    tokio::run(fut);
}

fn serve<CS>(incoming: CS, root: &str, index: &str, default: &str)
where
    CS: ConnectionStream + Send + 'static,
    CS::Item: Send + 'static,
{
    let fut = ServiceBuilder::new()
        .resource(FileServing::new(root, index, default))
        .serve(incoming);
    tokio::run(fut);
}
