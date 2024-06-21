using SIPSorcery.Net;
using System.Collections.Concurrent;
using Websocket.Client;
using WebSocketSharp;

namespace ezrtc
{
	public class EzRTCHost
	{
		public Uri hostURL;
		public string sessionId;
		public List<RTCIceServer> iceServers;
		private ConcurrentDictionary<string, RTCPeerConnection> peerConnections = new();
		private ConcurrentDictionary<string, RTCDataChannel> dataChannels = new();
		public Action<RTCDataChannel>? dataChannelOpen { get; set; }
		public Action<string>? dataChannelMessage { get; set; }

		public EzRTCHost(Uri hostURL, string sessionId, List<RTCIceServer>? iceServers = null)
		{
			this.hostURL = hostURL;
			this.sessionId = sessionId;
			this.iceServers = iceServers ?? new List<RTCIceServer>();
		}

		public void Start()
		{
			using (var websocketClient = new WebsocketClient(hostURL))
			{
				websocketClient.ReconnectTimeout = TimeSpan.FromSeconds(90);
				websocketClient.ReconnectionHappened.Subscribe(info =>
				{
					Console.WriteLine("Session joined");
					Console.WriteLine($"Connection changed: ${info.Type}");
					var joinMessage = SignalMessage.SessionJoin.Encode(sessionId, true);
					websocketClient.Send(joinMessage);
				});

				websocketClient.MessageReceived.Subscribe(async msg =>
				{
					Console.WriteLine($"Message received: {msg}");

					if (!msg.Text.IsNullOrEmpty())
					{
						if (msg.Text.Contains("SessionReady"))
						{
							await HandleSessionReady(msg, websocketClient);
						}

						if (msg.Text.Contains("SdpAnswer"))
						{
							await HandleSdpAnwser(msg, websocketClient);
						}

						if (msg.Text.Contains("IceCandidate"))
						{
							await HandleIceCandidate(msg, websocketClient);
						}
					}
				});

				websocketClient.Start();

				while (true) { }
			};
		}

		public async Task HandleSessionReady(ResponseMessage msg, WebsocketClient websocketClient)
		{
			var sessionReady = SignalMessage.SessionReady.Decode(msg.Text);

			RTCConfiguration config = new()
			{
				iceServers = iceServers
			};

			var peerConnection = new RTCPeerConnection(config);
			peerConnections.TryAdd(sessionReady.userId, peerConnection);
			var dataChannel = await peerConnection.createDataChannel("testing");
			dataChannels.TryAdd(sessionReady.userId, dataChannel);

			var offer = peerConnection.createOffer();
			Console.WriteLine("Created offer");

			await peerConnection.setLocalDescription(offer);
			Console.WriteLine("Local description set");

			var sdpOffer = SignalMessage.SdpOffer.Encode(sessionId, sessionReady.userId, peerConnection.localDescription.sdp.ToString());
			websocketClient.Send(sdpOffer);
		}

		public async Task HandleSdpAnwser(ResponseMessage msg, WebsocketClient websocketClient)
		{
			var sdpAnswer = SignalMessage.SdpAnswer.Decode(msg.Text);
			var peerConnection = peerConnections[sdpAnswer.userId];

			var answer = new RTCSessionDescriptionInit
			{
				type = RTCSdpType.answer,
				sdp = sdpAnswer.answer
			};

			if (peerConnection != null && peerConnection.signalingState == RTCSignalingState.have_local_offer)
			{
				var res = peerConnection.setRemoteDescription(answer);

				var dc = dataChannels[sdpAnswer.userId];

				dc.onopen += () =>
				{
					Console.WriteLine("Data channel open");
					dataChannelOpen?.Invoke(dc);
				};

				dc.onclose += () =>
				{
					Console.WriteLine("Data channel closed");
				};

				dc.onmessage += (dataCh, proto, msg) =>
				{
					var message = System.Text.Encoding.UTF8.GetString(msg);
					Console.WriteLine($"Message received");
					dataChannelMessage?.Invoke(message);
				};

				peerConnection.onconnectionstatechange += (state) =>
				{
					Console.WriteLine($"STATE CHANGED => {state}");
				};
			}
		}

		public async Task HandleIceCandidate(ResponseMessage msg, WebsocketClient websocketClient)
		{
			var incomingIceCandidate = SignalMessage.IceCandidate.Decode(msg.Text);
			var peerConnection = peerConnections[incomingIceCandidate.userId];

			var iceInit = new RTCIceCandidateInit
			{
				candidate = incomingIceCandidate.candidate.candidate,
				sdpMid = incomingIceCandidate.candidate.sdpMid,
				sdpMLineIndex = (ushort)incomingIceCandidate.candidate.sdpMLineIndex,
				usernameFragment = incomingIceCandidate.candidate.usernameFragment,
			};

			peerConnection.addIceCandidate(iceInit);

			Console.WriteLine("Ice candidate added");
		}

		public void sendMessageToAll(string message)
		{
			foreach (KeyValuePair<string, RTCDataChannel> entry in dataChannels)
			{
				var dataChannel = entry.Value;

				if (dataChannel.readyState == RTCDataChannelState.open)
				{
					dataChannel.send(message);
				}
			}
		}
	}
}
