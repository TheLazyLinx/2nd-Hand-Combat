import js from '@eslint/js';
import prettier from 'eslint-config-prettier';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';
import tseslint from 'typescript-eslint';

const WORKER_AND_WASM_PATTERNS = [
  {
    group: ['**/worker', '**/worker.ts', './worker', '../worker'],
    message: 'Do not import Worker internals directly; use BattleClient.',
  },
  {
    group: ['**/generated/wasm', '**/generated/wasm/*', '**/generated/wasm/**', '**/sim_wasm*'],
    message: 'Only apps/web/src/worker.ts may import generated WASM bindings.',
  },
];

export default tseslint.config(
  {
    ignores: [
      'dist/',
      'target/',
      '.tools/',
      'apps/web/src/generated/wasm/',
      'test-results/',
      'playwright-report/',
      'node_modules/',
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  ...svelte.configs.recommended,
  prettier,
  ...svelte.configs.prettier,
  {
    files: ['apps/web/src/**/*.{ts,svelte}'],
    ignores: ['apps/web/src/worker.ts'],
    languageOptions: {
      globals: {
        ...globals.browser,
      },
    },
  },
  {
    files: ['apps/web/src/**/*.svelte', 'apps/web/src/**/*.svelte.ts'],
    languageOptions: {
      parserOptions: {
        parser: tseslint.parser,
      },
    },
  },
  {
    files: ['apps/web/src/worker.ts'],
    languageOptions: {
      globals: {
        ...globals.worker,
      },
    },
    rules: {
      'no-restricted-imports': [
        'error',
        {
          paths: [
            {
              name: 'pixi.js',
              message: 'The simulation Worker must not depend on rendering.',
            },
            {
              name: 'svelte',
              message: 'The simulation Worker must not depend on Svelte.',
            },
          ],
          patterns: [
            {
              group: ['./components/*', './components/**', './lib/battlefield*', './lib/client*'],
              message: 'The simulation Worker must not import UI, renderer, or client modules.',
            },
          ],
        },
      ],
    },
  },
  // Boundary: only worker.ts may import WASM bindings; only BattleClient may instantiate Workers.
  {
    files: ['apps/web/src/**/*.{ts,svelte}'],
    ignores: ['apps/web/src/worker.ts'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          patterns: WORKER_AND_WASM_PATTERNS,
        },
      ],
    },
  },
  {
    files: ['apps/web/src/**/*.{ts,svelte}'],
    ignores: ['apps/web/src/lib/client.ts'],
    rules: {
      'no-restricted-syntax': [
        'error',
        {
          selector: 'NewExpression[callee.name="Worker"]',
          message:
            'Instantiate simulation Workers only inside BattleClient (apps/web/src/lib/client.ts).',
        },
      ],
    },
  },
  // Boundary: renderer displays snapshots only and never talks directly to BattleClient or UI components.
  {
    files: ['apps/web/src/lib/battlefield.ts'],
    rules: {
      'no-restricted-imports': [
        'error',
        {
          paths: [
            {
              name: 'svelte',
              message: 'The PixiJS renderer must stay independent of Svelte.',
            },
          ],
          patterns: [
            ...WORKER_AND_WASM_PATTERNS,
            {
              group: ['./client', './client.ts', '../components/*', '../components/**'],
              message:
                'The renderer receives snapshots from callers and must not import BattleClient or UI components.',
            },
          ],
        },
      ],
    },
  },
  {
    files: [
      'scripts/**/*.{js,mjs}',
      'tests/*.test.mjs',
      'playwright.config.ts',
      'apps/web/vite.config.ts',
      'eslint.config.js',
    ],
    languageOptions: {
      globals: {
        ...globals.node,
      },
    },
  },
  {
    files: ['tests/browser/**/*.ts'],
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
  },
);
