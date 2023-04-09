#[tokio::main]
pub async fn main() {
    let _ = pretty_env_logger::try_init();

    let service = simple_mdns::Service::new("_raop._tcp", "test", 1234, vec!["testtest"]);
    let server = simple_mdns::Server::new(vec![service]).unwrap();
    server.serve().await.unwrap();
}
