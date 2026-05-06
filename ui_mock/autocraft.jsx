/* global React, Slot, Row, Col */

// =====================================================================
// AUTOCRAFT VARIATION 1 — Request terminal (AE2 classic + queue)
// =====================================================================
const AutocraftV1 = () => (
  <div className="paper" style={{ padding: 16, height: "100%", display: "flex", flexDirection: "column", gap: 10 }}>
    <Row style={{ justifyContent: "space-between", alignItems:"flex-end" }}>
      <div>
        <div className="sk-h">CRAFTING TERMINAL</div>
        <div className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>request · plan · monitor · 3 CPUs idle</div>
      </div>
      <Row gap={6}>
        <span className="sk-tag sk-on">REQUEST</span>
        <span className="sk-tag">PATTERNS</span>
        <span className="sk-tag">CPUS</span>
        <span className="sk-tag">PLANNER</span>
      </Row>
    </Row>
    <hr className="sk-div" />
    <Row gap={12} style={{ flex: 1, alignItems:"stretch" }}>
      {/* left: searchable item list w/ "craftable" badge */}
      <Col gap={6} style={{ flex: 1.2 }}>
        <div className="sk-box" style={{ padding: "4px 8px" }}>
          <span className="sk-mono-sm">⌕ </span>
          <span className="sk-squig sk-mono-sm">circuit</span>
        </div>
        <div className="sk-box" style={{ padding: 6, flex: 1, overflow:"hidden" }}>
          <div style={{display:"grid", gridTemplateColumns:"repeat(7,1fr)", gap:3}}>
            {Array.from({length:35}).map((_,i)=>(
              <div key={i} style={{position:"relative"}}>
                <Slot filled
                  icon={["▢","◇","⛏","✦","◉","◆","▣","▤","▥","◈","✕","▦","▧","▨","▩"][i%15]}
                  qty={[256,64,1024,16,8,128][i%6]}
                />
                {i%3===0 && <span className="sk-tag sk-on" style={{
                  position:"absolute", top:-4, right:-4, fontSize:7, padding:"0 3px"
                }}>C</span>}
              </div>
            ))}
          </div>
        </div>
        <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>[C] = craftable on demand · 2,847 items indexed</span>
      </Col>
      {/* center: request panel */}
      <Col gap={6} style={{ width: 200 }}>
        <span className="sk-h-sm">REQUEST</span>
        <Row gap={6} style={{alignItems:"center"}}>
          <Slot filled style={{width:48, height:48}} icon="▦"/>
          <Col gap={1}>
            <span className="sk-mono-sm">circuit.advanced</span>
            <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>have: 12 · craft: ?</span>
          </Col>
        </Row>
        <Col gap={2}>
          <span className="sk-mono-xs">amount</span>
          <Row gap={4}>
            <button className="sk-btn" style={{padding:"2px 6px"}}>−</button>
            <div className="sk-box" style={{flex:1, padding:"4px 8px", textAlign:"center"}}>
              <span className="sk-h-sm">64</span>
            </div>
            <button className="sk-btn" style={{padding:"2px 6px"}}>+</button>
          </Row>
          <Row gap={3}>
            {[1,16,64,"½","stk"].map(n=>(
              <span key={n} className="sk-tag">{n}</span>
            ))}
          </Row>
        </Col>
        <hr className="sk-div" />
        <Col gap={2}>
          <span className="sk-h-xs">PREVIEW</span>
          <span className="sk-mono-xs">est. time: <b>2m 14s</b></span>
          <span className="sk-mono-xs">power peak: <b>4.2 MW</b></span>
          <span className="sk-mono-xs">7 sub-crafts · 2 missing</span>
        </Col>
        <Col gap={3}>
          <button className="sk-btn sk-accent" style={{justifyContent:"center"}}>▶ START CRAFT</button>
          <button className="sk-btn" style={{justifyContent:"center"}}>⊕ to queue</button>
          <button className="sk-btn" style={{justifyContent:"center"}}>◇ simulate</button>
        </Col>
      </Col>
      {/* right: live queue */}
      <Col gap={6} style={{ flex: 1 }}>
        <Row style={{justifyContent:"space-between"}}>
          <span className="sk-h-sm">QUEUE · 4 jobs</span>
          <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>3 CPUs · 1 free</span>
        </Row>
        {[
          {n:"steel.plate ×512", cpu:"CPU-α", pct:74, eta:"0:42"},
          {n:"circuit.basic ×128", cpu:"CPU-β", pct:31, eta:"1:55"},
          {n:"reactor.frame ×4", cpu:"CPU-γ", pct:8, eta:"5:10"},
          {n:"wire.copper ×2k", cpu:"queued", pct:0, eta:"—"},
        ].map((j,i)=>(
          <div key={i} className="sk-box" style={{ padding: 6 }}>
            <Row style={{justifyContent:"space-between"}}>
              <span className="sk-mono-sm" style={{fontWeight:700}}>{j.n}</span>
              <span className="sk-tag">{j.cpu}</span>
            </Row>
            <div className="sk-bar" style={{margin:"4px 0"}}><i style={{width:`${j.pct}%`}}/></div>
            <Row style={{justifyContent:"space-between"}}>
              <span className="sk-mono-xs">{j.pct}%</span>
              <span className="sk-mono-xs">eta {j.eta}</span>
              <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>[cancel] [pin]</span>
            </Row>
          </div>
        ))}
      </Col>
    </Row>
  </div>
);

// =====================================================================
// AUTOCRAFT VARIATION 2 — Pattern library / encoder
// =====================================================================
const AutocraftV2 = () => (
  <div className="paper" style={{ padding: 16, height: "100%", display: "flex", flexDirection: "column", gap: 10 }}>
    <Row style={{justifyContent:"space-between", alignItems:"flex-end"}}>
      <div>
        <div className="sk-h">PATTERN LIBRARY</div>
        <div className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>247 patterns · 14 machines · 3 conflicts</div>
      </div>
      <Row gap={6}>
        <span className="sk-tag">REQUEST</span>
        <span className="sk-tag sk-on">PATTERNS</span>
        <span className="sk-tag">CPUS</span>
        <span className="sk-tag">PLANNER</span>
      </Row>
    </Row>
    <hr className="sk-div" />
    <Row gap={12} style={{flex:1}}>
      {/* left: pattern list */}
      <Col gap={4} style={{ width: 240 }}>
        <div className="sk-box" style={{ padding: "4px 8px" }}>
          <span className="sk-mono-sm">⌕ filter patterns</span>
        </div>
        <Row gap={4} style={{flexWrap:"wrap"}}>
          <span className="sk-tag sk-on">all</span>
          <span className="sk-tag">crafting</span>
          <span className="sk-tag">smelt</span>
          <span className="sk-tag">processing</span>
          <span className="sk-tag">conflict</span>
        </Row>
        <div className="sk-box" style={{flex:1, padding:6, overflow:"hidden"}}>
          {[
            ["circuit.basic","CRAFT","ok"],
            ["circuit.adv","CRAFT","ok"],
            ["iron.plate","SMELT","ok"],
            ["steel.plate","SMELT","conflict"],
            ["coal.coke","COKE","ok"],
            ["copper.wire","CRAFT","ok"],
            ["gear.bronze","CRAFT","ok"],
            ["fuel.cell","CHEM","ok"],
            ["plastic.sheet","CHEM","ok"],
            ["concrete","MIX","ok"],
          ].map((p,i)=>(
            <Row key={i} style={{justifyContent:"space-between", padding:"3px 0", borderBottom:"1px dashed var(--ink-faint)"}}>
              <Row gap={5}>
                <Slot style={{width:18,height:18}} filled icon={["▢","◇","◆","▣","▤","◈","✕","▦","▧","▨"][i]}/>
                <span className="sk-mono-sm">{p[0]}</span>
              </Row>
              <Row gap={3}>
                <span className="sk-tag" style={{fontSize:7,padding:"0 3px"}}>{p[1]}</span>
                {p[2]==="conflict" && <span className="sk-tag sk-accent" style={{fontSize:7,padding:"0 3px"}}>!</span>}
              </Row>
            </Row>
          ))}
        </div>
      </Col>
      {/* center: pattern editor */}
      <Col gap={6} style={{flex:1}}>
        <span className="sk-h-sm">EDIT · steel.plate ×4</span>
        <Row gap={20} style={{alignItems:"center", justifyContent:"center", padding:"10px 0"}}>
          {/* inputs grid */}
          <Col gap={2} style={{alignItems:"center"}}>
            <span className="sk-mono-xs">INPUTS</span>
            <div style={{display:"grid", gridTemplateColumns:"repeat(3,1fr)", gap:2}}>
              {Array.from({length:9}).map((_,i)=>(
                <Slot key={i} style={{width:34,height:34}}
                  filled={[0,1,2,4].includes(i)}
                  icon={[0,1,2].includes(i)?"◆":i===4?"◉":null}
                  qty={[0,1,2].includes(i)?1:i===4?1:null}
                />
              ))}
            </div>
            <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>3× iron + 1× coal</span>
          </Col>
          <span className="sk-arrow">⇒</span>
          {/* outputs */}
          <Col gap={2} style={{alignItems:"center"}}>
            <span className="sk-mono-xs">OUTPUT</span>
            <Slot style={{width:54,height:54}} filled icon="▤" qty={4}/>
            <span className="sk-mono-xs">steel.plate</span>
          </Col>
          <Col gap={2} style={{alignItems:"center"}}>
            <span className="sk-mono-xs">+ BYPRODUCT</span>
            <Slot style={{width:34,height:34}} filled icon="·" qty={1}/>
            <span className="sk-mono-xs">slag</span>
          </Col>
        </Row>
        <hr className="sk-div" />
        <Row gap={10} style={{flexWrap:"wrap"}}>
          <Col gap={2}>
            <span className="sk-mono-xs">MACHINE</span>
            <div className="sk-box" style={{padding:"3px 8px"}}><span className="sk-mono-sm">▦ blast.furnace · LV3</span></div>
          </Col>
          <Col gap={2}>
            <span className="sk-mono-xs">TIME</span>
            <div className="sk-box" style={{padding:"3px 8px"}}><span className="sk-mono-sm">12.0 s</span></div>
          </Col>
          <Col gap={2}>
            <span className="sk-mono-xs">ENERGY</span>
            <div className="sk-box" style={{padding:"3px 8px"}}><span className="sk-mono-sm">480 EU/t</span></div>
          </Col>
          <Col gap={2}>
            <span className="sk-mono-xs">PRIORITY</span>
            <Row gap={2}>
              {[1,2,3,4,5].map(p=>(<span key={p} className={`sk-tag ${p===3?"sk-on":""}`}>{p}</span>))}
            </Row>
          </Col>
        </Row>
        <div className="sk-box sk-dashed" style={{padding:8, marginTop:"auto"}}>
          <Row style={{justifyContent:"space-between"}}>
            <span className="sk-mono-sm" style={{color:"#b88a00"}}>⚠ CONFLICT — 2 patterns produce steel.plate</span>
            <span className="sk-mono-xs sk-squig">resolve →</span>
          </Row>
        </div>
        <Row gap={6}>
          <button className="sk-btn sk-accent">save pattern</button>
          <button className="sk-btn">duplicate</button>
          <button className="sk-btn">delete</button>
          <button className="sk-btn" style={{marginLeft:"auto"}}>⊕ encode blank</button>
        </Row>
      </Col>
    </Row>
  </div>
);

// =====================================================================
// AUTOCRAFT VARIATION 3 — Multi-step planner (dependency tree)
// =====================================================================
const AutocraftV3 = () => {
  const Node = ({ label, qty, miss, x, y, w=90 }) => (
    <div className="sk-box" style={{
      position:"absolute", left:x, top:y, width: w, padding: "4px 6px",
      background: miss ? "#fff4d0" : "var(--paper)"
    }}>
      <Row gap={4}>
        <Slot style={{width:18,height:18}} filled icon="▢"/>
        <Col gap={0} style={{flex:1, minWidth:0}}>
          <span className="sk-mono-sm" style={{fontWeight:700, overflow:"hidden", textOverflow:"ellipsis", whiteSpace:"nowrap"}}>{label}</span>
          <span className="sk-mono-xs" style={{color: miss?"#b88a00":"var(--ink-faint)"}}>×{qty}{miss?" · need!":""}</span>
        </Col>
      </Row>
    </div>
  );
  // edges (x1,y1,x2,y2)
  const edges = [
    [100,40,210,40],
    [100,40,210,80],
    [100,40,210,120],
    [310,40,420,30],
    [310,40,420,70],
    [310,80,420,110],
    [310,80,420,150],
    [310,120,420,190],
    [520,30,630,30],
    [520,30,630,70],
    [520,110,630,110],
    [520,150,630,150],
    [520,190,630,190],
  ];
  return (
    <div className="paper" style={{ padding: 16, height: "100%", display:"flex", flexDirection:"column", gap:10 }}>
      <Row style={{justifyContent:"space-between", alignItems:"flex-end"}}>
        <div>
          <div className="sk-h">CRAFT PLANNER</div>
          <div className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>dep-tree · target: reactor.core ×1</div>
        </div>
        <Row gap={6}>
          <span className="sk-tag">REQUEST</span>
          <span className="sk-tag">PATTERNS</span>
          <span className="sk-tag">CPUS</span>
          <span className="sk-tag sk-on">PLANNER</span>
        </Row>
      </Row>
      <hr className="sk-div"/>
      <Row gap={10} style={{flex:1, alignItems:"stretch"}}>
        <Col gap={6} style={{flex:1}}>
          <Row gap={6} style={{alignItems:"center"}}>
            <span className="sk-mono-sm">target →</span>
            <Slot filled icon="◉" qty={1}/>
            <span className="sk-mono-sm" style={{fontWeight:700}}>reactor.core</span>
            <Row gap={2}>
              <button className="sk-btn" style={{padding:"2px 6px"}}>−</button>
              <span className="sk-mono-sm">1</span>
              <button className="sk-btn" style={{padding:"2px 6px"}}>+</button>
            </Row>
            <Row gap={4} style={{marginLeft:"auto"}}>
              <span className="sk-tag sk-on">tree</span>
              <span className="sk-tag">flow</span>
              <span className="sk-tag">gantt</span>
            </Row>
          </Row>
          <div className="sk-box" style={{flex:1, padding:8, position:"relative", overflow:"hidden"}}>
            {/* SVG for connection lines */}
            <svg width="100%" height="100%" style={{position:"absolute", inset:0, pointerEvents:"none"}}>
              {edges.map((e,i)=>(
                <path key={i} d={`M${e[0]},${e[1]+12} C${(e[0]+e[2])/2},${e[1]+12} ${(e[0]+e[2])/2},${e[3]+12} ${e[2]},${e[3]+12}`}
                  stroke="var(--ink)" strokeWidth="1" fill="none" strokeDasharray="2 2"/>
              ))}
            </svg>
            <Node label="reactor.core" qty={1} x={10} y={28} w={90}/>
            <Node label="frame.steel" qty={4} x={210} y={28} w={100}/>
            <Node label="circuit.adv" qty={2} x={210} y={68} w={100}/>
            <Node label="coolant" qty={6} x={210} y={108} w={100}/>
            <Node label="steel.plate" qty={16} x={420} y={18} w={100}/>
            <Node label="bolt.steel" qty={32} x={420} y={58} w={100}/>
            <Node label="circuit.basic" qty={2} x={420} y={98} w={100}/>
            <Node label="silicon" qty={4} x={420} y={138} w={100}/>
            <Node label="water" qty={6} x={420} y={178} w={100}/>
            <Node label="iron.ore" qty={48} x={630} y={18} w={90}/>
            <Node label="coal" qty={16} x={630} y={58} w={90} miss/>
            <Node label="copper.wire" qty={4} x={630} y={98} w={90}/>
            <Node label="sand" qty={4} x={630} y={138} w={90}/>
            <Node label="water" qty={6} x={630} y={178} w={90}/>
          </div>
          <Row gap={10}>
            <Col gap={1}>
              <span className="sk-mono-xs">est. total time</span>
              <span className="sk-h-sm">14m 22s</span>
            </Col>
            <Col gap={1}>
              <span className="sk-mono-xs">peak power</span>
              <span className="sk-h-sm">8.4 MW</span>
            </Col>
            <Col gap={1}>
              <span className="sk-mono-xs">missing inputs</span>
              <span className="sk-h-sm" style={{color:"#b88a00"}}>1 · coal</span>
            </Col>
            <button className="sk-btn sk-accent" style={{marginLeft:"auto", alignSelf:"center"}}>▶ commit plan</button>
          </Row>
        </Col>
        {/* side: critical path + warnings */}
        <Col gap={6} style={{width: 220}}>
          <span className="sk-h-sm">CRITICAL PATH</span>
          <div className="sk-box" style={{padding:6}}>
            <Col gap={2}>
              {["iron.ore →","smelt.plate →","frame.steel →","reactor.core"].map((s,i)=>(
                <span key={i} className="sk-mono-sm" style={{paddingLeft: i*8}}>{s}</span>
              ))}
            </Col>
          </div>
          <span className="sk-h-sm">WARNINGS</span>
          <div className="sk-box sk-dashed" style={{padding:6}}>
            <span className="sk-mono-xs">⚠ 16× coal missing</span><br/>
            <span className="sk-mono-xs sk-squig">find/auto-craft →</span>
          </div>
          <div className="sk-box sk-dashed" style={{padding:6}}>
            <span className="sk-mono-xs">⚠ 1 conflict on steel.plate</span><br/>
            <span className="sk-mono-xs sk-squig">choose pattern →</span>
          </div>
        </Col>
      </Row>
    </div>
  );
};

// =====================================================================
// AUTOCRAFT VARIATION 4 — CPU monitor (process-list style)
// =====================================================================
const AutocraftV4 = () => (
  <div className="paper" style={{padding:16, height:"100%", display:"flex", flexDirection:"column", gap:10}}>
    <Row style={{justifyContent:"space-between", alignItems:"flex-end"}}>
      <div>
        <div className="sk-h">CPU MONITOR</div>
        <div className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>4 clusters · 12 cores · htop-style</div>
      </div>
      <Row gap={6}>
        <span className="sk-tag">REQUEST</span>
        <span className="sk-tag">PATTERNS</span>
        <span className="sk-tag sk-on">CPUS</span>
        <span className="sk-tag">PLANNER</span>
      </Row>
    </Row>
    <hr className="sk-div"/>
    {/* CPU bars */}
    <Col gap={4}>
      {[
        ["CPU-α · 4-core",92,"steel.plate ×512"],
        ["CPU-β · 4-core",54,"circuit.basic ×128"],
        ["CPU-γ · 2-core",18,"reactor.frame ×4"],
        ["CPU-δ · 2-core",0,"— idle —"],
      ].map(([n,p,job],i)=>(
        <Row key={i} gap={8} style={{alignItems:"center"}}>
          <span className="sk-mono-sm" style={{width:120}}>{n}</span>
          <div className="sk-bar" style={{flex:1}}><i style={{width:`${p}%`}}/></div>
          <span className="sk-mono-sm" style={{width:40, textAlign:"right"}}>{p}%</span>
          <span className="sk-mono-sm" style={{width:200, color:"var(--ink-faint)"}}>{job}</span>
          <Row gap={2}><span className="sk-tag">⏸</span><span className="sk-tag">✕</span></Row>
        </Row>
      ))}
    </Col>
    <hr className="sk-div"/>
    {/* Process table */}
    <Col gap={4} style={{flex:1}}>
      <Row style={{justifyContent:"space-between"}}>
        <span className="sk-h-sm">PROCESSES · 17 active</span>
        <Row gap={4}>
          <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>sort:</span>
          <span className="sk-tag">pid</span>
          <span className="sk-tag sk-on">eta↓</span>
          <span className="sk-tag">pwr</span>
        </Row>
      </Row>
      <table style={{width:"100%", fontFamily:"var(--font-mono)", fontSize:11, borderCollapse:"collapse"}}>
        <thead>
          <tr style={{borderBottom:"1.5px solid var(--ink)"}}>
            {["pid","cpu","item","×","done","eta","pwr","stat"].map((h,i)=>(
              <th key={i} style={{textAlign:"left", padding:"3px 6px", fontFamily:"var(--font-label)"}}>{h}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {[
            ["#0421","α","steel.plate","512","74%","0:42","2.1MW","run"],
            ["#0422","α","└ iron.plate","48","spawned","0:08","0.4MW","sub"],
            ["#0419","β","circuit.basic","128","31%","1:55","1.8MW","run"],
            ["#0420","β","└ copper.wire","256","14%","0:50","0.6MW","sub"],
            ["#0418","γ","reactor.frame","4","8%","5:10","1.2MW","run"],
            ["#0417","—","wire.copper","2k","queued","—","—","wait"],
            ["#0416","—","plastic.sheet","64","queued","—","—","wait"],
            ["#0414","—","gear.bronze","32","blocked","—","—","ERR"],
          ].map((r,i)=>(
            <tr key={i} style={{
              borderBottom:"1px dashed var(--ink-faint)",
              background: r[7]==="ERR" ? "rgba(245,197,24,0.18)" : "transparent"
            }}>
              {r.map((c,j)=><td key={j} style={{padding:"2px 6px"}}>{c}</td>)}
            </tr>
          ))}
        </tbody>
      </table>
    </Col>
    <Row gap={6}>
      <button className="sk-btn">⏸ pause all</button>
      <button className="sk-btn">▶ resume</button>
      <button className="sk-btn">✕ cancel selected</button>
      <span className="sk-mono-xs" style={{marginLeft:"auto", color:"var(--ink-faint)"}}>total power: 5.5 MW · grid 14.0 MW</span>
    </Row>
  </div>
);

window.AutocraftV1 = AutocraftV1;
window.AutocraftV2 = AutocraftV2;
window.AutocraftV3 = AutocraftV3;
window.AutocraftV4 = AutocraftV4;
