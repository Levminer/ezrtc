import { SignalMessage } from "./protocol.js"

export class EzrtcClient {
	sessionId: string
	hostURL: string
	peerConnection = new RTCPeerConnection()
	#send = true
	#messageCallback?: (message: string) => void

	constructor(host: string, sessionId: string) {
		this.hostURL = host
		this.sessionId = sessionId

		const websocket = new WebSocket(host)

		websocket.onopen = (e) => {
			console.log("Connected to host", e)

			websocket.send(new SignalMessage().SessionJoin().Encode(sessionId, false))
		}

		websocket.onclose = (e) => {
			console.log("Closed connection with host", e)
		}

		websocket.onerror = (e) => {
			console.log("Error connecting with host", e)
		}

		websocket.onmessage = (e) => {
			const data = e.data.startsWith("ping") ? null : JSON.parse(e.data)

			console.log("Websocket event received", e)

			if (data != null) {
				if (data.SdpOffer) {
					const sdpOffer = new SignalMessage().SdpOffer().Decode(data)

					const offer: RTCSessionDescriptionInit = {
						type: "offer",
						sdp: sdpOffer.offer,
					}

					this.peerConnection.onicecandidate = (e) => {
						// Only send one ICE candidate
						if (this.#send) {
							websocket.send(new SignalMessage().SdpAnswer().Encode(sessionId, sdpOffer.userId, this.peerConnection.localDescription!.sdp))
							this.#send = false
						}
					}

					this.peerConnection.ondatachannel = (e) => {
						const dataChannel = e.channel

						dataChannel.onmessage = (e) => {
							this.#messageCallback?.(e.data)
							console.log(`Message received: ${e.data}`)
						}

						dataChannel.onopen = (e) => console.log("Data channel opened")
						dataChannel.onclose = (e) => console.log("Data channel closed")
					}

					this.peerConnection.setRemoteDescription(offer).then(() => {
						console.log("offer set")
					})

					this.peerConnection.createAnswer().then(async (a) => {
						await this.peerConnection.setLocalDescription(a)
						console.log("answer created")
					})
				}
			}
		}
	}

	onMessage(callback: (message: string) => void) {
		// Store the callback for later use
		this.#messageCallback = callback
	}
}
