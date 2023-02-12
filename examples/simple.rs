#[tokio::main]
pub async fn main() {
    let service = simple_mdns::Service::new("test", "test", 1234, vec!["testtest"]);
    let server = simple_mdns::Server::new(vec![service]).unwrap();
    server.serve().await.unwrap();
}
