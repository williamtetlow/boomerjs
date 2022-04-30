import { $$Component } from "../../compiler/out.js";

export default () => {
  const readableStream = new ReadableStream({
    async start(controller) {
      for (const chunk of $$Component) {
        const encoder = new TextEncoder("utf-8");

        console.log(typeof chunk);
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
