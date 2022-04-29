export function input_server() {
  async function posts(ascending) {
    return fetch(`/posts?ascending=${ascending}`);
  }
}

export function input_client() {
  const [sort, setSort] = useState(false);

  function toggleSort() {
    setSort(!sort());
  }

  function posts_proxy(ascending) {}
}

export function render() {
  return `<div><ul><boundary-component></boundary-component><button>Toggle Ordering</button>`;
}
