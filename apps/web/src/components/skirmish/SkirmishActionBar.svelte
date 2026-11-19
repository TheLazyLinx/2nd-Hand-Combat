<script lang="ts">
  /** Skirmish battle lifecycle action bar (Start, Pause, Resume, End battle, Reset). */
  interface Props {
    phase: 'loading' | 'setup' | 'running' | 'paused' | 'finished' | 'error';
    busy: boolean;
    canStart: boolean;
    hasStarting: boolean;
    onStart: () => void;
    onPause: () => void;
    onResume: () => void;
    onEndBattle: () => void;
    onReset: () => void;
  }

  let {
    phase,
    busy,
    canStart,
    hasStarting,
    onStart,
    onPause,
    onResume,
    onEndBattle,
    onReset,
  }: Props = $props();
</script>

<div class="action-bar">
  <div>
    <strong>
      {phase === 'setup'
        ? 'Prepare your forces'
        : phase === 'finished'
          ? 'Battle complete'
          : phase === 'paused'
            ? 'Battle paused'
            : 'Battle in progress'}
    </strong><small>
      {phase === 'setup'
        ? 'Place at least one soldier on each side.'
        : phase === 'paused'
          ? 'Resume when you are ready.'
          : 'Doctrine stays fixed during battle.'}
    </small>
  </div>
  <div class="actions">
    {#if phase === 'setup'}<button
        class="primary"
        data-testid="start-battle"
        disabled={busy || !canStart}
        onclick={onStart}
      >
        Start battle <span>&rarr;</span>
      </button>{:else if phase === 'running'}<button
        class="primary"
        data-testid="pause-battle"
        onclick={onPause}
      >
        Pause battle
      </button>{:else if phase === 'paused'}<button
        class="primary"
        data-testid="resume-battle"
        onclick={onResume}
      >
        Resume battle
      </button>{/if}{#if phase === 'running' || phase === 'paused'}<button
        data-testid="end-battle"
        onclick={onEndBattle}
      >
        End battle
      </button>{/if}{#if hasStarting && phase !== 'setup'}<button
        data-testid="reset-battle"
        onclick={onReset}
      >
        Reset
      </button>{/if}
  </div>
</div>
