/* global React, Slot, Row, Col */

// =====================================================================
// LOOKUP — Recipe deep-dive (3-pane: list / recipe / uses)
// =====================================================================
const LookupV2 = () => (
  <div className="paper" style={{padding:16, height:"100%", display:"flex", flexDirection:"column", gap:10}}>
    <Row style={{justifyContent:"space-between", alignItems:"flex-end"}}>
      <div>
        <div className="sk-h">INDEX · steel.plate</div>
        <div className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>browse → focus → uses</div>
      </div>
      <Row gap={6}>
        <span className="sk-tag sk-on">RECIPE</span>
        <span className="sk-tag">USES</span>
        <span className="sk-tag">CODEX</span>
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
        <div className="sk-box sk-thick" style={{padding:0, width:"100%"}}>
          <table style={{
            width:"100%", borderCollapse:"collapse",
            fontFamily:"var(--font-mono)", fontSize:11,
          }}>
            <thead>
              <tr style={{borderBottom:"1.5px solid var(--ink)"}}>
                {["#","role","item","qty","rate","notes"].map((h,i)=>(
                  <th key={i} style={{
                    textAlign: i===2 ? "left" : i>=3 ? "right" : "left",
                    padding:"5px 8px",
                    fontFamily:"var(--font-label)", fontWeight:400,
                    color:"var(--ink-soft)",
                    whiteSpace:"nowrap",
                  }}>{h}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {[
                {role:"in",  item:"iron.plate",  icon:"◆", qty:3, rate:"15.0 / s", notes:"hot"},
                {role:"in",  item:"coal.coke",   icon:"◆", qty:1, rate:"5.0 / s",  notes:"reducer"},
                {role:"in",  item:"oxygen",      icon:"◉", qty:1000, rate:"5.0 kL/s", notes:"fluid"},
                {role:"out", item:"steel.plate", icon:"▤", qty:4, rate:"20.0 / s", notes:"primary"},
                {role:"out", item:"slag.dust",   icon:"·", qty:1, rate:"0.5 / s",  notes:"byproduct · 50%"},
              ].map((r,i)=>{
                const isOut = r.role === "out";
                return (
                  <tr key={i} style={{borderBottom:"1px dashed var(--ink-faint)"}}>
                    <td style={{padding:"4px 8px", color:"var(--ink-faint)"}}>{i+1}</td>
                    <td style={{padding:"4px 8px"}}>
                      <span className={`sk-tag ${isOut?"sk-on":""}`} style={{fontSize:9}}>
                        {isOut ? "out ◂" : "in ▸"}
                      </span>
                    </td>
                    <td style={{padding:"4px 8px"}}>
                      <Row gap={6} style={{alignItems:"center"}}>
                        <Slot style={{width:20,height:20}} filled icon={r.icon}/>
                        <span className="sk-mono-sm" style={{fontWeight: isOut?700:500}}>{r.item}</span>
                      </Row>
                    </td>
                    <td style={{padding:"4px 8px", textAlign:"right"}}>
                      <span className="sk-mono-sm">×{r.qty}</span>
                    </td>
                    <td style={{padding:"4px 8px", textAlign:"right"}}>
                      <span className="sk-mono-xs" style={{color:"var(--ink-soft)"}}>{r.rate}</span>
                    </td>
                    <td style={{padding:"4px 8px", textAlign:"right"}}>
                      <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>{r.notes}</span>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
          <Row gap={10} style={{
            padding:"8px 10px", borderTop:"1.5px solid var(--ink)",
            background:"var(--paper-2)", alignItems:"center", flexWrap:"wrap",
          }}>
            <Col gap={0}>
              <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>MACHINE</span>
              <Row gap={5} style={{alignItems:"center"}}>
                <Slot style={{width:18,height:18}} filled icon="▦"/>
                <span className="sk-mono-sm" style={{fontWeight:700}}>blast furnace</span>
              </Row>
            </Col>
            <span style={{color:"var(--ink-faint)"}}>·</span>
            <Col gap={0}>
              <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>TIER</span>
              <span className="sk-mono-sm" style={{fontWeight:700}}>LV3</span>
            </Col>
            <span style={{color:"var(--ink-faint)"}}>·</span>
            <Col gap={0}>
              <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>TIME</span>
              <span className="sk-mono-sm" style={{fontWeight:700}}>12.0 s</span>
            </Col>
            <span style={{color:"var(--ink-faint)"}}>·</span>
            <Col gap={0}>
              <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>POWER</span>
              <span className="sk-mono-sm" style={{fontWeight:700}}>480 EU/t</span>
            </Col>
            <span style={{color:"var(--ink-faint)"}}>·</span>
            <Col gap={0}>
              <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>YIELD</span>
              <span className="sk-mono-sm" style={{fontWeight:700}}>×4</span>
            </Col>
          </Row>
        </div>
        <Row gap={6}>
          <button className="sk-btn sk-accent">▶ auto-craft</button>
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

window.LookupV2 = LookupV2;
