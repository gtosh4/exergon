/* global React, Slot, Row, Col */

// =====================================================================
// LOOKUP VARIATION 1 — NEI sidebar (right rail, classic)
// =====================================================================
const LookupV1 = () => (
  <div className="paper" style={{padding:16, height:"100%", display:"flex", gap:10}}>
    {/* simulated inventory in background */}
    <Col gap={6} style={{flex:1, opacity:0.55}}>
      <span className="sk-h">INVENTORY (bg)</span>
      <div style={{display:"grid", gridTemplateColumns:"repeat(9,1fr)", gap:3}}>
        {Array.from({length:36}).map((_,i)=>(
          <Slot key={i} filled={i<14} icon={i<14?"◇":null}/>
        ))}
      </div>
      <span className="sk-mono-xs">[ player inventory area ]</span>
    </Col>
    {/* NEI rail */}
    <Col gap={6} style={{width: 240}}>
      <Row style={{justifyContent:"space-between", alignItems:"flex-end"}}>
        <span className="sk-h">CODEX</span>
        <Row gap={3}>
          <span className="sk-tag">«</span>
          <span className="sk-mono-xs">1/47</span>
          <span className="sk-tag">»</span>
        </Row>
      </Row>
      <div className="sk-box" style={{padding:"4px 8px"}}>
        <span className="sk-mono-sm">⌕ </span><span className="sk-squig sk-mono-sm">@mod:steel</span>
      </div>
      <Row gap={3} style={{flexWrap:"wrap"}}>
        <span className="sk-tag sk-on">all</span>
        <span className="sk-tag">ore</span>
        <span className="sk-tag">tool</span>
        <span className="sk-tag">comp</span>
        <span className="sk-tag">food</span>
        <span className="sk-tag">★ fav</span>
      </Row>
      <div className="sk-box" style={{padding:6, flex:1}}>
        <div style={{display:"grid", gridTemplateColumns:"repeat(5,1fr)", gap:3}}>
          {Array.from({length:60}).map((_,i)=>(
            <Slot key={i} filled
              active={i===7}
              icon={["▢","◇","⛏","✦","◉","◆","▣","▤","▥","◈","✕","▦","▧","▨"][i%14]}
            />
          ))}
        </div>
      </div>
      <Row style={{justifyContent:"space-between"}}>
        <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>R = recipe · U = uses</span>
        <span className="sk-mono-xs">page ≪ 1 2 3 ≫</span>
      </Row>
      <hr className="sk-div"/>
      {/* hover preview */}
      <div className="sk-box sk-thick" style={{padding:8}}>
        <Row gap={6}>
          <Slot style={{width:38,height:38}} filled icon="▤"/>
          <Col gap={0} style={{flex:1}}>
            <span className="sk-mono-sm" style={{fontWeight:700}}>steel.plate</span>
            <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>id #0124 · @basemetals</span>
          </Col>
        </Row>
        <hr className="sk-div" style={{margin:"6px 0"}}/>
        <span className="sk-mono-xs">refined plate of carbon steel.<br/>used in 47 recipes.</span>
        <Row gap={4} style={{marginTop:6}}>
          <button className="sk-btn" style={{padding:"2px 6px"}}>R recipe</button>
          <button className="sk-btn" style={{padding:"2px 6px"}}>U uses</button>
          <button className="sk-btn" style={{padding:"2px 6px"}}>★</button>
        </Row>
      </div>
    </Col>
  </div>
);

// =====================================================================
// LOOKUP VARIATION 2 — Recipe deep-dive (3-pane: list / recipe / uses)
// =====================================================================
const LookupV2 = () => (
  <div className="paper" style={{padding:16, height:"100%", display:"flex", flexDirection:"column", gap:10}}>
    <Row style={{justifyContent:"space-between", alignItems:"flex-end"}}>
      <div>
        <div className="sk-h">CODEX · steel.plate</div>
        <div className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>browse → focus → uses</div>
      </div>
      <Row gap={6}>
        <span className="sk-tag sk-on">RECIPE</span>
        <span className="sk-tag">USES</span>
        <span className="sk-tag">DROPS</span>
        <span className="sk-tag">FUEL</span>
        <span className="sk-tag">WIKI</span>
      </Row>
    </Row>
    <hr className="sk-div"/>
    <Row gap={10} style={{flex:1, alignItems:"stretch"}}>
      {/* breadcrumb / list */}
      <Col gap={4} style={{width: 180}}>
        <div className="sk-box" style={{padding:"4px 8px"}}>
          <span className="sk-mono-sm">⌕ search</span>
        </div>
        <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>recent</span>
        {["steel.plate","circuit.adv","reactor.core","fuel.cell","coal.coke","wrench.steel"].map((n,i)=>(
          <Row key={i} gap={4} style={{padding:"2px 4px", background: i===0?"var(--ink)":"transparent", color: i===0?"var(--paper)":"inherit"}}>
            <Slot style={{width:16,height:16}} filled icon="·"/>
            <span className="sk-mono-sm">{n}</span>
          </Row>
        ))}
        <hr className="sk-div"/>
        <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>history ◀ ▶</span>
      </Col>
      {/* recipe focus */}
      <Col gap={6} style={{flex:1.4, alignItems:"center"}}>
        <Row style={{justifyContent:"space-between", width:"100%"}}>
          <span className="sk-h-sm">RECIPE 1 / 3</span>
          <Row gap={4}>
            <span className="sk-tag">‹ prev</span>
            <span className="sk-tag">next ›</span>
          </Row>
        </Row>
        <div className="sk-box sk-thick" style={{padding:14, width:"100%"}}>
          <Row gap={14} style={{alignItems:"center", justifyContent:"center"}}>
            {/* inputs 3x3 */}
            <Col gap={2} style={{alignItems:"center"}}>
              <span className="sk-mono-xs">INPUTS</span>
              <div style={{display:"grid", gridTemplateColumns:"repeat(3,1fr)", gap:3}}>
                {Array.from({length:9}).map((_,i)=>(
                  <Slot key={i} style={{width:36,height:36}}
                    filled={[0,1,2,4].includes(i)}
                    icon={[0,1,2].includes(i)?"◆":i===4?"◉":null}
                    qty={[0,1,2].includes(i)?1:i===4?1:null}
                  />
                ))}
              </div>
            </Col>
            <Col style={{alignItems:"center", gap:2}}>
              <span className="sk-arrow">⇒</span>
              <span className="sk-mono-xs">12.0 s</span>
              <span className="sk-mono-xs">480 EU/t</span>
            </Col>
            <Col gap={2} style={{alignItems:"center"}}>
              <span className="sk-mono-xs">OUTPUT</span>
              <Slot style={{width:54,height:54}} filled icon="▤" qty={4}/>
              <span className="sk-mono-xs">steel.plate ×4</span>
            </Col>
          </Row>
          <hr className="sk-div" style={{margin:"10px 0"}}/>
          <Row gap={10} style={{justifyContent:"center"}}>
            <Col gap={1} style={{alignItems:"center"}}>
              <span className="sk-mono-xs">MACHINE</span>
              <Slot filled icon="▦"/>
              <span className="sk-mono-xs">blast furnace</span>
            </Col>
            <Col gap={1} style={{alignItems:"center"}}>
              <span className="sk-mono-xs">TIER</span>
              <span className="sk-h-sm">LV3</span>
            </Col>
            <Col gap={1} style={{alignItems:"center"}}>
              <span className="sk-mono-xs">YIELD</span>
              <span className="sk-h-sm">×4</span>
            </Col>
            <Col gap={1} style={{alignItems:"center"}}>
              <span className="sk-mono-xs">BYPRODUCT</span>
              <Slot style={{width:24,height:24}} filled icon="·"/>
            </Col>
          </Row>
        </div>
        <Row gap={6}>
          <button className="sk-btn sk-accent">▶ auto-craft</button>
          <button className="sk-btn">⊕ to queue</button>
          <button className="sk-btn">★ favorite</button>
          <button className="sk-btn">⎘ copy id</button>
        </Row>
      </Col>
      {/* uses panel */}
      <Col gap={4} style={{width: 200}}>
        <span className="sk-h-sm">USED IN · 47</span>
        <div className="sk-box" style={{padding:4, flex:1}}>
          {[
            ["frame.steel","CRAFT",4],
            ["pipe.steel","ROLL",2],
            ["plate.armor","PRESS",1],
            ["bolt.steel","LATHE",16],
            ["rail.heavy","CRAFT",6],
            ["gear.steel","MOLD",1],
            ["beam.I","ROLL",3],
            ["sword.steel","SMITH",2],
            ["...","",""],
          ].map((r,i)=>(
            <Row key={i} gap={4} style={{padding:"2px 0", borderBottom:"1px dashed var(--ink-faint)"}}>
              <Slot style={{width:18,height:18}} filled icon="·"/>
              <span className="sk-mono-sm" style={{flex:1}}>{r[0]}</span>
              <span className="sk-mono-xs">×{r[2]}</span>
            </Row>
          ))}
        </div>
        <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>click any → jump</span>
      </Col>
    </Row>
  </div>
);

// =====================================================================
// LOOKUP VARIATION 3 — Cheatsheet (graph / network of relations)
// =====================================================================
const LookupV3 = () => {
  const center = { x: 380, y: 180, r: 34, label: "steel.plate" };
  const arms = [
    { angle: -90, label: "iron.plate", hint: "input ×3", r: 22 },
    { angle: -50, label: "coal", hint: "input ×1", r: 22 },
    { angle: -10, label: "frame.steel", hint: "uses ×4", r: 22 },
    { angle: 30,  label: "pipe.steel", hint: "uses ×2", r: 22 },
    { angle: 70,  label: "rail.heavy", hint: "uses ×6", r: 22 },
    { angle: 130, label: "blast.furn.", hint: "machine", r: 22 },
    { angle: 170, label: "iron.ingot", hint: "ancestor", r: 22 },
    { angle: 210, label: "iron.ore", hint: "ancestor", r: 22 },
    { angle: 250, label: "scrap.steel", hint: "alt input", r: 22 },
  ];
  const dist = 130;
  return (
    <div className="paper" style={{padding:16, height:"100%", display:"flex", flexDirection:"column", gap:10}}>
      <Row style={{justifyContent:"space-between", alignItems:"flex-end"}}>
        <div>
          <div className="sk-h">RELATIONS GRAPH</div>
          <div className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>spider view · click node = re-center</div>
        </div>
        <Row gap={6}>
          <span className="sk-tag">grid</span>
          <span className="sk-tag">recipe</span>
          <span className="sk-tag sk-on">graph</span>
          <span className="sk-tag">tree</span>
        </Row>
      </Row>
      <hr className="sk-div"/>
      <Row gap={10} style={{flex:1, alignItems:"stretch"}}>
        {/* graph canvas */}
        <div className="sk-box" style={{flex:1, position:"relative", overflow:"hidden", minHeight: 380}}>
          <svg width="100%" height="100%" style={{position:"absolute", inset:0}}>
            {arms.map((a,i)=>{
              const rad = (a.angle*Math.PI)/180;
              const x = center.x + Math.cos(rad)*dist;
              const y = center.y + Math.sin(rad)*dist;
              return (
                <g key={i}>
                  <path d={`M${center.x},${center.y} L${x},${y}`} stroke="var(--ink)" strokeWidth="1" strokeDasharray={i<2?"":"3 2"}/>
                </g>
              );
            })}
          </svg>
          {/* center node */}
          <div style={{
            position:"absolute", left: center.x-44, top: center.y-44, width: 88, height:88,
            border:"2.5px solid var(--ink)", background:"var(--accent)", borderRadius:"50%",
            display:"flex", alignItems:"center", justifyContent:"center",
            boxShadow:"3px 3px 0 var(--ink)"
          }}>
            <Col gap={1} style={{alignItems:"center"}}>
              <span className="sk-h-sm">▤</span>
              <span className="sk-mono-xs" style={{fontWeight:700}}>steel.plate</span>
            </Col>
          </div>
          {arms.map((a,i)=>{
            const rad = (a.angle*Math.PI)/180;
            const x = center.x + Math.cos(rad)*dist - 36;
            const y = center.y + Math.sin(rad)*dist - 22;
            return (
              <div key={i} className="sk-box" style={{
                position:"absolute", left:x, top:y, width:72, padding:"3px 4px", textAlign:"center"
              }}>
                <Col gap={0} style={{alignItems:"center"}}>
                  <span className="sk-mono-sm" style={{fontWeight:700, fontSize:9}}>{a.label}</span>
                  <span className="sk-mono-xs" style={{color:"var(--ink-faint)", fontSize:8}}>{a.hint}</span>
                </Col>
              </div>
            );
          })}
          {/* legend */}
          <div className="sk-box" style={{position:"absolute", bottom:8, right:8, padding:6}}>
            <Col gap={2}>
              <Row gap={4}><span style={{width:16,height:1,background:"var(--ink)",display:"inline-block",marginTop:6}}/><span className="sk-mono-xs">direct</span></Row>
              <Row gap={4}><span style={{width:16,height:1,background:"var(--ink)",display:"inline-block",marginTop:6,backgroundImage:"radial-gradient(circle,var(--ink) 1px,transparent 1px)",backgroundSize:"4px 1px"}}/><span className="sk-mono-xs">indirect</span></Row>
              <Row gap={4}><span style={{width:10,height:10,borderRadius:"50%",background:"var(--accent)",border:"1.5px solid var(--ink)",display:"inline-block"}}/><span className="sk-mono-xs">focus</span></Row>
            </Col>
          </div>
        </div>
        {/* sidebar info */}
        <Col gap={6} style={{width: 220}}>
          <span className="sk-h-sm">FOCUS · steel.plate</span>
          <div className="sk-box" style={{padding:8}}>
            <span className="sk-mono-xs">3 recipes · 47 uses · 2 byproducts</span>
            <hr className="sk-div" style={{margin:"5px 0"}}/>
            <Col gap={2}>
              <span className="sk-mono-xs">⌖ tier: LV3</span>
              <span className="sk-mono-xs">⌖ in 12 quests</span>
              <span className="sk-mono-xs">⌖ favorited</span>
            </Col>
          </div>
          <span className="sk-h-sm">SHORTCUTS</span>
          <div className="sk-box" style={{padding:6}}>
            <Col gap={2}>
              <Row style={{justifyContent:"space-between"}}><span className="sk-mono-xs">recenter</span><span className="sk-mono-xs">click</span></Row>
              <Row style={{justifyContent:"space-between"}}><span className="sk-mono-xs">expand</span><span className="sk-mono-xs">dbl-click</span></Row>
              <Row style={{justifyContent:"space-between"}}><span className="sk-mono-xs">recipe</span><span className="sk-mono-xs">R</span></Row>
              <Row style={{justifyContent:"space-between"}}><span className="sk-mono-xs">uses</span><span className="sk-mono-xs">U</span></Row>
              <Row style={{justifyContent:"space-between"}}><span className="sk-mono-xs">pin node</span><span className="sk-mono-xs">P</span></Row>
            </Col>
          </div>
        </Col>
      </Row>
    </div>
  );
};

// =====================================================================
// LOOKUP VARIATION 4 — Command palette / instant lookup overlay
// =====================================================================
const LookupV4 = () => (
  <div className="paper" style={{padding:0, height:"100%", position:"relative", overflow:"hidden"}}>
    {/* dimmed bg */}
    <div style={{position:"absolute", inset:0, padding:16, opacity:0.4}}>
      <span className="sk-h">[ INVENTORY behind ]</span>
      <div style={{display:"grid", gridTemplateColumns:"repeat(9,1fr)", gap:3, marginTop:10}}>
        {Array.from({length:36}).map((_,i)=>(<Slot key={i} filled={i<14}/>))}
      </div>
    </div>
    <div style={{position:"absolute", inset:0, background:"rgba(26,26,26,0.18)"}}/>
    {/* palette */}
    <div className="sk-box sk-double" style={{
      position:"absolute", top:50, left:"50%", transform:"translateX(-50%)",
      width: 560, padding:0, background:"var(--paper)"
    }}>
      <Row gap={8} style={{padding:"10px 14px", borderBottom:"1.5px solid var(--ink)", alignItems:"center"}}>
        <span className="sk-h-sm">⌕</span>
        <div style={{flex:1}}>
          <span className="sk-mono" style={{fontSize:14}}>steel</span>
          <span style={{
            display:"inline-block", width:1, height:16, background:"var(--ink)",
            verticalAlign:"middle", marginLeft:2, animation:"sk-blink 1s infinite"
          }}/>
        </div>
        <span className="sk-tag">CTRL+K</span>
        <span className="sk-tag">ESC</span>
      </Row>
      {/* result list */}
      <div style={{padding:6}}>
        {[
          {n:"steel.plate", t:"item", hint:"3 recipes · 47 uses", k:"R"},
          {n:"steel.ingot", t:"item", hint:"smelt iron+coal", k:""},
          {n:"steel.gear", t:"item", hint:"used in turbines", k:""},
          {n:"steelworks", t:"machine", hint:"LV3 · multiblock 3×3×3", k:""},
          {n:"@quest:Age of Steel", t:"quest", hint:"chapter 4 · 6/12 done", k:""},
          {n:"how do I make steel?", t:"tutorial", hint:"4-min walkthrough", k:""},
        ].map((r,i)=>(
          <Row key={i} gap={8} style={{
            padding:"6px 8px",
            background: i===0 ? "var(--ink)" : "transparent",
            color: i===0 ? "var(--paper)" : "inherit",
            alignItems:"center"
          }}>
            <Slot style={{width:24,height:24, borderColor: i===0?"var(--paper)":"var(--ink)"}}
              filled icon={r.t==="item"?"▤":r.t==="machine"?"▦":r.t==="quest"?"★":"?"}/>
            <Col gap={0} style={{flex:1}}>
              <span className="sk-mono-sm" style={{fontWeight:700}}>{r.n}</span>
              <span className="sk-mono-xs" style={{opacity:0.7}}>{r.t} · {r.hint}</span>
            </Col>
            {r.k && <span className="sk-tag" style={{
              background: i===0 ? "var(--paper)" : "var(--paper)",
              color:"var(--ink)"
            }}>{r.k}</span>}
            {i===0 && <span className="sk-mono-xs">↵</span>}
          </Row>
        ))}
      </div>
      <Row style={{padding:"6px 14px", borderTop:"1.5px solid var(--ink)", justifyContent:"space-between"}}>
        <Row gap={10}>
          <span className="sk-mono-xs">↑↓ nav</span>
          <span className="sk-mono-xs">↵ open</span>
          <span className="sk-mono-xs">⇥ filter</span>
          <span className="sk-mono-xs">⌘R recipe</span>
          <span className="sk-mono-xs">⌘U uses</span>
        </Row>
        <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>6 results · 0.4ms</span>
      </Row>
    </div>
    {/* preview card to the right */}
    <div className="sk-box sk-thick" style={{
      position:"absolute", top: 50, right: 24, width: 200, padding: 10
    }}>
      <Row gap={6}>
        <Slot filled icon="▤"/>
        <Col gap={0}>
          <span className="sk-mono-sm" style={{fontWeight:700}}>steel.plate</span>
          <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>preview</span>
        </Col>
      </Row>
      <hr className="sk-div" style={{margin:"6px 0"}}/>
      <span className="sk-mono-xs">refined plate of carbon steel.</span>
      <div className="sk-img" style={{height: 70, marginTop:6}}><span>recipe mini</span></div>
      <Row gap={3} style={{marginTop:6}}>
        <button className="sk-btn" style={{padding:"2px 5px", fontSize:10}}>open</button>
        <button className="sk-btn" style={{padding:"2px 5px", fontSize:10}}>queue</button>
      </Row>
    </div>
    <style>{`@keyframes sk-blink { 0%,50%{opacity:1} 51%,100%{opacity:0} }`}</style>
  </div>
);

window.LookupV1 = LookupV1;
window.LookupV2 = LookupV2;
window.LookupV3 = LookupV3;
window.LookupV4 = LookupV4;
