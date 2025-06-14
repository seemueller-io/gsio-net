use futures::StreamExt;
use worker::*;

#[event(fetch)]
async fn fetch(req: HttpRequest, _env: Env, _ctx: Context) -> Result<worker::Response> {
    let upgrade_header = match req.headers().get("Upgrade") {
        Some(h) => h.to_str().unwrap(),
        None => "",
    };
    if upgrade_header != "websocket" {
        return worker::Response::error("Expected Upgrade: websocket", 426);
    }

    let ws = WebSocketPair::new()?;
    let client = ws.client;
    let server = ws.server;
    server.accept()?;

    wasm_bindgen_futures::spawn_local(async move {
        let mut event_stream = server.events().expect("could not open stream");
        while let Some(event) = event_stream.next().await {
            match event.expect("received error in websocket") {
                WebsocketEvent::Message(msg) => server.send(&format!("echo: {:?}", &msg.text().unwrap())).unwrap(),
                WebsocketEvent::Close(event) => console_log!("{:?}", event),
            }
        }
    });
    worker::Response::from_websocket(client)
}