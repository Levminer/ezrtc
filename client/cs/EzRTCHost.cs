using Serilog;
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
		public ManualResetEvent exitEvent = new(false);
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
				websocketClient.ReconnectTimeout = TimeSpan.FromSeconds(70);
				websocketClient.ReconnectionHappened.Subscribe(info =>
				{
					Log.Information("Session joined");
					Log.Warning($"Connection changed: {info.Type}");
					var joinMessage = SignalMessage.SessionJoin.Encode(sessionId, true);
					websocketClient.Send(joinMessage);
				});

				websocketClient.MessageReceived.Subscribe(async msg =>
				{
					Log.Information($"Message received: {msg}");

					if (!msg.Text.IsNullOrEmpty())
					{
						if (msg.Text.Contains("SessionReady"))
						{
							await HandleSessionReady(msg, websocketClient);
						}

						if (msg.Text.Contains("SdpAnswer"))
						{
							await HandleSdpAnswer(msg, websocketClient);
						}

						if (msg.Text.Contains("IceCandidate"))
						{
							await HandleIceCandidate(msg, websocketClient);
						}
					}
				});

				websocketClient.Start();
				exitEvent.WaitOne();
			};
		}

		private async Task HandleSessionReady(ResponseMessage msg, WebsocketClient websocketClient)
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
			Log.Information("Created offer");

			await peerConnection.setLocalDescription(offer);
			Log.Information("Local description set");

			var sdpOffer = SignalMessage.SdpOffer.Encode(sessionId, sessionReady.userId, peerConnection.localDescription.sdp.ToString());
			websocketClient.Send(sdpOffer);
		}

		private async Task HandleSdpAnswer(ResponseMessage msg, WebsocketClient websocketClient)
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
					Log.Information("Data channel open");
					dataChannelOpen?.Invoke(dc);
				};

				dc.onclose += () =>
				{
					Log.Information("Data channel closed");
				};

				dc.onmessage += (dataCh, proto, msg) =>
				{
					Log.Information($"Message received");
					var message = System.Text.Encoding.UTF8.GetString(msg);
					dataChannelMessage?.Invoke(message);
				};

				peerConnection.onconnectionstatechange += (state) =>
				{
					Log.Information($"STATE CHANGED => {state}");

					if (state == RTCPeerConnectionState.failed)
					{
						dataChannels[sdpAnswer.userId].close();
						peerConnection.close();

						peerConnections.TryRemove(sdpAnswer.userId, out _);
						dataChannels.TryRemove(sdpAnswer.userId, out _);
					}
				};
			}
		}

		private async Task HandleIceCandidate(ResponseMessage msg, WebsocketClient websocketClient)
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

			Log.Information("Ice candidate added");
		}

		// Send message to a specific user
		public void SendMessage(string message, string userId)
		{
			var dataChannel = dataChannels[userId];

			if (dataChannel != null && dataChannel.readyState == RTCDataChannelState.open)
			{
				dataChannel.send(message);
			}
		}

		// Send a message to all users.
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
