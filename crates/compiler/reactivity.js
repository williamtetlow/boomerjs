server: {
  async function get() {
    const response = await fetch("https://dummyjson.com/products/1");

    const json = await response.json();

    return JSON.stringify(json);
  }
}

client: {
  const [page, setPage] = useState(1);

  function incrementPage() {
    setPage(page() + 1);
  }
}

<div>
  <p>Current Page {page()}</p>
  <pre>{await get()}</pre>
  <button onClick={incrementPage}>Next Page</button>
</div>;
