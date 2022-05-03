server: {
  async function get(page) {
    const response = await fetch(`https://dummyjson.com/products/${page}`);

    const json = await response.json();

    return JSON.stringify(json);
  }
}

client: {
  const [page, setPage] = useState(1);

  function incrementPage() {
    setPage((page) => page++);
  }
}

<div>
  <p>Current Page {page()}</p>
  <Await task={get()}>{(bla) => <pre>{bla}</pre>}</Await>
  <button onClick={incrementPage}>Next Page</button>
</div>;
