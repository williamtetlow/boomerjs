/**
 * 1. Parse to SWC AST
 * 2. Analyse the server and client blocks for declarations
 * 3. Transform JSX to SSR code
 */

function example1_in() {
  server: {
    const hello = "hello";
  }

  client: {
  }

  <h1>{hello}</h1>;
}

function example1_out() {
  const hello = "hello";

  return ["<h1>", hello, "</h1>"];
}

function example2_in() {
  server: {
    const hello = "hello";
    const world = "world";
  }

  client: {
  }

  <div>
    <h1>{hello}</h1>
    <h2>{world}</h2>
  </div>;
}

function example2_out() {
  const hello = "hello";
  const world = "world";

  return ["<div><h1>", hello, "</h1><h2>", world, "</h2></div>"];
}

function example2_out_alt() {
  const hello = "hello";
  const world = "world";

  return [
    "<div>",
    ["<h1>", hello, "</h1>"],
    ["<h2>", world, "</h2>"],
    "</div>",
  ];
}

<div>
  {" "}
  root, children: [div, children: [div, children: [h1]], p]
  <div>
    <div>
      <h1>1</h1>
    </div>
    <p>1</p>
  </div>
</div>;

<div />;

<div>
  <h1>{hello}</h1>
  <h2>{world}</h2>
</div>;

let t1 = [
  "<div>",
  ["<h1>", ["hello"], "</h1>"],
  ["<h2>", ["world"], "</h2>"],
  "</div>",
];

<div>
  {" "}
  // root
  <h1> // stack 1{hello}</h1>
  <h2>{world}</h2>
  <div>
    <ul>
      <li>1</li>
    </ul>
  </div>
</div>;

let t = [
  "<div>",
  ["<h1>", hello, "</h1>"],
  ["<h2>", world, "</h2>"],
  ["<div>", ["<ul>", ["<li>1</li>"], "</ul>"], "</div>"],
  "</div>",
];
