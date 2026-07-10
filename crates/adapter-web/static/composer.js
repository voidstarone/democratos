// Post composer enhancement — externalized from submit.html so the page can run
// under a strict `script-src 'self'` CSP (no inline scripts).
//
// Without JS the composer is a plain multipart form with a single multi-file
// input. Enhanced, each chosen or dragged file becomes a removable row with its
// own caption, and on submit we rebuild the multipart parts in order (one media
// part immediately followed by its caption). Any failure falls back to a native
// submit. The two translated strings it needs arrive as `data-*` attributes on
// the form, keeping this file static and language-agnostic.
"use strict";

(function () {
  var form = document.querySelector(".submit-form");
  if (!form) return;
  var dropLabel = form.querySelector(".dropzone");
  var pickInput = form.querySelector(".dropzone input[type=file]");
  var list = form.querySelector(".media-list");

  // Translated UI strings, passed from the server via data attributes.
  var CAPTION_PLACEHOLDER = form.getAttribute("data-caption-placeholder") || "";
  var BAD_URL_MSG = form.getAttribute("data-bad-url") || "";

  // Supported media by URL extension — mirrors the server's allowlist so a
  // dragged link is only accepted if the backend will accept it too.
  var IMG_EXT = /\.(png|jpe?g|gif|webp)(\?|#|$)/i;
  var VID_EXT = /\.(mp4|webm)(\?|#|$)/i;

  // JS enhancement: manage our own ordered set of media so each gets a caption
  // and a remove button. Each item is either an uploaded { file } or a dragged
  // { url, isVideo }. On submit we rebuild the multipart parts in order, one
  // media part (file OR media_url) immediately followed by its caption.
  var items = [];

  function itemIsVideo(it) {
    return it.file ? it.file.type.indexOf("video") === 0 : it.isVideo;
  }
  function itemSrc(it) { return it.file ? URL.createObjectURL(it.file) : it.url; }
  function itemName(it) { return it.file ? it.file.name : it.url; }

  function render() {
    list.innerHTML = "";
    items.forEach(function (it, i) {
      var row = document.createElement("div");
      row.className = "media-item";

      var prev = document.createElement("div");
      prev.className = "preview";
      var media = document.createElement(itemIsVideo(it) ? "video" : "img");
      if (media.tagName === "VIDEO") { media.muted = true; }
      media.src = itemSrc(it);
      prev.appendChild(media);

      var grow = document.createElement("div");
      grow.className = "grow";
      var name = document.createElement("div");
      name.className = "name";
      name.textContent = itemName(it);
      var cap = document.createElement("input");
      cap.type = "text";
      cap.placeholder = CAPTION_PLACEHOLDER;
      cap.value = it.caption;
      cap.addEventListener("input", function () { items[i].caption = cap.value; });
      grow.appendChild(name);
      grow.appendChild(cap);

      var rm = document.createElement("button");
      rm.type = "button";
      rm.className = "ghost remove";
      rm.textContent = "✕";
      rm.addEventListener("click", function () { items.splice(i, 1); render(); });

      row.appendChild(prev);
      row.appendChild(grow);
      row.appendChild(rm);
      list.appendChild(row);
    });
  }

  function addFiles(fileList) {
    Array.prototype.forEach.call(fileList, function (f) { items.push({ file: f, caption: "" }); });
    render();
  }

  // Pull the first usable URL out of a drop's data transfer. Dragging a link
  // gives text/uri-list or text/plain; dragging an <img> straight off another
  // page gives text/html we can sniff a src out of.
  function urlFromDrop(dt) {
    var raw = (dt.getData("text/uri-list") || dt.getData("text/plain") || "").trim();
    var url = raw.split("\n").map(function (s) { return s.trim(); })
      .filter(function (s) { return s && s.charAt(0) !== "#"; })[0] || "";
    if (!url) {
      var html = dt.getData("text/html");
      var m = html && html.match(/src\s*=\s*["']([^"']+)["']/i);
      if (m) { url = m[1]; }
    }
    return url;
  }

  // Classify a dragged URL, or return null if it isn't a supported image/video
  // over http(s).
  function classifyUrl(url) {
    if (!/^https?:\/\//i.test(url)) { return null; }
    if (VID_EXT.test(url)) { return { url: url, isVideo: true, caption: "" }; }
    if (IMG_EXT.test(url)) { return { url: url, isVideo: false, caption: "" }; }
    return null;
  }

  function flashReject() {
    dropLabel.classList.add("reject");
    var prev = dropLabel.getAttribute("title");
    dropLabel.setAttribute("title", BAD_URL_MSG);
    setTimeout(function () {
      dropLabel.classList.remove("reject");
      if (prev) { dropLabel.setAttribute("title", prev); } else { dropLabel.removeAttribute("title"); }
    }, 600);
  }

  // Take over the native input so it only feeds our list (and never submits
  // its own unlabelled parts).
  pickInput.removeAttribute("name");
  pickInput.addEventListener("change", function () {
    addFiles(pickInput.files);
    pickInput.value = "";
  });

  // Whether a drag carries something we can accept (files or, potentially, a
  // link) — used to invite only for relevant drags.
  function dragHasMedia(dt) {
    if (!dt) { return false; }
    var types = dt.types || [];
    return [].indexOf.call(types, "Files") !== -1
        || [].indexOf.call(types, "text/uri-list") !== -1
        || [].indexOf.call(types, "text/plain") !== -1
        || [].indexOf.call(types, "text/html") !== -1;
  }

  // Page-wide "inviting" state: the moment a drag enters the window, wake the
  // dropzone up so the user sees exactly where to aim. A dragenter/dragleave
  // depth counter avoids flicker as the pointer crosses child elements.
  var dragDepth = 0;
  window.addEventListener("dragenter", function (e) {
    if (!dragHasMedia(e.dataTransfer)) { return; }
    dragDepth++;
    dropLabel.classList.add("inviting");
  });
  window.addEventListener("dragleave", function (e) {
    // relatedTarget == null means the pointer left the window entirely.
    if (e.relatedTarget === null) { dragDepth = 0; }
    else { dragDepth = Math.max(0, dragDepth - 1); }
    if (dragDepth === 0) { dropLabel.classList.remove("inviting"); }
  });
  function endPageDrag() { dragDepth = 0; dropLabel.classList.remove("inviting"); }
  window.addEventListener("drop", endPageDrag);
  window.addEventListener("dragend", endPageDrag);

  // Over the zone itself: show the "drop it here" primed state and a copy cursor.
  ["dragenter", "dragover"].forEach(function (ev) {
    dropLabel.addEventListener(ev, function (e) {
      e.preventDefault();
      if (e.dataTransfer) { e.dataTransfer.dropEffect = "copy"; }
      dropLabel.classList.add("drag");
    });
  });
  dropLabel.addEventListener("dragleave", function (e) {
    e.preventDefault();
    dropLabel.classList.remove("drag");
  });
  dropLabel.addEventListener("drop", function (e) {
    e.preventDefault();
    dropLabel.classList.remove("drag");
    endPageDrag();
    var dt = e.dataTransfer;
    if (!dt) { return; }
    if (dt.files && dt.files.length) { addFiles(dt.files); return; }
    // No files: try to accept a dragged media link.
    var url = urlFromDrop(dt);
    if (!url) { return; }
    var media = classifyUrl(url);
    if (media) { items.push(media); render(); }
    else { flashReject(); }
  });

  // On submit, serialise our ordered items as paired media/caption parts:
  // uploaded files go as `file`, dragged links as `media_url`.
  form.addEventListener("submit", function (e) {
    e.preventDefault();
    var data = new FormData(form);
    data.delete("file");
    data.delete("media_url");
    data.delete("caption");
    items.forEach(function (it) {
      if (it.file) { data.append("file", it.file); }
      else { data.append("media_url", it.url); }
      data.append("caption", it.caption);
    });
    fetch(form.action, { method: "POST", body: data, credentials: "same-origin" })
      .then(function (r) { window.location = r.redirected ? r.url : "/"; })
      .catch(function () { form.submit(); });
  });
})();
