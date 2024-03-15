import { SignalMessage } from "./protocol.js"

/**
 * This class represents a host that connects to clients and can send and receive messages.
 * @param {string} host - The URL of the host to connect to.
 * @param {string} sessionId - The session ID to use for the connection.
 * @param {RTCIceServer[]} [iceServers] - The ICE servers to use for the connection.
 */
export class EzrtcHost {
	sessionId: string
	hostURL: string
	peerConnections = new Map<number, RTCPeerConnection>()
	dataChannels = new Map<number, RTCDataChannel>()
	#iceServers: RTCIceServer[] = []

	constructor(host: string, sessionId: string, iceServers?: RTCIceServer[]) {
		this.hostURL = host
		this.sessionId = sessionId
		this.#iceServers = iceServers ?? []

		const websocket = new WebSocket(host)

		websocket.onopen = (e) => {
			console.log("Connecting host", e)

			websocket.send(new SignalMessage().SessionJoin().Encode(sessionId, true))
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
				if (data.SessionReady) {
					const sessionReady = new SignalMessage().SessionReady().Decode(data)

					// create rtc peer connection
					const peerConnection = new RTCPeerConnection({
						iceServers: this.#iceServers,
					})
					this.peerConnections.set(sessionReady.userId, peerConnection)

					const dataChannel = peerConnection.createDataChannel(`send-${sessionReady.userId}`)
					this.dataChannels.set(sessionReady.userId, dataChannel)

					peerConnection.onicecandidate = (e) => {
						// Only send one ICE candidate
						console.log(e)
					}

					peerConnection.createOffer().then(async (a) => {
						await peerConnection.setLocalDescription(a)

						websocket.send(new SignalMessage().SdpOffer().Encode(sessionId, sessionReady.userId, peerConnection.localDescription!.sdp))
					})
				}

				if (data.SdpAnswer) {
					const sdpAnswer = new SignalMessage().SdpAnswer().Decode(data)
					const peerConnection = this.peerConnections.get(sdpAnswer.userId)

					const answer: RTCSessionDescriptionInit = {
						type: "answer",
						sdp: sdpAnswer.answer,
					}

					console.log(peerConnection?.connectionState)
					if (peerConnection && peerConnection.connectionState === "new") {
						peerConnection.setRemoteDescription(answer).then(() => {
							console.log("answer set")

							// get data channel
							const dataChannel = this.dataChannels.get(sdpAnswer.userId)

							if (dataChannel) {
								dataChannel.onopen = (e) => {
									console.log("Data channel opened")
								}

								dataChannel.onerror = (e) => {
									console.log("Data channel error", e)
								}

								dataChannel.onclose = (e) => {
									console.log("Data channel closed")
								}
							}
						})
					}
				}
			}
		}
	}

	sendMessage(message: string, userId: number) {
		const dataChannel = this.dataChannels.get(userId)
		if (dataChannel) {
			dataChannel.send(message)
		}
	}

	sendMessageToAll(message: string) {
		for (const [_userId, dataChannel] of this.dataChannels) {
			if (dataChannel.readyState === "open") {
				dataChannel.send(message)
			}
		}
	}
}
