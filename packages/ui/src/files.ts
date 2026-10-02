// DropZone が使うファイルの選り分け (純粋な関数)。

/**
 * ファイルが `accept` (`<input type="file">` の accept と同じ書き方。`.ugoe,.psd`・`image/*`・`image/png`) に合うか。
 * `accept` が空なら、どのファイルも合う。拡張子は大文字と小文字を区別しない。
 */
export function acceptsFile(
  file: { name: string; type: string },
  accept: string | undefined,
): boolean {
  const tokens = (accept ?? "")
    .split(",")
    .map((t) => t.trim().toLowerCase())
    .filter(Boolean);
  if (tokens.length === 0) return true;
  const name = file.name.toLowerCase();
  const type = file.type.toLowerCase();
  return tokens.some((token) => {
    if (token.startsWith(".")) return name.endsWith(token);
    if (token.endsWith("/*")) return type.startsWith(token.slice(0, -1));
    return type === token;
  });
}

/** ファイルを `accept` に合うものと合わないものに分ける。 */
export function partitionFiles<F extends { name: string; type: string }>(
  files: Iterable<F>,
  accept: string | undefined,
): { accepted: F[]; rejected: F[] } {
  const accepted: F[] = [];
  const rejected: F[] = [];
  for (const file of files) (acceptsFile(file, accept) ? accepted : rejected).push(file);
  return { accepted, rejected };
}
