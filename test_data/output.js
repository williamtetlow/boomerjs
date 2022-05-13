const hello = "Hello";
const world = "World";
export function render() {
  return [
    "<div>",
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
    ],
    "</div>",
  ];
}
