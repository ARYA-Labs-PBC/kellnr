import { describe, it, expect } from 'vitest'
import { escapeNonHtmlTags } from './escapeNonHtmlTags'

describe('escapeNonHtmlTags', () => {
  // THE regression: arya-core-belief 0.1.68 wrote `RSI Wave<S>` in prose. The
  // browser read <S> as strikethrough, it was never closed, and 94% of the
  // README rendered struck through.
  it('escapes an unclosed Rust generic that would strike through the rest', () => {
    const out = escapeNonHtmlTags('RSI Wave<S> type-state machine')
    expect(out).toBe('RSI Wave&lt;S&gt; type-state machine')
    expect(out).not.toContain('<S>')
  })

  // The other direction: DOMPurify drops an unknown element, taking the
  // author's visible text with it, so `<String>` silently vanished.
  it.each(['<String>', '<Sample>', '<MyBackend>', '<Proposed>', '<B>'])(
    'escapes the type name %s so it stays visible',
    (tag) => {
      expect(escapeNonHtmlTags(`takes ${tag} here`)).toContain(
        `&lt;${tag.slice(1, -1)}&gt;`,
      )
    },
  )

  it.each(['<f32>', '<f64>', '<port>', '<flex>', '<flexcomp>'])(
    'escapes lowercase non-HTML name %s',
    (tag) => {
      expect(escapeNonHtmlTags(`value ${tag}`)).toContain('&lt;')
    },
  )

  // `<I,Intent,O>` parses as tag `i` with junk attributes — italics, not HTML.
  it('escapes a multi-parameter generic that would parse as <i>', () => {
    expect(escapeNonHtmlTags('Pipeline<I,Intent,O>')).toBe('Pipeline&lt;I,Intent,O&gt;')
  })

  it.each(['<br>', '<img src="x.png">', '<details>', '<summary>', '<sub>', '<a href="#x">'])(
    'leaves real markup %s untouched',
    (tag) => {
      expect(escapeNonHtmlTags(`x ${tag} y`)).toContain(tag)
    },
  )

  it('leaves a deliberate lowercase <s> strikethrough alone', () => {
    expect(escapeNonHtmlTags('<s>gone</s>')).toBe('<s>gone</s>')
  })

  it('closing tags are escaped in step with their opener', () => {
    expect(escapeNonHtmlTags('<Wrapper>x</Wrapper>')).toBe('&lt;Wrapper&gt;x&lt;/Wrapper&gt;')
  })

  // Escaping inside code would surface a literal &lt; to the reader.
  it('does not touch inline code spans', () => {
    expect(escapeNonHtmlTags('use `Vec<T>` please')).toBe('use `Vec<T>` please')
  })

  it('does not touch fenced code blocks', () => {
    const md = 'before\n\n```rust\nlet x: Vec<T> = vec![];\n```\n\nafter Wave<S>'
    const out = escapeNonHtmlTags(md)
    expect(out).toContain('let x: Vec<T> = vec![];')
    expect(out).toContain('after Wave&lt;S&gt;')
  })

  it('handles tilde fences too', () => {
    const out = escapeNonHtmlTags('~~~\nVec<T>\n~~~')
    expect(out).toContain('Vec<T>')
  })

  it('is a no-op on empty input', () => {
    expect(escapeNonHtmlTags('')).toBe('')
  })
})
