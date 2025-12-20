import { Component, input, computed, signal, ViewEncapsulation } from '@angular/core';
import Prism from 'prismjs';

// Import additional languages
import 'prismjs/components/prism-rust';
import 'prismjs/components/prism-toml';
import 'prismjs/components/prism-bash';
import 'prismjs/components/prism-sql';
import 'prismjs/components/prism-json';
import 'prismjs/components/prism-yaml';
import 'prismjs/components/prism-typescript';
import 'prismjs/components/prism-graphql';

// Register custom Prax language (based on Rust/Prisma syntax)
Prism.languages['prax'] = {
  comment: [
    {
      pattern: /\/\/\/.*/,
      alias: 'doc-comment',
      greedy: true,
    },
    {
      pattern: /\/\/.*/,
      greedy: true,
    },
  ],
  string: {
    pattern: /"(?:[^"\\]|\\.)*"/,
    greedy: true,
  },
  keyword: /\b(?:model|enum|view|type|datasource|generator|plugin)\b/,
  'attribute': {
    pattern: /@+[\w.]+(?:\([^)]*\))?/,
    inside: {
      'attr-name': /@+[\w.]+/,
      'punctuation': /[()]/,
      'attr-value': {
        pattern: /[^()]+/,
        inside: {
          'string': /"[^"]*"/,
          'number': /\b\d+\b/,
          'boolean': /\b(?:true|false)\b/,
          'keyword': /\b(?:Cascade|SetNull|Restrict|NoAction|SetDefault)\b/,
        },
      },
    },
  },
  'type-name': /\b(?:Int|BigInt|Float|Decimal|String|Boolean|Bool|DateTime|Date|Time|Json|Bytes|Uuid|UUID|Cuid|Cuid2|NanoId|Ulid|ULID)\b/,
  'class-name': /\b[A-Z][a-zA-Z0-9_]*\b/,
  operator: /[?[\]]/,
  punctuation: /[{}(),]/,
  number: /\b\d+(?:\.\d+)?\b/,
  boolean: /\b(?:true|false)\b/,
};

// Alias for prisma syntax (maps to prax)
Prism.languages['prisma'] = Prism.languages['prax'];

@Component({
  selector: 'app-code-block',
  standalone: true,
  templateUrl: './code-block.component.html',
  styleUrl: './code-block.component.css',
  encapsulation: ViewEncapsulation.None,
})
export class CodeBlockComponent {
  code = input.required<string>();
  language = input<string>('');
  filename = input<string>('');

  copied = signal(false);

  highlightedCode = computed(() => {
    const lang = this.language();
    const code = this.code();

    if (lang && Prism.languages[lang]) {
      return Prism.highlight(code, Prism.languages[lang], lang);
    }

    // Try to auto-detect based on content
    if (code.includes('model ') && code.includes('@')) {
      return Prism.highlight(code, Prism.languages['prax'], 'prax');
    }
    if (code.includes('fn ') || code.includes('let ') || code.includes('impl ')) {
      return Prism.highlight(code, Prism.languages['rust'], 'rust');
    }

    // Return escaped code if no language detected
    return this.escapeHtml(code);
  });

  private escapeHtml(text: string): string {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  async copyCode() {
    try {
      await navigator.clipboard.writeText(this.code());
      this.copied.set(true);
      setTimeout(() => this.copied.set(false), 2000);
    } catch (err) {
      console.error('Failed to copy code:', err);
    }
  }
}
