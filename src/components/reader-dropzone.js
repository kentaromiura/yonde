class ReaderDropzone extends HTMLElement {
  constructor() {
    super();
    console.log("[ReaderDropzone] constructor called");
    this.attachShadow({ mode: "open" });
  }

  connectedCallback() {
    console.log("[ReaderDropzone] connectedCallback called");
    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: block;
          position: relative;
          border: 2px dashed var(--border, #3a3a3a);
          border-radius: var(--radius-lg, 12px);
          padding: var(--spacing-xl, 32px);
          text-align: center;
          transition: all 150ms ease;
          cursor: pointer;
        }
        :host(:hover) {
          border-color: var(--accent, #4a9eff);
          background: var(--bg-hover, #303030);
        }
        :host([dragging]) {
          border-color: var(--accent, #4a9eff);
          background: rgba(74, 158, 255, 0.1);
          transform: scale(1.02);
        }
        .icon {
          font-size: 48px;
          margin-bottom: var(--spacing-md, 16px);
          color: var(--fg-muted, #888888);
        }
        .title {
          font-size: var(--font-size-lg, 16px);
          color: var(--fg, #e8e8e8);
          margin-bottom: var(--spacing-sm, 8px);
        }
        .subtitle {
          font-size: var(--font-size-sm, 12px);
          color: var(--fg-muted, #888888);
        }
        input[type="file"] {
          display: none;
        }
      </style>
      <div class="icon">📁</div>
      <div class="title">Drop CBZ file here</div>
      <div class="subtitle">or click to browse</div>
      <input type="file" id="fileInput" accept=".cbz,.zip">
    `;

    this._fileInput = this.shadowRoot.getElementById("fileInput");
    this._isOverDropzone = false;
    console.log("[ReaderDropzone] fileInput element:", this._fileInput);

    this.addEventListener("click", () => {
      console.log("[ReaderDropzone] Clicked, opening file dialog");
      this._fileInput.click();
    });

    this._fileInput.addEventListener("change", (e) => {
      console.log("[ReaderDropzone] File input change event:", e);
      const file = e.target.files[0];
      console.log("[ReaderDropzone] Selected file:", file, "path:", file?.path);
      if (file && file.path) {
        this._handleFilePath(file.path);
      } else if (file) {
        console.log(
          "[ReaderDropzone] File has no path property, name:",
          file.name,
        );
      }
    });

    this._setupTauriDrop();
  }

  async _setupTauriDrop() {
    if (!window.__TAURI__) {
      console.log("[ReaderDropzone] No Tauri API available");
      return;
    }

    console.log("[ReaderDropzone] Setting up Tauri drop handler");

    const { getCurrentWebview } = window.__TAURI__.webview;
    const { getCurrentWindow } = window.__TAURI__.window;
    const webview = getCurrentWebview();
    const unlisten = await getCurrentWindow().onDragDropEvent((event) => {
      //const unlisten = await webview.onDragDropEvent((event) => {
      console.log(
        "[ReaderDropzone] DragDrop event:",
        event.payload.type,
        event.payload,
      );

      if (event.payload.type === "enter") {
        this._isOverDropzone = true;
        this.setAttribute("dragging", "");
        console.log("[ReaderDropzone] Drag enter, paths:", event.payload.paths);
      } else if (event.payload.type === "leave") {
        this._isOverDropzone = false;
        this.removeAttribute("dragging");
        console.log("[ReaderDropzone] Drag leave");
      } else if (event.payload.type === "drop") {
        this.removeAttribute("dragging");
        const paths = event.payload.paths;
        console.log("[ReaderDropzone] Drop event, paths:", paths);
        if (paths && paths.length > 0) {
          const path = paths[0];
          console.log("[ReaderDropzone] First path:", path);
          if (path.endsWith(".cbz") || path.endsWith(".zip")) {
            console.log(
              "[ReaderDropzone] Dispatching file-selected for:",
              path,
            );
            this._handleFilePath(path);
          } else {
            console.log("[ReaderDropzone] File is not cbz/zip");
          }
        }
      } else if (event.payload.type === "over") {
        console.log("[ReaderDropzone] Drag over at:", event.payload.position);
      }
    });

    console.log("[ReaderDropzone] Drop handler registered");
    this._unlisten = unlisten;
  }

  _handleFilePath(path) {
    console.log("[ReaderDropzone] _handleFilePath called with:", path);
    this.dispatchEvent(
      new CustomEvent("file-selected", {
        detail: { path },
        bubbles: true,
      }),
    );
    console.log("[ReaderDropzone] file-selected event dispatched");
  }

  reset() {
    this._fileInput.value = "";
  }

  disconnectedCallback() {
    if (this._unlisten) {
      this._unlisten();
    }
  }
}

customElements.define("reader-dropzone", ReaderDropzone);
