## Yonde

![Demo movie](screenshot/output.webp)

Known bugs:
- theme is not applied to dictionary and OCR text.
- On change fit mode OCR text is not resized
- Fit mode don't work correctly (width mode somehow cuts top of image)
- Scroll position is not reset when changing page
- OCR can be slow (I do some caching for already opened pages and pre-cache the next page but sometimes it takes a long time to parse).
- OCR sometimes don't find the text or get it wrong, I can't do much for this.


## On LLM usage:
- OCR code has been ported from koharu via:
Antigravity: I asked to extract the OCR part from koharu -> created a comic-ocr library using candle framework
Opencode (minimax M2.5, kimi 2.5, GLM 5): Asked to migrate comic-ocr to burn
Original work by minimax, not exactly working, after finishing free token switch to kimi that made OCR work and after those token were finished GLM made the rest work and help with removing unused parts; I've manually made some changes such as re-encode the model so it's half the size.
GLM also created some UI component after I asked nicely and the CBZ wrapper.

References:
===
The idea came from [Mokuro](https://github.com/kha-white/mokuro),
The main code is instead from my [Sakubi reader app](https://github.com/kentaromiura/sakubi-reader-app)
OCR code is a port of [Koharu](https://github.com/mayocream/koharu) from [candle framework](https://github.com/huggingface/candle) to [burn](https://github.com/tracel-ai/burn) as the latter supports AMD gpus better, original library is under https://github.com/mayocream/koharu/blob/main/LICENSE-APACHE which is compatible with the project MIT.
