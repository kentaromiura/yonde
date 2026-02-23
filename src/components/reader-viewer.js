class ReaderViewer extends HTMLElement {
  static observedAttributes = ['fit-mode'];
  
  constructor() {
    super();
    this.attachShadow({ mode: 'open' });
    this._overlays = [];
    this._naturalWidth = 0;
    this._naturalHeight = 0;
    this._renderedWidth = 0;
    this._renderedHeight = 0;
    this._offsetX = 0;
    this._offsetY = 0;
  }
  
  connectedCallback() {
    this.shadowRoot.innerHTML = `
      <style>
        :host {
          display: block;
          position: relative;
          width: 100%;
          height: 100%;
          overflow: auto;
          background: var(--bg, #1a1a1a);
        }
        .container {
          display: flex;
          align-items: center;
          justify-content: center;
          min-width: 100%;
          height: 100%;
        }
        .image-wrapper {
          position: relative;
          display: inline-block;
        }
        .image-wrapper.fit-contain {
          height: 100%;
        }
        .image-wrapper.fit-width {
          width: 100%;
          height: auto;
        }
        .image-wrapper.fit-original {
          /* natural size */
        }
        img {
          display: block;
          max-width: none;
        }
        .image-wrapper.fit-contain img {
          height: 100%;
          width: auto;
          max-width: none;
          object-fit: contain;
        }
        .image-wrapper.fit-width img {
          width: 100%;
          height: auto;
        }
        .overlay-layer {
          position: absolute;
          top: 0;
          left: 0;
          width: 100%;
          height: 100%;
          pointer-events: none;
        }
        .overlay-layer > * {
          pointer-events: auto;
        }
        .overlay {
          position: absolute;
          box-sizing: border-box;
        }
        .overlay.type-highlight {
          background: rgba(74, 158, 255, 0.3);
          border: 2px solid var(--accent, #4a9eff);
          border-radius: var(--radius-sm, 4px);
        }
        .overlay.type-ocr {
          background: rgba(74, 158, 255, 0.2);
          border: 1px solid rgba(74, 158, 255, 0.5);
          border-radius: var(--radius-sm, 4px);
          cursor: pointer;
          transition: background 150ms ease;
          display: flex;
          align-items: flex-start;
          justify-content: flex-start;
          overflow: hidden;
        }
        .overlay.type-ocr:hover {
          background: rgba(74, 158, 255, 0.35);
        }
        .overlay.type-ocr .ocr-text {
          background: rgba(0, 0, 0, 0.7);
          color: var(--fg, #e8e8e8);
          padding: 2px 4px;
          font-size: 10px;
          line-height: 1.2;
          white-space: pre-wrap;
          word-break: break-all;
          max-width: 100%;
          max-height: 100%;
          overflow: hidden;
          text-overflow: ellipsis;
          user-select: text;
          cursor: text;
        }
        .overlay.type-badge {
          background: var(--accent, #4a9eff);
          color: var(--bg, #1a1a1a);
          padding: var(--spacing-xs, 4px) var(--spacing-sm, 8px);
          border-radius: var(--radius-sm, 4px);
          font-size: var(--font-size-sm, 12px);
          font-weight: 500;
        }
        .overlay.type-tooltip {
          background: var(--bg-elevated, #252525);
          color: var(--fg, #e8e8e8);
          border: 1px solid var(--border, #3a3a3a);
          border-radius: var(--radius-md, 8px);
          padding: var(--spacing-sm, 8px) var(--spacing-md, 16px);
          font-size: var(--font-size-sm, 12px);
          box-shadow: var(--shadow-md, 0 4px 6px rgba(0, 0, 0, 0.4));
        }
        .placeholder {
          display: flex;
          align-items: center;
          justify-content: center;
          width: 100%;
          height: 100%;
          color: var(--fg-muted, #888888);
          font-size: var(--font-size-lg, 16px);
        }
      </style>
      <div class="container">
        <div class="placeholder" id="placeholder">Drop a CBZ file to start reading</div>
        <div class="image-wrapper fit-contain" id="wrapper" style="display: none;">
          <img id="image" alt="Manga page">
          <div class="overlay-layer" id="overlayLayer"></div>
        </div>
      </div>
    `;
    
    this._image = this.shadowRoot.getElementById('image');
    this._wrapper = this.shadowRoot.getElementById('wrapper');
    this._placeholder = this.shadowRoot.getElementById('placeholder');
    this._overlayLayer = this.shadowRoot.getElementById('overlayLayer');
    
    this._image.addEventListener('load', () => this._onImageLoad());
    this._observer = new ResizeObserver(() => this._updateOverlayPositions());
    this._observer.observe(this._wrapper);
  }
  
  disconnectedCallback() {
    this._observer?.disconnect();
  }
  
  attributeChangedCallback(name, oldVal, newVal) {
    if (name === 'fit-mode' && this._wrapper) {
      this._wrapper.className = `image-wrapper fit-${newVal || 'contain'}`;
      this._updateOverlayPositions();
    }
  }
  
  get fitMode() {
    return this.getAttribute('fit-mode') || 'contain';
  }
  
  set fitMode(value) {
    this.setAttribute('fit-mode', value);
  }
  
  setImage(src, naturalWidth, naturalHeight) {
    this._naturalWidth = naturalWidth;
    this._naturalHeight = naturalHeight;
    this._image.src = src;
    this._wrapper.style.display = '';
    this._placeholder.style.display = 'none';
  }
  
  clearImage() {
    this._image.src = '';
    this._wrapper.style.display = 'none';
    this._placeholder.style.display = '';
    this._naturalWidth = 0;
    this._naturalHeight = 0;
    this._renderedWidth = 0;
    this._renderedHeight = 0;
  }
  
  addOverlay(x, y, width, height, content, options = {}) {
    const overlay = document.createElement('div');
    overlay.className = `overlay type-${options.type || 'highlight'}`;
    
    if (options.type === 'ocr' && content) {
      const textEl = document.createElement('span');
      textEl.className = 'ocr-text';
      textEl.textContent = content;
      overlay.appendChild(textEl);
    } else {
      overlay.textContent = content || '';
    }
    
    const data = { element: overlay, x, y, width, height, options };
    this._overlays.push(data);
    this._overlayLayer.appendChild(overlay);
    this._updateOverlayPositions();
    
    return overlay;
  }
  
  removeOverlay(overlayElement) {
    const index = this._overlays.findIndex(o => o.element === overlayElement);
    if (index !== -1) {
      this._overlays.splice(index, 1);
      overlayElement.remove();
    }
  }
  
  clearOverlays() {
    this._overlays = [];
    this._overlayLayer.innerHTML = '';
  }
  
  getOverlayBounds() {
    return {
      renderedWidth: this._renderedWidth,
      renderedHeight: this._renderedHeight,
      offsetX: this._offsetX,
      offsetY: this._offsetY,
      naturalWidth: this._naturalWidth,
      naturalHeight: this._naturalHeight,
      scaleX: this._renderedWidth / this._naturalWidth || 1,
      scaleY: this._renderedHeight / this._naturalHeight || 1
    };
  }
  
  _onImageLoad() {
    if (!this._naturalWidth && !this._naturalHeight) {
      this._naturalWidth = this._image.naturalWidth;
      this._naturalHeight = this._image.naturalHeight;
    }
    this._updateOverlayPositions();
  }
  
  _updateOverlayPositions() {
    if (!this._naturalWidth || !this._image.clientWidth) return;
    
    const imgRect = this._image.getBoundingClientRect();
    const wrapperRect = this._wrapper.getBoundingClientRect();
    
    this._renderedWidth = this._image.clientWidth;
    this._renderedHeight = this._image.clientHeight;
    this._offsetX = imgRect.left - wrapperRect.left;
    this._offsetY = imgRect.top - wrapperRect.top;
    
    const scaleX = this._renderedWidth / this._naturalWidth;
    const scaleY = this._renderedHeight / this._naturalHeight;
    
    for (const overlay of this._overlays) {
      const { element, x, y, width, height } = overlay;
      element.style.left = `${x * scaleX}px`;
      element.style.top = `${y * scaleY}px`;
      element.style.width = `${width * scaleX}px`;
      element.style.height = `${height * scaleY}px`;
    }
  }
}

customElements.define('reader-viewer', ReaderViewer);
