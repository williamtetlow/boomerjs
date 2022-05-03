async function server_get(page) {
  const response = await fetch(`https://dummyjson.com/products/${page}`);
  const json = await response.json();
  return JSON.stringify(json);
}

export const client_page = 1;

export const client_bmr_events = "click";

export const client_component = () => {
  const [page, setPage] = useState(1);

  function incrementPage() {
    setPage((page) => page++);
  }
};

export const $$Component = () => [
  `<div >`,
  server_get(client_page).then((bla) => [`<pre>`, bla, `</pre>`]),
  `<button data-bmr-click="x123">Next Page</button>
  </div>`,
];
