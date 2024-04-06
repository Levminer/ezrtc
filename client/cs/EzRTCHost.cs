using SIPSorcery.Net;
using Websocket.Client;

namespace ezrtc
{
	public class EzRTCHost
	{
		public Uri hostURL;
		public string sessionId;
		public List<RTCIceServer> iceServers;
		private Dictionary<string, RTCPeerConnection> peerConnections = new();
		private Dictionary<string, RTCDataChannel> dataChannels = new();

		public EzRTCHost(Uri hostURL, string sessionId, List<RTCIceServer>? iceServers = null)
		{
			this.hostURL = hostURL;
			this.sessionId = sessionId;
			this.iceServers = iceServers ?? new List<RTCIceServer>();
		}

		public void Start()
		{
			var exitEvent = new ManualResetEvent(false);
			using var websocketClient = new WebsocketClient(hostURL);

			websocketClient.ReconnectTimeout = TimeSpan.FromSeconds(90);
			websocketClient.ReconnectionHappened.Subscribe(info =>
				Console.WriteLine($"Reconnection happened, type: {info.Type}"));

			RTCConfiguration config = new RTCConfiguration
			{
				iceServers = iceServers
			};

			websocketClient.MessageReceived.Subscribe(async msg =>
			{
				Console.WriteLine($"Message received: {msg}");

				if (msg.Text.Contains("SessionReady"))
				{
					var sessionReady = SignalMessage.SessionReady.Decode(msg.Text);

					var peerConnection = new RTCPeerConnection(config);
					peerConnections.Add(sessionReady.userId, peerConnection);

					var dataChannel = await peerConnection.createDataChannel($"send-{sessionReady.userId}");
					dataChannels.Add(sessionReady.userId, dataChannel);

					var offer = peerConnection.createOffer();

					await peerConnection.setLocalDescription(offer);

					var sdpOffer = SignalMessage.SdpOffer.Encode(sessionId, sessionReady.userId, peerConnection.localDescription.sdp.ToString());
					Console.WriteLine(sdpOffer);
					websocketClient.Send(sdpOffer);
				}

				if (msg.Text.Contains("SdpAnswer"))
				{
					var sdpAnswer = SignalMessage.SdpAnswer.Decode(msg.Text);
					var peerConnection = peerConnections[sdpAnswer.userId];

					var answer = new RTCSessionDescriptionInit
					{
						sdp = sdpAnswer.answer,
						type = RTCSdpType.answer,
					};

					if (peerConnection.connectionState == RTCPeerConnectionState.@new)
					{
						var result = peerConnection.setRemoteDescription(answer);

						if (result == SetDescriptionResultEnum.OK)
						{
							var dataChannel = dataChannels[sdpAnswer.userId];

							dataChannel.onopen += () =>
							{
								Console.WriteLine("dc open");
							};
						}
					}

				}
			});

			websocketClient.Start();

			Task.Run(() =>
			{
				var joinMessage = SignalMessage.SessionJoin.Encode(sessionId, true);

				websocketClient.Send(joinMessage);
			});

			exitEvent.WaitOne();
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
