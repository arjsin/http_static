use bytes::Bytes;
use futures::{
    future::{ok, poll_fn, Future},
    prelude::*,
    stream,
};
use http::{self, header};
use lazy_static::lazy_static;
use mime_guess::guess_mime_type as guess_mime;
use std::{
    collections::HashMap,
    fmt, io,
    path::{Path, PathBuf},
    sync::Arc,
};
use tower_web::{
    derive_resource, derive_resource_impl, error, impl_web, impl_web_clean_nested,
    impl_web_clean_top_level,
    response::{Context, Response, Serializer},
    util::BufStream,
};

#[derive(Clone, Debug)]
pub struct InMemoryServing {
    default: Option<Arc<InMemory>>,
    files: HashMap<PathBuf, Arc<InMemory>>,
}

impl InMemoryServing {
    pub fn new<P>(
        root: P,
        index: P,
        default: P,
    ) -> impl Future<Item = InMemoryServing, Error = io::Error>
    where
        P: AsRef<Path> + Send + 'static,
    {
        let default_mime = guess_mime(&default);
        let fut_default = Self::file_data(default)
            .map(move |data| Some(Arc::new(InMemory::new(data, default_mime.as_ref()))))
            .or_else(|_| Ok(None));

        let fut_files = Self::file_tree(root.as_ref().to_path_buf())
            .concat2()
            .and_then(|list| ok(stream::iter_ok(list)))
            .flatten_stream()
            .and_then(|path| Self::file_data(path.clone()).map(|data| (path, data)))
            .fold(HashMap::new(), move |mut map, (path, data)| {
                let in_mem = InMemory::new(data, guess_mime(&path).as_ref());
                let path = path.strip_prefix(root.as_ref()).unwrap();
                let path = if path.ends_with(index.as_ref()) {
                    path.parent().unwrap()
                } else {
                    path
                };
                map.insert(path.to_path_buf(), Arc::new(in_mem));
                ok::<_, io::Error>(map)
            });

        fut_default
            .join(fut_files)
            .map(|(default, files)| InMemoryServing { default, files })
    }

    fn file_data<P>(path: P) -> impl Future<Item = Vec<u8>, Error = io::Error>
    where
        P: AsRef<Path> + Send + 'static,
    {
        tokio::fs::File::open(path)
            .and_then(|file| tokio::io::read_to_end(file, vec![]))
            .map(|(_, data)| data)
    }

    fn file_tree<P>(path: P) -> impl Stream<Item = Vec<PathBuf>, Error = io::Error>
    where
        P: Into<PathBuf>,
    {
        stream::unfold(vec![path.into()], |paths| {
            if !paths.is_empty() {
                Some(Self::file_list(paths))
            } else {
                None
            }
        })
    }

    fn file_list(
        list: Vec<PathBuf>,
    ) -> impl Future<Item = (Vec<PathBuf>, Vec<PathBuf>), Error = io::Error> {
        stream::iter_ok(list)
            .and_then(tokio::fs::read_dir)
            .flatten()
            .and_then(move |file| {
                let path = file.path();
                poll_fn(move || file.poll_file_type()).map(move |file_type| (path, file_type))
            })
            .fold((vec![], vec![]), |(mut f, mut d), (path, file_type)| {
                if file_type.is_dir() {
                    d.push(path)
                } else {
                    f.push(path)
                }
                ok::<_, io::Error>((f, d))
            })
    }
}

lazy_static! {
    static ref NotFound: Arc<InMemory> = Arc::new(InMemory {
        data: Bytes::from("Not Found"),
        mime: "text/plain".into(),
        status: 404,
    });
}

impl_web! {
    impl InMemoryServing {
        #[get("/")]
        fn root(&self) -> Result<LocalArc<InMemory>, ()> {
            match self.files.get(&PathBuf::from("")) {
                Some(file) => Ok(LocalArc(file.clone())),
                None => match self.default {
                    Some(ref x) => Ok(LocalArc(x.clone())),
                    None => Ok(LocalArc(NotFound.clone())),
                },
            }
        }

        #[get("/*relative_path")]
        fn files(&self, relative_path: PathBuf) -> Result<LocalArc<InMemory>, ()> {
            match self.files.get(&relative_path) {
                Some(file) => Ok(LocalArc(file.clone())),
                None => match self.default {
                    Some(ref x) => Ok(LocalArc(x.clone())),
                    None => Ok(LocalArc(NotFound.clone())),
                },
            }
        }
    }
}

struct LocalArc<T>(Arc<T>);

struct InMemory {
    data: Bytes,
    mime: String,
    status: u16,
}

impl fmt::Debug for InMemory {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(type: {}, len: {})", self.mime, self.data.len())
    }
}

impl InMemory {
    fn new<B: Into<Bytes>, S: Into<String>>(data: B, mime: S) -> InMemory {
        InMemory {
            data: data.into(),
            mime: mime.into(),
            status: 200,
        }
    }
}

impl Response for LocalArc<InMemory> {
    type Buf = <Self::Body as BufStream>::Item;
    type Body = error::Map<Bytes>;

    fn into_http<S: Serializer>(
        self,
        _: &Context<S>,
    ) -> Result<http::Response<Self::Body>, tower_web::Error> {
        let content_type = header::HeaderValue::from_str(self.0.mime.as_ref()).unwrap();

        Ok(http::Response::builder()
            .status(self.0.status)
            .header(header::CONTENT_TYPE, content_type)
            .body(error::Map::new(self.0.data.clone()))
            .unwrap())
    }
}
