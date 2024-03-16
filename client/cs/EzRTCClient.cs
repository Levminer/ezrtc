using SIPSorcery.Net;
using System.Text;
using Websocket.Client;

namespace ezrtc
{
	public class EzRTCClient
	{
		public Uri hostURL;
		public string sessionId;
		public List<RTCIceServer> iceServers;

		public EzRTCClient(Uri hostURL, string sessionId, List<RTCIceServer>? iceServers = null)
		{
			this.hostURL = hostURL;
			this.sessionId = sessionId;
			this.iceServers = iceServers ?? new List<RTCIceServer>();
		}

		public void Start()
		{
			var exitEvent = new ManualResetEvent(false);
			using var websocketClient = new WebsocketClient(hostURL);

			websocketClient.ReconnectTimeout = TimeSpan.FromSeconds(30);
			websocketClient.ReconnectionHappened.Subscribe(info =>
				Console.WriteLine($"Reconnection happened, type: {info.Type}"));

			RTCConfiguration config = new RTCConfiguration
			{
				iceServers = iceServers,
			};

			var peerConnection = new RTCPeerConnection(config);

			websocketClient.MessageReceived.Subscribe(msg =>
			{
				Console.WriteLine($"Message received: {msg}");

				if (msg.Text.Contains("SdpOffer"))
				{
					var sdpOffer = SignalMessage.SdpOffer.Decode(msg.Text);

					var offer = new RTCSessionDescriptionInit
					{
						type = RTCSdpType.offer,
						sdp = sdpOffer.offer,
					};

					peerConnection.ondatachannel += (dc) =>
					{
						dc.onmessage += (dc, protocol, data) =>
						{
							Console.WriteLine(Encoding.UTF8.GetString(data));
						};
					};

					var result = peerConnection.setRemoteDescription(offer);

					if (result == SetDescriptionResultEnum.OK)
					{
						var answer = peerConnection.createAnswer();

						peerConnection.setLocalDescription(answer);

						peerConnection.onicecandidate += (candidate) =>
						{
							var lc = peerConnection.localDescription;

							var sdpAnswer = SignalMessage.SdpAnswer.Encode(sessionId, sdpOffer.userId, lc.sdp.ToString());

							Console.WriteLine(sdpAnswer);

							websocketClient.Send(sdpAnswer);
						};
					}
				}
			});
			websocketClient.Start();

			Task.Run(() =>
			{
				var joinMessage = SignalMessage.SessionJoin.Encode(sessionId, false);

				websocketClient.Send(joinMessage);
			});

			exitEvent.WaitOne();
		}
	}
}
