import { createServer } from "node:http";
import { httpClient as neunode, required } from "./client.js";

const model = required("MODEL_ID");
const upstream = required("UPSTREAM_URL").replace(/\/$/, "");
const publicUrl = required("PUBLIC_URL").replace(/\/$/, "");
const port = Number(process.env.PORT ?? "8080");
if (!Number.isInteger(port) || port < 1 || port > 65_535)
	throw new Error("Invalid PORT");

const server = createServer(async (request, response) => {
	try {
		if (request.method !== "POST" || request.url !== "/v1/chat/completions") {
			response.writeHead(404).end();
			return;
		}
		const body = await Array.fromAsync(request).then((chunks) =>
			Buffer.concat(chunks),
		);
		const upstreamResponse = await fetch(`${upstream}/v1/chat/completions`, {
			method: "POST",
			headers: {
				"content-type": "application/json",
				...(process.env.UPSTREAM_API_KEY
					? { authorization: `Bearer ${process.env.UPSTREAM_API_KEY}` }
					: {}),
			},
			body,
		});
		response.writeHead(
			upstreamResponse.status,
			Object.fromEntries(upstreamResponse.headers),
		);
		response.end(Buffer.from(await upstreamResponse.arrayBuffer()));
	} catch (error) {
		response.writeHead(502, { "content-type": "application/json" });
		response.end(JSON.stringify({ error: String(error) }));
	}
});

server.listen(port, async () => {
	await neunode.inference.registerProvider({
		name: process.env.PROVIDER_NAME ?? "Neunode Provider",
		endpoint: publicUrl,
		models: [model],
	});
	console.log(`Serving ${model} at ${publicUrl}/v1/chat/completions`);
});
