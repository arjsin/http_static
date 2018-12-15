use bytes::BytesMut;
use futures::future::{ok, Either};
use http::{self, header};
use mime_guess::guess_mime_type;
use mime_guess::Mime;
use std::sync::Arc;
use std::{io, path::PathBuf};
use tokio::{fs::File as TokioFile, prelude::Future};
use tower_web::{
    derive_resource, derive_resource_impl, error, impl_web, impl_web_clean_nested,
    impl_web_clean_top_level,
    response::{Context, Response, Serializer},
};

#[derive(Clone, Debug)]
pub struct FileServing {
    dir: Arc<PathBuf>,
    default: Arc<PathBuf>,
}

impl FileServing {
    pub fn new(dir: &str, default: &str) -> Self {
        FileServing {
            dir: Arc::new(PathBuf::from(dir)),
            default: Arc::new(PathBuf::from(default)),
        }
    }
}

impl_web! {
    impl FileServing {
        #[get("/")]
        fn root(&self) -> impl Future<Item = File, Error = io::Error> {
            let default = self.default.clone();
            TokioFile::open((*self.default).clone())
                .map(move |f| File::new(f, guess_mime_type(default.as_ref())))
        }

        #[get("/*relative_path")]
        fn files(&self, relative_path: PathBuf) -> impl Future<Item = File, Error = io::Error> {
            let mut path = PathBuf::from(".");
            path.push(relative_path);

            let dir = self.dir.clone();
            let default = self.default.clone();
            TokioFile::open(path.clone())
                .and_then(|f| f.metadata())
                .and_then(move |(f, m)| {
                    if m.is_dir() {
                        path.push(dir.as_ref());
                        Either::A(TokioFile::open(path).map(move |f| (f, guess_mime_type(dir.as_ref()))))
                    } else {
                        Either::B(ok((f, guess_mime_type(path))))
                    }
                })
                .or_else(move |_| {
                    TokioFile::open((*default).clone()).map(move |f| (f, guess_mime_type(default.as_ref())))
                })
                .map(|(f, mime)| File::new(f, mime))
        }
    }
}

struct File {
    file: TokioFile,
    mime: Mime,
}

impl File {
    fn new(file: TokioFile, mime: Mime) -> File {
        File { file, mime }
    }
}

impl Response for File {
    type Buf = io::Cursor<BytesMut>;
    type Body = error::Map<TokioFile>;

    fn into_http<S: Serializer>(
        self,
        _: &Context<S>,
    ) -> Result<http::Response<Self::Body>, tower_web::Error> {
        let content_type = header::HeaderValue::from_str(self.mime.as_ref()).unwrap();

        Ok(http::Response::builder()
            .status(200)
            .header(header::CONTENT_TYPE, content_type)
            .body(error::Map::new(self.file))
            .unwrap())
    }
}
