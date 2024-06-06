export interface IceCandidate {
	candidate: string
	sdpMid: string | null
	sdpMLineIndex: number | null
	usernameFragment: string | null
}

export class SignalMessage {
	SessionJoin() {
		return {
			Encode: (sessionId: string, isHost: boolean) => JSON.stringify({ SessionJoin: [sessionId, isHost] }),
		}
	}

	SessionReady() {
		return {
			Encode: (sessionId: string, userId: number) => JSON.stringify({ SessionReady: [sessionId, userId] }),
			Decode: (data: { SessionReady: any[] }): { sessionId: string; userId: number } => {
				return {
					sessionId: data.SessionReady[0],
					userId: data.SessionReady[1],
				}
			},
		}
	}

	SdpOffer() {
		return {
			Encode: (sessionId: string, userId: number, offer: string) => JSON.stringify({ SdpOffer: [sessionId, userId, offer] }),
			Decode: (data: { SdpOffer: any[] }): { sessionId: string; userId: number; offer: string } => {
				return {
					sessionId: data.SdpOffer[0],
					userId: data.SdpOffer[1],
					offer: data.SdpOffer[2],
				}
			},
		}
	}

	SdpAnswer() {
		return {
			Encode: (sessionId: string, userId: number, answer: string) => JSON.stringify({ SdpAnswer: [sessionId, userId, answer] }),
			Decode: (data: { SdpAnswer: any[] }): { sessionId: string; userId: number; answer: string } => {
				return {
					sessionId: data.SdpAnswer[0],
					userId: data.SdpAnswer[1],
					answer: data.SdpAnswer[2],
				}
			},
		}
	}

	IceCandidate() {
		return {
			Encode: (sessionId: string, userId: number, candidate: IceCandidate) =>
				JSON.stringify({ IceCandidate: [sessionId, userId, JSON.stringify(candidate)] }),
			Decode: (data: { IceCandidate: any[] }): { sessionId: string; userId: number; candidate: IceCandidate } => {
				return {
					sessionId: data.IceCandidate[0],
					userId: data.IceCandidate[1],
					candidate: JSON.parse(data.IceCandidate[2]),
				}
			},
		}
	}
}
