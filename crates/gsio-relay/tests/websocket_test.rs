// Integration testing must be written in tandem with Miniflare
// See https://github.com/cloudflare/workers-rs?tab=readme-ov-file#testing-with-miniflare   

// use worker::*;
// use wasm_bindgen_test::*;
// 
// wasm_bindgen_test_configure!(run_in_browser);
// 
// // Test the WebSocket upgrade check
// #[wasm_bindgen_test]
// async fn test_websocket_upgrade_check() {
//     // Create a mock request without the Upgrade header
//     let mut headers = Headers::new();
//     headers.set("Content-Type", "text/plain").unwrap();
//     
//     let req = Request::new_with_init(
//         "https://example.com/",
//         RequestInit::new()
//             .with_method(Method::Get)
//             .with_headers(headers),
//     ).unwrap();
//     
//     // Create a mock environment and context
//     let env = worker::Env::new();
//     let ctx = worker::Context::new();
//     
//     // Call the fetch function
//     let result = fetch(req, env, ctx).await;
//     
//     // Verify that the response is an error with status 426
//     assert!(result.is_ok());
//     let response = result.unwrap();
//     assert_eq!(response.status_code(), 426);
// }
// 
// // Test the WebSocket pair creation
// #[wasm_bindgen_test]
// async fn test_websocket_pair_creation() {
//     // Create a WebSocketPair
//     let ws_pair = WebSocketPair::new();
//     
//     // Verify that the pair was created successfully
//     assert!(ws_pair.is_ok());
//     
//     let pair = ws_pair.unwrap();
//     assert!(Some(pair.client).is_some());
//     assert!(Some(pair.server).is_some());
// }
// 
// // Test the WebSocket server accept
// #[wasm_bindgen_test]
// async fn test_websocket_server_accept() {
//     // Create a WebSocketPair
//     let ws_pair = WebSocketPair::new().unwrap();
//     let server = ws_pair.server;
//     
//     // Accept the connection
//     let result = server.accept();
//     
//     // Verify that the connection was accepted successfully
//     assert!(result.is_ok());
// }
// 
// // Test the WebSocket response creation
// #[wasm_bindgen_test]
// async fn test_websocket_response_creation() {
//     // Create a WebSocketPair
//     let ws_pair = WebSocketPair::new().unwrap();
//     let client = ws_pair.client;
//     
//     // Create a response from the WebSocket
//     let response = Response::from_websocket(client);
//     
//     // Verify that the response was created successfully
//     assert!(response.is_ok());
//     let ws_response = response.unwrap();
//     assert_eq!(ws_response.status_code(), 101); // Switching Protocols
// }