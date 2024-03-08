import { SignalMessage } from "./protocol.js"

export class EzrtcHost {
	sessionId: string
	host: string
	rc = new RTCPeerConnection()
	#send = true
	#messageChannel: any

	constructor(host: string, sessionId: string) {
		this.host = host
		this.sessionId = sessionId

		const websocket = new WebSocket(host)

		websocket.onopen = (e) => {
			console.log("Connecting host", e)

			websocket.send(new SignalMessage().SessionJoin().Encode(sessionId, true))
		}

		websocket.onclose = (e) => {
			console.log("Closed connection with host", e)
		}

		websocket.onerror = (e) => {
			console.log("Error connecting with host", e)
		}

		websocket.onmessage = (e) => {
			const data = e.data.startsWith("ping") ? null : JSON.parse(e.data)

			console.log("Websocket event received", e)

			if (data != null) {
				if (data.SessionReady) {
					const sessionReady = new SignalMessage().SessionReady().Decode(data)

					const sendChannel = this.rc.createDataChannel("send")
					sendChannel.onopen = (e) => console.log("Data channel opened")
					sendChannel.onclose = (e) => console.log("Data channel closed")
					this.#messageChannel = sendChannel

					this.rc.onicecandidate = (e) => {
						// Only send one ICE candidate
						console.log(e)
					}

					this.rc.createOffer().then(async (a) => {
						await this.rc.setLocalDescription(a)

						websocket.send(new SignalMessage().SdpOffer().Encode(sessionId, sessionReady.userId, this.rc.localDescription!.sdp))
					})
				}

				if (data.SdpAnswer) {
					const sdpAnswer = new SignalMessage().SdpAnswer().Decode(data)

					const answer: RTCSessionDescriptionInit = {
						type: "answer",
						sdp: sdpAnswer.answer,
					}

					this.rc.setRemoteDescription(answer).then(() => {
						console.log("answer set")

						this.#messageChannel.send(
							'{"cpu":{"name":"12th Gen Intel Core i5-12600KF","load":[{"name":"CPU Core #1 Thread #1","value":24.012142,"min":24.012142,"max":94.11648},{"name":"CPU Core #1 Thread #2","value":4.6124816,"min":0.94783306,"max":11.84864},{"name":"CPU Core #2 Thread #1","value":26.736206,"min":26.736206,"max":38.839703},{"name":"CPU Core #2 Thread #2","value":1.8008888,"min":0.56884885,"max":12.27296},{"name":"CPU Core #3 Thread #1","value":20.886045,"min":15.930807,"max":67.99318},{"name":"CPU Core #3 Thread #2","value":2.7930796,"min":0,"max":12.305599},{"name":"CPU Core #4 Thread #1","value":13.84055,"min":13.84055,"max":29.533285},{"name":"CPU Core #4 Thread #2","value":1.9011796,"min":0,"max":11.816639},{"name":"CPU Core #5 Thread #1","value":9.714973,"min":9.714973,"max":21.0872},{"name":"CPU Core #5 Thread #2","value":1.6389728,"min":1.6389728,"max":13.768321},{"name":"CPU Core #6 Thread #1","value":30.844784,"min":22.033918,"max":33.27564},{"name":"CPU Core #6 Thread #2","value":3.7766397,"min":3.4010172,"max":12.0208025},{"name":"CPU Core #7","value":2.775079,"min":2.775079,"max":12.034559},{"name":"CPU Core #8","value":3.0516624,"min":3.0516624,"max":42.257065},{"name":"CPU Core #9","value":7.386446,"min":7.386446,"max":18.265348},{"name":"CPU Core #10","value":5.3955197,"min":5.3955197,"max":13.065731}],"maxLoad":10.072923,"temperature":[{"name":"CPU Core #1","value":27,"min":27,"max":33},{"name":"CPU Core #2","value":27,"min":27,"max":30},{"name":"CPU Core #3","value":30,"min":30,"max":34},{"name":"CPU Core #4","value":28,"min":28,"max":30},{"name":"CPU Core #5","value":29,"min":29,"max":32},{"name":"CPU Core #6","value":27,"min":27,"max":34},{"name":"CPU Core #7","value":33,"min":31,"max":33},{"name":"CPU Core #8","value":33,"min":31,"max":33},{"name":"CPU Core #9","value":33,"min":31,"max":33},{"name":"CPU Core #10","value":33,"min":31,"max":33}],"power":[{"name":"CPU Package","value":25,"min":12,"max":37},{"name":"CPU Cores","value":18,"min":6,"max":30},{"name":"CPU Memory","value":0,"min":0,"max":0}],"clock":[{"name":"CPU Core #1","value":4483,"min":4483,"max":4484},{"name":"CPU Core #2","value":4483,"min":2889,"max":4483},{"name":"CPU Core #3","value":4483,"min":2491,"max":4483},{"name":"CPU Core #4","value":4483,"min":1196,"max":4483},{"name":"CPU Core #5","value":4483,"min":598,"max":4483},{"name":"CPU Core #6","value":4483,"min":498,"max":4483},{"name":"CPU Core #7","value":3388,"min":399,"max":3388},{"name":"CPU Core #8","value":3388,"min":399,"max":3388},{"name":"CPU Core #9","value":1993,"min":399,"max":2491},{"name":"CPU Core #10","value":897,"min":399,"max":2790}],"voltage":[{"name":"CPU Core #1","value":0.7,"min":0.68,"max":0.99},{"name":"CPU Core #2","value":0.7,"min":0.68,"max":1.07},{"name":"CPU Core #3","value":0.69,"min":0.68,"max":1.08},{"name":"CPU Core #4","value":0.67,"min":0.67,"max":1.08},{"name":"CPU Core #5","value":0.69,"min":0.67,"max":1.07},{"name":"CPU Core #6","value":0.69,"min":0.67,"max":1.07},{"name":"CPU Core #7","value":0.7,"min":0.67,"max":1.08},{"name":"CPU Core #8","value":0.7,"min":0.67,"max":1.08},{"name":"CPU Core #9","value":0.7,"min":0.68,"max":1.07},{"name":"CPU Core #10","value":0.7,"min":0.68,"max":1.06}],"info":[{"characteristics":63,"coreCount":10,"coreEnabled":10,"currentSpeed":4455,"externalClock":100,"family":205,"handle":76,"id":13829424153407064000,"l1CacheHandle":73,"l2CacheHandle":74,"l3CacheHandle":75,"manufacturerName":"Intel(R) Corporation","maxSpeed":4900,"processorType":3,"serial":"To Be Filled By O.E.M.","socket":64,"socketDesignation":"LGA1700","threadCount":16,"version":"12th Gen Intel(R) Core(TM) i5-12600KF"}]},"gpu":{"fan":[{"name":"GPU Fan 1","value":0,"min":0,"max":0},{"name":"GPU Fan 2","value":0,"min":0,"max":0}],"memory":[{"name":"D3D Dedicated Memory Used","value":1.9,"min":1.8,"max":1.9},{"name":"D3D Shared Memory Used","value":0.1,"min":0.1,"max":0.1},{"name":"GPU Memory Total","value":12,"min":12,"max":12},{"name":"GPU Memory Free","value":9.9,"min":9.9,"max":10},{"name":"GPU Memory Used","value":2.1,"min":2,"max":2.1}],"info":"20240215000000.000000-000","name":"NVIDIA GeForce RTX 4070","load":[{"name":"3D","value":0.19393042,"min":0,"max":0},{"name":"Copy","value":0.73950744,"min":0,"max":0},{"name":"Video Encode","value":0,"min":0,"max":0},{"name":"Video Decode","value":0,"min":0,"max":0}],"maxLoad":0.73950744,"temperature":[{"name":"GPU Core","value":37,"min":37,"max":37},{"name":"GPU Hot Spot","value":44,"min":43,"max":44}],"power":[{"name":"GPU Package","value":33,"min":33,"max":37}],"clock":[{"name":"GPU Core","value":2475,"min":810,"max":2475},{"name":"GPU Memory","value":10502,"min":5002,"max":10502}],"voltage":[]},"ram":{"load":[{"name":"Memory Used","value":12.8,"min":12.6,"max":12.8},{"name":"Memory Available","value":19,"min":19,"max":19.2},{"name":"Memory","value":40.3,"min":39.6,"max":40.3},{"name":"Virtual Memory Used","value":19.8,"min":19.5,"max":19.8},{"name":"Virtual Memory Available","value":16.7,"min":16.7,"max":17},{"name":"Virtual Memory","value":54.2,"min":53.4,"max":54.2}],"info":[{"bankLocator":"BANK 0","deviceLocator":"Controller0-ChannelA-DIMM0","manufacturerName":"A-DATA Technology","partNumber":"AX4U320016G16A-SB10","serialNumber":"10EA0400","size":16384,"speed":3200,"configuredSpeed":3200,"configuredVoltage":1200,"type":26},{"bankLocator":"BANK 0","deviceLocator":"Controller1-ChannelA-DIMM0","manufacturerName":"A-DATA Technology","partNumber":"DDR4 3200","serialNumber":"69700000","size":16384,"speed":3200,"configuredSpeed":3200,"configuredVoltage":1200,"type":26}],"layout":[{"bankLocator":"BANK 0","deviceLocator":"Controller0-ChannelA-DIMM0","manufacturerName":"A-DATA Technology","partNumber":"AX4U320016G16A-SB10","serialNumber":"10EA0400","size":16384,"speed":3200,"configuredSpeed":3200,"configuredVoltage":1200,"type":26},{"bankLocator":"BANK 0","deviceLocator":"Controller1-ChannelA-DIMM0","manufacturerName":"A-DATA Technology","partNumber":"DDR4 3200","serialNumber":"69700000","size":16384,"speed":3200,"configuredSpeed":3200,"configuredVoltage":1200,"type":26}]},"system":{"os":{"name":"Windows 11 Pro x64 10.0.22631","webView":"122.0.2365.66","app":"0.17.0","runtime":"1.5.240227000"},"storage":{"disks":[{"name":"KINGSTON SA400S37240G","id":{},"temperature":{"name":"Temperature","value":26,"min":26,"max":26},"totalSpace":221,"freeSpace":91,"health":"65","throughputRead":0,"throughputWrite":0,"dataRead":0,"dataWritten":0},{"name":"KINGSTON SA400S37240G","id":{},"temperature":{"name":"Temperature","value":26,"min":26,"max":26},"totalSpace":222,"freeSpace":11,"health":"81","throughputRead":0,"throughputWrite":3175981.5,"dataRead":0,"dataWritten":0},{"name":"WD Blue SN570 1TB","id":{},"temperature":{"name":"Temperature","value":36,"min":36,"max":36},"totalSpace":931,"freeSpace":280,"health":"100","throughputRead":0,"throughputWrite":0,"dataRead":18211,"dataWritten":4747}]},"motherboard":{"name":"ASUS PRIME H610M-K D4"},"monitor":{"monitors":[{"name":"G27F","resolution":"1920x1080","refreshRate":"144","primary":true},{"name":"V226HQL","resolution":"1920x1080","refreshRate":"60","primary":false}]},"network":{"interfaces":[{"name":"vEthernet (Default Switch)","description":"Hyper-V Virtual Ethernet Adapter","macAddress":"00155D7AD9AA","ipAddress":"172.23.0.1","mask":"255.255.240.0","gateway":"N/A","dns":"fec0:0:0:ffff::1%1","speed":"10000","uploadData":0,"downloadData":0,"throughputUpload":168,"throughputDownload":0},{"name":"Ethernet","description":"Realtek PCIe GBE Family Controller","macAddress":"581122AD3009","ipAddress":"192.168.1.2","mask":"255.255.255.0","gateway":"192.168.1.1","dns":"1.1.1.1","speed":"1000","uploadData":0.5,"downloadData":9.1,"throughputUpload":4246,"throughputDownload":5533}]},"bios":{"vendor":"American Megatrends Inc.","version":"2611","date":"2023. 08. 11."},"superIO":{"name":"Nuvoton NCT6798D","voltage":[{"name":"Vcore","value":0.66,"min":0.66,"max":1.06},{"name":"Voltage #2","value":1.01,"min":1.01,"max":1.01},{"name":"AVCC","value":3.39,"min":3.39,"max":3.39},{"name":"+3.3V","value":3.31,"min":3.31,"max":3.31},{"name":"Voltage #5","value":1,"min":1,"max":1},{"name":"Voltage #6","value":0.18,"min":0.18,"max":0.18},{"name":"Voltage #7","value":1.01,"min":1.01,"max":1.01},{"name":"3VSB","value":3.39,"min":3.39,"max":3.39},{"name":"VBat","value":3.14,"min":3.14,"max":3.14},{"name":"VTT","value":0.53,"min":0.53,"max":0.54},{"name":"Voltage #11","value":0.67,"min":0.67,"max":0.67},{"name":"Voltage #12","value":0.82,"min":0.82,"max":0.82},{"name":"Voltage #13","value":1.05,"min":1.05,"max":1.05},{"name":"Voltage #14","value":1.01,"min":1.01,"max":1.01},{"name":"Voltage #15","value":1.02,"min":1.02,"max":1.02}],"temperature":[{"name":"Temperature #1","value":33,"min":32,"max":33},{"name":"Temperature #2","value":25,"min":25,"max":25},{"name":"Temperature #3","value":26,"min":26,"max":26},{"name":"Temperature #4","value":8,"min":8,"max":8},{"name":"Temperature #5","value":-11,"min":-11,"max":-11},{"name":"Temperature #6","value":25,"min":25,"max":25}],"fan":[{"name":"Fan #1","value":0,"min":0,"max":0},{"name":"Fan #2","value":665,"min":665,"max":749},{"name":"Fan #3","value":0,"min":0,"max":0},{"name":"Fan #4","value":0,"min":0,"max":0},{"name":"Fan #5","value":0,"min":0,"max":0},{"name":"Fan #6","value":0,"min":0,"max":0},{"name":"Fan #7","value":0,"min":0,"max":0}],"fanControl":[{"name":"Fan #1","value":38,"min":38,"max":42},{"name":"Fan #2","value":32,"min":32,"max":38},{"name":"Fan #3","value":60,"min":60,"max":60},{"name":"Fan #4","value":60,"min":60,"max":60},{"name":"Fan #5","value":60,"min":60,"max":60},{"name":"Fan #6","value":100,"min":100,"max":100},{"name":"Fan #7","value":100,"min":100,"max":100}]}}}',
						)
					})
				}
			}
		}
	}
}
