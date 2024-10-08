import { IceCandidate, SignalMessage } from "./protocol.js"

/**
 * This class represents a client that connects to a host and can send and receive messages.
 * @param {string} host - The URL of the host to connect to.
 * @param {string} sessionId - The session ID to use for the connection.
 * @param {RTCIceServer[]} [iceServers] - The ICE servers to use for the connection.
 */
export class EzRTCClient {
	sessionId: string
	hostURL: string
	peerConnection: RTCPeerConnection
	dataChannel?: RTCDataChannel
	#messageCallback?: (message: string) => void

	constructor(host: string, sessionId: string, iceServers?: RTCIceServer[]) {
		this.hostURL = host
		this.sessionId = sessionId
		this.peerConnection = new RTCPeerConnection({
			iceServers: iceServers,
		})

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

							let iceInfo: IceCandidate = {
								candidate: e.candidate.candidate,
								sdpMid: e.candidate.sdpMid,
								sdpMLineIndex: e.candidate.sdpMLineIndex,
								usernameFragment: e.candidate.usernameFragment,
							}

							websocket.send(new SignalMessage().IceCandidate().Encode(sessionId, sdpOffer.userId, iceInfo))
						}
					}

					this.peerConnection.ondatachannel = (e) => {
						this.dataChannel = e.channel

						this.dataChannel.onmessage = (e) => {
							// Send received messages to the callback
							this.#messageCallback?.(e.data)
						}

						this.dataChannel.onopen = (e) => console.log("Data channel opened")
						this.dataChannel.onclose = (e) => console.log("Data channel closed")
					}

					this.peerConnection.setRemoteDescription(offer).then(() => {
						console.log("offer set")
					})

					this.peerConnection.createAnswer().then(async (a) => {
						await this.peerConnection.setLocalDescription(a)
						console.log("answer created")
					})

					this.peerConnection.onconnectionstatechange = (state) => {
						console.log("State changed", state.currentTarget)
					}
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

	/**
	 * Send a message to the other peer.
	 */
	sendMessage(message: string) {
		if (this.dataChannel?.readyState === "open") {
			this.dataChannel.send(message)
		}
	}
}
