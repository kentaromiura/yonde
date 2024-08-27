const { invoke } = window.__TAURI__.tauri;

async function getContent() {
  return await invoke("get_content", {});
}

const _unlisten = window.__TAURI__.event.listen("lookup", () => {
  if (document.querySelector("[data-id=lookup]")) {
    return;
  }
  let div = document.createElement("div");
  div.dataset.id = "lookup";
  div.innerHTML = `<input type="text"></input><button>Search</button><button>Close</button>`;
  document.documentElement.appendChild(div);
  div.style.position = "absolute";
  div.style.top = 0;
  div.style.left = 0;
  div.style.zIndex = 1;
  div.style.backgroundColor = "white";
  div.style.width = "100vw";
  div.style.borderBottom = "1px solid silver";
  div.style.boxShadow = "0 10px 10px lightsteelblue";
  div.style.padding = "1em";
  document.body.appendChild(div);
  const buttons = div.querySelectorAll("button");
  div.firstChild.onkeydown = (e) => {
    console.log(e);
    if (e.key === "Enter") {
      window.lookup(div.firstChild.value);
      document.body.removeChild(div);
    }
  };

  buttons[0].onclick = () => {
    window.lookup(div.firstChild.value);
    document.body.removeChild(div);
  };
  buttons[1].onclick = () => {
    document.body.removeChild(div);
  };
});

async function queryById(id) {
  return await invoke("query_by_id", { id });
}

async function query(word) {
  return await invoke("definition", {
    word,
  });
}

async function playAudio(audioId) {
  return await invoke("play_audio", {
    word: audioId,
  });
}

// based on https://stackoverflow.com/q/49758168
// as tauri osx webview don't support range.expand("word")
function expandRangeWord(node, range) {
  if (range.toString().trim() == "") return range;
  let newRange = range;
  // Find and include start of sentence containing clicked region:
  while (newRange.startOffset !== 0) {
    // start of node
    newRange.setStart(node, newRange.startOffset - 1); // back up 1 char
    if (newRange.toString().search(/^[.!?:\n ]\s*/) === 0) {
      // start of sentence
      newRange.setStart(node, newRange.startOffset + 1); // move forward char
      break;
    }
  }
  let done = false;
  while (!done) {
    // start of node
    try {
      newRange.setEnd(node, newRange.endOffset + 1); // more 1 char
    } catch {
      done = true;
    }
    let test = newRange.toString();
    if (/[.�!?:\n ]\s*$/.test(test)) {
      // end of sentence
      newRange.setEnd(node, newRange.endOffset - 1); // move back char
      done = true;
    }
  }
  return newRange;
}

// based on https://stackoverflow.com/a/3710561
function getWordAtPoint(elem, x, y, notExpand) {
  let isKanji = elem.classList && elem.classList.contains("kanji");
  if (isKanji && elem.innerText.length == 1) return elem.innerText;
  if (isKanji && notExpand) {
    var range = elem.ownerDocument.createRange();

    range.selectNodeContents(elem.childNodes[0]);

    for (var i = 0, max = elem.childNodes[0].length; i < max; i++) {
      range.setStart(elem.childNodes[0], i);
      range.setEnd(elem.childNodes[0], i + 1);

      if (
        range.getBoundingClientRect().left <= x &&
        range.getBoundingClientRect().right >= x &&
        range.getBoundingClientRect().top <= y &&
        range.getBoundingClientRect().bottom >= y
      ) {
        return range.toString();
      }
    }
    return range.toString();
  }
  if (elem.nodeType == elem.TEXT_NODE) {
    var range = elem.ownerDocument.createRange();
    range.selectNodeContents(elem);

    var currentPos = 0;
    var endPos = range.endOffset;
    while (currentPos + 1 < endPos) {
      range.setStart(elem, currentPos);
      range.setEnd(elem, currentPos + 1);
      if (
        range.getBoundingClientRect().left <= x &&
        range.getBoundingClientRect().right >= x &&
        range.getBoundingClientRect().top <= y &&
        range.getBoundingClientRect().bottom >= y
      ) {
        //range.expand("word");
        if (!notExpand) range = expandRangeWord(elem, range);

        var ret = range.toString();
        range.detach();
        return ret;
      }
      currentPos += 1;
    }
  } else {
    for (var i = 0; i < elem.childNodes.length; i++) {
      var range = elem.childNodes[i].ownerDocument.createRange();
      range.selectNodeContents(elem.childNodes[i]);
      if (
        range.getBoundingClientRect().left <= x &&
        range.getBoundingClientRect().right >= x &&
        range.getBoundingClientRect().top <= y &&
        range.getBoundingClientRect().bottom >= y
      ) {
        range.detach();
        return getWordAtPoint(elem.childNodes[i], x, y);
      } else {
        range.detach();
      }
    }
  }
  return null;
}

class MiniYT extends HTMLElement {
  _animateZoom = () => {
    this._el.classList.add("zoomed");
  };
  _parentALink = (el) => {
    let next = el.parentElement;
    while (next) {
      console.log(next);
      if (next.tagName === "A") return next;
      next = next.parentElement;
      if (next === document.body) return null;
    }
  };
  _cleanup = (html) => {
    return html
      .replace(/link/g, "custom-link")
      .replace(
        /svg\/accent\.svg/g,
        `data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0iVVRGLTgiIHN0YW5kYWxvbmU9Im5vIj8+CjxzdmcKICAgdmlld0JveD0iMCAwIDMwMCAzMDAiCiAgIHZlcnNpb249IjEuMSIKICAgaWQ9InN2ZzEiCiAgIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIKICAgeG1sbnM6c3ZnPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+CiAgPGRlZnMKICAgICBpZD0iZGVmczEiIC8+CiAgPHBhdGgKICAgICBkPSJNIDk5LjMxNjQwNiw1NC4xNjQwNjIgViAzMy4wNzAzMTIgSCAxOTQuNTMxMjUgViAxMzMuNDEyMTEgSCAxNzMuNDM3NSBWIDU0LjE2NDA2MiBaIgogICAgIGlkPSJ0ZXh0MSIKICAgICBzdHlsZT0iZm9udC1zaXplOjMwMHB4O2ZvbnQtZmFtaWx5OidTb3VyY2UgSGFuIFNhbnMgSlAgTWVkaXVtJywgc2Fucy1zZXJpZjt0ZXh0LWFuY2hvcjptaWRkbGU7ZmlsbDojZGMxNDNjIgogICAgIGFyaWEtbGFiZWw9IuKMnSIgLz4KPC9zdmc+Cg==`,
      )
      .replace(
        /svg\/heiban\.svg/g,
        `data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0iVVRGLTgiIHN0YW5kYWxvbmU9Im5vIj8+CjxzdmcKICAgdmlld0JveD0iMCAwIDMwMCAzMDAiCiAgIHZlcnNpb249IjEuMSIKICAgaWQ9InN2ZzEiCiAgIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIKICAgeG1sbnM6c3ZnPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyI+CiAgPGRlZnMKICAgICBpZD0iZGVmczEiIC8+CiAgPHBhdGgKICAgICBkPSJNIDIzOS45OTk5NCwyMSBIIDU5Ljk5OTk0MyBWIDQ1IEggMjM5Ljk5OTk0IFoiCiAgICAgaWQ9InRleHQxIgogICAgIHN0eWxlPSJmb250LXNpemU6MzAwcHg7Zm9udC1mYW1pbHk6J1NvdXJjZSBIYW4gU2FucyBKUCBNZWRpdW0nLCBzYW5zLXNlcmlmO3RleHQtYW5jaG9yOm1pZGRsZTtmaWxsOiNkYzE0M2MiCiAgICAgYXJpYS1sYWJlbD0i4o66IiAvPgo8L3N2Zz4K`,
      )
      .replace(/src=\"HanaMinA\/(.*?\.svg)/g, (x, id, ...rest) => {
        console.log("replace :", x, id, rest);
        let replacement = window.HanaMinA[id];
        return x.replace(x, `src="${replacement}`);
      })
      .replace(/<a /, '<a target="_blank"');
  };
  _onmousemove = (e) => {
    if (e.shiftKey) {
      let word = getWordAtPoint(e.target, e.x, e.y);
      word = word || getWordAtPoint(e.target, e.x, e.y, true);
      if (word) {
        this._search(word, e.y, e.x);
      }
    }
  };
  _search(word, y, x) {
    query(word).then((res) => {
      if (res != "not found") {
        this._el.classList.add("nicebg");
        this._el.removeChild(this._elClose);
        this._el.removeChild(this._elZoom);
        this._el.innerHTML = `<div style="padding: 1em; background-color: rgba(255,255,255,0.955)">${this._cleanup(res)}</div>`;
        this._el.insertBefore(this._elClose, this._el.firstChild);
        this._el.insertBefore(this._elZoom, this._el.firstChild);

        this._el.style.top = y + "px";
        this._el.style.left = x + "px";
        this._el.style.display = "block";
      } else {
        query(getWordAtPoint(e.target, e.x, e.y, true)).then((res) => {
          if (res === "not found") return;
          this._el.removeChild(this._elClose);
          this._el.removeChild(this._elZoom);
          this._el.innerHTML = `<div style="padding: 1em; background-color: rgba(255,255,255,0.955)">${this._cleanup(res)}</div>`;
          this._el.insertBefore(this._elClose, this._el.firstChild);
          this._el.insertBefore(this._elZoom, this._el.firstChild);

          this._el.style.top = y + "px";
          this._el.style.left = x + "px";
          this._el.style.display = "block";
        });
      }
    });
  }
  connectedCallback() {
    this._el = document.createElement("div");
    this._el.draggable = true;
    let bcr = null;
    this._el.ondragstart = (e) => {
      e.dataTransfer.dropEffect = "move";
      e.dataTransfer.effectAllowed = "move";
      let img = new Image(0, 0);
      img.src =
        "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";
      e.dataTransfer.setDragImage(img, 0, 0);

      bcr = this._el.getBoundingClientRect();
    };

    this._el.ondrag = (e) => {
      this._el.style.top = -40 + e.pageY + "px";
      this._el.style.left = -40 + e.pageX + "px";
    };
    this._el.onclick = (e) => {
      if (e.target.tagName && e.target.tagName !== "A") {
        let node = this._parentALink(e.target);
        if (node) {
          e.preventDefault();
          e.stopPropagation();
          if (node.href.startsWith("sound://")) {
            // f&f
            playAudio(
              node.href
                .replace("sound://kanji_alive_audio/", "")
                .replace(".opus", ""),
            );
          }
        }
        return false;
      }
      if (e.target.tagName && e.target.tagName === "A") {
        console.log("onClick", e.target.href);
        e.preventDefault();
        e.stopPropagation();
        if (e.target.href.startsWith("entry://")) {
          queryById(e.target.dataset.targetId).then((res) => {
            if (res !== "not found") {
              this._el.removeChild(this._elClose);
              this._el.removeChild(this._elZoom);
              this._el.innerHTML = `<div style="padding: 1em; background-color: rgba(255,255,255,0.955)">${this._cleanup(res)}</div>`;
              this._el.insertBefore(this._elClose, this._el.firstChild);
              this._el.insertBefore(this._elZoom, this._el.firstChild);
            }
          });
        }
        if (e.target.href.startsWith("sound://")) {
          // fire and forget.
          playAudio(
            e.target.href
              .replace("sound://kanji_alive_audio/", "")
              .replace(".opus", ""),
          );
        }
      }
    };
    this._elZoom = document.createElement("div");
    this._elZoom.innerText = "^";
    this._elZoom.style =
      "font-variant: small-caps; line-height:1;text-align: center;background-color:lightsteelblue; border: 1px solid steelblue; position:sticky; top:1.5em; right: 3.0em; cursor: pointer; width: 1em; height: 1em;border-radius:0.1em;float:right";
    this._elZoom.onclick = (e) => {
      e.preventDefault();
      e.stopPropagation();
      if (this._elZoom.innerText === "^") {
        this._el.classList.add("zoomed");
        this._el.classList.remove("reversed");
        this._elZoom.innerText = "_";
        this._elZoom.style.lineHeight = 0.7;
      } else {
        this._el.classList.add("reversed");

        this._elZoom.innerText = "^";
        this._elZoom.style.lineHeight = 1;
      }
    };

    this._elClose = document.createElement("div");
    this._elClose.innerText = "x";
    this._elClose.style =
      "font-variant: small-caps; line-height:0.7;text-align: center;background-color:lightsteelblue; border: 1px solid steelblue; position:sticky; top:1.5em; right: 1.5em; cursor: pointer; width: 1em; height: 1em;border-radius:0.1em;float:right";
    this._elClose.onclick = (e) => {
      e.preventDefault();
      e.stopPropagation();
      this._el.classList.remove("zoomed");
      this._el.classList.remove("reversed");
      this._el.style.display = "none";
      this._elZoom.innerText = "^";
      this._elZoom.style.lineHeight = 1;
    };
    this._el.style =
      "max-height: 35vh; overflow: auto; margin:1em; padding: 1em;position: absolute; background-color: white; border:5px solid lightsteelblue; min-width: 30em;display: none; position:'absolute'; top:-1000px;left:-1000px; border-radius: 0.3em;";

    document.body.appendChild(this._el);
    this._el.insertBefore(this._elClose, this._el.firstChild);
    this._el.insertBefore(this._elZoom, this._el.firstChild);
    window.addEventListener("mousemove", this._onmousemove);
    window.lookup = (word) => {
      this._search(word, 0, 0);
      this._animateZoom();
      this._elZoom.innerText = "_";
      this._elZoom.style.lineHeight = 0.7;
    };
  }
  disconnectedCallback() {
    window.addEventListener("mousemove", this._onmousemove);
  }
}

class KentaSinglePageView extends HTMLElement {
  connectedCallback() {
    // Set up observer
    this._pageHandler = document.createElement("div");
    this._pageHandler.dataset.pagehandler = "true";
    this._pageHandler.innerHTML = "X of Y";
    this.appendChild(this._pageHandler);
    this.observer = new MutationObserver(this.onMutation.bind(this));

    // Watch light dom for child node changes
    this.observer.observe(this, {
      childList: true,
    });
  }

  onMutation(mutations) {
    const all = Array.from(this.querySelectorAll("section")).filter(
      (s) => s.parentElement === this,
    );
    const updatePageHandler = () => {
      const visibro = all.findIndex((e) => e.classList.contains("visible"));
      this.scrollTop = 0;
      this._pageHandler.innerHTML = `<button id="btnLT">&lt;</button> ${visibro + 1} of ${all.length} <button id="btnGT">&gt;</button>`;
      this._pageHandler.querySelector("#btnLT").onclick = () => {
        if (visibro == 0) return;
        all[visibro].classList.remove("visible");
        all[visibro - 1].classList.add("visible");
        updatePageHandler();
      };
      this._pageHandler.querySelector("#btnGT").onclick = () => {
        if (visibro == all.length - 1) return;
        all[visibro].classList.remove("visible");
        all[visibro + 1].classList.add("visible");
        updatePageHandler();
      };
    };
    updatePageHandler();
  }
  disconnectedCallback() {
    this.observer.disconnect();
  }
}

customElements.define("kenta-single-page-view", KentaSinglePageView);
customElements.define("kenta-mini-yt", MiniYT);

// mostly comes from https://github.com/kentaromiura/sakubireader
const colorize = (text) =>
  text
    .replace(
      /[^ァ-ン\u3400-\u4DB5\u4E00-\u9FCB\uF900-\uFA6A><！a-zA-Z:0-9\"',.()!\-\s]+/g,
      (match) => {
        switch (match) {
          case "に":
            return "<b class=ni>に</b>";
          case "へ":
            return "<b class=he>へ</b>";
          case "から":
            return "<b class=kare>から</b>";
          case "でした":
            return "<b class=copula>でした</b>";
          case "だった":
            return "<b class=copula>だった</b>";
          case "だ":
            return "<b class=copula>だ</b>";
          case "です":
            return "<b class=copula>です</b>";
          case "の":
            return "<b class=possession>の</b>";
          case "て":
            return "<b class=te>て</b>";
          case "が":
            return "<b class=ga>が</b>";
          case "は":
            return "<b class=wa>は</b>";
          case "を":
            return "<b class=o>を</b>";
          case "か":
            return "<b class=ka>か</b>";
          case "と":
            return "<b class=yo>と</b>";
          case "も":
            return "<b class=mo>も</b>";
        }
        return match;
      },
    )
    .replace(/[ァ-ン]+/g, (match) => `<b class=katakana>${match}</b>`)
    .replace(
      /[\u3400-\u4DB5\u4E00-\u9FCB\uF900-\uFA6A]+/g,
      (match) => `<b class=kanji>${match}</b>`,
    )
    // for now let's open in the default browser.
    .replace(/<a\ /g, '<a target="_blank"');

window.addEventListener("DOMContentLoaded", () => {
  const container = document.querySelector("#container");
  getContent()
    .then((content) => {
      const tmp = document.querySelector("#test-template");
      tmp.innerHTML = content;
      const allSections = tmp.content.querySelectorAll("section");
      const fragment = document.createDocumentFragment();
      Array.from(allSections)
        .slice(Array.from(allSections).findIndex((el) => el.id === "toc") + 1)
        .forEach((el, index) => {
          const copy = el.cloneNode(true);
          const newSection = document.createElement("section");
          if (index == 0) {
            newSection.classList.add("visible");
          }
          newSection.innerHTML = colorize(copy.innerHTML);
          // walk the tree to substitute the .
          const walker = document.createTreeWalker(
            newSection,
            NodeFilter.SHOW_TEXT,
          );
          while (walker.nextNode()) {
            if (walker.currentNode.textContent.includes(".")) {
              let span = document.createElement("span");
              span.innerHTML = walker.currentNode.textContent.replace(
                /\./g,
                " 。<br><br>",
              );
              walker.currentNode.parentNode.replaceChild(
                span,
                walker.currentNode,
              );
            }
          }

          fragment.appendChild(newSection);
        });
      container.appendChild(fragment);
    })
    .catch((error) => {
      document.querySelector("#container").innerHTML = error;
    });
});
