// Developer account switcher — paints a floating dock of two stacked bars: the
// top bar lists every test account and switches the browser's session to any of
// them with one click; the bottom bar holds the dev controls (spin up a new
// account). Splitting switching from controls keeps the create form from
// jostling the account buttons as they wrap. It is entirely self-gating: it asks
// the server for /dev/accounts and, if the server was not started with `--dev`
// (404), it draws nothing. So shipping this file in production is harmless.
(function () {
  "use strict";

  async function post(url, body) {
    return fetch(url, {
      method: "POST",
      headers: { "Content-Type": "application/x-www-form-urlencoded" },
      body: body,
    });
  }

  function styles() {
    var css = [
      ".devdock{position:fixed;left:0;right:0;bottom:0;z-index:9999;display:flex;",
      "flex-direction:column}",
      ".devbar{display:flex;gap:.4rem;align-items:center;flex-wrap:wrap;padding:.4rem .6rem;",
      "background:#15211c;color:#fff;font:13px/1.3 system-ui,sans-serif;",
      "border-top:2px solid var(--accent,#0a7d5a)}",
      // The controls bar sits below the switcher; a thin divider separates the
      // two without a second heavy accent rule.
      ".devbar.devbar-controls{border-top:1px solid #3a4a43}",
      ".devbar .tag{font-weight:700;letter-spacing:.05em;color:var(--accent,#0a7d5a);",
      "text-transform:uppercase;font-size:11px}",
      ".devbar button{font:inherit;cursor:pointer;border-radius:999px;padding:.15rem .6rem;",
      "border:1px solid var(--accent,#0a7d5a);background:transparent;color:#fff}",
      ".devbar button.current{background:var(--accent,#0a7d5a);font-weight:600;cursor:default}",
      ".devbar form{display:flex;gap:.3rem;margin-left:auto}",
      ".devbar input{font:inherit;padding:.15rem .5rem;border-radius:6px;border:1px solid #3a4a43;",
      "background:#0d1714;color:#fff}",
      ".devbar input::placeholder{color:#8a9a93}",
    ].join("");
    var el = document.createElement("style");
    el.textContent = css;
    document.head.appendChild(el);
  }

  // A single dev bar with its leading uppercase tag label.
  function bar(label) {
    var el = document.createElement("div");
    el.className = "devbar";
    var tag = document.createElement("span");
    tag.className = "tag";
    tag.textContent = label;
    el.appendChild(tag);
    return el;
  }

  function render(data) {
    var dock = document.createElement("div");
    dock.className = "devdock";

    // Top bar: the account switcher — who we are acting as.
    var pov = bar("dev · acting as");
    if (!data.users.length) {
      var none = document.createElement("span");
      none.textContent = "no accounts yet";
      pov.appendChild(none);
    }

    data.users.forEach(function (u) {
      var b = document.createElement("button");
      b.textContent = u.handle;
      if (u.id === data.current) {
        b.className = "current";
        b.title = "current point of view";
      } else {
        b.addEventListener("click", async function () {
          b.disabled = true;
          var r = await post("/dev/switch", "id=" + encodeURIComponent(u.id));
          if (r.ok) location.reload();
          else b.disabled = false;
        });
      }
      pov.appendChild(b);
    });

    // Bottom bar: the dev controls — stacked beneath the switcher so the create
    // form never reflows into the account buttons.
    var controls = bar("dev · controls");
    controls.classList.add("devbar-controls");

    var form = document.createElement("form");
    var input = document.createElement("input");
    input.placeholder = "new test handle…";
    input.setAttribute("aria-label", "new test account handle");
    var add = document.createElement("button");
    add.type = "submit";
    add.textContent = "+ create";
    form.appendChild(input);
    form.appendChild(add);
    form.addEventListener("submit", async function (e) {
      e.preventDefault();
      var handle = input.value.trim();
      if (!handle) return;
      add.disabled = true;
      var r = await post("/dev/create", "handle=" + encodeURIComponent(handle));
      if (r.ok) location.reload();
      else add.disabled = false;
    });
    controls.appendChild(form);

    dock.appendChild(pov);
    dock.appendChild(controls);
    document.body.appendChild(dock);
  }

  async function init() {
    var res;
    try {
      res = await fetch("/dev/accounts", { headers: { "X-Requested-With": "fetch" } });
    } catch (_) {
      return;
    }
    if (!res.ok) return; // not in dev mode — stay invisible
    var data = await res.json();
    styles();
    render(data);
  }

  init();
})();
