// tech-tree-v6.jsx — V6 · TIER-PAGES (GTNH-questbook style)
//
// Each tier is its own page (tabs T0…T4). Within a page, nodes are placed
// spatially, softly grouped by their research-line tag (subway colors).
// Milestones appear as BRIDGE CARDS on both adjacent tabs:
//   - "exit gate" at the right edge of tier N's page
//   - "entry gate" at the left edge of tier N+1's page
// Cross-tier prereq edges (to other pages) become port stubs at the
// page margins with a label like "→ chip-i (T3)".
//
// Reads window.__ttTweaks (fogStyle, showLockedEdges).
// Depends on globals defined in tech-tree.jsx: TT, ttTweaks, ttClass,
// ttRange, FogText, TTTopbar, TTSearchBar, TTLeftRail, TTRightRail,
// TTInspector.

function TTTierPages(){
  const fogStyle = ttTweaks().fogStyle || "silhouette";
  const showLocked = !!ttTweaks().showLockedEdges;

  // research-line colors — one per category (8 total)
  const LINE_COLORS = {
    extract:"#8b6914",  smelt:"#a85a2c",   process:"#3d8b6b",
    power:"#c23b22",    logistics:"#3a6ea8", science:"#7a3d8b",
    explore:"#2e7d32",  fab:"#6b6b8b",
  };
  // milestone gates: MS[n] = node ID of the milestone that unlocks tier n
  // T1 has no gate (always available), so MS[1] is absent
  const MS = {
    2:"t2-gate", 3:"t3-gate", 4:"t4-gate",  5:"t5-gate",
    6:"t6-gate", 7:"t7-gate", 8:"t8-gate",  9:"t9-gate", 10:"t10-gate",
  };

  const [page, setPage] = React.useState(3); // start on T3 Contact — most visually rich
  const [selected, setSelected] = React.useState(MS[3] || "alien-lab");

  // ── per-page layout ─────────────────────────────────────────────────────
  // X = prerequisite depth via BFS on in-page edges (prerequisite graph drives layout)
  // Y = research-line category row
  // Nodes sharing (depth, row) are spread into adjacent sub-columns.

  const TAG_ORDER = ["extract","smelt","process","power","logistics","science","explore","fab"];

  const techsOnPage = TT.techs.filter(t => t.tier === page);

  const { layout, maxCol } = React.useMemo(()=>{
    const out = {};
    const ids = new Set(techsOnPage.map(t=>t.id));

    // build in-page adjacency
    const predCount = {}, succs = {};
    ids.forEach(id=>{ predCount[id]=0; succs[id]=[]; });
    TT.edges.forEach(([a,b])=>{
      if(ids.has(a) && ids.has(b)){ predCount[b]++; succs[a].push(b); }
    });

    // BFS topological depth from source nodes (no in-page predecessors)
    const depth = {};
    const queue = [...ids].filter(id=>!predCount[id]);
    queue.forEach(id=>{ depth[id]=0; });
    let qi=0;
    while(qi<queue.length){
      const cur=queue[qi++];
      (succs[cur]||[]).forEach(nxt=>{
        const d=(depth[cur]||0)+1;
        if(depth[nxt]===undefined||depth[nxt]<d) depth[nxt]=d;
        if(!queue.includes(nxt)) queue.push(nxt);
      });
    }

    // bucket by depth only; sort within bucket by tag for loose visual grouping
    const depthBuckets = {};
    techsOnPage.forEach(t=>{
      const d = depth[t.id]||0;
      (depthBuckets[d]=depthBuckets[d]||[]).push(t.id);
    });
    Object.values(depthBuckets).forEach(grp=>{
      grp.sort((a,b)=> TAG_ORDER.indexOf(TT.byId[a]?.tag) - TAG_ORDER.indexOf(TT.byId[b]?.tag));
    });

    techsOnPage.forEach(t=>{
      const d = depth[t.id]||0;
      const grp = depthBuckets[d];
      const pos = grp.indexOf(t.id);
      out[t.id] = { col: d, row: pos, rowCount: grp.length };
    });

    const mc = Math.max(...Object.values(out).map(l=>l.col), 3);
    return { layout:out, maxCol:mc };
  }, [page]);

  // ── stage geometry ──────────────────────────────────────────────────────
  const COL_W  = 180;
  const GUTTER = 130;
  const STAGE_H = 760;
  const STAGE_W = Math.max(1080, GUTTER*2 + 120 + (maxCol+1)*COL_W);
  const innerH = STAGE_H - 60;

  const nodeXY = (id)=>{
    const L = layout[id];
    if (!L) return null;
    const x = GUTTER + 76 + L.col * COL_W;
    const margin = 50;
    const y = L.rowCount <= 1
      ? STAGE_H / 2
      : margin + L.row * (innerH - 2*margin) / (L.rowCount - 1);
    return { x, y };
  };

  // ── bridge cards (milestones gating in/out of this page) ─────────────────
  const entryGateId = MS[page];        // milestone that, when revealed, gates THIS page
  const exitGateId  = MS[page + 1];    // milestone in NEXT tier that this page leads up to

  // T0 has no entry gate (you start here); T4 has no exit
  const entryGate = entryGateId ? TT.byId[entryGateId] : null;
  const exitGate  = exitGateId ? TT.byId[exitGateId] : null;

  // ── cross-page prereq stubs ─────────────────────────────────────────────
  // For each tech on this page, find prereq edges from PREVIOUS tier (entry stubs)
  // and outgoing edges to NEXT tier (exit stubs). Group by direction.
  const inStubs  = []; // {fromId, toId, color}
  const outStubs = [];
  TT.edges.forEach(([a,b])=>{
    const A = TT.byId[a], B = TT.byId[b];
    if (!A || !B) return;
    if (B.tier === page && A.tier === page - 1 && a !== entryGateId) {
      // cross-page incoming — but only show if the gating milestone isn't the source
      inStubs.push({ from:a, to:b, color: LINE_COLORS[A.tag] });
    }
    if (A.tier === page && B.tier === page + 1 && b !== exitGateId) {
      outStubs.push({ from:a, to:b, color: LINE_COLORS[B.tag] });
    }
  });

  // intra-page edges (both endpoints on this page)
  const intraEdges = TT.edges.filter(([a,b])=>{
    const A = TT.byId[a], B = TT.byId[b];
    return A && B && A.tier === page && B.tier === page;
  });
  // also: edges from entryGate to any node on this page (rendered from the bridge card)
  const fromEntryGate = entryGate ? TT.edges.filter(([a,b])=> a===entryGateId && TT.byId[b]?.tier === page) : [];
  // and: edges from any node on this page to exitGate
  const toExitGate = exitGate ? TT.edges.filter(([a,b])=> b===exitGateId && TT.byId[a]?.tier === page) : [];

  // ── render helpers ──────────────────────────────────────────────────────
  const NodeCard = ({ t, x, y, color })=>{
    const knTier = TT.knowledge[t.id]||0;
    const isSel = selected === t.id;
    if (knTier === 0) {
      return (
        <div className="tt-node" style={{ left:x-58, top:y-22, width:116 }}>
          <div className="tt-node-card tt-t1" style={{
            opacity:0.55, borderStyle:"dashed", padding:"6px 8px"
          }}>
            <div className="sk-mono-xs" style={{ color:"var(--ink-faint)" }}>UNKNOWN · {t.tag}</div>
            <div style={{ marginTop:4 }}><span className="tt-redact" style={{ width:60 }}/></div>
          </div>
        </div>
      );
    }
    return (
      <div className="tt-node" style={{ left:x-62, top:y-26, width:124 }}>
        <div onClick={()=>setSelected(t.id)}
             className={`tt-node-card ${ttClass(t, TT.knowledge)} ${isSel?"tt-selected":""}`}
             style={{
               width:124, padding:"6px 8px", position:"relative",
               borderLeft: `4px solid ${color}`,
             }}>
          <div style={{ display:"flex", justifyContent:"space-between", alignItems:"flex-start" }}>
            <span className="sk-mono-xs" style={{ color:"var(--ink-faint)" }}>{t.tag}</span>
            <span className="sk-mono-xs">{knTier===3?"●":knTier===2?"~":"?"}</span>
          </div>
          <div style={{ marginTop:3 }}>
            <FogText tier={knTier} fogStyle={fogStyle} glyph={t.glyph} name={t.name} short/>
          </div>
          {knTier>=2 && (
            <div className="sk-mono-xs" style={{ marginTop:3, color:"var(--ink-soft)" }}>
              {knTier===3 ? "12.0/s" : ttRange(12)+"/s"}
            </div>
          )}
        </div>
      </div>
    );
  };

  // bridge gate card — appears in left or right gutter
  const GateCard = ({ side, ms, label, subLabel })=>{
    if (!ms) return null;
    const knTier = TT.knowledge[ms.id] || 0;
    const isSel = selected === ms.id;
    const x = side === "left" ? GUTTER/2 : STAGE_W - GUTTER/2;
    const y = STAGE_H/2;
    return (
      <div className="tt-node" style={{ left:x-66, top:y-72, width:132 }}>
        {/* "tab/notch" indicator that this card spans pages */}
        <div className="sk-mono-xs"
             style={{ textAlign:"center", color:"var(--ink-faint)", marginBottom:4, letterSpacing:0.6 }}>
          {label}
        </div>
        <div onClick={()=>setSelected(ms.id)}
             className={`tt-node-card tt-milestone ${isSel?"tt-selected":""}`}
             style={{
               width:132, padding:10, position:"relative",
               background: knTier>=2 ? "var(--accent)" : "var(--paper-2)",
               borderWidth: 2.5,
             }}>
          <div style={{ display:"flex", justifyContent:"space-between", alignItems:"center" }}>
            <span className="sk-tag sk-on">MS</span>
            <span style={{ fontFamily:"var(--font-hand)", fontSize:18 }}>
              {knTier>=2 ? ms.glyph : "?"}
            </span>
          </div>
          <div className="sk-h sk-h-xs" style={{ marginTop:4, lineHeight:1.1 }}>
            <FogText tier={knTier} fogStyle={fogStyle} glyph="" name={ms.name} short/>
          </div>
          <div className="sk-mono-xs" style={{ marginTop:4, color:"var(--ink-soft)" }}>
            {subLabel}
          </div>
        </div>
        {/* page-spanning sigil */}
        <div className="sk-mono-xs" style={{
          textAlign:"center", marginTop:4, color:"var(--ink-soft)",
          fontStyle:"italic"
        }}>
          ↔ also on T{side==="left" ? page-1 : page+1}
        </div>
      </div>
    );
  };

  // tab strip
  const Tabs = ()=>(
    <div style={{
      display:"flex", borderBottom:"1.5px solid var(--ink)",
      background:"var(--paper-2)", flexShrink:0
    }}>
      {[1,2,3,4,5,6,7,8,9,10].map(n=>{
        const tierName = ["","LANDFALL","ROOTS","CONTACT","REACH","SALVAGE","TRAVERSE","INTERFACE","REVELATION","FORGE","TRANSCENDENCE"][n];
        const ms = MS[n] ? TT.byId[MS[n]] : null;
        const gateOpen = !ms || (TT.knowledge[ms.id]||0) >= 2;
        const isOn = page === n;
        return (
          <div key={n} onClick={()=>{ setPage(n); }} style={{
            cursor:"pointer", padding:"10px 18px",
            borderRight: n<4 ? "1.5px solid var(--ink)" : "none",
            borderBottom: isOn ? "3px solid var(--accent)" : "3px solid transparent",
            marginBottom: -1.5,
            background: isOn ? "var(--paper)" : "transparent",
            display:"flex", flexDirection:"column", gap:2, minWidth:130,
            opacity: gateOpen ? 1 : 0.55,
          }}>
            <div style={{ display:"flex", alignItems:"center", gap:6 }}>
              <span className="sk-tag sk-on">T{n}</span>
              <span className="sk-h sk-h-xs">{tierName}</span>
              {!gateOpen && <span className="sk-mono-xs" style={{ color:"var(--ink-faint)" }}>· locked</span>}
            </div>
            <div className="sk-mono-xs" style={{ color:"var(--ink-soft)" }}>
              {TT.techs.filter(t=>t.tier===n && (TT.knowledge[t.id]||0) >= 2).length}/{TT.techs.filter(t=>t.tier===n).length} nodes
            </div>
          </div>
        );
      })}
      <div style={{ flex:1 }}/>
      <div style={{ padding:"10px 14px", display:"flex", alignItems:"center", gap:8 }}>
        <button className="sk-btn" onClick={()=>setPage(Math.max(1, page-1))}>← prev tier</button>
        <button className="sk-btn" onClick={()=>setPage(Math.min(10, page+1))}>next tier →</button>
      </div>
    </div>
  );

  // edge style helper
  const edgeClass = (a,b)=>{
    const ka = TT.knowledge[a]||0, kb = TT.knowledge[b]||0;
    if (ka>=2 && kb>=2) return "tt-edge-known";
    if (ka>=2 || kb>=2) return "tt-edge-partial";
    return "tt-edge-locked";
  };

  // line-tag color for an edge (use destination's tag color)
  const edgeColor = (a,b)=>{
    const B = TT.byId[b];
    return B ? LINE_COLORS[B.tag] : "var(--ink)";
  };

  // is the edge crossing research lines (different tag)?
  const isCrossLine = (a,b)=>{
    const A = TT.byId[a], B = TT.byId[b];
    return A && B && A.tag !== B.tag;
  };

  return (
    <div className="paper" style={{ height:"100%", display:"flex", flexDirection:"column" }}>
      <TTTopbar mode="TIER-PAGES"/>
      <Tabs/>

      <div style={{ flex:1, display:"grid", gridTemplateColumns:"56px 1fr 280px", overflow:"hidden" }}>
        <TTLeftRail/>

        <div style={{ position:"relative", overflow:"auto", background:"var(--paper)" }}>
          <div style={{ position:"relative", width: STAGE_W, height: STAGE_H, margin:"0 auto" }}>

            {/* swim-lane backgrounds */}
            <svg width={STAGE_W} height={STAGE_H} style={{ position:"absolute", inset:0, display:"block" }}>
              {/* page header strip */}
              <rect x="0" y="0" width={STAGE_W} height="36" fill="var(--paper-2)"/>
              <text x={STAGE_W/2} y="24" fontSize="13" fontFamily="var(--font-mono)"
                    fill="var(--ink-soft)" textAnchor="middle" letterSpacing="3">
                — TIER {page} · {["","LANDFALL","ROOTS","CONTACT","REACH","SALVAGE","TRAVERSE","INTERFACE","REVELATION","FORGE","TRANSCENDENCE"][page]} —
              </text>

              {/* gutter divider lines */}
              <line x1={GUTTER} y1="40" x2={GUTTER} y2={STAGE_H-10}
                    stroke="var(--ink-faint)" strokeDasharray="4 4"/>
              <line x1={STAGE_W-GUTTER} y1="40" x2={STAGE_W-GUTTER} y2={STAGE_H-10}
                    stroke="var(--ink-faint)" strokeDasharray="4 4"/>
              <text x={GUTTER-8} y="48" fontSize="9" fontFamily="var(--font-mono)"
                    fill="var(--ink-faint)" textAnchor="end" transform={`rotate(-90 ${GUTTER-8} 48)`}>
                ENTRY GATE
              </text>
              <text x={STAGE_W-GUTTER+8} y="48" fontSize="9" fontFamily="var(--font-mono)"
                    fill="var(--ink-faint)" transform={`rotate(-90 ${STAGE_W-GUTTER+8} 48)`}>
                EXIT GATE
              </text>

              {/* intra-page edges */}
              {intraEdges.map(([a,b],i)=>{
                const A = nodeXY(a), B = nodeXY(b);
                if (!A || !B) return null;
                const cls = edgeClass(a,b);
                if (cls==="tt-edge-locked" && !showLocked) return null;
                const cross = isCrossLine(a,b);
                const dx = (B.x-A.x)*0.4;
                const d = `M${A.x+50},${A.y} C${A.x+50+dx},${A.y} ${B.x-50-dx},${B.y} ${B.x-50},${B.y}`;
                return (
                  <path key={i} d={d}
                        stroke={cross ? "var(--ink-soft)" : edgeColor(a,b)}
                        strokeWidth={cross ? 1.5 : 2.5}
                        strokeDasharray={cross ? "6 4" : "none"}
                        fill="none"
                        opacity={cls==="tt-edge-partial" ? 0.55 : 1}/>
                );
              })}

              {/* edges from entry gate (left bridge) into page */}
              {entryGate && fromEntryGate.map(([a,b],i)=>{
                const A = { x: GUTTER/2 + 40, y: STAGE_H/2 };
                const B = nodeXY(b); if (!B) return null;
                const cls = edgeClass(a,b);
                if (cls==="tt-edge-locked" && !showLocked) return null;
                const dx = (B.x - A.x) * 0.4;
                return (
                  <path key={"eg"+i}
                        d={`M${A.x},${A.y} C${A.x+dx},${A.y} ${B.x-50-dx},${B.y} ${B.x-50},${B.y}`}
                        stroke={edgeColor(a,b)} strokeWidth="2" fill="none" opacity="0.85"/>
                );
              })}
              {/* edges from page → exit gate */}
              {exitGate && toExitGate.map(([a,b],i)=>{
                const A = nodeXY(a); if (!A) return null;
                const B = { x: STAGE_W - GUTTER/2 - 40, y: STAGE_H/2 };
                const cls = edgeClass(a,b);
                if (cls==="tt-edge-locked" && !showLocked) return null;
                const dx = (B.x - A.x) * 0.4;
                return (
                  <path key={"xg"+i}
                        d={`M${A.x+50},${A.y} C${A.x+50+dx},${A.y} ${B.x-dx},${B.y} ${B.x},${B.y}`}
                        stroke={edgeColor(a,b)} strokeWidth="2" fill="none" opacity="0.85"
                        strokeDasharray="2 0"/>
                );
              })}

              {/* incoming cross-page port stubs (left margin) */}
              {inStubs.map(({ from, to, color }, i)=>{
                const B = nodeXY(to); if (!B) return null;
                const yStub = 80 + (i * 38) % (STAGE_H - 200);
                return (
                  <g key={"in"+i}>
                    <path d={`M${GUTTER+8},${yStub} L${GUTTER+24},${yStub} L${B.x-50},${B.y}`}
                          stroke={color} strokeWidth="1.5" fill="none"
                          strokeDasharray="4 3" opacity="0.7"/>
                    <circle cx={GUTTER+8} cy={yStub} r="3" fill={color}/>
                  </g>
                );
              })}
              {/* outgoing cross-page port stubs (right margin) */}
              {outStubs.map(({ from, to, color }, i)=>{
                const A = nodeXY(from); if (!A) return null;
                const yStub = 80 + (i * 38) % (STAGE_H - 200);
                return (
                  <g key={"out"+i}>
                    <path d={`M${A.x+50},${A.y} L${STAGE_W-GUTTER-24},${yStub} L${STAGE_W-GUTTER-8},${yStub}`}
                          stroke={color} strokeWidth="1.5" fill="none"
                          strokeDasharray="4 3" opacity="0.7"/>
                    <circle cx={STAGE_W-GUTTER-8} cy={yStub} r="3" fill={color}/>
                  </g>
                );
              })}
            </svg>

            {/* port-stub labels — text rendered as DOM so it picks up the fog */}
            {inStubs.map(({ from, to, color }, i)=>{
              const tA = TT.byId[from];
              const yStub = 80 + (i * 38) % (STAGE_H - 200);
              const knA = TT.knowledge[from] || 0;
              return (
                <div key={"inl"+i} style={{
                  position:"absolute", left: 6, top: yStub - 9, width: GUTTER - 12,
                  fontFamily:"var(--font-mono)", fontSize:9, lineHeight:1.2,
                  textAlign:"right", color:"var(--ink-soft)", pointerEvents:"none"
                }}>
                  <span style={{ color, marginRight:4 }}>← T{tA.tier}</span>
                  <FogText tier={knA} fogStyle={fogStyle} glyph="" name={tA.name} short/>
                </div>
              );
            })}
            {outStubs.map(({ from, to, color }, i)=>{
              const tB = TT.byId[to];
              const yStub = 80 + (i * 38) % (STAGE_H - 200);
              const knB = TT.knowledge[to] || 0;
              return (
                <div key={"outl"+i} style={{
                  position:"absolute", right: 6, top: yStub - 9, width: GUTTER - 12,
                  fontFamily:"var(--font-mono)", fontSize:9, lineHeight:1.2,
                  textAlign:"left", color:"var(--ink-soft)", pointerEvents:"none"
                }}>
                  <span style={{ color, marginRight:4 }}>T{tB.tier} →</span>
                  <FogText tier={knB} fogStyle={fogStyle} glyph="" name={tB.name} short/>
                </div>
              );
            })}

            {/* nodes — exclude entry gate; it renders as the left GateCard */}
            {techsOnPage.filter(t => t.id !== entryGateId).map(t=>{
              const xy = nodeXY(t.id); if (!xy) return null;
              return <NodeCard key={t.id} t={t} x={xy.x} y={xy.y}
                               color={LINE_COLORS[t.tag] || "#888"}/>;
            })}

            {/* bridge gates */}
            <GateCard side="left"  ms={entryGate} label="ENTRY · gate from previous tier"
                      subLabel={`unlocks T${page} stratum`}/>
            <GateCard side="right" ms={exitGate} label="EXIT · gate to next tier"
                      subLabel={`reveal to open T${page+1}`}/>

            {/* legend */}
            <div className="sk-box" style={{
              position:"absolute", left:14, bottom:14, padding:8,
              fontFamily:"var(--font-mono)", fontSize:9, lineHeight:1.5,
              background:"var(--paper)", maxWidth:240
            }}>
              <div style={{ fontWeight:700, marginBottom:4 }}>edges</div>
              <div><span style={{ display:"inline-block", width:18, height:2, background:"#3a6ea8", verticalAlign:"middle" }}/> &nbsp; same line (line color)</div>
              <div><span style={{ display:"inline-block", width:18, height:0, borderTop:"1.5px dashed var(--ink-soft)", verticalAlign:"middle" }}/> &nbsp; cross-line dependency</div>
              <div><span style={{ display:"inline-block", width:18, height:0, borderTop:"1.5px dashed var(--ink-faint)", verticalAlign:"middle" }}/> &nbsp; → cross-tier port (jump)</div>
            </div>

            {/* annotation */}
            <div className="sk-annot" style={{ left: GUTTER+20, top: STAGE_H-32, transform:"rotate(-1deg)" }}>
              milestones span two pages — same card, different tab — so you can always trace where you came from.
            </div>
          </div>
        </div>

        <TTRightRail>
          <TTInspector tech={TT.byId[selected]} knTier={TT.knowledge[selected]||1}/>

          <div className="sk-div" style={{ marginTop:14, marginBottom:10 }}/>
          <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginBottom:6 }}>cross-tier dependencies</div>
          {inStubs.length>0 && (
            <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", lineHeight:1.6 }}>
              <span style={{ fontWeight:700 }}>incoming · {inStubs.length}</span>
              <div>{inStubs.slice(0,4).map((s,i)=>{
                const tA = TT.byId[s.from];
                return (<span key={"in"+i+s.from} style={{ display:"inline-block", marginRight:6 }}>
                  <span style={{ color:s.color }}>●</span> {tA.name.slice(0,12)}
                </span>);
              })}{inStubs.length>4 && "…"}</div>
            </div>
          )}
          {outStubs.length>0 && (
            <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", lineHeight:1.6, marginTop:6 }}>
              <span style={{ fontWeight:700 }}>outgoing · {outStubs.length}</span>
              <div>{outStubs.slice(0,4).map((s,i)=>{
                const tB = TT.byId[s.to];
                return (<span key={"out"+i+s.to} style={{ display:"inline-block", marginRight:6 }}>
                  <span style={{ color:s.color }}>●</span> {tB.name.slice(0,12)}
                </span>);
              })}{outStubs.length>4 && "…"}</div>
            </div>
          )}
        </TTRightRail>
      </div>
    </div>
  );
}
