export interface NavItem {
  href: string;
  label: string;
  /** SVG path `d` attributes */
  icon: string[];
  /** Use `fill="currentColor"` instead of stroke styling */
  iconFill?: boolean;
}

export interface NavSection {
  title: string;
  items: NavItem[];
}

export const githubUrl = 'https://github.com/quinnjr/prax';

export const githubIcon =
  'M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z';

const cogIcon = [
  'M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z',
  'M15 12a3 3 0 11-6 0 3 3 0 016 0z',
];

const boltIcon = ['M13 10V3L4 14h7v7l9-11h-7z'];

const databaseIcon = [
  'M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4',
];

const circleIcon = ['M12 2C6.477 2 2 6.477 2 12s4.477 10 10 10 10-4.477 10-10S17.523 2 12 2z'];

const serverIcon = [
  'M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01',
];

const flaskIcon = [
  'M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z',
];

const linkIcon = [
  'M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1',
];

const barChartIcon = [
  'M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z',
];

const reportIcon = [
  'M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z',
];

export const navSections: NavSection[] = [
  {
    title: 'Getting Started',
    items: [
      {
        href: '/',
        label: 'Introduction',
        icon: [
          'M3 12l2-2m0 0l7-7 7 7M5 10v10a1 1 0 001 1h3m10-11l2 2m-2-2v10a1 1 0 01-1 1h-3m-6 0a1 1 0 001-1v-4a1 1 0 011-1h2a1 1 0 011 1v4a1 1 0 001 1m-6 0h6',
        ],
      },
      { href: '/quickstart', label: 'Quick Start', icon: boltIcon },
      {
        href: '/installation',
        label: 'Installation',
        icon: ['M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4'],
      },
      { href: '/configuration', label: 'Configuration', icon: cogIcon },
      { href: '/performance', label: 'Performance', icon: boltIcon },
    ],
  },
  {
    title: 'Schema',
    items: [
      {
        href: '/schema/overview',
        label: 'Schema Overview',
        icon: [
          'M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z',
        ],
      },
      {
        href: '/schema/models',
        label: 'Models',
        icon: [
          'M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10',
        ],
      },
      {
        href: '/schema/fields',
        label: 'Fields & Types',
        icon: ['M4 6h16M4 10h16M4 14h16M4 18h16'],
      },
      { href: '/schema/relations', label: 'Relations', icon: linkIcon },
      {
        href: '/schema/attributes',
        label: 'Attributes',
        icon: [
          'M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z',
        ],
      },
      { href: '/schema/enums', label: 'Enums', icon: ['M4 6h16M4 10h16M4 14h16M4 18h16'] },
      {
        href: '/schema/views',
        label: 'Views',
        icon: [
          'M15 12a3 3 0 11-6 0 3 3 0 016 0z',
          'M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z',
        ],
      },
      { href: '/schema/generators', label: 'Generators & Datasources', icon: cogIcon },
      { href: '/schema/server-groups', label: 'Server Groups', icon: serverIcon },
    ],
  },
  {
    title: 'Queries',
    items: [
      { href: '/queries/crud', label: 'CRUD Operations', icon: databaseIcon },
      {
        href: '/queries/filtering',
        label: 'Filtering',
        icon: [
          'M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z',
        ],
      },
      {
        href: '/queries/pagination',
        label: 'Pagination',
        icon: [
          'M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2',
        ],
      },
      { href: '/queries/aggregations', label: 'Aggregations', icon: barChartIcon },
      {
        href: '/queries/raw-sql',
        label: 'Raw SQL',
        icon: ['M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4'],
      },
      {
        href: '/queries/procedures',
        label: 'Procedures & Functions',
        icon: ['M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z'],
      },
      {
        href: '/queries/triggers',
        label: 'Triggers & Events',
        icon: [
          'M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9',
        ],
      },
      {
        href: '/queries/sequences',
        label: 'Sequences & Identity',
        icon: ['M7 20l4-16m2 16l4-16M6 9h14M4 15h14'],
      },
      {
        href: '/queries/search',
        label: 'Full-Text Search',
        icon: ['M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z'],
      },
      {
        href: '/queries/json',
        label: 'JSON Operations',
        icon: [
          'M17 8h2a2 2 0 012 2v6a2 2 0 01-2 2h-2v4l-4-4H9a1.994 1.994 0 01-1.414-.586m0 0L11 14h4a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2v4l.586-.586z',
        ],
      },
      {
        href: '/queries/cte',
        label: 'CTEs & Window Functions',
        icon: [
          'M4 5a1 1 0 011-1h14a1 1 0 011 1v2a1 1 0 01-1 1H5a1 1 0 01-1-1V5zM4 13a1 1 0 011-1h6a1 1 0 011 1v6a1 1 0 01-1 1H5a1 1 0 01-1-1v-6zM16 13a1 1 0 011-1h2a1 1 0 011 1v6a1 1 0 01-1 1h-2a1 1 0 01-1-1v-6z',
        ],
      },
      {
        href: '/queries/upsert',
        label: 'Upsert & Conflicts',
        icon: [
          'M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15',
        ],
      },
    ],
  },
  {
    title: 'Database',
    items: [
      { href: '/database/postgresql', label: 'PostgreSQL', icon: circleIcon, iconFill: true },
      { href: '/database/mysql', label: 'MySQL', icon: circleIcon, iconFill: true },
      { href: '/database/sqlite', label: 'SQLite', icon: circleIcon, iconFill: true },
      { href: '/database/mssql', label: 'MSSQL', icon: circleIcon, iconFill: true },
      { href: '/database/mongodb', label: 'MongoDB', icon: circleIcon, iconFill: true },
      { href: '/database/duckdb', label: 'DuckDB', icon: barChartIcon },
      {
        href: '/database/migrations',
        label: 'Migrations',
        icon: ['M7 16V4m0 0L3 8m4-4l4 4m6 0v12m0 0l4-4m-4 4l-4-4'],
      },
      { href: '/database/seeding', label: 'Seeding', icon: ['M12 6v6m0 0v6m0-6h6m-6 0H6'] },
    ],
  },
  {
    title: 'Integrations',
    items: [
      { href: '/integrations/armature', label: 'Armature', icon: flaskIcon },
      { href: '/integrations/axum', label: 'Axum', icon: serverIcon },
      { href: '/integrations/actix', label: 'Actix-web', icon: databaseIcon },
    ],
  },
  {
    title: 'Advanced',
    items: [
      { href: '/advanced/connection', label: 'Connection & Config', icon: linkIcon },
      {
        href: '/advanced/middleware',
        label: 'Middleware',
        icon: [
          'M12 6V4m0 2a2 2 0 100 4m0-4a2 2 0 110 4m-6 8a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4m6 6v10m6-2a2 2 0 100-4m0 4a2 2 0 110-4m0 4v2m0-6V4',
        ],
      },
      {
        href: '/advanced/errors',
        label: 'Error Handling',
        icon: [
          'M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z',
        ],
      },
      { href: '/advanced/performance', label: 'Advanced Performance', icon: boltIcon },
      {
        href: '/advanced/security',
        label: 'Security & Access Control',
        icon: [
          'M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z',
        ],
      },
      {
        href: '/advanced/multitenancy',
        label: 'Multi-Tenancy',
        icon: [
          'M19 21V5a2 2 0 00-2-2H7a2 2 0 00-2 2v16m14 0h2m-2 0h-5m-9 0H3m2 0h5M9 7h1m-1 4h1m4-4h1m-1 4h1m-5 10v-5a1 1 0 011-1h2a1 1 0 011 1v5m-4 0h4',
        ],
      },
      {
        href: '/advanced/caching',
        label: 'Caching',
        icon: ['M5 8h14M5 8a2 2 0 110-4h14a2 2 0 110 4M5 8v10a2 2 0 002 2h10a2 2 0 002-2V8m-9 4h4'],
      },
      { href: '/advanced/profiling', label: 'Memory Profiling', icon: reportIcon },
      {
        href: '/advanced/extensions',
        label: 'Extensions & Plugins',
        icon: [
          'M17 14v6m-3-3h6M6 10h2a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v2a2 2 0 002 2zm10 0h2a2 2 0 002-2V6a2 2 0 00-2-2h-2a2 2 0 00-2 2v2a2 2 0 002 2zM6 20h2a2 2 0 002-2v-2a2 2 0 00-2-2H6a2 2 0 00-2 2v2a2 2 0 002 2z',
        ],
      },
      {
        href: '/advanced/replication',
        label: 'Replication & HA',
        icon: ['M8 7h12m0 0l-4-4m4 4l-4 4m0 6H4m0 0l4 4m-4-4l4-4'],
      },
      {
        href: '/advanced/advanced-queries',
        label: 'Advanced Queries',
        icon: [
          'M9.663 17h4.673M12 3v1m6.364 1.636l-.707.707M21 12h-1M4 12H3m3.343-5.657l-.707-.707m2.828 9.9a5 5 0 117.072 0l-.548.547A3.374 3.374 0 0014 18.469V19a2 2 0 11-4 0v-.531c0-.895-.356-1.754-.988-2.386l-.548-.547z',
        ],
      },
    ],
  },
  {
    title: 'CLI',
    items: [
      {
        href: '/cli/introspection',
        label: 'Introspection',
        icon: [
          'M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4m0 5c0 2.21-3.582 4-8 4s-8-1.79-8-4',
        ],
      },
    ],
  },
  {
    title: 'Examples',
    items: [{ href: '/examples', label: 'Code Examples', icon: flaskIcon }],
  },
];
