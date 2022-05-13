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
