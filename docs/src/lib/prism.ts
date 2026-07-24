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
  attribute: {
    pattern: /@+[\w.]+(?:\([^)]*\))?/,
    inside: {
      'attr-name': /@+[\w.]+/,
      punctuation: /[()]/,
      'attr-value': {
        pattern: /[^()]+/,
        inside: {
          string: /"[^"]*"/,
          number: /\b\d+\b/,
          boolean: /\b(?:true|false)\b/,
          keyword: /\b(?:Cascade|SetNull|Restrict|NoAction|SetDefault)\b/,
        },
      },
    },
  },
  'type-name':
    /\b(?:Int|BigInt|Float|Decimal|String|Boolean|Bool|DateTime|Date|Time|Json|Bytes|Uuid|UUID|Cuid|Cuid2|NanoId|Ulid|ULID)\b/,
  'class-name': /\b[A-Z][a-zA-Z0-9_]*\b/,
  operator: /[?[\]]/,
  punctuation: /[{}(),]/,
  number: /\b\d+(?:\.\d+)?\b/,
  boolean: /\b(?:true|false)\b/,
};

// Alias for prisma syntax (maps to prax)
Prism.languages['prisma'] = Prism.languages['prax'];

export default Prism;
