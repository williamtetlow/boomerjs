async function get() {
  const response = await fetch("https://dummyjson.com/products/1");
  const json = await response.json();
  return JSON.stringify(json);
}
export const $$Component = [
  `<div >`,
  get().then((bla) => [`<pre>`, bla, `</pre>`]),
  `</div>`,
];

/**
 * ctx.in_await
 *
 * if task === server_func {
 *   // 1 check that the child is one expression and it's a function
 *   // 2 transform to Promise.resolve(get()).then((bla) => ...)
 * }
 */
<div>
  <Await task={get()}>{(bla) => <pre>{bla}</pre>}</Await>
</div>;
