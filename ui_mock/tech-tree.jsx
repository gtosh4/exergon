// tech-tree.jsx — exergon/ui · procedural tech tree wireframes
//
// Five variations on a "fog-of-war" tech tree. All share:
//   - the same fictitious tech graph (TT data model)
//   - three knowledge tiers per node:
//       T1  KNOWN     — node exists, params hidden       (silhouette / redacted)
//       T2  PARTIAL   — broad params visible, ranges     (some text, some bars)
//       T3  REVEALED  — full recipe, buildable           (numbers, glyphs)
//   - milestone nodes that gate tiers
//   - a shared reveal overlay (variation 06)
//
// Tweaks read from window.__ttTweaks: density, vibe, fogStyle, milestoneStyle,
// showLockedEdges.

// ════════════════════════════════════════════════════════════════════════════
// DATA — fictitious "exergon" tech graph
// ════════════════════════════════════════════════════════════════════════════
const TT = (()=>{
  // tech glyphs are stylised, not drawn from any IRL franchise
  const techs = [
    // tier 0 — starter
    { id:"basics",    tier:0, name:"manual basics",   tag:"craft",   ms:false, glyph:"⌬" },
    { id:"smelt-i",   tier:0, name:"hearth smelting", tag:"smelt",   ms:false, glyph:"▣" },
    { id:"sift",      tier:0, name:"sieving",         tag:"refine",  ms:false, glyph:"⋮⋮" },

    // tier 1 — gate "industrial age"
    { id:"steam",     tier:1, name:"steam vessel",    tag:"power",   ms:true,  glyph:"≋" },
    { id:"crusher",   tier:1, name:"jaw crusher",     tag:"refine",  ms:false, glyph:"◇" },
    { id:"smelt-ii",  tier:1, name:"reverberatory",   tag:"smelt",   ms:false, glyph:"▤" },
    { id:"flux",      tier:1, name:"flux fluid",      tag:"chem",    ms:false, glyph:"≈" },
    { id:"pumps",     tier:1, name:"piston pumps",    tag:"chem",    ms:false, glyph:"◍" },

    // tier 2 — gate "electric age"
    { id:"dynamo",    tier:2, name:"dynamo array",    tag:"power",   ms:true,  glyph:"⚡" },
    { id:"coil-i",    tier:2, name:"copper coils",    tag:"electric",ms:false, glyph:"〰" },
    { id:"smelt-iii", tier:2, name:"bessemer line",   tag:"smelt",   ms:false, glyph:"▥" },
    { id:"chem-i",    tier:2, name:"reagent flask",   tag:"chem",    ms:false, glyph:"⌽" },
    { id:"motor-i",   tier:2, name:"servo motor",     tag:"electric",ms:false, glyph:"◉" },
    { id:"plate",     tier:2, name:"ferro-laminate",  tag:"smelt",   ms:false, glyph:"▨" },

    // tier 3 — gate "logic age"
    { id:"logic",     tier:3, name:"logic substrate", tag:"logic",   ms:true,  glyph:"▦" },
    { id:"sand",      tier:3, name:"silica wafer",    tag:"refine",  ms:false, glyph:"◊" },
    { id:"chip-i",    tier:3, name:"control chip",    tag:"logic",   ms:false, glyph:"▦" },
    { id:"chem-ii",   tier:3, name:"catalytic bed",   tag:"chem",    ms:false, glyph:"◎" },
    { id:"motor-ii",  tier:3, name:"high-torque",     tag:"electric",ms:false, glyph:"✦" },

    // tier 4 — gate "exotic age"
    { id:"exotic",    tier:4, name:"exergon core",    tag:"power",   ms:true,  glyph:"✺" },
    { id:"plasma",    tier:4, name:"plasma loom",     tag:"chem",    ms:false, glyph:"☄" },
    { id:"mind",      tier:4, name:"mind-link",       tag:"logic",   ms:false, glyph:"⌖" },
  ];
  // edges (prerequisite → unlocks)
  const edges = [
    ["basics","crusher"], ["basics","smelt-i"],
    ["smelt-i","smelt-ii"], ["sift","crusher"], ["sift","flux"],
    ["smelt-ii","steam"], ["crusher","steam"], ["flux","steam"],
    ["smelt-ii","pumps"], ["pumps","steam"],
    ["steam","dynamo"], ["steam","coil-i"], ["steam","smelt-iii"],
    ["coil-i","dynamo"], ["smelt-iii","dynamo"],
    ["pumps","chem-i"], ["coil-i","motor-i"],
    ["smelt-iii","plate"], ["chem-i","plate"],
    ["dynamo","logic"], ["motor-i","logic"], ["plate","logic"],
    ["chem-i","sand"], ["sand","chip-i"], ["logic","chip-i"],
    ["chem-i","chem-ii"], ["motor-i","motor-ii"], ["motor-ii","logic"],
    ["chip-i","exotic"], ["chem-ii","exotic"], ["motor-ii","exotic"],
    ["exotic","plasma"], ["exotic","mind"],
  ];
  // simulated "current run" knowledge state
  const knowledge = {
    basics:3, "smelt-i":3, sift:3,
    steam:3, crusher:3, "smelt-ii":3, flux:2, pumps:2,
    dynamo:2, "coil-i":2, "smelt-iii":2, "chem-i":3, "motor-i":2, plate:2,
    logic:1, sand:1, "chip-i":1, "chem-ii":1, "motor-ii":1,
    exotic:1, plasma:0, mind:0,
  };
  // wishlist (player marked these for next reveal)
  const wishlist = new Set(["chip-i","sand"]);

  // research economy — for the reveal panel
  const cost = (t, toTier) => {
    // higher tier costs more; revealing to T3 always more than T2
    const base = { 0:5, 1:12, 2:30, 3:75, 4:160 }[t.tier] || 10;
    return toTier === 3 ? Math.round(base * 2.2) : base;
  };

  const byId = Object.fromEntries(techs.map(t=>[t.id,t]));
  return { techs, edges, knowledge, wishlist, cost, byId };
})();

// ════════════════════════════════════════════════════════════════════════════
// SHARED HELPERS
// ════════════════════════════════════════════════════════════════════════════
function ttTweaks(){ return (typeof window!=="undefined" && window.__ttTweaks) || {}; }

// Render a node's name/glyph with the right fog-of-war treatment for its tier.
function FogText({ tier, fogStyle, glyph, name, short=false }){
  // tier 3 — full reveal
  if (tier === 3){
    return (
      <span style={{ display:"inline-flex", alignItems:"center", gap:6 }}>
        <span style={{ fontFamily:"var(--font-hand)", fontSize:14, lineHeight:1 }}>{glyph}</span>
        <span>{name}</span>
      </span>
    );
  }
  // tier 2 — partial: name visible, params hidden elsewhere
  if (tier === 2){
    return (
      <span style={{ display:"inline-flex", alignItems:"center", gap:6 }}>
        <span style={{ fontFamily:"var(--font-hand)", fontSize:14, lineHeight:1, opacity:0.85 }}>{glyph}</span>
        <span style={{ opacity: 0.85 }}>{name}</span>
      </span>
    );
  }
  // tier 1 — known to exist; the fog style controls how we mask it
  switch (fogStyle){
    case "redact": return (
      <span style={{ display:"inline-flex", alignItems:"center", gap:6 }}>
        <span className="tt-redact" style={{ width: short?16:18 }}/>
        <span className="tt-redact" style={{ width: short?40:64 }}/>
      </span>
    );
    case "sketchy": return (
      <span style={{ display:"inline-flex", alignItems:"center", gap:6, fontFamily:"var(--font-hand)", fontSize:13, color:"var(--ink-faint)" }}>
        <span>?</span><span>{"~".repeat(short?5:8)}</span>
      </span>
    );
    case "polaroid": return (
      <span style={{ display:"inline-flex", alignItems:"center", gap:6, opacity:0.35, filter:"blur(0.6px)" }}>
        <span>{glyph}</span>
        <span>{name.replace(/[a-z]/gi,"·")}</span>
      </span>
    );
    case "microfilm": return (
      <span style={{ display:"inline-flex", alignItems:"center", gap:6, color:"var(--ink-faint)" }}>
        <span style={{ fontFamily:"var(--font-mono)" }}>fragment</span>
        <span style={{ fontFamily:"var(--font-mono)", fontSize:9 }}>#{Math.abs(hash(name))%9999}</span>
      </span>
    );
    case "silhouette":
    default: return (
      <span style={{ display:"inline-flex", alignItems:"center", gap:6 }}>
        <span className="tt-fog-glyph">{glyph}</span>
        <span className="tt-fog-glyph tt-fog-soft">{name}</span>
      </span>
    );
  }
}

// tiny string-hash for stable mock IDs
function hash(s){ let h=0; for(let i=0;i<s.length;i++) h=((h<<5)-h+s.charCodeAt(i))|0; return h; }

function ttClass(tech, kn){
  const t = kn[tech.id] ?? 0;
  return `tt-t${Math.max(t,1)}` + (tech.ms?" tt-milestone":"") + (TT.wishlist.has(tech.id)?" tt-wishlist":"");
}

// pretty range for partial info
function ttRange(rate){
  const lo = Math.max(1, Math.round(rate*0.6));
  const hi = Math.round(rate*1.4);
  return `~${lo}–${hi}`;
}


// ════════════════════════════════════════════════════════════════════════════
// 00 · NORTH STAR — reading guide (V6-only)
// ════════════════════════════════════════════════════════════════════════════
function TTNorthStar(){
  return (
    <div className="paper" style={{ height:"100%", padding:24, display:"flex", flexDirection:"column", gap:18 }}>
      <div>
        <div className="sk-h">tech tree — tier-paged questbook</div>
        <div className="sk-mono-sm" style={{ color:"var(--ink-soft)", marginTop:6, maxWidth:820 }}>
          a procedural run gives the player ~120 nodes split across five tiers. each starts as a <span className="sk-squig">silhouette</span>,
          becomes <span className="sk-squig">partial</span> with rough numbers, then <span className="sk-squig">fully revealed</span> and buildable.
          the tree reads as a <b>questbook</b>: each tier is its own page · subway-style research lines colour the within-tier flow ·
          milestones bridge adjacent pages · cross-tier dependencies become labeled port stubs at the page margins.
        </div>
      </div>
      <div className="sk-div"/>
      <div style={{ display:"grid", gridTemplateColumns:"repeat(3, 1fr)", gap:14, flex:1 }}>
        {[
          { tag:"PAGES",      t:"tier = page",       blurb:"T0…T4 each their own tab. a player on T2 sees only the electric age stratum — no scrolling past 120 nodes." },
          { tag:"BRIDGES",    t:"milestones span",   blurb:"the milestone gating into a tier is the right-edge card on the previous page AND the left-edge card on the next. always know where you came from." },
          { tag:"LINES",      t:"research colours",  blurb:"smelt · refine · chem · electric · logic · power. each tag is a swim-lane with a colour. within-line edges use that colour; cross-line edges go dashed." },
          { tag:"PORTS",      t:"cross-tier stubs",  blurb:"a node on T2 that depends on something from T1 shows a coloured stub at the left margin labelled with the source · click to jump pages." },
          { tag:"FOG",        t:"3 knowledge tiers", blurb:"T1 known (silhouette) · T2 partial (ranges) · T3 revealed (buildable). pick the fog metaphor in tweaks." },
          { tag:"REVEAL",     t:"the action",        blurb:"clicking any node opens the shared reveal panel · cost · tier ladder · prereq chain · before/after diff." },
        ].map(c=>(
          <div key={c.tag} className="sk-box" style={{ padding:14, display:"flex", flexDirection:"column", gap:8 }}>
            <div style={{ display:"flex", alignItems:"center", gap:8 }}>
              <span className="sk-tag sk-on">{c.tag}</span>
              <span className="sk-h sk-h-sm">{c.t}</span>
            </div>
            <div className="sk-mono-sm" style={{ lineHeight:1.55, color:"var(--ink-soft)" }}>
              {c.blurb}
            </div>
          </div>
        ))}
      </div>
      <div className="sk-mono-xs" style={{ color:"var(--ink-faint)" }}>
        always-on: search (tag · tier · "reveals X") · wishlist stars · click any node for the reveal panel · locked edges optional via tweak.
      </div>
    </div>
  );
}

// ════════════════════════════════════════════════════════════════════════════
// SHARED CHROME — topbar, search, sidebar, etc.
// ════════════════════════════════════════════════════════════════════════════
function TTTopbar({ mode, research=128, frontier="exergon core" }){
  return (
    <div style={{
      borderBottom:"1.5px solid var(--ink)", padding:"6px 12px",
      display:"flex", alignItems:"center", gap:10, flexShrink:0, background:"var(--paper)"
    }}>
      <span className="sk-tag sk-on">{mode}</span>
      <span className="sk-mono" style={{ color:"var(--ink-soft)" }}>research:</span>
      <span className="sk-tag sk-accent">{research} R</span>
      <span className="sk-mono-sm" style={{ color:"var(--ink-soft)" }}>frontier · {frontier}</span>
      <div style={{ flex:1 }}/>
      <button className="sk-btn">search</button>
      <button className="sk-btn">wishlist (2)</button>
      <button className="sk-btn">filter</button>
      <button className="sk-btn sk-on">reveal queue</button>
    </div>
  );
}

function TTSearchBar(){
  return (
    <div style={{ display:"flex", alignItems:"center", gap:6, padding:8, borderBottom:"1.5px dashed var(--ink-soft)" }}>
      <span className="sk-mono-xs" style={{ color:"var(--ink-soft)" }}>FIND</span>
      <input className="tt-search" placeholder="tag:smelt · tier:3 · reveals:plate · or any name…"/>
      <span className="tt-chip tt-on">smelt</span>
      <span className="tt-chip">electric</span>
      <span className="tt-chip">chem</span>
      <span className="tt-chip">logic</span>
      <span className="tt-chip">power</span>
      <span className="tt-chip tt-accent">unlocks recipe</span>
    </div>
  );
}

function TTLeftRail(){
  return (
    <div style={{
      borderRight:"1.5px solid var(--ink)", padding:"8px 4px",
      display:"flex", flexDirection:"column", alignItems:"center", gap:8,
      background:"var(--paper-2)"
    }}>
      {["⌂","⚙","⌬","▦","✺","⋯"].map((g,i)=>(
        <div key={i} className="sk-box" style={{ width:36, height:36, display:"flex", alignItems:"center", justifyContent:"center" }}>
          <span style={{ fontFamily:"var(--font-hand)", fontSize:18 }}>{g}</span>
        </div>
      ))}
    </div>
  );
}

function TTRightRail({ children }){
  return (
    <div style={{ borderLeft:"1.5px solid var(--ink)", padding:"10px 12px", background:"var(--paper)", overflow:"auto" }}>
      {children}
    </div>
  );
}

// inspector — the right rail content; shows what the player knows about a tech.
function TTInspector({ tech, knTier }){
  const t = tech;
  const fogStyle = ttTweaks().fogStyle || "silhouette";
  return (
    <div style={{ display:"flex", flexDirection:"column", gap:10 }}>
      <div className="sk-mono-xs" style={{ color:"var(--ink-faint)", textTransform:"uppercase", letterSpacing:0.6 }}>
        selected · tier {t.tier} · knowledge T{knTier}
      </div>
      <div className="sk-h sk-h-sm">
        <FogText tier={knTier} fogStyle={fogStyle} glyph={t.glyph} name={t.name}/>
      </div>
      <div className="sk-div"/>
      <div className="sk-mono-xs" style={{ color:"var(--ink-soft)" }}>tag</div>
      <div><span className="tt-chip tt-on">{t.tag}</span> {t.ms && <span className="tt-chip tt-accent">milestone</span>}</div>

      <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:6 }}>inputs</div>
      <div style={{ display:"flex", gap:4 }}>
        {[0,1,2].map(i=>{
          if (knTier === 3) return <div key={i} className="sk-slot sk-filled"><span className="sk-icon">{["◇","≈","◍"][i]}</span><span className="sk-qty">{[2,1,3][i]}</span></div>;
          if (knTier === 2) return <div key={i} className="sk-slot"><span className="sk-icon" style={{ opacity:0.4 }}>?</span></div>;
          return <div key={i} className="sk-slot" style={{ background:"repeating-linear-gradient(135deg, var(--paper) 0 4px, var(--paper-2) 4px 8px)", borderStyle:"dashed" }}/>;
        })}
      </div>

      <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:6 }}>output rate</div>
      {knTier === 3 && <div className="sk-mono">12.0/s · ferro-laminate</div>}
      {knTier === 2 && <div className="sk-mono" style={{ color:"var(--ink-soft)" }}>{ttRange(12)}/s · plate-class</div>}
      {knTier === 1 && <div><span className="tt-redact" style={{ width: 110 }}/></div>}

      <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:6 }}>flavour</div>
      <div className="sk-mono-sm" style={{ color:"var(--ink-soft)", lineHeight:1.5 }}>
        {knTier >= 2
          ? "rolled and re-rolled, the laminate holds form even when red-hot."
          : "scattered references in the field journal."}
      </div>

      <div className="sk-div" style={{ marginTop:10 }}/>
      <button className="sk-btn sk-accent" style={{ justifyContent:"center" }}>
        reveal → T{Math.min(3, knTier+1)} · {TT.cost(t, knTier+1)} R
      </button>
      <button className="sk-btn" style={{ justifyContent:"center" }}>
        ★ {TT.wishlist.has(t.id) ? "wishlisted" : "add to wishlist"}
      </button>
    </div>
  );
}


// ════════════════════════════════════════════════════════════════════════════
// 06 · REVEAL OVERLAY — shared modal for tier 1 → 2 → 3
// ════════════════════════════════════════════════════════════════════════════
function TTRevealOverlay(){
  // we draw the constellation behind, dimmed, with the modal floating
  return (
    <div className="paper" style={{ height:"100%", position:"relative" }}>
      {/* faint paper backdrop */}
      <div style={{ position:"absolute", inset:0, opacity:0.35, pointerEvents:"none", background:"repeating-linear-gradient(135deg, var(--paper) 0 6px, var(--paper-2) 6px 8px)" }}/>
      {/* modal */}
      <div style={{
        position:"absolute", inset:0, background:"rgba(26,26,26,0.18)",
        display:"flex", alignItems:"center", justifyContent:"center"
      }}>
        <div className="sk-box sk-thick" style={{ width:1100, height:740, padding:0, display:"grid", gridTemplateColumns:"380px 1fr", background:"var(--paper)" }}>
          {/* LEFT — the focus card */}
          <div style={{ borderRight:"1.5px solid var(--ink)", padding:18, display:"flex", flexDirection:"column", gap:10 }}>
            <div style={{ display:"flex", justifyContent:"space-between", alignItems:"center" }}>
              <span className="sk-tag sk-on">REVEAL</span>
              <span className="sk-mono-xs" style={{ color:"var(--ink-faint)" }}>esc · close</span>
            </div>

            <div className="sk-box" style={{ padding:14, background:"var(--paper)" }}>
              <div className="sk-mono-xs" style={{ color:"var(--ink-faint)" }}>FILE · T3 · #4821</div>
              <div className="sk-h sk-h-sm" style={{ marginTop:4 }}>
                <span style={{ fontFamily:"var(--font-hand)", fontSize:18, marginRight:6 }}>▦</span>
                control chip
              </div>
              <div style={{ display:"flex", gap:6, marginTop:6 }}>
                <span className="tt-chip">logic</span>
                <span className="tt-chip">recipe</span>
                <span className="tt-chip tt-accent">★ wishlist</span>
              </div>
            </div>

            {/* tier ladder */}
            <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:6 }}>knowledge ladder</div>
            <div style={{ display:"flex", flexDirection:"column", gap:8 }}>
              <TierRow tier={1} state="done"
                cost="—"
                title="known to exist"
                blurb="appears on tree, no params."/>
              <TierRow tier={2} state="done"
                cost="30 R"
                title="partial revealed"
                blurb="approx inputs, output range, machine class."/>
              <TierRow tier={3} state="next"
                cost="75 R"
                title="fully revealed"
                blurb="exact recipe, all params, buildable."
                accent/>
            </div>

            <div className="sk-div"/>
            <button className="sk-btn sk-accent" style={{ justifyContent:"center", padding:"8px 12px" }}>
              <span style={{ fontFamily:"var(--font-hand)", fontSize:18 }}>↧</span> REVEAL → T3 · 75 R
            </button>
            <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", textAlign:"center" }}>
              you have <b>128 R</b> · 53 R remaining after
            </div>
            <button className="sk-btn" style={{ justifyContent:"center", marginTop:4 }}>
              ★ keep on wishlist · queue for next
            </button>
          </div>

          {/* RIGHT — what reveal will give you */}
          <div style={{ padding:18, display:"flex", flexDirection:"column", gap:14, overflow:"hidden" }}>
            <div className="sk-h">what you'll learn</div>
            <div className="sk-div"/>

            <div style={{ display:"grid", gridTemplateColumns:"1fr 1fr", gap:12 }}>
              {/* before */}
              <div className="sk-box sk-dashed" style={{ padding:12 }}>
                <div className="sk-mono-xs" style={{ color:"var(--ink-faint)" }}>BEFORE · T2 partial</div>
                <div className="sk-mono-sm" style={{ marginTop:8, lineHeight:1.7 }}>
                  inputs   · <span className="tt-redact" style={{ width:60 }}/> + <span className="tt-redact" style={{ width:32 }}/><br/>
                  output   · ~7–17/s plate-class<br/>
                  machine  · bench-class T2<br/>
                  modules  · <span className="tt-redact" style={{ width:24 }}/> slots<br/>
                  flavour  · scattered references…
                </div>
              </div>
              {/* after */}
              <div className="sk-box sk-thick" style={{ padding:12 }}>
                <div className="sk-mono-xs">AFTER · T3 revealed</div>
                <div className="sk-mono-sm" style={{ marginTop:8, lineHeight:1.7 }}>
                  inputs   · 2× silica wafer · 1× copper wire<br/>
                  output   · 12.0/s control chip<br/>
                  machine  · assembly bench T3<br/>
                  modules  · 2 slots (P/S compatible)<br/>
                  flavour  · "the pattern is the thing."
                </div>
              </div>
            </div>

            {/* prereq chain */}
            <div className="sk-h sk-h-sm" style={{ marginTop:6 }}>prerequisite chain</div>
            <div style={{
              display:"flex", alignItems:"center", gap:6, padding:10,
              border:"1.5px solid var(--ink)", background:"var(--paper-2)", overflowX:"auto"
            }}>
              <ChainCard tier="T0" name="hearth smelt" state={3}/>
              <span className="sk-arrow">→</span>
              <ChainCard tier="T1" name="reverberatory" state={3}/>
              <span className="sk-arrow">→</span>
              <ChainCard tier="T1" name="steam vessel" state={3} ms/>
              <span className="sk-arrow">→</span>
              <ChainCard tier="T2" name="dynamo array" state={2} ms/>
              <span className="sk-arrow">→</span>
              <ChainCard tier="T3" name="silica wafer" state={1}/>
              <span className="sk-arrow">→</span>
              <ChainCard tier="T3" name="control chip" state={2} self/>
            </div>
            <div className="sk-mono-xs" style={{ color:"var(--ink-soft)" }}>
              chain shows current knowledge · click any link to focus that node and reveal it instead.
            </div>

            <div className="sk-h sk-h-sm" style={{ marginTop:6 }}>also unlocked when revealed</div>
            <div style={{ display:"grid", gridTemplateColumns:"repeat(3, 1fr)", gap:8 }}>
              {[
                ["recipe","control chip recipe","2× silica · 1× wire → 1× chip"],
                ["item","control chip","item slot in inventory codex"],
                ["machine","assembly bench T3","new placeable in the build menu"],
                ["edge","→ exergon core","prereq edge to milestone T4"],
              ].map((row,i)=>(
                <div key={i} className="sk-box" style={{ padding:8, fontSize:10, lineHeight:1.4 }}>
                  <div style={{ display:"flex", justifyContent:"space-between" }}>
                    <span className="tt-chip tt-on">{row[0]}</span>
                    <span style={{ fontFamily:"var(--font-hand)", fontSize:14 }}>+</span>
                  </div>
                  <div style={{ marginTop:4, fontWeight:600 }}>{row[1]}</div>
                  <div style={{ marginTop:2, color:"var(--ink-soft)" }}>{row[2]}</div>
                </div>
              ))}
            </div>

            <div className="sk-annot" style={{ position:"static", marginTop:6, color:"var(--ink-soft)" }}>
              this same panel handles T1→T2 and T2→T3 · the "next" row is the live action
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// little helper components for the reveal panel
function TierRow({ tier, state, cost, title, blurb, accent=false }){
  // state: done | next | locked
  return (
    <div className="sk-box" style={{
      padding:8, display:"grid", gridTemplateColumns:"40px 1fr auto", gap:8, alignItems:"center",
      background: accent ? "var(--accent)" : (state==="done" ? "var(--paper-2)" : "var(--paper)"),
      borderStyle: state==="locked" ? "dashed" : "solid",
      opacity: state==="locked" ? 0.55 : 1,
    }}>
      <div className="sk-h" style={{ textAlign:"center" }}>T{tier}</div>
      <div>
        <div className="sk-mono" style={{ fontWeight:600 }}>{title}</div>
        <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:2 }}>{blurb}</div>
      </div>
      <div className="sk-mono-xs" style={{ textAlign:"right" }}>
        <div style={{ fontWeight:700 }}>{cost}</div>
        <div style={{ color:"var(--ink-soft)" }}>{state==="done"?"✓ owned":state==="next"?"available":"locked"}</div>
      </div>
    </div>
  );
}
function ChainCard({ tier, name, state, ms=false, self=false }){
  return (
    <div className="sk-box" style={{
      padding:6, minWidth:120, position:"relative",
      background: ms ? "var(--accent)" : "var(--paper)",
      outline: self ? "2px dashed var(--ink)" : "none",
      outlineOffset: 2,
      borderStyle: state===1 ? "dashed" : "solid"
    }}>
      <div className="sk-mono-xs" style={{ color:"var(--ink-faint)" }}>{tier} {ms?"· MS":""}</div>
      <div className="sk-mono" style={{ marginTop:2, fontWeight:600,
        opacity: state===1 ? 0.5 : 1
      }}>
        {state===1 ? <span className="tt-redact" style={{ width:80 }}/> : name}
      </div>
      <div className="sk-mono-xs" style={{ marginTop:2, color:"var(--ink-soft)" }}>
        {state===3?"●revealed":state===2?"~partial":"?known"}
      </div>
    </div>
  );
}