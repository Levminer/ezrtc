# ezrtc

-   Easy cross-platform WebRTC communication.

## Getting started

-   Ezrtc is currently unstable and not recommended for production use. The API is subject to change.

### JavaScript

-   Install the package using npm:

`npm i ezrtc`

-   Example usage:
```js
// Host
import { EzrtcHost } from "ezrtc"

let host = new EzrtcHost("wss://test.levminer.com/one-to-many", "random_session_id")

setInterval(() => {
	host.sendMessageToAll("test message")
}, 1000)

// Client
import { EzrtcClient } from "ezrtc"

let client = new EzrtcClient("wss://test.levminer.com/one-to-many", "random_session_id")

client.onMessage((message) => {
	console.log(message) // "test message"
})
```

### Rust

## Client

-   A simple WebRTC client that connects to the signaling server using WebSockets.

## Server

-   A simple signaling server that allows multiple clients to connect to each other and exchange data using WebSockets.

## Credits

-   Server based on: [wasm-peers](https://github.com/wasm-peers/wasm-peers)
