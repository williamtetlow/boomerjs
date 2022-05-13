async function get() {
    const response = await fetch("https://dummyjson.com/products/1");
    const json = await response.json();
    return JSON.stringify(json);
}
export const $$Component = [`<div >

  <Await task=`,get(),`>`,(bla)=><pre >{bla}</pre>
,`</Await>

</div>`]