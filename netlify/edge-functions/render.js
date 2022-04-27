const page = [
  "<html><head><script type='text/javascript' src='/a_big_script.js'></script><title>Hello</title></head><body><h1>Hello",
  () => "world",
  "</h1>",
  async () => {
    await new Promise((res) =>
      setTimeout(() => {
        res();
      }, 3000)
    );

    return "<h2>subtitle</h2>";
  },
  "</body></html>",
];

export default () => {
  const readableStream = new ReadableStream({
    async start(controller) {
      for (const chunk of page) {
        const encoder = new TextEncoder("utf-8");

        if (typeof chunk === "string") {
          controller.enqueue(encoder.encode(chunk));
        }

        if (typeof chunk === "function") {
          controller.enqueue(encoder.encode(await chunk()));
        }
      }
      controller.close();
    },
    cancel() {
      console.log("closed");
    },
  });

  return new Response(readableStream);
};
