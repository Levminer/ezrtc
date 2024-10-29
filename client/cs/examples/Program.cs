using ezrtc;
using SIPSorcery.Net;

namespace examples
{
	internal class Program
	{
		internal static EzRTCHost EzRTCHost = new(new Uri("ws://localhost:9001/one-to-many"), "random-session-id", new List<RTCIceServer> { new RTCIceServer { urls = "stun:stun.cloudflare.com:3478" } });

		private static void Main(string[] args)
		{
			Task.Run(EzRTCHost.Start);
			while (true) { }
		}
	}
}
