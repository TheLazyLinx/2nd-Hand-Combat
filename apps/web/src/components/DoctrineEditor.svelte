<script lang="ts">
  import type { Doctrine, UnitPlacement } from '../lib/types';
  export let unit: UnitPlacement;
  export let busy: boolean;
  export let onClose: () => void;
  export let onChange: (field: keyof Doctrine, value: Doctrine[keyof Doctrine]) => void;
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
  <label for="position-preference">Position preference</label>
  <select id="position-preference" data-testid="position-preference" disabled={busy} value={unit.doctrine.positionPreference} onchange={(e) => onChange('positionPreference', e.currentTarget.value as Doctrine['positionPreference'])}><option value="balanced">Balanced</option><option value="cover">Cover</option><option value="concealment">Concealment</option></select>
  <label for="route-preference">Route preference</label>
  <select id="route-preference" data-testid="route-preference" disabled={busy} value={unit.doctrine.routePreference} onchange={(e) => onChange('routePreference', e.currentTarget.value as Doctrine['routePreference'])}><option value="fastest">Fastest</option><option value="protected">Protected</option></select>
  <button class="delete" disabled={busy} onclick={onRemove}>Remove soldier</button>
</section>
