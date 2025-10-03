import { EzRTCClient } from "../../../dist/index.js"

document.querySelector("#connect").addEventListener("click", () => {
	const client = new EzRTCClient("wss://rtc-usw.coresmonitor.com/one-to-many", "crs_6969", [
		{
			urls: "turn:turn.cloudflare.com:3478?transport=udp",
			username: "g09fa7f7c9c934ceb7910804c051876cb6f24beb588edc0eac083d7f8b789e0f",
			credential: "1181cb6f1eeb7e3f32faa1480e38460e333d2bc33fc9635b629cb1066934696c",
		},
	])

	client.onMessage((message) => {
		console.log("Message received", message)
	})

	client.sendMessage("test message")
})
