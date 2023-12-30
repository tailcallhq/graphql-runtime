pub mod server;
pub mod server_config;
pub mod http_2;
pub mod http_1;
fn log_launch_and_open_browser(sc: &server_config::ServerConfig) {
    let addr = sc.addr().to_string();
    log::info!("🚀 Tailcall launched at [{}] over {}", addr, sc.http_version());
    if sc.graphiql() {
        let url = sc.graphiql_url();
        log::info!("🌍 Playground: {}", url);

        let _ = webbrowser::open(url.as_str());
    }
}
