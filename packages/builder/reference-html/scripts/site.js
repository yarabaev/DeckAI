(() => {
  const input = document.getElementById("ref-q");
  if (!input) {
    // No filter — still wire theme toggle if present (not on this page)
    return;
  }
  const cards = Array.from(document.querySelectorAll(".ref-card"));
  const counter = document.querySelector("[data-count]");
  const empty = document.querySelector(".ref-empty");
  const emptyQ = document.querySelector("[data-empty-q]");
  function apply() {
    const q = input.value.trim().toLowerCase();
    let shown = 0;
    for (const c of cards) {
      const m = !q || c.dataset.haystack.includes(q);
      c.hidden = !m;
      if (m) shown++;
    }
    if (counter) counter.textContent = shown + " of " + cards.length + " elements";
    if (empty) empty.hidden = shown > 0 || q === "";
    if (emptyQ && q && shown === 0) emptyQ.textContent = q;
    const url = new URL(location.href);
    if (q) url.searchParams.set("q", q); else url.searchParams.delete("q");
    history.replaceState(null, "", url);
  }
  const initial = new URL(location.href).searchParams.get("q");
  if (initial) { input.value = initial; apply(); }
  input.addEventListener("input", apply);
  document.addEventListener("keydown", (e) => {
    const t = e.target;
    const editable = t && t.matches && t.matches('input, textarea, select, [contenteditable=""], [contenteditable="true"], [role="textbox"]');
    if (e.key === "/" && document.activeElement !== input && !editable) {
      e.preventDefault();
      input.focus();
    }
  });
})();
