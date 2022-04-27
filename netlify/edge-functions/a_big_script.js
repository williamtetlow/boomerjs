export default async () => {
  await new Promise((res) =>
    setTimeout(() => {
      res();
    }, 1000)
  );

  return new Response('(function(){ console.log("hello"); })()');
};
