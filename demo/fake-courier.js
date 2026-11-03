// Prints made-up delivery logs so the Logs tab has something to show.
const owls = ["hedwig", "errol", "hermes", "pigwidgeon"];
const letters = ["acceptance-letter", "howler", "newsletter", "receipt", "reminder"];

const log = (msg) => console.log(`${new Date().toISOString()} ${msg}`);

log("info  courier connected to rabbitmq:5672");
log("info  consuming queue 'outgoing-post'");

setInterval(() => {
  const owl = owls[Math.floor(Math.random() * owls.length)];
  const letter = letters[Math.floor(Math.random() * letters.length)];
  const ms = 200 + Math.floor(Math.random() * 1800);
  log(`info  delivered ${letter.padEnd(18)} via ${owl.padEnd(10)} ${ms}ms`);
}, 2500);
