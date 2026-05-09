/* global React, Slot, Row, Col */

// =====================================================================
// AUTOCRAFT — CPU monitor (process-list / htop-style)
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

window.AutocraftV4 = AutocraftV4;
