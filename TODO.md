- Move from demo to real:
  - On drag of CBZ file it should call src-tauri api passing the path, tauri app should return the list of files in the CBZ
  - API to stream image API should return also the OCR data.
  	- Include comic-ocr inside src-tauri
  	- Include cbz inside src-tauri and expose API for opening and streaming cbz pages/list pages
   - cache for image/ocr data will be added on UI later

- The dictionary view don't look too good on dark mode
 - Fix it by using better style for text that can't be seen in dark mode


- change the name from mokuro-app to something else...
