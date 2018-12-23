# http_static
Simple static file command line HTTP server written in Rust language using tower-web.
This supports custom Index file and `404 Not Found` handler both default to `index.html`.
It can be useful for serving static websites.
It also provides an in memory file serving mode which can be useful when there is only a
small amount data which the server needs to serve. In memory file serving is much faster
and does not read files once the server is initialized which does all the reading.
This mode can be useful for serving web apps.

The project is work in progress.

## Compile
These instructions assume rust and cargo to be installed.
If you don't have them, please install using [rustup](https://rustup.rs).

### To getting the repository

```sh
git clone https://github.com/arjsin/http_static.git
cd http_static
# Future instructions assume this directory
```

### To build
Release build provides good performance while running the program.
```sh
cargo build --release
```

### To run
Run to listen on default port
```sh
cargo run --release
```

Run to listen on default port with in memory files
```sh
cargo run --release -- -m
```

The help prints all the possible options
```sh
cargo run --release -- -h
```
```
http_static
A lightwight static file server for HTTP

USAGE:
    http_static [FLAGS] [OPTIONS]

FLAGS:
    -h, --help         Prints help information
    -m, --in_memory    Sets in memory file server
    -V, --version      Prints version information

OPTIONS:
    -d, --default <PATH>      Sets path of a file which is served when the file requested in not available, default: index.html
    -i, --index <FILENAME>    Sets file name inside each directory to be served at path of directory, default: index.html
    -l, --listen <ADDRESS>    Sets the address to listen on, default: [::1]:8080
    -r, --root <PATH>         Sets path of a directory for serving files, default: .
```

## TODO
- [x] File serving
- [x] In memory file serving
- [ ] Logging
- [ ] In memory serving, `304 Not Modified` support
- [ ] In memory serving, file change notification support
- [ ] HTTPS support
- [ ] Directory listing
