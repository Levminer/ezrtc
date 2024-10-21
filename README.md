# ezrtc

-   Easy cross-platform WebRTC communication with data channels and a simple signaling server.
-   Currently supports: JavaScript/TypeScript, Rust and C#.

> [!WARNING]
> ezrtc is currently unstable and not recommended for production use. The API is subject to change.

## Client

-   A simple WebRTC client that connects to the signaling server using WebSockets.

[![NPM Version](<https://img.shields.io/npm/v/ezrtc?label=Client%20(npm)>)](https://www.npmjs.com/package/ezrtc)
[![Crates.io Version](<https://img.shields.io/crates/v/ezrtc?label=Client%20(crates)>)](https://crates.io/crates/ezrtc)
![NuGet Version](<https://img.shields.io/nuget/v/ezrtc?label=Client%20(NuGet)>)

## Server

-   A simple signaling server that allows multiple clients to connect to each other and exchange data using WebSockets.

[![Crates.io Version](<https://img.shields.io/crates/v/ezrtc-server?label=Server%20(crates)>)](https://crates.io/crates/ezrtc-server)
[![Docker Image Version](<https://img.shields.io/docker/v/levminer/ezrtc-server?label=Server%20(Docker%20Hub)>)](https://hub.docker.com/r/levminer/ezrtc-server)

## Credits

-   Server based on: [wasm-peers](https://github.com/wasm-peers/wasm-peers)
-   Licensed under [MIT](https://github.com/levminer/ezrtc/blob/main/LICENSE.md)
