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
		private ConcurrentDictionary<string, List<RTCIceCandidateInit>> pendingIceCandidates = new();
		private ConcurrentDictionary<string, bool> remoteDescriptionSet = new();
		public ManualResetEvent exitEvent = new(false);
		public Action<RTCDataChannel>? dataChannelOpen { get; set; }
		public Action<string>? dataChannelMessage { get; set; }
		public Action<WebsocketClient, string>? keepAliveMessage { get; set; }

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

						if (msg.Text.Contains("KeepAlive"))
						{
							await HandleKeepAlive(msg, websocketClient);
						}
					}
				});

				websocketClient.Start();
				exitEvent.WaitOne();
			};
		}

		private async Task HandleKeepAlive(ResponseMessage msg, WebsocketClient websocketClient)
		{
			if (keepAliveMessage != null)
			{
				keepAliveMessage.Invoke(websocketClient, msg.Text);
			}
			else
			{
				var keepAlive = SignalMessage.KeepAlive.Decode(msg.Text);

				var message = SignalMessage.KeepAlive.Encode(keepAlive.userId, new Status { is_host = true, session_id = sessionId, version = "0.5.0" });

				websocketClient.Send(message);
			}
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
			pendingIceCandidates.TryAdd(sessionReady.userId, new List<RTCIceCandidateInit>());
			remoteDescriptionSet.TryAdd(sessionReady.userId, false);
			
			var dataChannel = await peerConnection.createDataChannel("testing");
			dataChannels.TryAdd(sessionReady.userId, dataChannel);

			// Set up ICE candidate handler to send candidates to the client
			peerConnection.onicecandidate += (iceCandidate) =>
			{
				if (iceCandidate != null)
				{
					Log.Information($"Host sending ICE candidate: {iceCandidate.candidate}");
					
					var iceCandidateInfo = new IceCandidateInfo
					{
						candidate = iceCandidate.candidate,
						sdpMid = iceCandidate.sdpMid,
						sdpMLineIndex = iceCandidate.sdpMLineIndex,
						usernameFragment = iceCandidate.usernameFragment
					};

					var iceCandidateMessage = SignalMessage.IceCandidate.Encode(sessionId, sessionReady.userId, iceCandidateInfo);
					websocketClient.Send(iceCandidateMessage);
				}
				else
				{
					Log.Information("Host ICE gathering complete (null candidate)");
				}
			};

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
			
			if (!peerConnections.TryGetValue(sdpAnswer.userId, out var peerConnection))
			{
				Log.Warning($"Peer connection not found for user ID: {sdpAnswer.userId}");
				return;
			}

			var answer = new RTCSessionDescriptionInit
			{
				type = RTCSdpType.answer,
				sdp = sdpAnswer.answer
			};

			if (peerConnection != null && peerConnection.signalingState == RTCSignalingState.have_local_offer)
			{
				var res = peerConnection.setRemoteDescription(answer);
				Log.Information("Answer set");
				
				// Mark remote description as set and add any pending ICE candidates
				remoteDescriptionSet.TryUpdate(sdpAnswer.userId, true, false);
				
				if (pendingIceCandidates.TryGetValue(sdpAnswer.userId, out var pendingCandidates))
				{
					foreach (var candidate in pendingCandidates)
					{
						try
						{
							peerConnection.addIceCandidate(candidate);
							Log.Information("Queued ICE candidate added");
						}
						catch (Exception ex)
						{
							Log.Error($"Error adding queued ICE candidate: {ex.Message}");
						}
					}
					pendingCandidates.Clear();
				}

				if (!dataChannels.TryGetValue(sdpAnswer.userId, out var dc))
				{
					Log.Warning($"Data channel not found for user ID: {sdpAnswer.userId}");
					return;
				}

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
						if (dataChannels.TryGetValue(sdpAnswer.userId, out var dataChannel))
						{
							dataChannel.close();
							dataChannels.TryRemove(sdpAnswer.userId, out _);
						}
						
						peerConnection.close();
						peerConnections.TryRemove(sdpAnswer.userId, out _);
					}
				};
			}
		}

		private async Task HandleIceCandidate(ResponseMessage msg, WebsocketClient websocketClient)
		{
			var incomingIceCandidate = SignalMessage.IceCandidate.Decode(msg.Text);
			
			if (!peerConnections.TryGetValue(incomingIceCandidate.userId, out var peerConnection))
			{
				Log.Warning($"Peer connection not found for user ID: {incomingIceCandidate.userId}");
				return;
			}

			var iceInit = new RTCIceCandidateInit
			{
				candidate = incomingIceCandidate.candidate.candidate,
				sdpMid = incomingIceCandidate.candidate.sdpMid,
				sdpMLineIndex = (ushort)incomingIceCandidate.candidate.sdpMLineIndex,
				usernameFragment = incomingIceCandidate.candidate.usernameFragment,
			};

			// Queue candidates if remote description hasn't been set yet
			if (remoteDescriptionSet.TryGetValue(incomingIceCandidate.userId, out var isRemoteDescSet) && !isRemoteDescSet)
			{
				Log.Information("Queueing ICE candidate (remote description not set yet)");
				if (pendingIceCandidates.TryGetValue(incomingIceCandidate.userId, out var queue))
				{
					queue.Add(iceInit);
				}
			}
			else
			{
				try
				{
					peerConnection.addIceCandidate(iceInit);
					Log.Information("Ice candidate added successfully");
				}
				catch (Exception ex)
				{
					Log.Error($"Error adding ICE candidate: {ex.Message}");
				}
			}
		}

		// Send message to a specific user
		public void SendMessage(string message, string userId)
		{
			if (!dataChannels.TryGetValue(userId, out var dataChannel))
			{
				Log.Warning($"Data channel not found for user ID: {userId}");
				return;
			}

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
