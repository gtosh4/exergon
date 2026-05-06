// planner.jsx — production planner wireframes
// 4 variations, all hybrids of SANKEY + COCKPIT (graph + deep inspector).
// Shared sketch primitives come from wireframe.css (.sk-box, .sk-slot, etc.)

// ─── shared data: same recipe graph across all variations ───────────────────
const PLAN = {
  goal: { item:"ferro-laminate", rate:60 },
  // columns of nodes (raw → goal)
  cols: [
    [
      { id:"ore",  label:"raw ore",  count:6, machine:"Crusher T2", rate:75 },
      { id:"sand", label:"sand",     count:1, machine:"Crusher T1", rate:4, alert:true, demand:6 },
    ],
    [
      { id:"cu",   label:"copper",   count:3, machine:"Smelter T2", rate:15 },
      { id:"pig",  label:"pig ingot",count:4, machine:"Smelter T2", rate:30 },
      { id:"coke", label:"coke",     count:2, machine:"Coker T2",   rate:12 },
      { id:"flux", label:"flux",     count:1, machine:"Reactor T2", rate:20, fluid:true },
    ],
    [
      { id:"wire", label:"copper wire",count:2, machine:"Bench T2",      rate:30 },
      { id:"steel",label:"steel",      count:5, machine:"Converter T3",  rate:60 },
    ],
    [
      { id:"plate",label:"ferro-laminate",count:8, machine:"Press T3",   rate:60, goal:true },
    ],
  ],
  ribbons: [
    ["ore","cu",   15],
    ["ore","pig",  30],
    ["ore","coke", 30],
    ["sand","flux", 4, true],
    ["cu","wire",  15],
    ["pig","steel",30],
    ["coke","steel",12],
    ["flux","steel",20],
    ["wire","plate",30],
    ["steel","plate",60],
  ],
};

function nodeOf(id){
  for (const col of PLAN.cols) for (const n of col) if (n.id===id) return n;
}
function colOf(id){
  for (let i=0;i<PLAN.cols.length;i++) for (const n of PLAN.cols[i]) if (n.id===id) return i;
}

function mapWidth(rate, mode){
  if (mode==="sqrt") return Math.sqrt(rate) * 6;
  if (mode==="log")  return Math.log2(rate+1) * 8;
  return Math.max(3, rate * 0.6);
}

const ITEM_GLYPH = { ore:"◇", sand:"⋮", cu:"▥", pig:"▣", coke:"◍", flux:"≈",
                     wire:"〰", steel:"▤", plate:"▨" };

// ─── tiny visual helpers (reuse style from wireframe.css) ───────────────────
function Tag({ children, on=false, accent=false, alert=false }){
  return <span className={`sk-tag ${on?"sk-on":""} ${accent?"sk-accent":""}`}
    style={alert?{ background:"#a31919", color:"#fff", borderColor:"#a31919" }:undefined}>{children}</span>;
}
function HLabel({ children, sm=false, xs=false }){
  return <div className={`sk-h ${sm?"sk-h-sm":""} ${xs?"sk-h-xs":""}`}>{children}</div>;
}
function Slot({ icon, size=28, fluid=false, alert=false }){
  return (
    <div className={`sk-slot sk-filled ${alert?"sk-alert":""}`}
         style={{ width:size, height:size, borderRadius: fluid? "50%": 2 }}>
      <span className="sk-icon" style={{ fontSize: Math.round(size*0.55) }}>{icon}</span>
    </div>
  );
}
function Bar({ pct=70, alert=false, height=6 }){
  return (
    <div className="sk-bar" style={{ height, background:"var(--paper-2)", borderColor: alert? "#a31919" : "var(--ink)" }}>
      <i style={{ width: `${Math.min(100,pct)}%`,
                  background: alert
                    ? "repeating-linear-gradient(45deg, #a31919 0 2px, transparent 2px 5px)"
                    : "repeating-linear-gradient(45deg, var(--ink) 0 2px, transparent 2px 5px)"}}/>
    </div>
  );
}
function Row({ k,v, alert=false }){
  return (
    <div style={{ display:"flex", justifyContent:"space-between", padding:"2px 0" }}>
      <span className="sk-mono-sm" style={{ color:"var(--ink-soft)" }}>{k}</span>
      <span className="sk-mono-sm" style={{ color: alert? "#a31919":"var(--ink)" }}>{v}</span>
    </div>
  );
}
const inputStyle = {
  border:"1.5px solid var(--ink)", background:"var(--paper)",
  padding:"2px 6px", fontSize:12, fontFamily:"var(--font-mono)", borderRadius:0
};

// ════════════════════════════════════════════════════════════════════════════
// SANKEY core renderer — used by all 4 variations w/ different layouts
// ════════════════════════════════════════════════════════════════════════════
function Sankey({ width, height, selected, onSelect, mapMode="linear", colCount=4, dense=false, padX=24, padY=24 }){
  const cols = PLAN.cols;
  const colW = dense? 150: 200;
  const innerW = width - padX*2;
  const colX = cols.map((_,i)=> padX + (innerW - colW) * (i/(colCount-1)));

  const nodeH = (n) => Math.max(50, mapWidth(n.rate, mapMode) * 1.3 + 32);

  function colYs(idx){
    const list = cols[idx];
    const totalH = list.reduce((s,n)=>s + nodeH(n) + 14, -14);
    let y = (height - totalH) / 2;
    return list.map(n=>{ const top = y; y += nodeH(n) + 14; return top; });
  }
  const ys = cols.map((_,i)=>colYs(i));

  return (
    <svg viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none"
         style={{ width:"100%", height:"100%", display:"block" }}>
      <defs>
        <pattern id="hatch" patternUnits="userSpaceOnUse" width="6" height="6" patternTransform="rotate(45)">
          <line x1="0" y1="0" x2="0" y2="6" stroke="var(--ink)" strokeWidth="1.5" opacity="0.55"/>
        </pattern>
        <pattern id="hatch-alert" patternUnits="userSpaceOnUse" width="6" height="6" patternTransform="rotate(45)">
          <line x1="0" y1="0" x2="0" y2="6" stroke="#a31919" strokeWidth="1.5" opacity="0.7"/>
        </pattern>
        <pattern id="hatch-sel" patternUnits="userSpaceOnUse" width="6" height="6" patternTransform="rotate(45)">
          <line x1="0" y1="0" x2="0" y2="6" stroke="#b88a00" strokeWidth="2" opacity="0.95"/>
        </pattern>
      </defs>

      {/* ribbons */}
      {PLAN.ribbons.map((r,i)=>{
        const [a,b,rate,alert] = r;
        const ca = colOf(a), cb = colOf(b);
        if (ca===undefined||cb===undefined) return null;
        const ia = cols[ca].findIndex(n=>n.id===a);
        const ib = cols[cb].findIndex(n=>n.id===b);
        const A = cols[ca][ia], B = cols[cb][ib];
        const yA = ys[ca][ia] + nodeH(A)/2;
        const yB = ys[cb][ib] + nodeH(B)/2;
        const xA = colX[ca] + colW;
        const xB = colX[cb];
        const tw = mapWidth(rate, mapMode);
        const dx = (xB - xA) * 0.5;
        const top = `M${xA},${yA - tw/2} C${xA+dx},${yA - tw/2} ${xB-dx},${yB - tw/2} ${xB},${yB - tw/2}`;
        const bot = `L${xB},${yB + tw/2} C${xB-dx},${yB + tw/2} ${xA+dx},${yA + tw/2} ${xA},${yA + tw/2} Z`;
        const isSelEdge = selected && (selected===a || selected===b);
        const fill = alert? "url(#hatch-alert)" : isSelEdge? "url(#hatch-sel)" : "url(#hatch)";
        const stroke = alert? "#a31919" : isSelEdge? "#b88a00" : "var(--ink)";
        return (
          <g key={i} style={{ pointerEvents:"none" }}>
            <path d={top + " " + bot} fill={fill} stroke={stroke} strokeWidth="1"/>
            {tw > 8 && (
              <g transform={`translate(${(xA+xB)/2}, ${(yA+yB)/2})`}>
                <rect x="-22" y="-8" width="44" height="16" fill="var(--paper)" stroke="var(--ink)" strokeWidth="1"/>
                <text textAnchor="middle" y="4" fontFamily="JetBrains Mono" fontSize="10"
                      fill={alert?"#a31919":"var(--ink)"}>{rate}/s</text>
              </g>
            )}
          </g>
        );
      })}

      {/* nodes */}
      {cols.map((list, ci) => list.map((n, ni) => {
        const y = ys[ci][ni];
        const h = nodeH(n);
        const sel = selected === n.id;
        return (
          <g key={n.id} transform={`translate(${colX[ci]}, ${y})`}
             onClick={()=>onSelect && onSelect(n.id)} style={{ cursor:"pointer" }}>
            <rect width={colW} height={h}
                  fill={sel?"var(--paper-2)":n.goal?"var(--paper-2)":"var(--paper)"}
                  stroke={n.alert?"#a31919":sel?"#b88a00":"var(--ink)"}
                  strokeWidth={sel?3:n.goal?2.5:1.5}
                  strokeDasharray={sel?"6 4":""}/>
            <text x="10" y="20" fontFamily="var(--font-hand)" fontWeight="700"
                  fontSize={dense?16:18} fill="var(--ink)">{n.label}</text>
            {!dense && (
              <text x="10" y={36} fontFamily="JetBrains Mono" fontSize="9" fill="var(--ink-soft)">{n.machine}</text>
            )}
            <text x={colW-10} y="20" textAnchor="end" fontFamily="var(--font-hand)" fontWeight="700"
                  fontSize={dense?16:20} fill="var(--ink)">×{n.count}</text>
            <text x={colW-10} y={36} textAnchor="end" fontFamily="JetBrains Mono" fontSize="9"
                  fill={n.alert?"#a31919":"var(--ink-soft)"}>{n.rate}/s{n.alert?" ⚠":""}</text>
            {h > 60 && (
              <g transform={`translate(10, ${h-16})`}>
                <rect width={colW-20} height="6" fill="none" stroke="var(--ink)" strokeWidth="1"/>
                <rect width={(colW-20)*(n.alert?0.4:0.92)} height="6"
                      fill={n.alert?"#a31919":"var(--ink)"} opacity="0.85"/>
              </g>
            )}
          </g>
        );
      }))}
    </svg>
  );
}

// ════════════════════════════════════════════════════════════════════════════
// V1 — SANKEY + COCKPIT (side-by-side, the "obvious" combination)
// ════════════════════════════════════════════════════════════════════════════
function PlannerV1(){
  const t = (typeof window!=="undefined" && window.__plannerTweaks) || {};
  const [sel, setSel] = React.useState("steel");

  return (
    <div className="paper" style={{ height:"100%", display:"flex", flexDirection:"column" }}>
      <PlannerTopbar mode="SANKEY · COCKPIT" goal="60.0/s ferro-laminate" />

      <div style={{ flex:1, display:"grid", gridTemplateColumns:"56px 1fr 380px", overflow:"hidden" }}>
        <PlannerLeftRail/>

        <div style={{ position:"relative", background:"var(--paper)" }}>
          <Sankey width={1000} height={780} selected={sel} onSelect={setSel} mapMode={t.sankeyMap||"linear"}/>
          <div className="sk-annot" style={{ left:16, top:14 }}>
            ribbon width = items/sec<br/>
            <span style={{ color:"#a31919" }}>red hatch = bottleneck</span>
          </div>
          <div className="sk-annot" style={{ right:16, bottom:18, textAlign:"right" }}>
            click any node →<br/>inspector on right
          </div>
        </div>

        <Inspector selectedId={sel}/>
      </div>

      <PlannerStatusbar tip="click sankey node = inspect · ⌘B balance · ⌘L lock · ⌘M sweep modules" />
    </div>
  );
}

// ════════════════════════════════════════════════════════════════════════════
// V2 — FULL-WIDTH SANKEY + INSPECTOR DRAWER (sankey is the canvas)
//      Inspector slides in over the right edge with a mini "local-sankey"
//      showing only the selected node's immediate ribbons.
// ════════════════════════════════════════════════════════════════════════════
function PlannerV2(){
  const t = (typeof window!=="undefined" && window.__plannerTweaks) || {};
  const [sel, setSel] = React.useState("steel");
  const [drawer, setDrawer] = React.useState(true);

  return (
    <div className="paper" style={{ height:"100%", display:"flex", flexDirection:"column" }}>
      <PlannerTopbar mode="SANKEY · DRAWER" goal="60.0/s ferro-laminate"/>

      <div style={{ flex:1, display:"grid", gridTemplateColumns:"56px 1fr", overflow:"hidden", position:"relative" }}>
        <PlannerLeftRail/>

        <div style={{ position:"relative", background:"var(--paper)" }}>
          <Sankey width={1380} height={780} selected={sel} onSelect={(id)=>{setSel(id); setDrawer(true);}}
                  mapMode={t.sankeyMap||"linear"}/>

          {/* drawer */}
          {drawer && (
            <div className="sk-box" style={{
              position:"absolute", top:14, right:14, bottom:14, width:420,
              background:"var(--paper-2)", borderColor:"var(--ink)",
              boxShadow:"-3px 3px 0 var(--ink)",
              display:"flex", flexDirection:"column", overflow:"hidden"
            }}>
              {/* drawer header */}
              <div style={{ padding:"10px 12px", borderBottom:"1.5px solid var(--ink)",
                            display:"flex", alignItems:"center", gap:8 }}>
                <span className="sk-mono-xs" style={{ color:"var(--ink-soft)", letterSpacing:2 }}>INSPECTING</span>
                <span style={{ flex:1 }}/>
                <button className="sk-btn" style={{ padding:"1px 6px", fontSize:11, boxShadow:"none" }}
                        onClick={()=>setDrawer(false)}>× close (esc)</button>
              </div>

              {/* mini local sankey for selected node */}
              <div style={{ padding:10, borderBottom:"1.5px dashed var(--ink)" }}>
                <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", letterSpacing:2, marginBottom:6 }}>LOCAL FLOW</div>
                <LocalSankey nodeId={sel} mapMode={t.sankeyMap||"linear"}/>
              </div>

              {/* full inspector below */}
              <div style={{ flex:1, overflow:"auto" }}>
                <InspectorBody selectedId={sel}/>
              </div>
            </div>
          )}

          {!drawer && (
            <button className="sk-btn" style={{ position:"absolute", top:14, right:14, fontSize:11 }}
                    onClick={()=>setDrawer(true)}>← inspect {nodeOf(sel)?.label}</button>
          )}
        </div>
      </div>

      <PlannerStatusbar tip="esc close drawer · / search · drag drawer to dock left"/>
    </div>
  );
}

function LocalSankey({ nodeId, mapMode }){
  const node = nodeOf(nodeId);
  if (!node) return null;
  const ins  = PLAN.ribbons.filter(r=>r[1]===nodeId);
  const outs = PLAN.ribbons.filter(r=>r[0]===nodeId);
  const W = 380, rowH = 28;
  const total = Math.max(ins.length, outs.length, 1);
  const H = total * rowH + 50;
  return (
    <svg viewBox={`0 0 ${W} ${H}`} style={{ width:"100%", height:H, display:"block" }}>
      <defs>
        <pattern id="lh" patternUnits="userSpaceOnUse" width="6" height="6" patternTransform="rotate(45)">
          <line x1="0" y1="0" x2="0" y2="6" stroke="var(--ink)" strokeWidth="1.5" opacity="0.55"/>
        </pattern>
      </defs>
      {/* center node */}
      <rect x={W/2-50} y={H/2-22} width="100" height="44" fill="var(--paper-2)" stroke="#b88a00" strokeWidth="2.5"/>
      <text x={W/2} y={H/2-4} textAnchor="middle" fontFamily="var(--font-hand)" fontWeight="700" fontSize="16">{node.label}</text>
      <text x={W/2} y={H/2+12} textAnchor="middle" fontFamily="JetBrains Mono" fontSize="9" fill="var(--ink-soft)">{node.rate}/s · ×{node.count}</text>

      {/* ins on left */}
      {ins.map((r,i)=>{
        const yA = 30 + i*rowH;
        const yB = H/2;
        const tw = Math.max(3, mapWidth(r[2], mapMode)*0.6);
        const top = `M0,${yA - tw/2} C${W/3},${yA - tw/2} ${W/3},${yB - tw/2} ${W/2-50},${yB - tw/2}`;
        const bot = `L${W/2-50},${yB + tw/2} C${W/3},${yB + tw/2} ${W/3},${yA + tw/2} 0,${yA + tw/2} Z`;
        return (
          <g key={"i"+i}>
            <path d={top+" "+bot} fill={r[3]?"#a31919":"url(#lh)"} stroke={r[3]?"#a31919":"var(--ink)"} strokeWidth="1"/>
            <text x="6" y={yA-4} fontFamily="JetBrains Mono" fontSize="10">{nodeOf(r[0])?.label}</text>
            <text x="6" y={yA+12} fontFamily="JetBrains Mono" fontSize="9" fill={r[3]?"#a31919":"var(--ink-soft)"}>{r[2]}/s {r[3]?"⚠":""}</text>
          </g>
        );
      })}
      {/* outs on right */}
      {outs.map((r,i)=>{
        const yA = H/2;
        const yB = 30 + i*rowH;
        const tw = Math.max(3, mapWidth(r[2], mapMode)*0.6);
        const top = `M${W/2+50},${yA - tw/2} C${W*2/3},${yA - tw/2} ${W*2/3},${yB - tw/2} ${W},${yB - tw/2}`;
        const bot = `L${W},${yB + tw/2} C${W*2/3},${yB + tw/2} ${W*2/3},${yA + tw/2} ${W/2+50},${yA + tw/2} Z`;
        return (
          <g key={"o"+i}>
            <path d={top+" "+bot} fill="url(#lh)" stroke="var(--ink)" strokeWidth="1"/>
            <text x={W-6} y={yB-4} textAnchor="end" fontFamily="JetBrains Mono" fontSize="10">{nodeOf(r[1])?.label}</text>
            <text x={W-6} y={yB+12} textAnchor="end" fontFamily="JetBrains Mono" fontSize="9" fill="var(--ink-soft)">{r[2]}/s</text>
          </g>
        );
      })}
    </svg>
  );
}

// ════════════════════════════════════════════════════════════════════════════
// V3 — DUAL-SANKEY (overview top + zoomed sub-sankey bottom + slim inspector)
// ════════════════════════════════════════════════════════════════════════════
function PlannerV3(){
  const t = (typeof window!=="undefined" && window.__plannerTweaks) || {};
  const [sel, setSel] = React.useState("steel");

  return (
    <div className="paper" style={{ height:"100%", display:"flex", flexDirection:"column" }}>
      <PlannerTopbar mode="DUAL · SANKEY" goal="60.0/s ferro-laminate"/>

      <div style={{ flex:1, display:"grid", gridTemplateColumns:"56px 1fr 320px", overflow:"hidden" }}>
        <PlannerLeftRail/>

        <div style={{ display:"grid", gridTemplateRows:"1fr 8px 1fr", background:"var(--paper)", overflow:"hidden" }}>
          {/* overview */}
          <div style={{ position:"relative", borderBottom:"1.5px solid var(--ink)" }}>
            <div className="sk-mono-xs" style={{ position:"absolute", top:8, left:10, letterSpacing:2,
                                                 color:"var(--ink-soft)", zIndex:1 }}>OVERVIEW · whole plan</div>
            <Sankey width={1080} height={380} selected={sel} onSelect={setSel}
                    mapMode={t.sankeyMap||"linear"} dense/>
            {/* highlight box around the selected branch */}
            <div className="sk-annot" style={{ right:14, top:8 }}>
              click → drills bottom view
            </div>
          </div>

          <div style={{ background:"var(--paper-2)", borderTop:"1.5px solid var(--ink)",
                        borderBottom:"1.5px solid var(--ink)", display:"flex", alignItems:"center",
                        padding:"0 10px", fontFamily:"var(--font-mono)", fontSize:9, color:"var(--ink-soft)",
                        letterSpacing:2 }}>
            DRILL ▾ · {nodeOf(sel)?.label} branch
          </div>

          {/* zoomed branch */}
          <div style={{ position:"relative" }}>
            <BranchSankey nodeId={sel} mapMode={t.sankeyMap||"linear"}/>
          </div>
        </div>

        {/* slim inspector */}
        <SlimInspector selectedId={sel}/>
      </div>

      <PlannerStatusbar tip="↑↓ ←→ pan branches · 0 reset · ⌘B balance"/>
    </div>
  );
}

// only ribbons in the upstream cone of nodeId, plus the node itself + immediate downstream
function BranchSankey({ nodeId, mapMode }){
  const upstream = new Set([nodeId]);
  // walk all ancestors
  let frontier = [nodeId];
  while (frontier.length){
    const next = [];
    for (const id of frontier){
      for (const r of PLAN.ribbons){
        if (r[1]===id && !upstream.has(r[0])) { upstream.add(r[0]); next.push(r[0]); }
      }
    }
    frontier = next;
  }
  // and one downstream hop
  for (const r of PLAN.ribbons) if (r[0]===nodeId) upstream.add(r[1]);

  const filteredCols = PLAN.cols.map(col => col.filter(n=>upstream.has(n.id))).filter(c=>c.length);
  const ribbons = PLAN.ribbons.filter(r=>upstream.has(r[0]) && upstream.has(r[1]));

  const W = 1080, H = 380;
  const colW = 180;
  const colCount = filteredCols.length;
  const colX = filteredCols.map((_,i)=> 24 + (W-48-colW) * (colCount<2?0:i/(colCount-1)));
  const nodeH = (n)=>Math.max(50, mapWidth(n.rate, mapMode)*1.4 + 32);
  const ys = filteredCols.map(col=>{
    const total = col.reduce((s,n)=>s+nodeH(n)+14, -14);
    let y = (H-total)/2;
    return col.map(n=>{ const top=y; y+=nodeH(n)+14; return top; });
  });

  function loc(id){
    for (let ci=0; ci<filteredCols.length; ci++){
      const ni = filteredCols[ci].findIndex(n=>n.id===id);
      if (ni>=0) return { ci, ni, n:filteredCols[ci][ni] };
    }
  }

  return (
    <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" style={{ width:"100%", height:"100%", display:"block" }}>
      <defs>
        <pattern id="bh" patternUnits="userSpaceOnUse" width="6" height="6" patternTransform="rotate(45)">
          <line x1="0" y1="0" x2="0" y2="6" stroke="#b88a00" strokeWidth="2" opacity="0.9"/>
        </pattern>
        <pattern id="bha" patternUnits="userSpaceOnUse" width="6" height="6" patternTransform="rotate(45)">
          <line x1="0" y1="0" x2="0" y2="6" stroke="#a31919" strokeWidth="2" opacity="0.9"/>
        </pattern>
      </defs>
      {ribbons.map((r,i)=>{
        const A = loc(r[0]), B = loc(r[1]);
        if (!A||!B) return null;
        const yA = ys[A.ci][A.ni] + nodeH(A.n)/2;
        const yB = ys[B.ci][B.ni] + nodeH(B.n)/2;
        const xA = colX[A.ci] + colW;
        const xB = colX[B.ci];
        const tw = Math.max(4, mapWidth(r[2], mapMode));
        const dx = (xB - xA) * 0.5;
        const top = `M${xA},${yA - tw/2} C${xA+dx},${yA - tw/2} ${xB-dx},${yB - tw/2} ${xB},${yB - tw/2}`;
        const bot = `L${xB},${yB + tw/2} C${xB-dx},${yB + tw/2} ${xA+dx},${yA + tw/2} ${xA},${yA + tw/2} Z`;
        return (
          <g key={i}>
            <path d={top+" "+bot} fill={r[3]?"url(#bha)":"url(#bh)"} stroke={r[3]?"#a31919":"#b88a00"} strokeWidth="1"/>
            <g transform={`translate(${(xA+xB)/2}, ${(yA+yB)/2})`}>
              <rect x="-22" y="-9" width="44" height="18" fill="var(--paper)" stroke="var(--ink)" strokeWidth="1"/>
              <text textAnchor="middle" y="4" fontFamily="JetBrains Mono" fontSize="10" fill={r[3]?"#a31919":"var(--ink)"}>{r[2]}/s</text>
            </g>
          </g>
        );
      })}
      {filteredCols.map((col,ci)=>col.map((n,ni)=>{
        const y = ys[ci][ni], h = nodeH(n);
        const sel = n.id===nodeId;
        return (
          <g key={n.id} transform={`translate(${colX[ci]}, ${y})`}>
            <rect width={colW} height={h}
                  fill={sel?"var(--paper-2)":"var(--paper)"}
                  stroke={n.alert?"#a31919":sel?"#b88a00":"var(--ink)"}
                  strokeWidth={sel?3:1.5}/>
            <text x="10" y="20" fontFamily="var(--font-hand)" fontWeight="700" fontSize="18">{n.label}</text>
            <text x="10" y="36" fontFamily="JetBrains Mono" fontSize="9" fill="var(--ink-soft)">{n.machine}</text>
            <text x={colW-10} y="20" textAnchor="end" fontFamily="var(--font-hand)" fontWeight="700" fontSize="20">×{n.count}</text>
            <text x={colW-10} y="36" textAnchor="end" fontFamily="JetBrains Mono" fontSize="9" fill={n.alert?"#a31919":"var(--ink-soft)"}>{n.rate}/s{n.alert?" ⚠":""}</text>
            {h>60 && (
              <g transform={`translate(10, ${h-16})`}>
                <rect width={colW-20} height="6" fill="none" stroke="var(--ink)"/>
                <rect width={(colW-20)*(n.alert?0.4:0.92)} height="6" fill={n.alert?"#a31919":"var(--ink)"} opacity="0.85"/>
              </g>
            )}
          </g>
        );
      }))}
    </svg>
  );
}

// ════════════════════════════════════════════════════════════════════════════
// V4 — SANKEY-SPINE: vertical sankey (goal at top, raws at bottom),
//      inspector docks on the right with a connected "umbilical" annotation.
// ════════════════════════════════════════════════════════════════════════════
function PlannerV4(){
  const t = (typeof window!=="undefined" && window.__plannerTweaks) || {};
  const [sel, setSel] = React.useState("flux"); // pre-select the bottleneck-adjacent node

  return (
    <div className="paper" style={{ height:"100%", display:"flex", flexDirection:"column" }}>
      <PlannerTopbar mode="SPINE" goal="60.0/s ferro-laminate"/>

      <div style={{ flex:1, display:"grid", gridTemplateColumns:"56px 1fr 360px", overflow:"hidden" }}>
        <PlannerLeftRail/>

        <div style={{ position:"relative", background:"var(--paper)", overflow:"hidden" }}>
          <VerticalSankey selected={sel} onSelect={setSel} mapMode={t.sankeyMap||"linear"}/>

          {/* umbilical overlay — drawn from the selected node toward the inspector edge */}
          <svg style={{ position:"absolute", inset:0, width:"100%", height:"100%", pointerEvents:"none" }}>
            <UmbilicalLine selectedId={sel}/>
          </svg>

          <div className="sk-annot" style={{ left:18, top:14 }}>
            ↑ goal flows up<br/>raws ↓ feed in
          </div>
        </div>

        <Inspector selectedId={sel}/>
      </div>

      <PlannerStatusbar tip="ribbons flow upward · inspector tracks selection · ⌘M sweep modules"/>
    </div>
  );
}

function VerticalSankey({ selected, onSelect, mapMode }){
  const cols = PLAN.cols; // tier rows in the vertical layout (col 0 = bottom, last = top)
  const W = 980, H = 770;
  const rowH = 130;
  const totalRows = cols.length;
  const padTop = 30, padBottom = 30;
  const usableH = H - padTop - padBottom;
  const rowY = cols.map((_,i)=> padTop + (totalRows>1? (usableH - rowH) * ((totalRows-1-i)/(totalRows-1)): usableH/2 - rowH/2));
  const nodeW = (n) => Math.max(56, mapWidth(n.rate, mapMode)*1.3 + 30);

  function rowXs(idx){
    const list = cols[idx];
    const total = list.reduce((s,n)=>s+nodeW(n)+18, -18);
    let x = (W-total)/2;
    return list.map(n=>{ const left=x; x+=nodeW(n)+18; return left; });
  }
  const xs = cols.map((_,i)=>rowXs(i));

  return (
    <svg viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" style={{ width:"100%", height:"100%", display:"block" }}>
      <defs>
        <pattern id="vh" patternUnits="userSpaceOnUse" width="6" height="6" patternTransform="rotate(135)">
          <line x1="0" y1="0" x2="0" y2="6" stroke="var(--ink)" strokeWidth="1.5" opacity="0.55"/>
        </pattern>
        <pattern id="vha" patternUnits="userSpaceOnUse" width="6" height="6" patternTransform="rotate(135)">
          <line x1="0" y1="0" x2="0" y2="6" stroke="#a31919" strokeWidth="1.5" opacity="0.7"/>
        </pattern>
        <pattern id="vhs" patternUnits="userSpaceOnUse" width="6" height="6" patternTransform="rotate(135)">
          <line x1="0" y1="0" x2="0" y2="6" stroke="#b88a00" strokeWidth="2" opacity="0.95"/>
        </pattern>
      </defs>

      {/* ribbons go upward (a in lower row → b in upper row) */}
      {PLAN.ribbons.map((r,i)=>{
        const ca = colOf(r[0]), cb = colOf(r[1]);
        const ia = cols[ca].findIndex(n=>n.id===r[0]);
        const ib = cols[cb].findIndex(n=>n.id===r[1]);
        const A = cols[ca][ia], B = cols[cb][ib];
        const xA = xs[ca][ia] + nodeW(A)/2;
        const xB = xs[cb][ib] + nodeW(B)/2;
        const yA = rowY[ca];           // top of A (since A is below B and ribbon goes up)
        const yB = rowY[cb] + rowH;    // bottom of B
        const tw = Math.max(4, mapWidth(r[2], mapMode));
        const dy = (yA - yB) * 0.5;
        const isSel = selected===r[0]||selected===r[1];
        const fill = r[3]?"url(#vha)":isSel?"url(#vhs)":"url(#vh)";
        const stroke = r[3]?"#a31919":isSel?"#b88a00":"var(--ink)";
        const left  = `M${xA - tw/2},${yA} C${xA - tw/2},${yA-dy} ${xB - tw/2},${yB+dy} ${xB - tw/2},${yB}`;
        const right = `L${xB + tw/2},${yB} C${xB + tw/2},${yB+dy} ${xA + tw/2},${yA-dy} ${xA + tw/2},${yA} Z`;
        return (
          <g key={i}>
            <path d={left+" "+right} fill={fill} stroke={stroke} strokeWidth="1"/>
          </g>
        );
      })}

      {/* nodes */}
      {cols.map((row, ci)=>row.map((n, ni)=>{
        const x = xs[ci][ni], y = rowY[ci];
        const w = nodeW(n);
        const sel = selected === n.id;
        return (
          <g key={n.id} transform={`translate(${x}, ${y})`}
             data-node-id={n.id}
             onClick={()=>onSelect && onSelect(n.id)} style={{ cursor:"pointer" }}>
            <rect width={w} height={rowH}
                  fill={sel?"var(--paper-2)":n.goal?"var(--paper-2)":"var(--paper)"}
                  stroke={n.alert?"#a31919":sel?"#b88a00":"var(--ink)"}
                  strokeWidth={sel?3:n.goal?2.5:1.5}
                  strokeDasharray={sel?"6 4":""}/>
            <text x={w/2} y="22" textAnchor="middle" fontFamily="var(--font-hand)" fontWeight="700" fontSize="18">{n.label}</text>
            <text x={w/2} y="40" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="9" fill="var(--ink-soft)">{n.machine}</text>
            <text x={w/2} y="76" textAnchor="middle" fontFamily="var(--font-hand)" fontWeight="700" fontSize="22">×{n.count}</text>
            <text x={w/2} y="94" textAnchor="middle" fontFamily="JetBrains Mono" fontSize="10" fill={n.alert?"#a31919":"var(--ink-soft)"}>{n.rate}/s{n.alert?" ⚠":""}</text>
            <g transform={`translate(8, ${rowH-16})`}>
              <rect width={w-16} height="6" fill="none" stroke="var(--ink)"/>
              <rect width={(w-16)*(n.alert?0.4:0.92)} height="6" fill={n.alert?"#a31919":"var(--ink)"} opacity="0.85"/>
            </g>
          </g>
        );
      }))}

      {/* tier labels on the side */}
      {cols.map((_,i)=>(
        <text key={i} x="14" y={rowY[i]+rowH/2} fontFamily="JetBrains Mono" fontSize="9" fill="var(--ink-faint)" letterSpacing="2">
          {["RAW","INTER","SUB","GOAL"][i]}
        </text>
      ))}
    </svg>
  );
}

// dashed line + handwritten note pointing from selected node to inspector edge
function UmbilicalLine({ selectedId }){
  // We don't actually have measured DOM coords here — draw a stylized arrow
  // from the right edge of the (assumed centered) sankey toward the right.
  return (
    <g>
      <path d="M 700 380 Q 880 360 980 360" stroke="#b88a00" strokeWidth="2" fill="none" strokeDasharray="4 4"/>
      <text x="780" y="350" fontFamily="var(--font-hand)" fontSize="14" fill="#b88a00">tracking →</text>
    </g>
  );
}

// ════════════════════════════════════════════════════════════════════════════
// INSPECTOR — full and slim variants
// ════════════════════════════════════════════════════════════════════════════
function Inspector({ selectedId }){
  return (
    <div className="sk-box" style={{ background:"var(--paper-2)", borderLeft:"1.5px solid var(--ink)",
                                     borderTop:"none", borderBottom:"none", borderRight:"none",
                                     boxShadow:"none", display:"flex", flexDirection:"column",
                                     overflow:"auto" }}>
      <InspectorHeader selectedId={selectedId}/>
      <InspectorBody selectedId={selectedId}/>
    </div>
  );
}

function InspectorHeader({ selectedId }){
  const n = nodeOf(selectedId);
  if (!n) return null;
  return (
    <div style={{ padding:12, borderBottom:"1.5px solid var(--ink)", display:"flex", flexDirection:"column", gap:4 }}>
      <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", letterSpacing:2 }}>
        INSPECTOR · {n.goal?"GOAL":n.alert?"BOTTLENECK":"NODE"}
      </div>
      <div style={{ display:"flex", alignItems:"center", gap:8 }}>
        <Slot icon={ITEM_GLYPH[selectedId]||"◇"} size={36} fluid={n.fluid}/>
        <div style={{ flex:1, display:"flex", flexDirection:"column" }}>
          <HLabel>{n.label}</HLabel>
          <span className="sk-mono-sm" style={{ color:"var(--ink-soft)" }}>{n.machine}</span>
        </div>
        {n.alert ? <Tag alert>LOW</Tag> : n.goal? <Tag accent>GOAL</Tag>: <Tag>OK</Tag>}
      </div>
    </div>
  );
}

function InspectorBody({ selectedId }){
  const n = nodeOf(selectedId);
  if (!n) return null;
  const ins = PLAN.ribbons.filter(r=>r[1]===selectedId);
  return (
    <>
      <Section label="RECIPE" right={<button className="sk-btn" style={{ padding:"1px 6px", fontSize:11 }}>swap (3 alts)</button>}>
        <div style={{ display:"flex", alignItems:"center", gap:6, flexWrap:"wrap" }}>
          {ins.length===0 && <span className="sk-mono-sm" style={{ color:"var(--ink-faint)" }}>(raw extraction — no inputs)</span>}
          {ins.map((r,i)=>(
            <React.Fragment key={i}>
              <Slot icon={ITEM_GLYPH[r[0]]||"◇"} size={28} fluid={nodeOf(r[0])?.fluid} alert={r[3]}/>
              <span className="sk-mono-sm" style={{ color:r[3]?"#a31919":"var(--ink)" }}>{r[2]}/s</span>
            </React.Fragment>
          ))}
          {ins.length>0 && <span className="sk-arrow">→</span>}
          <Slot icon={ITEM_GLYPH[selectedId]||"◇"} size={32} fluid={n.fluid}/>
          <span className="sk-mono-sm">{n.rate}/s</span>
        </div>
        <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:6 }}>
          base cycle 4.0s · machine speed 1.5× · effective {(n.rate/n.count).toFixed(1)}/s per unit
        </div>
      </Section>

      <Section label="THROUGHPUT">
        <div style={{ display:"flex", alignItems:"center", gap:8 }}>
          <span className="sk-mono-sm">target</span>
          <input className="sk-mono" defaultValue={n.rate.toFixed(1)} style={inputStyle}/>
          <select className="sk-mono" style={inputStyle} defaultValue="/s">
            <option>/s</option><option>/min</option><option>belts</option>
          </select>
          {n.goal && <Tag on>🔒 GOAL</Tag>}
        </div>
        <div style={{ display:"flex", gap:8, marginTop:8, alignItems:"center" }}>
          <span className="sk-mono-sm" style={{ minWidth:60 }}>machines</span>
          <input className="sk-mono" defaultValue={n.count} style={{...inputStyle, width:50}}/>
          <span className="sk-mono-xs" style={{ color:"var(--ink-soft)" }}>auto-solved · ⌘L to lock</span>
        </div>
        {n.alert && (
          <div className="sk-mono-xs" style={{ marginTop:8, padding:6, border:"1.5px solid #a31919",
                                               color:"#a31919", lineHeight:1.5, background:"var(--paper)" }}>
            ⚠ supply {n.rate}/s, demand {n.demand}/s · short {n.demand-n.rate}/s
            <button className="sk-btn sk-accent" style={{ marginTop:6, fontSize:11, padding:"1px 6px" }}>add ×1 machine</button>
          </div>
        )}
      </Section>

      <Section label="MODULES" right={<button className="sk-btn sk-accent" style={{ padding:"1px 6px", fontSize:11 }}>sweep ↺</button>}>
        <div style={{ display:"flex", gap:4, marginBottom:6 }}>
          {(n.alert? ["·","·","·","·"]: ["P","P","S","·"]).map((m,i)=>(
            <div key={i} className="sk-slot" style={{ width:30, height:30, background:"var(--paper)",
                                                      borderStyle:m==="·"?"dashed":"solid"}}>
              <span style={{ fontFamily:"var(--font-mono)", fontSize:13, fontWeight:700 }}>{m==="·"?"":m}</span>
            </div>
          ))}
        </div>
        <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", lineHeight:1.5 }}>
          {n.alert? "no modules installed" : "+30% productivity, +50% speed · +18 kW, +0.4 pollution/s"}
        </div>
        <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:8, padding:6,
                                             border:"1px dashed var(--ink)", lineHeight:1.5 }}>
          sweep: <strong>P/P/P/S</strong> saves ×1 machine, +12% power
        </div>
      </Section>

      <Section label="BEACONS">
        <div style={{ display:"flex", alignItems:"center", gap:8 }}>
          <span className="sk-mono-sm" style={{ flex:1 }}>nearby beacons</span>
          <input className="sk-mono" defaultValue={n.alert?"0":"2"} style={{...inputStyle, width:50}}/>
        </div>
        <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:4 }}>
          shared S/S → +60% effective speed
        </div>
      </Section>

      <Section label="POWER & FOOTPRINT">
        <Row k="draw"      v={`${n.count*36} kW`}/>
        <Row k="footprint" v={`${n.count*6} × ${n.count*4} tiles`}/>
        <Row k="pollution" v={`${(n.count*0.4).toFixed(1)} /s`}/>
      </Section>

      <div style={{ marginTop:"auto", padding:12, borderTop:"1.5px dashed var(--ink)",
                    display:"flex", gap:6, flexWrap:"wrap" }}>
        <button className="sk-btn">duplicate</button>
        <button className="sk-btn">isolate path</button>
        <button className="sk-btn">to sub-floor</button>
        <button className="sk-btn" style={{ marginLeft:"auto" }}>delete</button>
      </div>
    </>
  );
}

function SlimInspector({ selectedId }){
  const n = nodeOf(selectedId);
  if (!n) return null;
  return (
    <div style={{ borderLeft:"1.5px solid var(--ink)", background:"var(--paper-2)",
                  display:"flex", flexDirection:"column", overflow:"auto" }}>
      <InspectorHeader selectedId={selectedId}/>
      <Section label="QUICK">
        <Row k="machines" v={`×${n.count}`}/>
        <Row k="output" v={`${n.rate}/s`}/>
        <Row k="cycle" v="4.0s"/>
        <Row k="power" v={`${n.count*36} kW`}/>
      </Section>
      <Section label="MODULES">
        <div style={{ display:"flex", gap:3 }}>
          {(n.alert? ["·","·","·","·"]: ["P","P","S","·"]).map((m,i)=>(
            <div key={i} className="sk-slot" style={{ width:24, height:24, background:"var(--paper)",
                                                      borderStyle:m==="·"?"dashed":"solid"}}>
              <span style={{ fontFamily:"var(--font-mono)", fontSize:11, fontWeight:700 }}>{m==="·"?"":m}</span>
            </div>
          ))}
        </div>
        <button className="sk-btn sk-accent" style={{ marginTop:8, fontSize:11, padding:"2px 6px" }}>sweep ↺</button>
      </Section>
      {n.alert && (
        <Section label="⚠ BOTTLENECK">
          <div className="sk-mono-xs" style={{ color:"#a31919", lineHeight:1.5 }}>
            short {n.demand-n.rate}/s
          </div>
          <button className="sk-btn sk-accent" style={{ marginTop:6, fontSize:11, padding:"1px 6px", width:"100%", justifyContent:"center" }}>auto-fix</button>
        </Section>
      )}
      <div style={{ marginTop:"auto", padding:10, display:"flex", gap:4, flexWrap:"wrap",
                    borderTop:"1.5px dashed var(--ink)" }}>
        <button className="sk-btn" style={{ fontSize:11, padding:"2px 6px" }}>swap recipe</button>
        <button className="sk-btn" style={{ fontSize:11, padding:"2px 6px" }}>isolate</button>
      </div>
    </div>
  );
}

// ════════════════════════════════════════════════════════════════════════════
// RECIPE PICKER OVERLAY
// ════════════════════════════════════════════════════════════════════════════
function PlannerRecipePicker(){
  const cats = ["all","unlocked","smelting","chemistry","assembly","fluid","waste"];
  const recipes = [
    { name:"steel · BoF",        out:"steel ×1 / 4s",  in:["pig","coke","flux"], tier:"T3", unlocked:true,  default:true },
    { name:"steel · arc",        out:"steel ×3 / 12s", in:["pig","coke"],        tier:"T3", unlocked:true,  default:false },
    { name:"steel · clean (DRI)",out:"steel ×2 / 6s",  in:["pig","wire","flux"], tier:"T4", unlocked:false, default:false },
    { name:"steel · scrap",      out:"steel ×4 / 8s",  in:["coke","flux"],       tier:"T2", unlocked:true,  default:false },
    { name:"steel · molten",     out:"molten steel 8/s", in:["pig","coke"],      tier:"T4", unlocked:false, default:false },
  ];
  return (
    <div className="paper" style={{ height:"100%", display:"flex", flexDirection:"column", padding:0 }}>
      <PlannerTopbar mode="PICKER · alt-recipes" goal="select recipe for · steel" />

      <div style={{ flex:1, display:"grid", gridTemplateColumns:"220px 1fr 320px", overflow:"hidden" }}>
        <div style={{ borderRight:"1.5px solid var(--ink)", padding:12, display:"flex", flexDirection:"column", gap:8 }}>
          <input className="sk-mono" placeholder="/ search recipes…" style={{...inputStyle, width:"100%"}}/>
          <div style={{ display:"flex", flexDirection:"column", gap:4 }}>
            {cats.map((c,i)=>(
              <button key={c} className={`sk-btn ${i===1?"sk-on":""}`} style={{
                justifyContent:"flex-start", boxShadow:"none", padding:"4px 8px"
              }}>
                <span className="sk-mono-sm" style={{ flex:1, textAlign:"left" }}>{c}</span>
                <span className="sk-mono-xs" style={{ opacity:0.7 }}>{[5,4,7,3,6,2,2][i]}</span>
              </button>
            ))}
          </div>

          <div style={{ marginTop:12 }}>
            <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", letterSpacing:2, marginBottom:6 }}>FILTERS</div>
            <label style={lblRow}><input type="checkbox" defaultChecked/> unlocked only</label>
            <label style={lblRow}><input type="checkbox"/> show fluid recipes</label>
            <label style={lblRow}><input type="checkbox" defaultChecked/> consider modules</label>
          </div>
        </div>

        <div style={{ overflow:"auto" }}>
          <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", padding:"10px 14px", letterSpacing:2 }}>
            5 RECIPES PRODUCE STEEL
          </div>
          {recipes.map((r,i)=>(
            <div key={r.name} className="sk-box" style={{
              margin:"0 14px 10px", padding:12, display:"grid",
              gridTemplateColumns:"30px 1fr auto", gap:10, alignItems:"center",
              opacity: r.unlocked? 1: 0.5,
              background: r.default? "var(--paper-2)": "var(--paper)",
              borderWidth: r.default? 2.5:1.5
            }}>
              <span style={{ display:"flex", flexDirection:"column", alignItems:"center" }}>
                <input type="radio" name="rec" defaultChecked={r.default}/>
              </span>
              <div style={{ display:"flex", flexDirection:"column", gap:6 }}>
                <div style={{ display:"flex", gap:8, alignItems:"center" }}>
                  <HLabel sm>{r.name}</HLabel>
                  <Tag>{r.tier}</Tag>
                  {!r.unlocked && <Tag>🔒 locked</Tag>}
                  {r.default && <Tag accent>current</Tag>}
                </div>
                <div style={{ display:"flex", alignItems:"center", gap:6, flexWrap:"wrap" }}>
                  {r.in.map(it=>(
                    <Slot key={it} icon={ITEM_GLYPH[it]||"◇"} size={28}/>
                  ))}
                  <span className="sk-arrow">→</span>
                  <Slot icon="▤" size={32}/>
                  <span className="sk-mono-sm" style={{ color:"var(--ink-soft)" }}>· {r.out}</span>
                </div>
              </div>
              <button className="sk-btn" style={{ alignSelf:"start" }}>use →</button>
            </div>
          ))}

          <div className="sk-box sk-dashed" style={{ margin:"10px 14px", padding:14, textAlign:"center" }}>
            <span className="sk-mono-sm" style={{ color:"var(--ink-soft)" }}>
              drag any recipe onto the canvas to instantiate · or press <Tag>↵</Tag> to use selected
            </span>
          </div>
        </div>

        <div style={{ borderLeft:"1.5px solid var(--ink)", padding:14,
                      display:"flex", flexDirection:"column", gap:12, background:"var(--paper-2)" }}>
          <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", letterSpacing:2 }}>COMPARE · @ 60/s</div>
          <div>
            <HLabel sm>BoF (current)</HLabel>
            <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:6, lineHeight:1.7 }}>
              <Row k="machines"  v="×5 Converter T3"/>
              <Row k="raw inputs" v="ore 75/s · sand 4/s"/>
              <Row k="power"      v="180 kW"/>
              <Row k="pollution"  v="2.0 /s"/>
            </div>
          </div>
          <div className="sk-div"/>
          <div>
            <HLabel sm>arc (alt)</HLabel>
            <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:6, lineHeight:1.7 }}>
              <Row k="machines"  v="×3 Arc T3"/>
              <Row k="raw inputs" v="ore 60/s (no flux)"/>
              <Row k="power"      v="320 kW"/>
              <Row k="pollution"  v="3.4 /s"/>
            </div>
            <div className="sk-mono-xs" style={{ marginTop:8, padding:6, border:"1px dashed var(--ink)", lineHeight:1.5 }}>
              switching saves <strong>×2 machines</strong> &amp; flux line, costs <strong>+78%</strong> power
            </div>
          </div>

          <div style={{ marginTop:"auto", display:"flex", gap:6 }}>
            <button className="sk-btn sk-accent" style={{ flex:1 }}>apply selected</button>
            <button className="sk-btn">cancel</button>
          </div>
        </div>
      </div>

      <PlannerStatusbar tip="↑↓ pick · ↵ use · F filter · esc close" />
    </div>
  );
}
const lblRow = { display:"flex", alignItems:"center", gap:6, padding:"3px 0", fontFamily:"var(--font-mono)", fontSize:11 };

// ─── shared chrome ──────────────────────────────────────────────────────────
function PlannerTopbar({ mode, goal }){
  return (
    <div style={{ display:"flex", alignItems:"center", gap:10, padding:"8px 12px",
                  borderBottom:"1.5px solid var(--ink)", background:"var(--paper-2)" }}>
      <span className="sk-h sk-h-sm" style={{ letterSpacing:0.5 }}>EXERGON · planner</span>
      <Tag on>{mode}</Tag>
      <span className="sk-mono-sm" style={{ color:"var(--ink-soft)" }}>· goal: {goal}</span>
      <span style={{ flex:1 }}/>
      <span className="sk-mono-xs" style={{ color:"var(--ink-soft)" }}>units</span>
      <div style={{ display:"flex" }}>
        {["/s","/min","stk/m","blt"].map((u,i)=>(
          <button key={u} className={`sk-btn ${i===0?"sk-on":""}`} style={{
            padding:"2px 7px", fontSize:11, boxShadow:"none",
            borderLeft: i===0? "1.5px solid var(--ink)":"none" }}>{u}</button>
        ))}
      </div>
      <button className="sk-btn" style={{ fontSize:11 }}>↶ undo</button>
      <button className="sk-btn" style={{ fontSize:11 }}>save plan</button>
      <button className="sk-btn sk-accent" style={{ fontSize:11 }}>balance ⌘B</button>
    </div>
  );
}

function PlannerLeftRail(){
  const items = [
    { k:"⌖", t:"goal" }, { k:"▦", t:"recipes" }, { k:"⚙", t:"machines" },
    { k:"◴", t:"power" }, { k:"≣", t:"floors" }, { k:"⌕", t:"find" },
    { k:"↗", t:"export" },
  ];
  return (
    <div style={{ borderRight:"1.5px solid var(--ink)", display:"flex", flexDirection:"column",
                  alignItems:"center", padding:"8px 0", gap:6, background:"var(--paper-2)" }}>
      {items.map((it,i)=>(
        <button key={i} className={`sk-btn ${i===0?"sk-on":""}`} style={{
          width:38, height:38, padding:0, justifyContent:"center", flexDirection:"column",
          fontSize:16, boxShadow:"none", border:"1.5px solid var(--ink)"
        }} title={it.t}>
          <span style={{ fontFamily:"var(--font-hand)", fontSize:18, lineHeight:1 }}>{it.k}</span>
          <span style={{ fontSize:7, fontFamily:"var(--font-mono)", letterSpacing:0.5, opacity:0.7 }}>{it.t}</span>
        </button>
      ))}
    </div>
  );
}

function PlannerStatusbar({ tip }){
  return (
    <div style={{ display:"flex", alignItems:"center", gap:14, padding:"4px 12px",
                  borderTop:"1.5px solid var(--ink)", background:"var(--paper-2)",
                  fontFamily:"var(--font-mono)", fontSize:10, color:"var(--ink-soft)" }}>
      <Tag>32 machines</Tag>
      <Tag>980 kW</Tag>
      <Tag>2 floors</Tag>
      <Tag alert>1 bottleneck</Tag>
      <span style={{ flex:1 }}/>
      <span>{tip}</span>
    </div>
  );
}

function Section({ label, children, right }){
  return (
    <div style={{ padding:"10px 12px", borderBottom:"1px dashed var(--ink)" }}>
      <div style={{ display:"flex", alignItems:"center", marginBottom:6 }}>
        <span className="sk-mono-xs" style={{ color:"var(--ink-soft)", letterSpacing:2 }}>{label}</span>
        <span style={{ flex:1 }}/>
        {right}
      </div>
      {children}
    </div>
  );
}

// expose
Object.assign(window, {
  PlannerV1, PlannerV2, PlannerV3, PlannerV4, PlannerRecipePicker
});
