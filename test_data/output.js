const hello = "Hello";
const world = "World";

const state = {
  count: 1,
};

function set_count(count) {
  state.count = count;
  update_button();
}

function get_count() {
  return state.count;
}

function update_button() {
  let btn = document.querySelectorAll('[data-bmr="id1"]')[0];

  let template = "Count: ±";

  btn.innerHTML = template.slice(0, 7) + state.count;
}

export function render() {
  return [
    "<div>",
    [
      "<button>",
      [
        "<h2>",
        [
          "<h1>",
          [
            `

  `,
            hello,
          ],
          "</h1>",
          `

  `,
          world,
        ],
        "</h2>",
        `

  `,
        count(),
      ],
      "</button>",
      `

`,
    ],
    "</div>",
  ];
}
