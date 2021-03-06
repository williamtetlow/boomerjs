import { render } from "../../test_data/output.js";
import { test } from "../../test.js";

export default () => {
  console.log(test());
  const readableStream = new ReadableStream({
    start(controller) {
      const stream = (chunks) => {
        for (const chunk of chunks) {
          if (Array.isArray(chunk)) {
            stream(chunk);
          } else {
            const encoder = new TextEncoder("utf-8");
            controller.enqueue(encoder.encode(chunk));
          }
        }
      };

      stream(render());

      controller.close();
    },
    cancel() {
      console.log("closed");
    },
  });

  return new Response(readableStream);
};
