using System.Text.Json;

namespace ezrtc
{
	public class ICandidate
	{
		public string candidate { get; set; }
		public string sdpMid { get; set; }
		public int sdpMLineIndex { get; set; }
		public string usernameFragment { get; set; }
	}

	public class Status
	{
		public string? session_id { get; set; }
		public bool? is_host { get; set; }
		public string? version { get; set; }
	}

	public class SignalMessage
	{
		public class KeepAlive
		{
			public class KeepAliveInput
			{
				public object[] KeepAlive { get; set; }
			}

			public class KeepAliveOutput
			{
				public string userId;
				public Status status;
			}

			public static KeepAliveOutput Decode(string data)
			{
				var message = JsonSerializer.Deserialize<KeepAliveInput>(data);
				object[] keepAlive = message.KeepAlive;

				return new KeepAliveOutput
				{
					userId = keepAlive[0].ToString(),
					status = JsonSerializer.Deserialize<Status>(keepAlive[1].ToString())
				};
			}

			public static string Encode(string userId, Status status)
			{
				var message = new { KeepAlive = new object[] { Convert.ToUInt32(userId), status } };
				return JsonSerializer.Serialize(message);
			}
		}

		public class SessionJoin
		{
			public static string Encode(string sessionId, bool isHost)
			{
				var message = new { SessionJoin = new object[] { sessionId, isHost } };
				return JsonSerializer.Serialize(message);
			}
		}

		public class SessionReady
		{
			public class SessionReadyInput
			{
				public object[] SessionReady { get; set; }
			}

			public class SessionReadyOutput
			{
				public string sessionId;
				public string userId;
			}

			public static SessionReadyOutput Decode(string data)
			{
				var message = JsonSerializer.Deserialize<SessionReadyInput>(data);
				object[] sessionReady = message.SessionReady;

				return new SessionReadyOutput
				{
					sessionId = sessionReady[0].ToString(),
					userId = sessionReady[1].ToString(),
				};
			}
		}

		public class SdpOffer
		{
			public class SdpOfferInput
			{
				public object[] SdpOffer { get; set; }
			}

			public class SdpOfferOutput
			{
				public string sessionId;
				public string userId;
				public string offer;
			}

			public static string Encode(string sessionId, string userId, string offer)
			{
				var message = new { SdpOffer = new object[] { sessionId, Convert.ToUInt32(userId), offer } };
				return JsonSerializer.Serialize(message);
			}

			public static SdpOfferOutput Decode(string data)
			{
				var message = JsonSerializer.Deserialize<SdpOfferInput>(data);
				object[] sdpOffer = message.SdpOffer;

				return new SdpOfferOutput
				{
					sessionId = sdpOffer[0].ToString(),
					userId = sdpOffer[1].ToString(),
					offer = sdpOffer[2].ToString()
				};
			}
		}

		public class SdpAnswer
		{
			public class SdpAnswerInput
			{
				public object[] SdpAnswer { get; set; }
			}

			public class SdpAnswerOutput
			{
				public string sessionId;
				public string userId;
				public string answer;
			}

			public static SdpAnswerOutput Decode(string data)
			{
				var message = JsonSerializer.Deserialize<SdpAnswerInput>(data);
				object[] sdpOffer = message.SdpAnswer;

				return new SdpAnswerOutput
				{
					sessionId = sdpOffer[0].ToString(),
					userId = sdpOffer[1].ToString(),
					answer = sdpOffer[2].ToString()
				};
			}

			public static string Encode(string sessionId, string userId, string answer)
			{
				var message = new { SdpAnswer = new object[] { sessionId, Convert.ToUInt32(userId), answer } };
				return JsonSerializer.Serialize(message);
			}
		}

		public class IceCandidate
		{
			public class IceCandidateInput
			{
				public object[] IceCandidate { get; set; }
			}

			public class IceCandidateOutput
			{
				public string sessionId;
				public string userId;
				public ICandidate candidate;
			}

			public static IceCandidateOutput Decode(string data)
			{
				var message = JsonSerializer.Deserialize<IceCandidateInput>(data);
				object[] sdpOffer = message.IceCandidate;

				return new IceCandidateOutput
				{
					sessionId = sdpOffer[0].ToString(),
					userId = sdpOffer[1].ToString(),
					candidate = JsonSerializer.Deserialize<ICandidate>(sdpOffer[2].ToString()),
				};
			}
		}
	}
}
