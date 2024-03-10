import { SignalMessage } from "./protocol.js"

/**
 * This class represents a client that connects to a host and can send and receive messages.
 * @param {string} host - The URL of the host to connect to.
 * @param {string} sessionId - The session ID to use for the connection.
 */
export class EzrtcClient {
	sessionId: string
	hostURL: string
	peerConnection = new RTCPeerConnection()
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
						// Only send ICE candidates if Candidate is present
						if (e.candidate) {
							websocket.send(
								new SignalMessage().SdpAnswer().Encode(sessionId, sdpOffer.userId, this.peerConnection.localDescription!.sdp),
							)
						}
					}

					this.peerConnection.ondatachannel = (e) => {
						const dataChannel = e.channel

						dataChannel.onmessage = (e) => {
							// Send received messages to the callback
							this.#messageCallback?.(e.data)
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

	/**
	 * This callback is called when a message is received from the other peer.
	 */
	onMessage(callback: (message: string) => void) {
		this.#messageCallback = callback
	}
}
