import { SignalMessage } from "./protocol.js"

/**
 * This class represents a host that connects to clients and can send and receive messages.
 * @param {string} host - The URL of the host to connect to.
 * @param {string} sessionId - The session ID to use for the connection.
 * @param {RTCIceServer[]} [iceServers] - The ICE servers to use for the connection.
 */
export class EzRTCHost {
	sessionId: string
	hostURL: string
	peerConnections = new Map<number, RTCPeerConnection>()
	dataChannels = new Map<number, RTCDataChannel>()
	#iceServers: RTCIceServer[] = []
	#pendingIceCandidates = new Map<number, RTCIceCandidateInit[]>()
	#remoteDescriptionSet = new Map<number, boolean>()

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
					this.#pendingIceCandidates.set(sessionReady.userId, [])
					this.#remoteDescriptionSet.set(sessionReady.userId, false)

					const dataChannel = peerConnection.createDataChannel(`send-${sessionReady.userId}`)
					this.dataChannels.set(sessionReady.userId, dataChannel)

					peerConnection.onicecandidate = (e) => {
						// Send ICE candidates to the client
						if (e.candidate) {
							console.log("Host sending ICE candidate:", e.candidate)
							const iceInfo = {
								candidate: e.candidate.candidate,
								sdpMid: e.candidate.sdpMid,
								sdpMLineIndex: e.candidate.sdpMLineIndex,
								usernameFragment: e.candidate.usernameFragment,
							}

							websocket.send(new SignalMessage().IceCandidate().Encode(sessionId, sessionReady.userId, iceInfo))
						} else {
							console.log("Host ICE gathering complete (null candidate)")
						}
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

					if (peerConnection && peerConnection.signalingState === "have-local-offer") {
						peerConnection.setRemoteDescription(answer).then(() => {
							console.log("Answer set")
							this.#remoteDescriptionSet.set(sdpAnswer.userId, true)

							// Add any pending ICE candidates now that remote description is set
							const pendingCandidates = this.#pendingIceCandidates.get(sdpAnswer.userId) || []
							pendingCandidates.forEach((candidate) => {
								peerConnection
									.addIceCandidate(candidate)
									.then(() => console.log("Queued ICE candidate added"))
									.catch((err) => console.error("Error adding queued ICE candidate:", err))
							})
							this.#pendingIceCandidates.set(sdpAnswer.userId, [])

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

				if (data.IceCandidate) {
					const iceCandidate = new SignalMessage().IceCandidate().Decode(data)
					console.log("Host received ICE candidate:", iceCandidate)
					const peerConnection = this.peerConnections.get(iceCandidate.userId)

					if (peerConnection) {
						const candidate: RTCIceCandidateInit = {
							candidate: iceCandidate.candidate.candidate,
							sdpMid: iceCandidate.candidate.sdpMid,
							sdpMLineIndex: iceCandidate.candidate.sdpMLineIndex,
							usernameFragment: iceCandidate.candidate.usernameFragment,
						}

						// Queue candidates if remote description hasn't been set yet
						const remoteDescSet = this.#remoteDescriptionSet.get(iceCandidate.userId) || false
						if (!remoteDescSet) {
							console.log("Queueing ICE candidate (remote description not set yet)")
							const queue = this.#pendingIceCandidates.get(iceCandidate.userId) || []
							queue.push(candidate)
							this.#pendingIceCandidates.set(iceCandidate.userId, queue)
						} else {
							peerConnection
								.addIceCandidate(candidate)
								.then(() => console.log("Ice candidate added successfully"))
								.catch((err) => console.error("Error adding ICE candidate:", err))
						}
					}
				}
			}
		}
	}

	/**
	 * Send a message to a specific user.
	 */
	sendMessage(message: string, userId: number) {
		const dataChannel = this.dataChannels.get(userId)
		if (dataChannel) {
			dataChannel.send(message)
		}
	}

	/**
	 * Send a message to all users.
	 */
	sendMessageToAll(message: string) {
		for (const [_userId, dataChannel] of this.dataChannels) {
			if (dataChannel.readyState === "open") {
				dataChannel.send(message)
			}
		}
	}
}
