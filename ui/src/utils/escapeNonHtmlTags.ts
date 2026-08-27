/**
 * Escape angle-bracket sequences in README markdown that are Rust type syntax
 * rather than HTML.
 *
 * Markdown passes raw HTML through, so a README that writes `Wave<S>` in prose
 * hands the browser an `<s>` element — strikethrough — and if it is never closed
 * (it never is; it is a generic parameter) everything after it renders struck
 * through. arya-core-belief lost 94% of its README that way. `<String>`,
 * `<Sample>`, `<MyBackend>` are worse in a different direction: DOMPurify drops
 * the unknown element and the author's visible text disappears with it.
 *
 * Escaping (rather than stripping) is the point: the reader should SEE `Vec<T>`.
 *
 * The rule leans on a convention that is reliable in a Rust registry:
 *   - a tag name containing an uppercase letter is a generic/type, never HTML
 *     (`<S>`, `<String>`, `<MyBackend>`, `<I,Intent,O>`)
 *   - an all-lowercase name that is not a known HTML element is also not HTML
 *     (`<f32>`, `<f64>`, `<port>`, `<flex>`)
 *   - anything else is left alone, so real markup keeps working
 *     (`<br>`, `<img src=…>`, `<details>`, `<sub>`, and a deliberate `<s>`)
 *
 * Code spans and fenced blocks are skipped entirely — markdown already renders
 * those literally, and escaping there would surface a visible `&lt;`.
 */

/** HTML elements a README may legitimately use. Lowercase comparison only. */
const HTML_TAGS = new Set([
  'a', 'abbr', 'b', 'blockquote', 'br', 'caption', 'center', 'code', 'col', 'colgroup',
  'dd', 'del', 'details', 'div', 'dl', 'dt', 'em', 'figcaption', 'figure', 'h1', 'h2',
  'h3', 'h4', 'h5', 'h6', 'hr', 'i', 'img', 'ins', 'kbd', 'li', 'mark', 'ol', 'p',
  'picture', 'pre', 'q', 's', 'samp', 'section', 'small', 'source', 'span', 'strong',
  'sub', 'summary', 'sup', 'table', 'tbody', 'td', 'tfoot', 'th', 'thead', 'tr', 'u',
  'ul', 'var', 'video',
])

/** True when `<name rest>` should be shown as text instead of parsed as HTML. */
function isTypeSyntax(name: string, rest: string): boolean {
  // `<I,Intent,O>` — the browser reads this as tag `i` with junk attributes.
  // Real HTML always separates attributes from the tag name with whitespace.
  if (rest !== '' && !/^[\s/]/.test(rest)) return true
  if (/[A-Z]/.test(name)) return true
  return !HTML_TAGS.has(name.toLowerCase())
}

export function escapeNonHtmlTags(markdown: string): string {
  if (!markdown) return markdown

  // Split into code / non-code regions. Fenced blocks first so that a stray
  // backtick inside a fence cannot open a phantom code span.
  const regions = markdown.split(/(^[ \t]*(?:```|~~~)[\s\S]*?^[ \t]*(?:```|~~~)[ \t]*$|`+[^`\n]*`+)/m)

  return regions
    .map((region) => {
      if (region === undefined) return ''
      const isCode = /^[ \t]*(?:```|~~~)/.test(region) || /^`+[^`\n]*`+$/.test(region)
      if (isCode) return region
      return region.replace(
        /<(\/?)([A-Za-z][A-Za-z0-9-]*)([^<>]*)>/g,
        (match, slash: string, name: string, rest: string) =>
          isTypeSyntax(name, rest) ? `&lt;${slash}${name}${rest}&gt;` : match,
      )
    })
    .join('')
}
