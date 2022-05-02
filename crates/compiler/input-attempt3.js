import { useState } from "@boomerjs/reactive";

server: {
  async function posts(ascending) {
    return fetch(`/posts?ascending=${ascending}`);
  }
}

// if this doesn't exist we know it's static
client: {
  const [sort, setSort] = useState(false);

  function toggleSort() {
    setSort(!sort());
  }
}

<div>
  <ul>
    <for each={await posts(sort())}>{(post) => <li>{post.name}</li>}</for>
  </ul>
  <button onClick={toggleSort}>Toggle Ordering</button>
</div>;
