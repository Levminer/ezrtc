import { EzRTCHost } from "../../../dist/index.js"

document.querySelector("#connect").addEventListener("click", () => {
	const host = new EzRTCHost("ws://localhost:9001/one-to-many", "random_session_id", [
		{
			urls: "stun:stun.cloudflare.com:3478",
		},
	])

	setInterval(() => {
		host.sendMessageToAll("test")
	}, 1000)
})
