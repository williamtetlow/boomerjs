/**
 * This is my goal for v0.1
 *
 * - Everything is treated as HTML by default
 * - Server and client is separated into different block scopes
 * - We detect client side reactivity and compile it (count and setCount have a concrete reactive implementation)
 *
 * I'm not sure if this will work but lets find out :D
 */

server: {
  const hello = "Hello";
  const world = "World";
}

client: {
  const [count, setCount] = useState(0);
}

<div>
  <h1>{hello}</h1>
  <h2>{world}</h2>
  <button onClick={() => setCount(count() + 1)}>{count()}</button>
</div>;
