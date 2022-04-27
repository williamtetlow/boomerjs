const queuingStrategy = new CountQueuingStrategy({ highWaterMark: 1 });
let result = "";
const writableStream = new WritableStream(
  {
    write(chunk) {
      return new Promise((resolve, reject) => {
        result += chunk;
        resolve();
      });
    },
    abort(err) {
      console.log("Sink error:", err);
    },
  },
  queuingStrategy
);

const page = [
  "<html><head><title>Hello</title></head><body><h1>Hello",
  () => " world",
  "</h1></body></html>",
];

const writer = writableStream.getWriter();

for (const chunk of page) {
  let toWrite = "";

  if (typeof chunk === "string") {
    toWrite = chunk;
  }

  if (typeof chunk === "function") {
    toWrite = chunk();
  }

  await writer.ready
    .then(() => {
      return writer.write(toWrite);
    })
    .then(() => {
      console.log("Chunk written to sink.");
    })
    .catch((err) => {
      console.log("Chunk error:", err);
    });
}

await writer.ready
  .then(() => {
    writer.close();
  })
  .then(() => {
    console.log("All chunks written");
  })
  .catch((err) => {
    console.log("Stream error:", err);
  });

console.log(result);
