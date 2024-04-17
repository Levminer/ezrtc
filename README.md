# ezrtc

-   Easy cross-platform WebRTC communication with a simple signaling server.
-   Currently supports: JavaScript, C#, Rust.

> [!WARNING]
> Ezrtc is currently unstable and not recommended for production use. The API is subject to change.

## Client

-   A simple WebRTC client that connects to the signaling server using WebSockets.

![NPM Version](<https://img.shields.io/npm/v/ezrtc?label=Client%20(npm)>)
![NuGet Version](<https://img.shields.io/nuget/v/ezrtc?label=Client%20(nuget)>)

### JavaScript

-   Install the package using [npm](https://www.npmjs.com/package/ezrtc): `npm i ezrtc`
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

### C#

-   Install the package using [NuGet](https://www.nuget.org/packages/ezrtc): `dotnet add package ezrtc`
-   Example usage:

```cs
// Host

// Client
```

## Server

![Crates.io Version](<https://img.shields.io/crates/v/ezrtc-server?label=Server%20(Crates)>)

-   A simple signaling server that allows multiple clients to connect to each other and exchange data using WebSockets.

## Credits

-   Server based on: [wasm-peers](https://github.com/wasm-peers/wasm-peers)
-   Licensed under [MIT](https://github.com/levminer/ezrtc/blob/main/LICENSE.md)
