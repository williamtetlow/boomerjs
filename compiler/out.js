const hello = "Hell yeah";
function boomer() {
    return "you boomer";
}
async function getJSON() {
    const response = await fetch("https://dummyjson.com/products/1");
    const json = await response.json();
    return JSON.stringify(json);
}
export const $$Component = [`<div>

  <h1>

    `,hello,` `,boomer(),`

  </h1>

  <ul>

    <li>Post Name</li>

  </ul>

  <pre>`,getJSON(),`</pre>

</div>`]
