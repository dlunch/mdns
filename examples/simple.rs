#[tokio::main]
pub async fn main() {
    let _ = pretty_env_logger::try_init();

    let service = mdns_responder::Service::new("_raop._tcp", "test", 1234, vec!["testtest"]);
    let server = mdns_responder::Server::new(vec![service]).unwrap();
    server.serve().await.unwrap();
}
