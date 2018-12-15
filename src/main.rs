use tower_web::ServiceBuilder;

mod file_serving;

use self::file_serving::FileServing;

pub fn main() {
    let addr = "127.0.0.1:8080".parse().expect("Invalid address");
    println!("Listening on http://{}", addr);

    ServiceBuilder::new()
        .resource(FileServing::new("index.html", "./index.html"))
        .run(&addr)
        .unwrap();
}
