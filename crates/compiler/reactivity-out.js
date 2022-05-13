/**
 * Server Bundle
 */

async function get() {
  const response = await fetch("https://dummyjson.com/products/1");

  const json = await response.json();

  return JSON.stringify(json);
}

export const $$Component = () => [
  `<div>`,
  `<p bmr-i="x1">Current Page `,
  get(),
  `</pre>
  <button bmr-i="x2">Next Page</button>`,
];

/**
 * Client Bundle
 */
const pTemplate = `Current Page ±`;

const scope = {
  page: 1,
};

function set_el_1(scope) {
  const el = document.querySelectorAll('[bmr-i="x1"]')[0];
  el.innerHTML = pTemplate.slice(0, 13) + scope.page;
}

function page() {
  return scope.page;
}

function setPage(page) {
  if (scope.page === page) return;

  scope.page = page;
  set_el_1(scope);
}

function incrementPage() {
  setPage(page() + 1);
}

document
  .querySelectorAll('[bmr-i="x2"]')
  .addEventListener("click", incrementPage);
