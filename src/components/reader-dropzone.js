class ReaderDropzone extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
  }
  
  connectedCallback() {
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
    
    this._fileInput = this.shadowRoot.getElementById('fileInput');
    
    this.addEventListener('dragover', (e) => {
      e.preventDefault();
      this.setAttribute('dragging', '');
    });
    
    this.addEventListener('dragleave', (e) => {
      e.preventDefault();
      this.removeAttribute('dragging');
    });
    
    this.addEventListener('drop', (e) => {
      e.preventDefault();
      this.removeAttribute('dragging');
      const file = e.dataTransfer.files[0];
      if (file) this._handleFile(file);
    });
    
    this.addEventListener('click', () => {
      this._fileInput.click();
    });
    
    this._fileInput.addEventListener('change', (e) => {
      const file = e.target.files[0];
      if (file) this._handleFile(file);
    });
  }
  
  _handleFile(file) {
    this.dispatchEvent(new CustomEvent('file-selected', {
      detail: { file },
      bubbles: true
    }));
  }
  
  reset() {
    this._fileInput.value = '';
  }
}

customElements.define('reader-dropzone', ReaderDropzone);
