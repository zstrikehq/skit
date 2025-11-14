const http = require("http");

const PORT = process.env.PORT || 3000;
const API_KEY = process.env.API_KEY || "not set";
const API_URL = process.env.API_URL || "not set";

const server = http.createServer((req, res) => {
    if (req.url === "/") {
        res.writeHead(200, { "Content-Type": "text/html" });
        res.end(`
<!DOCTYPE html>
<html>
<head>
  <title>SKIT Demo Server</title>
  <style>
    body { font-family: Arial, sans-serif; margin: 50px; }
    .env-var { background: #f0f0f0; padding: 10px; margin: 10px 0; border-left: 4px solid #007acc; }
    .secret { color: #d63384; }
    .config { color: #198754; }
  </style>
</head>
<body>
  <h1>SKIT Environment Demo</h1>
  <p>This server displays environment variables injected by SKIT.</p>

  <h2>Environment Variables:</h2>

  <div class="env-var">
    <strong>API_KEY</strong> (secret):
    <span class="secret">${API_KEY}</span>
  </div>

  <div class="env-var">
    <strong>API_URL</strong> (config):
    <span class="config">${API_URL}</span>
  </div>

  <h3>Usage Example:</h3>
  <pre>
# Store secrets in SKIT safe
skit init
skit set API_KEY "sk-1234567890abcdef"
skit set API_URL "https://api.example.com" --plain

# Run this server with injected environment variables
skit exec -- node server.js
  </pre>

  <p><em>Server running on port ${PORT}</em></p>
</body>
</html>
    `);
    } else {
        res.writeHead(404, { "Content-Type": "text/plain" });
        res.end("Not Found");
    }
});

server.listen(PORT, () => {
    console.log(`🚀 Demo server running on http://localhost:${PORT}`);
    console.log(`📋 Environment variables:`);
    console.log(
        `   API_KEY: ${API_KEY ? "***" + API_KEY.slice(-4) : "not set"}`,
    );
    console.log(`   API_URL: ${API_URL}`);
    console.log(`\n🔐 To inject secrets: skit exec -- node examples/server.js`);
});
