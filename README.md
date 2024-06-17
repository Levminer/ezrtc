# ezrtc

-   Easy cross-platform WebRTC communication with data channels and a simple signaling server.
-   Currently supports: JavaScript/TypeScript, Rust.

> [!WARNING]
> Ezrtc is currently unstable and not recommended for production use. The API is subject to change.

## Client

-   A simple WebRTC client that connects to the signaling server using WebSockets.

[![NPM Version](<https://img.shields.io/npm/v/ezrtc?label=Client%20(npm)>)](https://www.npmjs.com/package/ezrtc)
[![Crates.io Version](<https://img.shields.io/crates/v/ezrtc?label=Client%20(crates)>)](https://crates.io/crates/ezrtc)

### JS/TS

-   Install the package using [npm](https://www.npmjs.com/package/ezrtc): `npm i ezrtc`
-   Example usage:

```js
// Host
import { EzRTCHost } from "ezrtc"

let host = new EzRTCHost("wss://test.levminer.com/one-to-many", "random_session_id")

setInterval(() => {
	host.sendMessageToAll("test message")
}, 1000)

// Client
import { EzRTCClient } from "ezrtc"

let client = new EzRTCClient("wss://test.levminer.com/one-to-many", "random_session_id")

client.onMessage((message) => {
	console.log(message) // "test message"
})
```

## Server

[![Crates.io Version](<https://img.shields.io/crates/v/ezrtc-server?label=Server%20(Crates)>)](https://crates.io/crates/ezrtc-server)
[![Docker Image Version](https://img.shields.io/docker/v/levminer/ezrtc-server?label=Server%20(Docker%20Hub))](https://hub.docker.com/r/levminer/ezrtc-server)


-   A simple signaling server that allows multiple clients to connect to each other and exchange data using WebSockets.

## Credits

-   Server based on: [wasm-peers](https://github.com/wasm-peers/wasm-peers)
-   Licensed under [MIT](https://github.com/levminer/ezrtc/blob/main/LICENSE.md)
