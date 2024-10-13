import { EzRTCClient } from "../../../dist/index.js"

document.querySelector("#connect").addEventListener("click", () => {
	const client = new EzRTCClient("ws://localhost:9001/one-to-many", "random_session_id", [
		{
			urls: "stun:stun.cloudflare.com:3478",
		},
	])

	client.onMessage((message) => {
		console.log("Message received", message)
	})

	client.sendMessage("test message")
})
