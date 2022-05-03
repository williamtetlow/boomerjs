const $a_class = ".bmr-p",
  $a_tmpl = "Current Page §";
$b_class = ".bmr-btn";

let page = 1;

function set_page(p) {
  page = p;
  set_$a_1(document.getElementsByClassName($a_class)[0], page);
}

function set_$a_1(el, page) {
  el.innerHTML = $a_tmpl.replace(/§/, page);
}

function c_b(e) {
  set_page(page + 1);
}

document.addEventListener("click", (e) => {
  if (e.target.dataset.click) {
    this[e.target.dataset.click](e);
  }
});
