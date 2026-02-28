const { invoke } = window.__TAURI__.core;

class ReaderContainer extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: "open" });
    this._currentPath = null;
    this._pages = [];
    this._currentIndex = 0;
    this._ocrEnabled = false;
    this._ocrInitialized = false;
    this._ocrCache = {};
  }

  connectedCallback() {
    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: flex;
          flex-direction: column;
          height: 100%;
          width: 100%;
          background: var(--bg, #1a1a1a);
        }
        .header {
          flex-shrink: 0;
        }
        .content {
          flex: 1;
          position: relative;
          overflow: hidden;
        }
        .footer {
          flex-shrink: 0;
        }
        reader-viewer {
          width: 100%;
          height: 100%;
        }
        .loading {
          position: absolute;
          top: 50%;
          left: 50%;
          transform: translate(-50%, -50%);
          color: var(--fg-muted, #888888);
          font-size: var(--font-size-lg, 16px);
        }
        .dropzone-container {
          position: absolute;
          top: 0;
          left: 0;
          right: 0;
          bottom: 0;
          display: flex;
          align-items: center;
          justify-content: center;
        }
        .hidden { display: none !important; }
      </style>
      <div class="header">
        <reader-toolbar>
          <reader-nav id="nav" slot="start"></reader-nav>
          <reader-indicator id="indicator" slot="end"></reader-indicator>
          <button id="ocrBtn" slot="end" title="Enable OCR">OCR</button>
        </reader-toolbar>
      </div>
      <div class="content">
        <div class="dropzone-container" id="dropzoneContainer">
          <reader-dropzone id="dropzone"></reader-dropzone>
        </div>
        <reader-viewer id="viewer" class="hidden"></reader-viewer>
        <div class="loading hidden" id="loading">Loading...</div>
      </div>
    `;

    this._dropzone = this.shadowRoot.getElementById("dropzone");
    this._dropzoneContainer =
      this.shadowRoot.getElementById("dropzoneContainer");
    this._viewer = this.shadowRoot.getElementById("viewer");
    this._nav = this.shadowRoot.getElementById("nav");
    this._indicator = this.shadowRoot.getElementById("indicator");
    this._loading = this.shadowRoot.getElementById("loading");
    this._ocrBtn = this.shadowRoot.getElementById("ocrBtn");

    this._dropzone.addEventListener("file-selected", (e) =>
      this._onFileSelected(e),
    );
    this._nav.addEventListener("navigate", (e) => this._onNavigate(e));
    this._ocrBtn.addEventListener("click", () => this._toggleOcr());

    window.addEventListener("keydown", (e) => this._onKeyDown(e));
  }

  async _onFileSelected(e) {
    const { path } = e.detail;
    if (!path) return;

    this._showLoading(true);

    try {
      this._pages = await invoke("open_cbz", { path });
      this._pages.sort();
      console.log("sorted pages", this._pages);
      this._currentPath = path;
      this._currentIndex = 0;

      this._dropzoneContainer.classList.add("hidden");
      this._viewer.classList.remove("hidden");

      this._indicator.total = this._pages.length;
      this._indicator.current = 1;
      this._updateNavState();

      await this._loadPage(0);
    } catch (err) {
      console.error("Failed to open CBZ:", err);
      alert("Failed to open CBZ: " + err);
    } finally {
      this._showLoading(false);
    }
  }

  async getPageWithOcr({ path, pageName }) {
    const cachePath = `${path}:${pageName}`;
    if (this._ocrCache[cachePath]) return this._ocrCache[cachePath];
    const result = await invoke("get_page_with_ocr", {
      path,
      pageName,
    });
    this._ocrCache[cachePath] = result;
    return result;
  }

  async _loadPage(index) {
    if (index < 0 || index >= this._pages.length) return;

    this._showLoading(true);
    this._currentIndex = index;

    try {
      const pageName = this._pages[index];

      if (this._ocrEnabled && this._ocrInitialized) {
        const result = await getPageWithOcr({
          path: this._currentPath,
          pageName,
        });

        //   await invoke("get_page_with_ocr", {
        //   path: this._currentPath,
        //   pageName,
        // });

        const dataUrl = `data:${result.mime_type};base64,${result.image}`;
        this._viewer.setImage(dataUrl, result.width, result.height);

        this._viewer.clearOverlays();
        for (const ocr of result.ocr_results) {
          const [x1, y1, x2, y2] = ocr.bbox;
          this._viewer.addOverlay(x1, y1, x2 - x1, y2 - y1, "", {
            type: "highlight",
          });
        }
      } else {
        const result = await invoke("get_page", {
          path: this._currentPath,
          pageName,
        });

        const dataUrl = `data:${result.mime_type};base64,${result.image}`;
        this._viewer.setImage(dataUrl, result.width, result.height);
        this._viewer.clearOverlays();
      }

      this._indicator.current = index + 1;
      this._updateNavState();
    } catch (err) {
      console.error("Failed to load page:", err);
    } finally {
      this._showLoading(false);
    }
  }

  _onNavigate(e) {
    const { direction } = e.detail;
    if (direction === "prev") {
      this._loadPage(this._currentIndex - 1);
    } else {
      this._loadPage(this._currentIndex + 1);
    }
  }

  _onKeyDown(e) {
    if (this._pages.length === 0) return;

    if (e.key === "ArrowLeft") {
      this._loadPage(this._currentIndex - 1);
    } else if (e.key === "ArrowRight") {
      this._loadPage(this._currentIndex + 1);
    }
  }

  _updateNavState() {
    this._nav.hasPrev = this._currentIndex > 0;
    this._nav.hasNext = this._currentIndex < this._pages.length - 1;
  }

  async _toggleOcr() {
    if (!this._ocrInitialized) {
      this._ocrBtn.disabled = true;
      this._ocrBtn.textContent = "Loading OCR...";

      try {
        await invoke("init_ocr");
        this._ocrInitialized = true;
        this._ocrEnabled = true;
        this._ocrBtn.textContent = "OCR: ON";
        this._ocrBtn.style.color = "var(--success)";

        if (this._pages.length > 0) {
          await this._loadPage(this._currentIndex);
        }
      } catch (err) {
        console.error("Failed to initialize OCR:", err);
        this._ocrBtn.textContent = "OCR: Error";
        this._ocrBtn.style.color = "var(--error)";
      } finally {
        this._ocrBtn.disabled = false;
      }
    } else {
      this._ocrEnabled = !this._ocrEnabled;
      this._ocrBtn.textContent = this._ocrEnabled ? "OCR: ON" : "OCR: OFF";
      this._ocrBtn.style.color = this._ocrEnabled ? "var(--success)" : "";

      if (this._pages.length > 0) {
        await this._loadPage(this._currentIndex);
      }
    }
  }

  _showLoading(show) {
    if (show) {
      this._loading.classList.remove("hidden");
    } else {
      this._loading.classList.add("hidden");
    }
  }
}

customElements.define("reader-container", ReaderContainer);
