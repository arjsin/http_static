use std::{
    io::{Read, Result as IoResult, Write},
    net::SocketAddr,
};
use tokio::{io::Error as TokioError, net::TcpStream, prelude::*};
use tokio_rustls::rustls::ServerSession;
use tokio_rustls::TlsStream;
use tower_web::net::Connection;

pub struct TlsConnection(TlsStream<TcpStream, ServerSession>);

impl From<TlsStream<TcpStream, ServerSession>> for TlsConnection {
    fn from(s: TlsStream<TcpStream, ServerSession>) -> Self {
        Self(s)
    }
}

impl Connection for TlsConnection {
    fn peer_addr(&self) -> Option<SocketAddr> {
        TcpStream::peer_addr(self.0.get_ref().0).ok()
    }
}

impl Read for TlsConnection {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.0.read(buf)
    }
}

impl Write for TlsConnection {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.0.flush()
    }
}

impl AsyncRead for TlsConnection {}

impl AsyncWrite for TlsConnection {
    fn shutdown(&mut self) -> Result<Async<()>, TokioError> {
        self.0.shutdown()
    }
}
