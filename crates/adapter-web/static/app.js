// Progressive enhancement.
//
// The whole app works with plain HTML form submits and full-page reloads. This
// script is purely additive: it intercepts vote forms and submits them in the
// background, updating the tally in place. If JS is disabled, fails, or the
// request errors, the native form submit takes over and nothing is lost.
"use strict";

// Replay a one-shot animation: the .pulse class carries the keyframe, and it's
// removed on completion so the same element can pulse again on the next vote.
// This is what makes a cast vote visibly land; it never runs on first paint.
function pulse(el) {
  if (!el) return;
  el.classList.remove("pulse");
  void el.offsetWidth; // reflow, so re-adding the class restarts the animation
  el.classList.add("pulse");
  el.addEventListener("animationend", () => el.classList.remove("pulse"), { once: true });
}

// Post up/down votes: submit in the background, update the score and the
// active arrow in place. Falls back to a native submit (full reload) on error.
// Extracted so lazily-appended feed cards (see the feed loader below) can be
// wired up the same way as the ones present at first paint.
function bindPostVote(form) {
  form.addEventListener("submit", async (event) => {
    const dir = event.submitter && event.submitter.value;
    if (!dir) return; // let the browser handle it natively

    event.preventDefault();
    const body = new URLSearchParams();
    body.set("dir", dir);

    try {
      const res = await fetch(form.action, {
        method: "POST",
        headers: { "X-Requested-With": "fetch" },
        body,
      });
      if (!res.ok) throw new Error("vote failed");
      const data = await res.json();

      const box = form.closest("[data-post]");
      if (box) {
        const score = box.querySelector(".score");
        if (score) {
          score.textContent = data.score;
          pulse(score);
        }
        const up = box.querySelector("button.up");
        const down = box.querySelector("button.down");
        if (up) up.classList.toggle("active", data.vote === "up");
        if (down) down.classList.toggle("active", data.vote === "down");
        // Spring the arrow the vote landed on.
        if (data.vote === "up") pulse(up);
        else if (data.vote === "down") pulse(down);
      }
    } catch (_e) {
      form.submit(); // graceful fallback
    }
  });
}
document.querySelectorAll('form[data-enhance="postvote"]').forEach(bindPostVote);

// Feed pagination — progressive enhancement.
//
// Every feed renders a real "load more" link to `?page=N` (or, in the "paged"
// account preference, plain prev/next links). With no JS, clicking it loads the
// next page as its own document — nothing here is required. Enhanced, we turn
// that link into infinite scroll: when it nears the viewport we fetch the next
// page as a bare fragment and append its cards in place.
//
// Whether we lazy-load is driven by the link's data-mode, set server-side from
// the viewer's account preference:
//   "lazy" → always lazy-load; "auto" → lazy-load unless the browser asks for
//   reduced motion; "pages" → the link isn't emitted, so this never runs.
(function () {
  const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  function shouldLazyLoad(link) {
    const mode = link.dataset.mode;
    return mode === "lazy" || (mode === "auto" && !reduce);
  }

  function watch(link) {
    if (!shouldLazyLoad(link) || typeof IntersectionObserver !== "function") return;
    const io = new IntersectionObserver((entries) => {
      if (entries.some((e) => e.isIntersecting)) {
        io.disconnect();
        loadMore(link);
      }
    }, { rootMargin: "600px" });
    io.observe(link);
  }

  async function loadMore(link) {
    try {
      const res = await fetch(link.href, { headers: { "X-Requested-With": "fetch" } });
      if (!res.ok) throw new Error("feed page failed");
      const doc = new DOMParser().parseFromString(await res.text(), "text/html");
      const els = Array.from(doc.body.children);
      // The fragment is: the new cards, then (maybe) the next "load more" link.
      const nextLink = els.find((e) => e.matches && e.matches('[data-enhance="more"]'));
      const container = document.querySelector("[data-feed]") || link.parentNode;
      els
        .filter((e) => e !== nextLink && !(e.classList && e.classList.contains("pager")))
        .forEach((card) => {
          container.appendChild(card);
          card.querySelectorAll &&
            card.querySelectorAll('form[data-enhance="postvote"]').forEach(bindPostVote);
        });
      if (nextLink) {
        // Reuse the one link element, re-point it, and watch for the next page.
        link.href = nextLink.href;
        link.dataset.mode = nextLink.dataset.mode;
        watch(link);
      } else {
        link.remove(); // reached the end
      }
    } catch (_e) {
      // Leave the link in place — a manual click still loads the next page.
    }
  }

  document.querySelectorAll('[data-enhance="more"]').forEach(watch);
})();

// Reporting a post: submit in the background and swap the disclosure for an
// inline acknowledgement, so reporting never yanks the reader off to the
// moderation queue. Falls back to a native submit (full reload) on error.
document.querySelectorAll('form[data-enhance="report"]').forEach((form) => {
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    const body = new URLSearchParams(new FormData(form));

    try {
      const res = await fetch(form.action, {
        method: "POST",
        headers: { "X-Requested-With": "fetch" },
        body,
      });
      if (!res.ok) throw new Error("report failed");

      const done = document.createElement("p");
      done.className = "reported";
      done.textContent = form.dataset.done || "Reported.";
      (form.closest("details") || form).replaceWith(done);
    } catch (_e) {
      form.submit(); // graceful fallback
    }
  });
});

// In-page composer. The "+ Post" link normally loads /submit as its own page
// (works with no JS). Enhanced, clicking it fetches that page and shows the
// composer in a modal dialog over the current page, so composing never leaves
// the feed. Any failure falls back to a normal navigation to /submit.
(function () {
  const link = document.querySelector('a[data-enhance="composer"]');
  if (!link || typeof HTMLDialogElement !== "function") return;

  let dialog = null;
  let loaded = false;

  function build() {
    dialog = document.createElement("dialog");
    dialog.className = "composer-dialog";
    const panel = document.createElement("div");
    panel.className = "composer-panel";
    dialog.appendChild(panel);
    // A click on the backdrop (outside the panel) closes; Escape closes natively.
    dialog.addEventListener("click", (e) => {
      if (e.target === dialog) dialog.close();
    });
    document.body.appendChild(dialog);
    return panel;
  }

  async function load(panel) {
    const res = await fetch(link.href, { headers: { "X-Requested-With": "fetch" } });
    if (!res.ok) throw new Error("composer unavailable");
    const doc = new DOMParser().parseFromString(await res.text(), "text/html");
    const composer = doc.querySelector(".composer");
    if (!composer) throw new Error("no composer in response");

    // Lift the composer's own scoped styles in once (its <style> carries an id).
    const css = doc.getElementById("composer-styles");
    if (css && !document.getElementById("composer-styles")) {
      document.head.appendChild(css.cloneNode(true));
    }

    // The composer's enhancement is an external script (CSP forbids inline
    // scripts). Drop the fetched <script> tag and, once the form is in the DOM,
    // load /static/composer.js so it wires up the injected form.
    const script = doc.querySelector("main script");
    if (script) script.remove();

    const close = document.createElement("button");
    close.type = "button";
    close.className = "ghost composer-close";
    close.setAttribute("aria-label", "close");
    close.textContent = "✕";
    close.addEventListener("click", () => dialog.close());

    panel.appendChild(close);
    panel.appendChild(composer);
    const live = document.createElement("script");
    live.src = "/static/composer.js";
    panel.appendChild(live);
  }

  link.addEventListener("click", async (e) => {
    // Let modified/middle clicks open the full page in a new tab as usual.
    if (e.button !== 0 || e.metaKey || e.ctrlKey || e.shiftKey || e.altKey) return;
    e.preventDefault();
    try {
      const panel = dialog ? dialog.firstChild : build();
      if (!loaded) {
        await load(panel);
        loaded = true;
      }
      dialog.showModal();
    } catch (_e) {
      window.location = link.href; // graceful fallback to the full page
    }
  });
})();

// Found page: mirror the server's slug derivation into a live preview as the
// name is typed. Cosmetic only — the server derives the authoritative slug on
// submit — so a no-JS founder simply sees no preview.
document.querySelectorAll('form[data-enhance="slugify"]').forEach((form) => {
  const name = form.querySelector('input[name="name"]');
  const preview = form.querySelector(".slug-preview");
  const out = preview && preview.querySelector("strong");
  if (!name || !out) return;

  // Mirrors domain::slugify: keep ASCII alphanumerics, collapse every other run
  // to a single hyphen, trim the ends, cap the length.
  function slugify(s) {
    let slug = "";
    let pending = false;
    for (let i = 0; i < s.length && slug.length < 48; i++) {
      if (/[a-z0-9]/i.test(s[i])) {
        if (pending && slug) slug += "-";
        pending = false;
        slug += s[i].toLowerCase();
      } else {
        pending = true;
      }
    }
    return slug;
  }

  function update() {
    const slug = slugify(name.value);
    if (slug) {
      out.textContent = "d/" + slug;
      preview.hidden = false;
    } else {
      preview.hidden = true;
    }
  }
  name.addEventListener("input", update);
  update();
});

// Language switcher: submit the little <form> the moment a new language is
// picked. Replaces an inline `onchange` handler so the page needs no inline
// scripts (CSP `script-src 'self'`). No-JS users still get the <noscript> OK
// button the template renders.
document.querySelectorAll("select[data-autosubmit]").forEach((sel) => {
  sel.addEventListener("change", () => {
    if (sel.form) sel.form.submit();
  });
});

// Founding petition page: upgrade the shareable path to a full absolute URL when
// JS is present (the bare same-site path is the no-JS fallback), and select-all
// on focus for one-tap copy. Replaces an inline <script> and an inline `onfocus`
// handler so no inline JS is needed.
(function () {
  const el = document.getElementById("share-url");
  if (!el) return;
  if (el.value && el.value.charAt(0) === "/") {
    el.value = location.origin + el.value;
  }
  el.addEventListener("focus", () => el.select());
})();

document.querySelectorAll('form[data-enhance="vote"]').forEach((form) => {
  form.addEventListener("submit", async (event) => {
    // We need to know which button (aye/nay) was pressed.
    const choice = event.submitter && event.submitter.value;
    if (!choice) return; // let the browser handle it natively

    event.preventDefault();
    const body = new URLSearchParams();
    body.set("choice", choice);

    try {
      const res = await fetch(form.action, {
        method: "POST",
        headers: { "X-Requested-With": "fetch" },
        body,
      });
      if (!res.ok) throw new Error("vote failed");
      const tally = await res.json();

      const box = form.closest("[data-proposal]");
      if (box) {
        const aye = box.querySelector(".aye");
        const nay = box.querySelector(".nay");
        if (aye) { aye.textContent = tally.aye; pulse(aye); }
        if (nay) { nay.textContent = tally.nay; pulse(nay); }
      }
      // One ballot per voter: disable the buttons once cast.
      form.querySelectorAll("button").forEach((b) => (b.disabled = true));
    } catch (_e) {
      // Graceful fallback: real submit, full reload, server handles everything.
      form.submit();
    }
  });
});
