export class SignalMessage {
	SessionJoin() {
		return {
			Encode: (sessionId: string, isHost: boolean) => JSON.stringify({ SessionJoin: [sessionId, isHost] }),
		}
	}

	SessionReady() {
		return {
			Encode: (sessionId: string, userId: number) => JSON.stringify({ SessionReady: [sessionId, userId] }),
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
		}
	}

	IceCandidate() {
		return {
			Encode: (sessionId: string, userId: number, candidate: string) => JSON.stringify({ IceCandidate: [sessionId, userId, candidate] }),
		}
	}
}
