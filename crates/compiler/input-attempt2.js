server: {
  async function posts(ascending) {
    return fetch(`/posts?ascending=${ascending}`);
  }
}

<div>
  <ul>
    <for each={await posts()}>{(post) => <li>{post.name}</li>}</for>
  </ul>
</div>;
