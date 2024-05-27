using SIPSorcery.Net;
using System.Diagnostics;
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
		public Action<RTCDataChannel>? onDataChannelOpen { get; set; }

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
			{
				Debug.WriteLine($"Connection changed: ${info.Type}");
				var joinMessage = SignalMessage.SessionJoin.Encode(sessionId, true);
				websocketClient.Send(joinMessage);
			});

			RTCConfiguration config = new RTCConfiguration
			{
				iceServers = iceServers
			};

			websocketClient.MessageReceived.Subscribe(async msg =>
			{
				Debug.WriteLine($"Message received: {msg}");

				if (msg.Text.Contains("SessionReady"))
				{
					var sessionReady = SignalMessage.SessionReady.Decode(msg.Text);

					var peerConnection = new RTCPeerConnection(config);
					peerConnections.Add(sessionReady.userId, peerConnection);

					var dataChannel = await peerConnection.createDataChannel($"send-{sessionReady.userId}");
					dataChannels.Add(sessionReady.userId, dataChannel);
					dataChannel.onopen += () =>
					{
						onDataChannelOpen?.Invoke(dataChannel);
					};

					var offer = peerConnection.createOffer();

					await peerConnection.setLocalDescription(offer);

					var sdpOffer = SignalMessage.SdpOffer.Encode(sessionId, sessionReady.userId, peerConnection.localDescription.sdp.ToString());
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

					if (peerConnection != null && peerConnection.connectionState == RTCPeerConnectionState.@new)
					{
						var res = peerConnection.setRemoteDescription(answer);

						peerConnection.onconnectionstatechange += (state) =>
						{
							Debug.WriteLine(state.ToString());

							if (state == RTCPeerConnectionState.failed || state == RTCPeerConnectionState.disconnected)
							{
								dataChannels[sdpAnswer.userId].close();
								peerConnection.close();

								peerConnections.Remove(sdpAnswer.userId);
								dataChannels.Remove(sdpAnswer.userId);
							}
						};
					}
				}
			});

			websocketClient.Start();
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
