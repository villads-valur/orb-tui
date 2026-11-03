// Prints made-up request logs so the Logs tab has something to show.
const http = require("http");

const routes = [
  ["GET", "/api/vaults/713", 200],
  ["GET", "/api/vaults/687", 200],
  ["GET", "/api/exchange/galleons", 200],
  ["POST", "/api/transfers", 201],
  ["GET", "/api/accounts/me", 200],
  ["POST", "/api/withdrawals", 200],
  ["GET", "/api/vaults/999", 404],
  ["DELETE", "/api/sessions/current", 204],
  ["GET", "/healthz", 200],
];

const log = (msg) => console.log(`${new Date().toISOString()} ${msg}`);

http
  .createServer((req, res) => {
    res.end("ok");
  })
  .listen(3000, () => {
    log("info  connected to postgres at postgres:5432/gringotts");
    log("info  connected to redis at redis:6379");
    log("info  vault storage ready at vault:80");
    log("info  gringotts api listening on :3000");
  });

setInterval(() => {
  const [method, path, status] = routes[Math.floor(Math.random() * routes.length)];
  const ms = 3 + Math.floor(Math.random() * 120);
  const level = status >= 400 ? "warn " : "info ";
  log(`${level} ${method.padEnd(6)} ${path.padEnd(24)} ${status} ${ms}ms`);
}, 1500);
