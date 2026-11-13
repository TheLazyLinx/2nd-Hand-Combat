<script lang="ts">
  import type { Catalog, Doctrine, UnitPlacement } from '../lib/types';
  export let unit: UnitPlacement;
  export let busy: boolean;
  export let catalog: Catalog;
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
  <details>
    <summary>Grenades</summary>
    <label for="grenade-policy">Grenade policy</label>
    <select id="grenade-policy" data-testid="grenade-policy" disabled={busy} value={unit.doctrine.grenades.policy} onchange={(e) => onChange('grenades', {...unit.doctrine.grenades, policy: e.currentTarget.value as Doctrine['grenades']['policy']})}><option value="never">Never</option><option value="clusters">Enemy clusters</option><option value="anySafeTarget">Any safe target</option><option value="emptyRifleOnly">Only when rifle is empty</option></select>
    {#if unit.doctrine.grenades.policy !== 'never'}
      <label for="grenade-distance">Maximum throw distance (m)</label><input id="grenade-distance" data-testid="grenade-distance" type="number" {...catalog.doctrineRules.grenadeMaxDistance} disabled={busy} value={unit.doctrine.grenades.maxDistance} oninput={(e) => onChange('grenades', {...unit.doctrine.grenades, maxDistance: Number(e.currentTarget.value)})}/>
      {#if unit.doctrine.grenades.policy === 'clusters'}<label for="grenade-enemies">Minimum enemies in blast</label><input id="grenade-enemies" data-testid="grenade-enemies" type="number" {...catalog.doctrineRules.grenadeMinEnemies} disabled={busy} value={unit.doctrine.grenades.minEnemies} oninput={(e) => onChange('grenades', {...unit.doctrine.grenades, minEnemies: Number(e.currentTarget.value)})}/>{/if}
      <label for="grenade-safety">Friendly safety margin (m)</label><input id="grenade-safety" data-testid="grenade-safety" type="number" {...catalog.doctrineRules.grenadeSafetyMargin} disabled={busy} value={unit.doctrine.grenades.safetyMargin} oninput={(e) => onChange('grenades', {...unit.doctrine.grenades, safetyMargin: Number(e.currentTarget.value)})}/>
    {/if}
  </details>
  <details>
    <summary>Medical supplies</summary>
    <label for="medical-policy">Medical policy</label>
    <select id="medical-policy" data-testid="medical-policy" disabled={busy} value={unit.doctrine.medical.policy} onchange={(e) => onChange('medical', {...unit.doctrine.medical, policy: e.currentTarget.value as Doctrine['medical']['policy']})}><option value="never">Never</option><option value="outOfContact">When out of contact</option><option value="lowHealth">At low health</option></select>
    {#if unit.doctrine.medical.policy !== 'never'}
      <label for="medical-hp">Use at or below HP (%)</label><input id="medical-hp" data-testid="medical-hp" type="number" {...catalog.doctrineRules.medicalHpThreshold} disabled={busy} value={unit.doctrine.medical.hpThreshold} oninput={(e) => onChange('medical', {...unit.doctrine.medical, hpThreshold: Number(e.currentTarget.value)})}/>
      {#if unit.doctrine.medical.policy === 'outOfContact'}<label for="medical-safe">Time out of contact (seconds)</label><input id="medical-safe" data-testid="medical-safe" type="number" {...catalog.doctrineRules.medicalSafeSeconds} disabled={busy} value={unit.doctrine.medical.safeSeconds} oninput={(e) => onChange('medical', {...unit.doctrine.medical, safeSeconds: Number(e.currentTarget.value)})}/>{/if}
      <label for="medical-cooldown">Time between uses (seconds)</label><input id="medical-cooldown" data-testid="medical-cooldown" type="number" {...catalog.doctrineRules.medicalCooldownSeconds} disabled={busy} value={unit.doctrine.medical.cooldownSeconds} oninput={(e) => onChange('medical', {...unit.doctrine.medical, cooldownSeconds: Number(e.currentTarget.value)})}/>
    {/if}
  </details>
  <label for="ammo-behavior">When rifle ammunition runs out</label><select id="ammo-behavior" data-testid="ammo-behavior" disabled={busy} value={unit.doctrine.outOfAmmoBehavior} onchange={(e) => onChange('outOfAmmoBehavior', e.currentTarget.value as Doctrine['outOfAmmoBehavior'])}><option value="withdraw">Withdraw</option><option value="continueDoctrine">Continue doctrine</option></select>
  <button class="delete" disabled={busy} onclick={onRemove}>Remove soldier</button>
</section>
