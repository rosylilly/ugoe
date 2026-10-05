// 受け手が受け取った背景の画像を描けるか (docs/studio.md#受け手が読めなかったとき)。背景は CSS で描くので、
// 壊れた画像でも CSS は何も言わない。同じ blob URL を画像の要素でデコードして確かめる (T0836)。

/** `url` の画像をデコードできるか。 */
export async function decodeImage(url: string): Promise<boolean> {
  const image = new Image();
  image.src = url;
  try {
    await image.decode();
    return true;
  } catch {
    return false;
  }
}
