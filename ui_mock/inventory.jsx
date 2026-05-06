/* global React, Slot, Row, Col */

// =====================================================================
// INVENTORY VARIATION 1 — Classic grid + character + sidebar (familiar)
// =====================================================================
const InventoryV1 = () => (
  <div className="paper" style={{ padding: 16, height: "100%", display: "flex", flexDirection: "column", gap: 10 }}>
    <Row style={{ justifyContent: "space-between", alignItems: "flex-end" }}>
      <div>
        <div className="sk-h">INVENTORY</div>
        <div className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>27 slots · 4.2/12 stacks</div>
      </div>
      <Row gap={6}>
        <span className="sk-tag sk-on">ALL</span>
        <span className="sk-tag">ORE</span>
        <span className="sk-tag">TOOL</span>
        <span className="sk-tag">FOOD</span>
      </Row>
    </Row>
    <hr className="sk-div" />
    <Row gap={14} style={{ alignItems: "flex-start", flex: 1 }}>
      {/* left: character + equip */}
      <Col gap={6} style={{ width: 130 }}>
        <div className="sk-img" style={{ width: 120, height: 160 }}><span>character</span></div>
        <span className="sk-h-xs">EQUIPPED</span>
        <Row gap={4} style={{ flexWrap: "wrap" }}>
          {["⛑","▼","▦","▣","◇","✦"].map((g,i) => (
            <Col key={i} gap={1} style={{ alignItems: "center" }}>
              <Slot filled icon={g} />
              <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>{["head","chest","legs","feet","main","off"][i]}</span>
            </Col>
          ))}
        </Row>
      </Col>
      {/* center: bag grid */}
      <Col gap={4} style={{ flex: 1 }}>
        <Row style={{ justifyContent: "space-between" }}>
          <span className="sk-h-xs">BAG · 27</span>
          <Row gap={4}>
            <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>sort:</span>
            <span className="sk-tag">A-Z</span>
            <span className="sk-tag">QTY</span>
            <span className="sk-tag">MOD</span>
          </Row>
        </Row>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(9, 1fr)", gap: 3 }}>
          {Array.from({length: 27}).map((_,i) => {
            const filled = i < 14;
            return <Slot key={i} filled={filled}
              icon={filled ? "▢◇⛏✦◉◆▣▤▥◈✕▦▧▨"[i] : null}
              qty={filled ? [64,32,1,16,8,1,4,12,2,1,32,16,8,1][i] : null}
            />;
          })}
        </div>
        <hr className="sk-div" />
        <span className="sk-h-xs">HOTBAR</span>
        <Row gap={3}>
          {Array.from({length:9}).map((_,i) => (
            <Slot key={i} filled={i<5} active={i===0} icon={["⛏","◇","","✦","","◉","","",""][i]} />
          ))}
        </Row>
      </Col>
      {/* right: stats */}
      <Col gap={6} style={{ width: 160 }}>
        <span className="sk-h-xs">STATS</span>
        {[
          ["HP","87/100"],
          ["SAT","42/100"],
          ["O₂","100"],
          ["TEMP","21°C"],
          ["RAD","0.0 rem"],
          ["XP","lvl 14"],
        ].map(([k,v],i) => (
          <Row key={i} style={{ justifyContent:"space-between" }}>
            <span className="sk-mono-sm">{k}</span>
            <span className="sk-mono-sm">{v}</span>
          </Row>
        ))}
        <hr className="sk-div" />
        <span className="sk-h-xs">QUICK</span>
        <button className="sk-btn">⊕ deposit all</button>
        <button className="sk-btn">↺ sort bag</button>
        <button className="sk-btn">⌫ trash</button>
      </Col>
    </Row>
  </div>
);

// =====================================================================
// INVENTORY VARIATION 2 — Spatial (Resident Evil / Tetris-style grid)
// =====================================================================
const InventoryV2 = () => {
  const cell = 22;
  const cols = 16, rows = 10;
  // pre-placed items: {x,y,w,h,label}
  const items = [
    {x:0,y:0,w:1,h:2,l:"PIPE"},
    {x:1,y:0,w:2,h:1,l:"WRENCH"},
    {x:3,y:0,w:1,h:1,l:"GEAR"},
    {x:5,y:0,w:3,h:2,l:"REACTOR CORE"},
    {x:9,y:0,w:2,h:2,l:"TURBINE"},
    {x:12,y:0,w:1,h:1,l:"◇"},
    {x:13,y:0,w:1,h:1,l:"◆"},
    {x:0,y:3,w:2,h:2,l:"BATTERY"},
    {x:3,y:3,w:1,h:1,l:"FUSE"},
    {x:5,y:3,w:4,h:1,l:"CIRCUIT BOARD"},
    {x:0,y:6,w:1,h:1,l:"·"},
    {x:1,y:6,w:1,h:1,l:"·"},
    {x:2,y:6,w:1,h:1,l:"·"},
    {x:6,y:6,w:2,h:3,l:"COOLANT TANK"},
    {x:11,y:5,w:5,h:4,l:"DRILL HEAD MK-III"},
  ];
  return (
    <div className="paper" style={{ padding: 16, height: "100%", display: "flex", flexDirection: "column", gap: 10 }}>
      <Row style={{ justifyContent: "space-between", alignItems: "flex-end" }}>
        <div>
          <div className="sk-h">SATCHEL</div>
          <div className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>16×10 · weight 23.4/40 kg</div>
        </div>
        <Row gap={6}>
          <span className="sk-tag">↻ rotate R</span>
          <span className="sk-tag">⇄ auto-fit</span>
          <span className="sk-tag sk-on">drag&drop</span>
        </Row>
      </Row>
      <hr className="sk-div" />
      {/* grid */}
      <div style={{ position:"relative", width: cols*cell + 2, height: rows*cell + 2, border:"1.5px solid var(--ink)" }}>
        {/* faint cell grid */}
        {Array.from({length: cols*rows}).map((_,i) => (
          <div key={i} style={{ position:"absolute",
            left: (i%cols)*cell, top: Math.floor(i/cols)*cell,
            width: cell, height: cell,
            borderRight: "1px dashed rgba(26,26,26,0.15)",
            borderBottom: "1px dashed rgba(26,26,26,0.15)" }} />
        ))}
        {items.map((it, i) => (
          <div key={i} style={{
            position: "absolute",
            left: it.x*cell + 2,
            top: it.y*cell + 2,
            width: it.w*cell - 4,
            height: it.h*cell - 4,
            border: "1.5px solid var(--ink)",
            background: "repeating-linear-gradient(45deg, transparent 0 3px, rgba(26,26,26,0.10) 3px 4px)",
            display: "flex", alignItems:"center", justifyContent:"center",
            fontFamily:"var(--font-mono)", fontSize: 8,
            textAlign:"center", padding:2, lineHeight:1.1
          }}>{it.l}</div>
        ))}
      </div>
      <Row gap={20} style={{ alignItems:"flex-start" }}>
        <Col gap={4} style={{ flex: 1 }}>
          <span className="sk-h-xs">WEIGHT</span>
          <div className="sk-bar" style={{height: 14}}><i style={{width:"58%"}}/></div>
          <Row style={{justifyContent:"space-between"}}>
            <span className="sk-mono-xs">23.4 kg</span>
            <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>40 kg max · over = slow</span>
          </Row>
        </Col>
        <Col gap={4} style={{ width: 220 }}>
          <span className="sk-h-xs">HOTBAR</span>
          <Row gap={2}>
            {Array.from({length:9}).map((_,i)=>(
              <Slot key={i} filled={i<6} active={i===0} icon={["⛏","◇","","✦","◉","",""][i]} />
            ))}
          </Row>
        </Col>
      </Row>
      <span className="sk-annot" style={{position:"static", color:"var(--ink-soft)"}}>note: items take physical space — bigger drills won't fit</span>
    </div>
  );
};

// =====================================================================
// INVENTORY VARIATION 3 — Data table / brutalist list mode
// =====================================================================
const InventoryV3 = () => {
  const rows = [
    ["#001","iron.ore","ORE",64,"·",2.4],
    ["#002","copper.ore","ORE",32,"·",1.6],
    ["#003","wrench.steel","TOOL",1,"234/250",0.8],
    ["#004","circuit.basic","COMP",16,"·",0.4],
    ["#005","battery.4ah","COMP",4,"82%",1.2],
    ["#006","gear.bronze","COMP",24,"·",0.6],
    ["#007","pipe.stl.10","BUILD",128,"·",0.1],
    ["#008","fuel.cell","FUEL",8,"·",2.0],
    ["#009","wood.plank","BUILD",256,"·",0.05],
    ["#010","bread.loaf","FOOD",6,"fresh",0.3],
    ["#011","drill.mk3","TOOL",1,"new",6.0],
    ["#012","reactor.core","COMP",1,"unstable",4.0],
  ];
  return (
    <div className="paper" style={{ padding: 16, height: "100%", display: "flex", flexDirection: "column", gap: 8 }}>
      <Row style={{ justifyContent:"space-between", alignItems:"flex-end" }}>
        <div>
          <div className="sk-h">INV.LIST</div>
          <div className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>data view · 12 entries · 19.55 kg</div>
        </div>
        <Row gap={6}>
          <span className="sk-tag">[grid]</span>
          <span className="sk-tag sk-on">[table]</span>
          <span className="sk-tag">[graph]</span>
        </Row>
      </Row>
      <Row gap={6}>
        <div className="sk-box" style={{flex:1, padding: "4px 8px"}}>
          <span className="sk-mono-sm">⌕ filter:</span> <span className="sk-squig sk-mono-sm">tag:ore qty&gt;10</span>
        </div>
        <button className="sk-btn">+ column</button>
        <button className="sk-btn">↓ export csv</button>
      </Row>
      <table style={{ width: "100%", fontFamily: "var(--font-mono)", fontSize: 11, borderCollapse: "collapse" }}>
        <thead>
          <tr style={{ borderBottom: "1.5px solid var(--ink)" }}>
            {["ID","name","tag","qty","state","kg"].map((h,i)=>(
              <th key={i} style={{ textAlign:"left", padding:"4px 6px", fontFamily:"var(--font-label)", fontSize:11 }}>
                {h} <span style={{color:"var(--ink-faint)"}}>↕</span>
              </th>
            ))}
            <th style={{ textAlign:"right", padding:"4px 6px", fontFamily:"var(--font-label)", fontSize:11, width: 90 }}>actions</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((r,i) => (
            <tr key={i} style={{ borderBottom: "1px dashed var(--ink-faint)", background: i===2 ? "rgba(245,197,24,0.18)" : "transparent" }}>
              {r.map((c,j) => (
                <td key={j} style={{ padding:"3px 6px" }}>{c}</td>
              ))}
              <td style={{ padding:"3px 6px", textAlign:"right" }}>
                <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>[use] [drop] [⌕]</span>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <Row style={{ justifyContent:"space-between", marginTop: "auto" }}>
        <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>↑↓ navigate · ENTER = use · DEL = drop</span>
        <Row gap={3}>
          {Array.from({length:9}).map((_,i)=>(
            <Slot key={i} style={{ width: 22, height: 22 }} filled={i<5} active={i===0} />
          ))}
        </Row>
      </Row>
    </div>
  );
};

// =====================================================================
// INVENTORY VARIATION 4 — Dual-pane (bag ↔ network/storage drives)
// =====================================================================
const InventoryV4 = () => (
  <div className="paper" style={{ padding: 16, height: "100%", display: "flex", flexDirection: "column", gap: 10 }}>
    <Row style={{ justifyContent:"space-between", alignItems:"flex-end" }}>
      <div>
        <div className="sk-h">TRANSFER</div>
        <div className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>player ↔ network storage</div>
      </div>
      <Row gap={6}>
        <span className="sk-tag sk-on">↔ both</span>
        <span className="sk-tag">→ deposit</span>
        <span className="sk-tag">← withdraw</span>
      </Row>
    </Row>
    <hr className="sk-div" />
    <Row gap={12} style={{ flex: 1, alignItems: "stretch" }}>
      {/* left: player */}
      <Col gap={6} style={{ flex: 1 }}>
        <Row style={{justifyContent:"space-between"}}>
          <span className="sk-h-sm">▼ PLAYER</span>
          <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>27 slots · 14 used</span>
        </Row>
        <div className="sk-box" style={{ padding: 6, flex: 1 }}>
          <div style={{display:"grid", gridTemplateColumns:"repeat(6,1fr)", gap:3}}>
            {Array.from({length:24}).map((_,i)=>(
              <Slot key={i} filled={i<14}
                icon={i<14 ? "▢◇⛏✦◉◆▣▤▥◈✕▦▧▨"[i] : null}
                qty={i<14 ? [64,32,1,16,8,1,4,12,2,1,32,16,8,1][i] : null}
              />
            ))}
          </div>
        </div>
      </Col>
      {/* center: arrow / batch ops */}
      <Col gap={6} style={{ width: 90, alignItems: "center", justifyContent:"center" }}>
        <button className="sk-btn">→ ALL</button>
        <span className="sk-arrow">⇄</span>
        <button className="sk-btn">← ALL</button>
        <hr className="sk-div" style={{width:"100%"}}/>
        <span className="sk-mono-xs" style={{color:"var(--ink-faint)", textAlign:"center"}}>shift-click<br/>= one stack</span>
        <span className="sk-mono-xs" style={{color:"var(--ink-faint)", textAlign:"center"}}>ctrl-click<br/>= matching</span>
      </Col>
      {/* right: storage network */}
      <Col gap={6} style={{ flex: 1.6 }}>
        <Row style={{justifyContent:"space-between"}}>
          <span className="sk-h-sm">▣ NETWORK · LVL-2 DRIVES</span>
          <Row gap={4}>
            <span className="sk-tag">⌕ search</span>
            <span className="sk-tag">A-Z</span>
            <span className="sk-tag sk-on">qty▼</span>
          </Row>
        </Row>
        <div className="sk-box" style={{ padding: 6, flex: 1, position:"relative" }}>
          <div style={{display:"grid", gridTemplateColumns:"repeat(10,1fr)", gap:3}}>
            {Array.from({length:60}).map((_,i)=>(
              <Slot key={i} filled
                icon={["▢","◇","⛏","✦","◉","◆","▣","▤","▥","◈","✕","▦","▧","▨","▩","◇","◆","▢","◉","✦"][i%20]}
                qty={[16384,8192,4096,2048,1024,512,256,128,64,32][i%10]}
              />
            ))}
          </div>
          <div className="sk-annot" style={{ bottom: 4, right: 8 }}>scrollable · 8 drives connected</div>
        </div>
        <Row gap={4}>
          <Col gap={2} style={{flex:1}}>
            <span className="sk-mono-xs">drive 1/8 · 64k</span>
            <div className="sk-bar"><i style={{width:"82%"}}/></div>
          </Col>
          <Col gap={2} style={{flex:1}}>
            <span className="sk-mono-xs">cells used</span>
            <div className="sk-bar"><i style={{width:"34%"}}/></div>
          </Col>
        </Row>
      </Col>
    </Row>
  </div>
);

window.InventoryV1 = InventoryV1;
window.InventoryV2 = InventoryV2;
window.InventoryV3 = InventoryV3;
window.InventoryV4 = InventoryV4;
