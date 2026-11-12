<script lang="ts">
  import { onMount } from 'svelte';
  import MapMaker from './components/MapMaker.svelte';
  import Skirmish from './components/Skirmish.svelte';
  import type { EditorSession } from './lib/editor';
  import type { ScenarioDocument } from './lib/types';
  let screen: 'home' | 'options' | 'maker' | 'skirmish' = 'home';
  let editorSession: EditorSession | null = null;
  let initialDocument: ScenarioDocument | null = null;
  let showHp = true;
  let fullscreen = false;
  let error = '';
  async function toggleFullscreen() {
    error = '';
    try { if (document.fullscreenElement) await document.exitFullscreen(); else await document.documentElement.requestFullscreen(); }
    catch (e) { error = `Fullscreen unavailable: ${e instanceof Error ? e.message : String(e)}`; }
  }
  onMount(() => {
    const change = () => { fullscreen = !!document.fullscreenElement; };
    document.addEventListener('fullscreenchange', change);
    return () => document.removeEventListener('fullscreenchange', change);
  });
</script>

<svelte:head><title>2nd Hand Combat</title><meta name="description" content="A small tactical sandbox of autonomous infantry." /></svelte:head>
{#if screen === 'home'}
  <main class="landing">
    <div class="landing-top"><span>FIELD MANUAL / 01</span><span>INFANTRY SANDBOX</span></div>
    <div class="landing-content">
      <p class="eyebrow">A tactical sandbox</p><h1>2nd Hand<br/><em>Combat</em><span class="period">.</span></h1>
      <p class="intro">Set the doctrine. Place your soldiers. Watch the battle unfold.</p>
      <div class="menu"><button class="menu-main" onclick={() => { initialDocument = null; screen = 'skirmish'; }}><span>01</span><strong>Skirmish</strong><b>&rarr;</b></button><button onclick={() => screen = 'maker'}><span>02</span><strong>MapMaker</strong><b>&rarr;</b></button><button disabled><span>03</span><strong>Campaign</strong><small>COMING LATER</small></button><button disabled><span>04</span><strong>Puzzles</strong><small>COMING LATER</small></button><button onclick={() => { error = ''; screen = 'options'; }}><span>05</span><strong>Options</strong><b>&rarr;</b></button></div>
    </div><div class="landing-foot">A STUDY IN SMALL-UNIT TACTICS <span>&bull;</span> EST. 01</div>
  </main>
{:else if screen === 'options'}
  <main class="options-page"><header class="topbar"><button class="back" onclick={() => screen = 'home'}>&larr; Back</button><span>FIELD MANUAL / OPTIONS</span></header><section class="options-card"><p class="eyebrow">Preferences / 01</p><h1>Options<span class="period">.</span></h1><p>Make the battlefield your own.</p><div class="option-row"><div><strong>Fullscreen</strong><small>{fullscreen ? 'On' : 'Off'}</small></div><button onclick={toggleFullscreen}>{fullscreen ? 'Exit fullscreen' : 'Enter fullscreen'}</button></div><div class="option-row"><div><strong>Health indicators</strong><small>Show health above soldiers during battle</small></div><label class="check"><input type="checkbox" bind:checked={showHp}/> Show</label></div>{#if error}<p class="error" role="alert">{error}</p>{/if}</section></main>
{:else if screen === 'maker'}
  <MapMaker session={editorSession} onSession={(session) => editorSession = session} onTest={(document) => { initialDocument = structuredClone(document); screen = 'skirmish'; }} onBack={() => { editorSession = null; screen = 'home'; }} />
{:else}
  <Skirmish {showHp} {initialDocument} returnToMaker={editorSession !== null && initialDocument !== null} onBack={() => { screen = editorSession !== null && initialDocument !== null ? 'maker' : 'home'; initialDocument = null; }} />
{/if}
