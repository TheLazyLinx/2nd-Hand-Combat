<script lang="ts">
  import type { UnitPlacement } from '../lib/types';
  export let unit: UnitPlacement;
  export let busy: boolean;
  export let onClose: () => void;
  export let onChange: (field: 'engagementDistance' | 'fallbackHp', value: number) => void;
  export let onRemove: () => void;
</script>

<section class="editor" data-testid="selected-unit">
  <div class="editor-title">
    <div><p class="eyebrow">INDIVIDUAL DOCTRINE</p><h3>Infantry #{unit.id}</h3></div>
    <button class="text-button" onclick={onClose} aria-label="Close soldier details">&times;</button>
  </div>
  <label for="distance">Engagement distance <strong>{unit.doctrine.engagementDistance}</strong></label>
  <input id="distance" data-testid="engagement-distance" type="range" min="2" max="12" step="1" disabled={busy} value={unit.doctrine.engagementDistance} oninput={(e) => onChange('engagementDistance', Number(e.currentTarget.value))}/>
  <div class="range-labels"><span>CLOSE</span><span>FAR</span></div>
  <label for="fallback">Fall back at or below HP <strong>{unit.doctrine.fallbackHp}%</strong></label>
  <input id="fallback" data-testid="fallback-hp" type="range" min="0" max="100" step="1" disabled={busy} value={unit.doctrine.fallbackHp} oninput={(e) => onChange('fallbackHp', Number(e.currentTarget.value))}/>
  <div class="range-labels"><span>NEVER</span><span>EARLY</span></div>
  <button class="delete" disabled={busy} onclick={onRemove}>Remove soldier</button>
</section>
