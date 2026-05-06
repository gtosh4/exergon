/* global React, Slot, Row, Col */
const { useState: useStateInt } = React;

// ============================================================
// INTEGRATED TERMINAL v3 — mass-based, codex right, bookmarks left
// ============================================================
const IntegratedTerminal = () => {
  const [mainView, setMainView] = useStateInt("network");
  const [codexOpen, setCodexOpen] = useStateInt(true);
  const [codexFull, setCodexFull] = useStateInt(false);
  const [acDrawer, setAcDrawer] = useStateInt(null); // null | 'patterns' | 'planner' | 'cpus'
  const [selected, setSelected] = useStateInt("steel.plate");

  // [name, tag, qty, kgEa, craftable, trendPerMin]
  const items = [
    ["iron.plate","ORE",24576,0.4,true,+128],
    ["copper.plate","ORE",18432,0.4,true,+64],
    ["steel.plate","ORE",4096,0.4,true,-16],
    ["circuit.basic","COMP",2048,0.05,true,+4],
    ["circuit.adv","COMP",128,0.05,true,+2],
    ["gear.bronze","COMP",1024,0.6,false,0],
    ["pipe.stl.10","BUILD",8192,0.1,false,-32],
    ["fuel.cell","FUEL",256,2.0,true,+8],
    ["wire.copper","COMP",16384,0.05,true,+256],
    ["plastic.sheet","COMP",512,0.1,true,+12],
    ["coal","FUEL",0,0.5,false,-8],         // qty 0
    ["reactor.core","COMP",0,4.0,true,0],    // qty 0
  ];

  const totalKg = items.reduce((s, r) => s + r[2] * r[3], 0);

  return (
    <div className="paper density-dense" style={{ height: "100%", display: "flex", flexDirection: "column", fontFamily: "var(--font-mono)" }}>
      {/* TOP BAR */}
      <Row style={{ padding: "8px 14px", borderBottom: "2px solid var(--ink)", justifyContent: "space-between", background: "var(--paper-2)" }}>
        <Row gap={10} style={{ alignItems: "center" }}>
          <span className="sk-h" style={{ fontSize: 22 }}>EXERGON · TERMINAL</span>
          <Row gap={3}>
            <span className={`sk-tag ${mainView === "network" ? "sk-on" : ""}`} onClick={() => setMainView("network")}>▣ MAIN</span>
            <span className={`sk-tag ${mainView === "subnet-α" ? "sk-on" : ""}`} onClick={() => setMainView("subnet-α")}>◫ workshop-α</span>
            <span className={`sk-tag ${mainView === "subnet-β" ? "sk-on" : ""}`} onClick={() => setMainView("subnet-β")}>◫ smelter-β</span>
            <span className="sk-tag" style={{ opacity: 0.55 }}>+ subnet</span>
          </Row>
        </Row>
        <Row gap={6}>
          <span className="sk-tag">F1 help</span>
          <span className="sk-tag">CTRL+K</span>
          <span className="sk-tag">ESC close</span>
        </Row>
      </Row>

      {/* MASS-BASED CAPACITY STRIP */}
      <Row gap={10} style={{ padding: "6px 14px", borderBottom: "1.5px solid var(--ink)", alignItems: "center", background: "var(--paper)" }}>
        <Col gap={1} style={{ flex: 2 }}>
          <Row style={{ justifyContent: "space-between" }}>
            <span className="sk-mono-xs"><b>MASS · main network</b> · 14 drives</span>
            <span className="sk-mono-xs">
              <b>{(totalKg/1000).toFixed(2)} t</b> / 16.00 t · {((totalKg/16000)*100).toFixed(1)}%
            </span>
          </Row>
          <div className="sk-bar" style={{ height: 8 }}><i style={{ width: `${(totalKg/16000)*100}%` }}/></div>
          <Row style={{ justifyContent: "space-between" }}>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>drives at 64% mass · 36% headroom</span>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>over capacity = throughput stalls</span>
          </Row>
        </Col>
        <Col gap={1} style={{ flex: 1 }}>
          <Row style={{ justifyContent: "space-between" }}>
            <span className="sk-mono-xs"><b>UNIQUE</b></span>
            <span className="sk-mono-xs">2,847 / 8,192 cells · 34.7%</span>
          </Row>
          <div className="sk-bar" style={{ height: 8 }}><i style={{ width: "34.7%" }}/></div>
        </Col>
        <Col gap={1} style={{ flex: 1 }}>
          <Row style={{ justifyContent: "space-between" }}>
            <span className="sk-mono-xs"><b>POWER</b></span>
            <span className="sk-mono-xs">8.2 / 14.0 MW · 58.6%</span>
          </Row>
          <div className="sk-bar" style={{ height: 8 }}><i style={{ width: "58.6%" }}/></div>
        </Col>
      </Row>

      {/* MAIN ROW — bookmarks | items+actions | codex */}
      <Row gap={0} style={{ flex: 1, alignItems: "stretch", overflow: "hidden" }}>

        {/* LEFT: BOOKMARKS / TODO */}
        <Col style={{ width: 220, borderRight: "1.5px solid var(--ink)", background: "var(--paper-2)" }}>
          <Row gap={4} style={{ padding: "6px 8px", borderBottom: "1.5px solid var(--ink)", justifyContent: "space-between" }}>
            <span className="sk-h-sm">★ BOOKMARKS</span>
            <Row gap={2}>
              <button className="sk-btn" style={{ padding: "1px 4px", fontSize: 9 }}>+ new</button>
            </Row>
          </Row>
          <div style={{ flex: 1, overflow: "auto", padding: 6 }}>
            {/* TODO list — bookmarks with target qty */}
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>TODO · target qty</span>
            <div className="sk-box" style={{ padding: 4, marginTop: 4, marginBottom: 8 }}>
              {[
                { n: "reactor.core",  have: 0,    want: 4,    pri: "★★★", done: false },
                { n: "fuel.cell",     have: 256,  want: 1024, pri: "★★",  done: false },
                { n: "circuit.adv",   have: 128,  want: 512,  pri: "★★",  done: false },
                { n: "steel.plate",   have: 4096, want: 4000, pri: "★",   done: true  },
                { n: "coal",          have: 0,    want: 2048, pri: "★★★", done: false },
              ].map((b, i) => {
                const pct = Math.min(100, (b.have / b.want) * 100);
                return (
                  <Col key={i} gap={1} style={{ padding: "3px 2px", borderBottom: "1px dashed var(--ink-faint)" }}>
                    <Row gap={3} style={{ alignItems: "center" }}>
                      <span className="sk-mono-xs" style={{ width: 16 }}>{b.done ? "☑" : "☐"}</span>
                      <Slot style={{ width: 14, height: 14 }} filled icon="·"/>
                      <span className="sk-mono-sm" style={{ flex: 1, fontWeight: 600, textDecoration: b.done ? "line-through" : "none" }}>{b.n}</span>
                      <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>{b.pri}</span>
                    </Row>
                    <Row gap={3} style={{ alignItems: "center", paddingLeft: 22 }}>
                      <div className="sk-bar" style={{ flex: 1, height: 4 }}><i style={{ width: `${pct}%` }}/></div>
                      <span className="sk-mono-xs" style={{ width: 60, textAlign: "right" }}>
                        {b.have.toLocaleString()}/{b.want.toLocaleString()}
                      </span>
                    </Row>
                  </Col>
                );
              })}
            </div>

            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>QUICK ACCESS</span>
            <div className="sk-box" style={{ padding: 4, marginTop: 4, marginBottom: 8 }}>
              <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 2 }}>
                {["▤","◇","⛏","◉","▦","✦","◆","▣"].map((ic, i) => (
                  <Slot key={i} filled icon={ic} qty={[4096,24576,1,256,512,128,1024,2048][i]}/>
                ))}
              </div>
            </div>

            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>SAVED FILTERS</span>
            <Col gap={2} style={{ marginTop: 4 }}>
              {["@craftable", "qty<10", "tag:tool", "missing now", "high-mass"].map((f, i) => (
                <Row key={i} gap={4} style={{ padding: "2px 4px", border: "1px dashed var(--ink)", background: i === 3 ? "rgba(245,197,24,0.20)" : "var(--paper)" }}>
                  <span className="sk-mono-xs">⌕</span>
                  <span className="sk-mono-sm" style={{ flex: 1 }}>{f}</span>
                  <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>{[12,3,47,2,18][i]}</span>
                </Row>
              ))}
            </Col>
          </div>
          <Row gap={3} style={{ padding: 6, borderTop: "1.5px solid var(--ink)" }}>
            <button className="sk-btn" style={{ flex: 1, justifyContent: "center", padding: "2px 4px", fontSize: 10 }}>⊕ from selection</button>
          </Row>
        </Col>

        {/* CENTER: ITEMS TABLE + ACTIONS + AC DRAWER */}
        <Col style={{ flex: 1, minWidth: 0, background: "var(--paper)" }}>
          {/* search + sort */}
          <Row gap={6} style={{ padding: "6px 10px", borderBottom: "1.5px solid var(--ink)", alignItems: "center" }}>
            <span className="sk-h-sm">{mainView === "network" ? "▣ MAIN NETWORK" : `◫ ${mainView}`}</span>
            <div className="sk-box" style={{ flex: 1, padding: "2px 8px" }}>
              <span className="sk-mono-sm">⌕ </span><span className="sk-squig sk-mono-sm">tag:ore qty&gt;100</span>
            </div>
            <Row gap={3}>
              <span className="sk-tag">[grid]</span>
              <span className="sk-tag sk-on">[table]</span>
              <span className="sk-tag">[graph]</span>
            </Row>
            <Row gap={3}>
              <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>sort:</span>
              <span className="sk-tag">name</span>
              <span className="sk-tag sk-on">qty▼</span>
              <span className="sk-tag">kg total</span>
            </Row>
          </Row>

          {/* table */}
          <div style={{ flex: 1, overflow: "auto" }}>
            <table style={{ width: "100%", fontFamily: "var(--font-mono)", fontSize: 11, borderCollapse: "collapse" }}>
              <thead>
                <tr style={{ borderBottom: "1.5px solid var(--ink)", background: "var(--paper-2)", position: "sticky", top: 0 }}>
                  {["", "name", "tag", "qty", "Δ/min", "kg ea", "kg total", "C", "actions"].map((h, i) => (
                    <th key={i} style={{ textAlign: i >= 3 && i <= 6 ? "right" : "left", padding: "4px 8px", fontFamily: "var(--font-label)", fontSize: 11 }}>
                      {h}{h && h !== "C" && h !== "actions" && h !== "" && <span style={{ color: "var(--ink-faint)" }}> ↕</span>}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {items.map((r, i) => {
                  const sel = r[0] === selected;
                  const kgTotal = r[2] * r[3];
                  const trend = r[5];
                  const empty = r[2] === 0;
                  return (
                    <tr key={i} onClick={() => setSelected(r[0])} style={{
                      borderBottom: "1px dashed var(--ink-faint)",
                      background: sel ? "rgba(245,197,24,0.30)" : "transparent",
                      cursor: "pointer",
                      opacity: empty ? 0.55 : 1
                    }}>
                      <td style={{ padding: "3px 8px", width: 24 }}>
                        <Slot style={{ width: 18, height: 18, opacity: empty ? 0.4 : 1 }} filled icon={"▢◇⛏✦◉◆▣▤▥◈✕▦"[i]}/>
                      </td>
                      <td style={{ padding: "3px 8px", fontWeight: 600 }}>{r[0]}</td>
                      <td style={{ padding: "3px 8px" }}>
                        <span className="sk-tag" style={{ fontSize: 8, padding: "0 3px" }}>{r[1]}</span>
                      </td>
                      <td style={{ padding: "3px 8px", textAlign: "right", fontWeight: empty ? 400 : 700 }}>
                        {empty ? <span style={{ color: "#9a1a1a" }}>0</span> : r[2].toLocaleString()}
                      </td>
                      <td style={{ padding: "3px 8px", textAlign: "right", color: trend > 0 ? "#1a6b1a" : trend < 0 ? "#9a1a1a" : "var(--ink-faint)" }}>
                        {trend > 0 ? `+${trend}` : trend === 0 ? "—" : trend}
                      </td>
                      <td style={{ padding: "3px 8px", textAlign: "right", color: "var(--ink-faint)" }}>{r[3]}</td>
                      <td style={{ padding: "3px 8px", textAlign: "right" }}>
                        {empty ? "—" : kgTotal.toLocaleString(undefined, { maximumFractionDigits: 1 })}
                      </td>
                      <td style={{ padding: "3px 8px" }}>{r[4] && <span className="sk-tag sk-on" style={{ fontSize: 8, padding: "0 3px" }}>C</span>}</td>
                      <td style={{ padding: "3px 8px", textAlign: "right" }}>
                        <Row gap={2} style={{ justifyContent: "flex-end" }}>
                          <span className="sk-tag" style={{ fontSize: 8 }}>★ bm</span>
                          {r[4] && <span className="sk-tag sk-accent" style={{ fontSize: 8 }}>▶ craft</span>}
                          <span className="sk-tag" style={{ fontSize: 8 }}>⌕ codex</span>
                        </Row>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>

          {/* status footer + AC drawer toggle */}
          <Row style={{ padding: "5px 10px", borderTop: "1.5px solid var(--ink)", justifyContent: "space-between", alignItems: "center", background: "var(--paper-2)" }}>
            <Row gap={10}>
              <span className="sk-mono-xs">12 of 2,847</span>
              <span className="sk-mono-xs">selected: <b>{selected}</b></span>
              <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>↑↓ select · ENTER = craft · A = auto · B = bookmark</span>
            </Row>
            <Row gap={3}>
              <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>auto-craft:</span>
              <span className={`sk-tag ${acDrawer === "patterns" ? "sk-on" : ""}`} onClick={() => setAcDrawer(acDrawer === "patterns" ? null : "patterns")}>patterns 247</span>
              <span className={`sk-tag ${acDrawer === "planner" ? "sk-on" : ""}`} onClick={() => setAcDrawer(acDrawer === "planner" ? null : "planner")}>planner</span>
              <span className={`sk-tag ${acDrawer === "cpus" ? "sk-on" : ""}`} onClick={() => setAcDrawer(acDrawer === "cpus" ? null : "cpus")}>cpus 3/4 ●</span>
            </Row>
          </Row>

          {/* AC DRAWER — slides up over the bottom of the table */}
          {acDrawer && (
            <div style={{ borderTop: "2px solid var(--ink)", background: "var(--paper-2)", maxHeight: 240, overflow: "auto" }}>
              <Row style={{ padding: "5px 10px", borderBottom: "1.5px solid var(--ink)", justifyContent: "space-between", alignItems: "center" }}>
                <span className="sk-h-sm">AUTO-CRAFT · {acDrawer}</span>
                <button className="sk-btn" style={{ padding: "1px 5px", fontSize: 10 }} onClick={() => setAcDrawer(null)}>✕ close</button>
              </Row>
              <div style={{ padding: 8 }}>
                {acDrawer === "patterns" && <PatternsRow/>}
                {acDrawer === "planner" && <PlannerRow/>}
                {acDrawer === "cpus" && <CpusRow/>}
              </div>
            </div>
          )}
        </Col>

        {/* RIGHT: CODEX (NEI-style) */}
        {codexOpen && (
          <Col style={{ width: codexFull ? 480 : 240, borderLeft: "1.5px solid var(--ink)", background: "var(--paper-2)" }}>
            <Row gap={4} style={{ padding: "6px 8px", borderBottom: "1.5px solid var(--ink)", justifyContent: "space-between" }}>
              <span className="sk-h-sm">⌕ CODEX</span>
              <Row gap={3}>
                <button className="sk-btn" style={{ padding: "1px 5px", fontSize: 9 }} onClick={() => setCodexFull(!codexFull)}>{codexFull ? "⇲ shrink" : "⇱ expand"}</button>
                <button className="sk-btn" style={{ padding: "1px 5px", fontSize: 9 }} onClick={() => setCodexOpen(false)}>✕</button>
              </Row>
            </Row>
            <div style={{ padding: 8, flex: 1, overflow: "auto", display: "flex", flexDirection: "column", gap: 6 }}>
              <div className="sk-box" style={{ padding: "3px 6px" }}>
                <span className="sk-mono-sm">⌕ </span><span className="sk-squig sk-mono-sm">@mod:steel</span>
              </div>
              <Row gap={2} style={{ flexWrap: "wrap" }}>
                <span className="sk-tag sk-on">all</span>
                <span className="sk-tag">ore</span>
                <span className="sk-tag">tool</span>
                <span className="sk-tag">comp</span>
                <span className="sk-tag">★</span>
              </Row>

              {!codexFull && (
                <>
                  <div className="sk-box" style={{ padding: 4 }}>
                    <div style={{ display: "grid", gridTemplateColumns: "repeat(6,1fr)", gap: 2 }}>
                      {Array.from({ length: 60 }).map((_, i) => (
                        <Slot key={i} filled active={i === 7} icon={["▢","◇","⛏","✦","◉","◆","▣","▤","▥","◈","✕","▦","▧","▨"][i % 14]}/>
                      ))}
                    </div>
                  </div>
                  <div className="sk-box sk-thick" style={{ padding: 6 }}>
                    <Row gap={5}>
                      <Slot filled icon="▤"/>
                      <Col gap={0} style={{ flex: 1 }}>
                        <span className="sk-mono-sm" style={{ fontWeight: 700 }}>steel.plate</span>
                        <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>3 recipes · 47 uses · 0.4 kg</span>
                      </Col>
                    </Row>
                    <Row gap={3} style={{ marginTop: 4, flexWrap: "wrap" }}>
                      <button className="sk-btn" style={{ padding: "1px 5px", fontSize: 9 }} onClick={() => setCodexFull(true)}>R recipe</button>
                      <button className="sk-btn" style={{ padding: "1px 5px", fontSize: 9 }} onClick={() => setCodexFull(true)}>U uses</button>
                      <button className="sk-btn sk-accent" style={{ padding: "1px 5px", fontSize: 9 }}>▶ craft</button>
                      <button className="sk-btn" style={{ padding: "1px 5px", fontSize: 9 }}>★ bm</button>
                    </Row>
                  </div>
                </>
              )}

              {codexFull && <CodexDeepDive/>}
            </div>
          </Col>
        )}

        {!codexOpen && (
          <button className="sk-btn" style={{ padding: "2px 6px", fontSize: 10, alignSelf: "flex-start", margin: 6 }} onClick={() => setCodexOpen(true)}>⌕ codex</button>
        )}
      </Row>

      {/* HOTBAR */}
      <div style={{ borderTop: "2px solid var(--ink)", background: "var(--paper-2)", padding: "6px 14px" }}>
        <Row gap={10} style={{ alignItems: "center" }}>
          <Col gap={1} style={{ width: 100 }}>
            <span className="sk-h-xs">HOTBAR</span>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>shift+wheel = bank</span>
          </Col>
          <Col gap={2} style={{ flex: 1 }}>
            {[
              { tag: "A · TOOLS", active: true, icons: ["⛏","◇","◆","✦",null,"◉",null,null,"▣"] },
              { tag: "B · BUILD", active: false, icons: ["▦","▧",null,"▨","▩",null,null,"▤","▥"] },
              { tag: "C · COMBAT", active: false, icons: ["✕","◈",null,"✦",null,null,"◉",null,null] },
            ].map((bank, bi) => (
              <Row key={bi} gap={6} style={{ opacity: bank.active ? 1 : 0.4, alignItems: "center" }}>
                <span className={`sk-tag ${bank.active ? "sk-on" : ""}`} style={{ width: 70, textAlign: "center" }}>{bank.tag}</span>
                <Row gap={2}>
                  {bank.icons.map((ic, i) => (
                    <Slot key={i} filled={!!ic} active={bank.active && i === 0} icon={ic} qty={ic ? [16, 1, null, 32, 8, null, null, 4, 1][i] : null}/>
                  ))}
                </Row>
                {bank.active && (
                  <Row gap={3} style={{ marginLeft: 8 }}>
                    {[1,2,3,4,5,6,7,8,9].map(n => (
                      <span key={n} className="sk-mono-xs" style={{ width: 26, textAlign: "center", color: "var(--ink-faint)" }}>{n}</span>
                    ))}
                  </Row>
                )}
              </Row>
            ))}
          </Col>
          <Col gap={2} style={{ width: 140, alignItems: "flex-end" }}>
            <Row gap={4}><span className="sk-tag">HP 87</span><span className="sk-tag">SAT 42</span></Row>
            <Row gap={4}><span className="sk-tag">O₂ 100</span><span className="sk-tag">XP lv14</span></Row>
          </Col>
        </Row>
      </div>
    </div>
  );
};

// ─── AC DRAWER PANES ─────────────────────────────────────────
const PatternsRow = () => (
  <Row gap={8} style={{ alignItems: "stretch" }}>
    <Col gap={4} style={{ flex: 1 }}>
      <Row gap={4} style={{ flexWrap: "wrap" }}>
        <span className="sk-tag sk-on">all 247</span>
        <span className="sk-tag">CRAFT</span>
        <span className="sk-tag">SMELT</span>
        <span className="sk-tag">CHEM</span>
        <span className="sk-tag sk-accent">! 3</span>
      </Row>
      <div className="sk-box" style={{ padding: 4, maxHeight: 140, overflow: "auto" }}>
        {[
          ["circuit.basic","CRAFT","ok"],
          ["circuit.adv","CRAFT","ok"],
          ["iron.plate","SMELT","ok"],
          ["steel.plate","SMELT","conflict"],
          ["coal.coke","COKE","ok"],
          ["copper.wire","CRAFT","ok"],
          ["gear.bronze","CRAFT","ok"],
          ["fuel.cell","CHEM","ok"],
        ].map((p, i) => (
          <Row key={i} gap={4} style={{ padding: "2px 4px", borderBottom: "1px dashed var(--ink-faint)", background: i === 3 ? "rgba(245,197,24,0.20)" : "transparent" }}>
            <Slot style={{ width: 14, height: 14 }} filled icon="·"/>
            <span className="sk-mono-sm" style={{ flex: 1 }}>{p[0]}</span>
            <span className="sk-tag" style={{ fontSize: 7, padding: "0 3px" }}>{p[1]}</span>
            {p[2] === "conflict" && <span className="sk-tag sk-accent" style={{ fontSize: 7, padding: "0 3px" }}>!</span>}
          </Row>
        ))}
      </div>
    </Col>
    <Col gap={4} style={{ flex: 1.4 }}>
      <Row style={{ justifyContent: "space-between" }}>
        <span className="sk-h-xs">EDIT · steel.plate ×4</span>
        <span className="sk-mono-xs" style={{ color: "#b88a00" }}>⚠ conflict</span>
      </Row>
      <Row gap={10} style={{ alignItems: "center", justifyContent: "center" }}>
        <div style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 2 }}>
          {Array.from({ length: 9 }).map((_, i) => (
            <Slot key={i} style={{ width: 22, height: 22 }} filled={[0,1,2,4].includes(i)} icon={[0,1,2].includes(i) ? "◆" : i === 4 ? "◉" : null}/>
          ))}
        </div>
        <span className="sk-arrow" style={{ fontSize: 18 }}>⇒</span>
        <Slot style={{ width: 32, height: 32 }} filled icon="▤" qty={4}/>
      </Row>
      <Row gap={4} style={{ flexWrap: "wrap" }}>
        <span className="sk-tag" style={{ fontSize: 8 }}>blast.fur LV3</span>
        <span className="sk-tag" style={{ fontSize: 8 }}>12s</span>
        <span className="sk-tag" style={{ fontSize: 8 }}>480 EU/t</span>
        <span className="sk-tag" style={{ fontSize: 8 }}>prio 3</span>
      </Row>
      <Row gap={3}>
        <button className="sk-btn sk-accent" style={{ flex: 1, justifyContent: "center", padding: "3px 4px", fontSize: 10 }}>save</button>
        <button className="sk-btn" style={{ padding: "3px 6px", fontSize: 10 }}>dup</button>
        <button className="sk-btn" style={{ padding: "3px 6px", fontSize: 10 }}>+ blank</button>
      </Row>
    </Col>
  </Row>
);

const PlannerRow = () => (
  <Row gap={8} style={{ alignItems: "stretch" }}>
    <Col gap={4} style={{ flex: 1.5 }}>
      <Row gap={4} style={{ alignItems: "center" }}>
        <span className="sk-mono-sm">target →</span>
        <Slot filled icon="◉" qty={1}/>
        <span className="sk-mono-sm" style={{ flex: 1, fontWeight: 700 }}>reactor.core</span>
        <button className="sk-btn" style={{ padding: "1px 4px", fontSize: 9 }}>−</button>
        <span className="sk-mono-sm">1</span>
        <button className="sk-btn" style={{ padding: "1px 4px", fontSize: 9 }}>+</button>
      </Row>
      <div className="sk-box" style={{ padding: 4, maxHeight: 140, overflow: "auto" }}>
        {[
          { d: 0, n: "reactor.core", q: 1, miss: false },
          { d: 1, n: "frame.steel", q: 4, miss: false },
          { d: 2, n: "steel.plate", q: 16, miss: false },
          { d: 3, n: "iron.ore", q: 48, miss: false },
          { d: 3, n: "coal", q: 16, miss: true },
          { d: 2, n: "bolt.steel", q: 32, miss: false },
          { d: 1, n: "circuit.adv", q: 2, miss: false },
          { d: 1, n: "coolant", q: 6, miss: false },
        ].map((r, i) => (
          <Row key={i} gap={3} style={{ padding: `1px 4px 1px ${r.d * 12 + 4}px`, background: r.miss ? "rgba(245,197,24,0.20)" : "transparent", borderBottom: "1px dashed var(--ink-faint)" }}>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>└</span>
            <Slot style={{ width: 14, height: 14 }} filled icon="·"/>
            <span className="sk-mono-sm" style={{ flex: 1 }}>{r.n}</span>
            <span className="sk-mono-xs">×{r.q}</span>
            {r.miss && <span className="sk-tag sk-accent" style={{ fontSize: 7, padding: "0 2px" }}>!</span>}
          </Row>
        ))}
      </div>
    </Col>
    <Col gap={6} style={{ flex: 1, justifyContent: "space-between" }}>
      <Col gap={2}>
        <Row style={{ justifyContent: "space-between" }}><span className="sk-mono-xs">est. time</span><span className="sk-mono-sm"><b>14m 22s</b></span></Row>
        <Row style={{ justifyContent: "space-between" }}><span className="sk-mono-xs">peak power</span><span className="sk-mono-sm"><b>8.4 MW</b></span></Row>
        <Row style={{ justifyContent: "space-between" }}><span className="sk-mono-xs">mass added</span><span className="sk-mono-sm"><b>+24.6 kg</b></span></Row>
        <Row style={{ justifyContent: "space-between" }}><span className="sk-mono-xs">missing</span><span className="sk-mono-sm" style={{ color: "#b88a00" }}><b>1 · coal ×16</b></span></Row>
      </Col>
      <Col gap={3}>
        <button className="sk-btn sk-accent" style={{ justifyContent: "center" }}>▶ commit plan</button>
        <button className="sk-btn" style={{ justifyContent: "center" }}>⊕ add to bookmarks</button>
      </Col>
    </Col>
  </Row>
);

const CpusRow = () => (
  <Row gap={8} style={{ alignItems: "stretch" }}>
    <Col gap={3} style={{ flex: 1 }}>
      {[
        ["α","4c", 92, "steel.plate ×512"],
        ["β","4c", 54, "circuit.basic ×128"],
        ["γ","2c", 18, "reactor.frame ×4"],
        ["δ","2c", 0, "— idle —"],
      ].map(([n, c, p, job], i) => (
        <Col key={i} gap={1}>
          <Row style={{ justifyContent: "space-between" }}>
            <span className="sk-mono-sm"><b>CPU-{n}</b> · {c}</span>
            <span className="sk-mono-xs">{p}%</span>
          </Row>
          <div className="sk-bar" style={{ height: 5 }}><i style={{ width: `${p}%` }}/></div>
          <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>{job}</span>
        </Col>
      ))}
    </Col>
    <Col gap={3} style={{ flex: 1.5 }}>
      <Row style={{ justifyContent: "space-between" }}>
        <span className="sk-h-xs">PROCESSES · 17</span>
        <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>total 5.5 MW</span>
      </Row>
      <div style={{ overflow: "auto", border: "1.5px solid var(--ink)", maxHeight: 120 }}>
        <table style={{ width: "100%", fontFamily: "var(--font-mono)", fontSize: 10, borderCollapse: "collapse" }}>
          <thead>
            <tr style={{ borderBottom: "1.5px solid var(--ink)", background: "var(--paper-2)", position: "sticky", top: 0 }}>
              {["pid","cpu","item","×","eta","stat"].map((h, i) => (
                <th key={i} style={{ textAlign: "left", padding: "2px 4px", fontFamily: "var(--font-label)" }}>{h}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {[
              ["#0421","α","steel.plate","512","0:42","run"],
              ["#0422","α","└ iron.plate","48","0:08","sub"],
              ["#0419","β","circuit.basic","128","1:55","run"],
              ["#0420","β","└ copper.wire","256","0:50","sub"],
              ["#0418","γ","reactor.frame","4","5:10","run"],
              ["#0417","—","wire.copper","2k","—","wait"],
              ["#0414","—","gear.bronze","32","—","ERR"],
            ].map((r, i) => (
              <tr key={i} style={{ borderBottom: "1px dashed var(--ink-faint)", background: r[5] === "ERR" ? "rgba(245,197,24,0.20)" : "transparent" }}>
                {r.map((c, j) => <td key={j} style={{ padding: "1px 4px" }}>{c}</td>)}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </Col>
  </Row>
);

const CodexDeepDive = () => (
  <Col gap={6} style={{ flex: 1, overflow: "auto" }}>
    <Row gap={3}>
      <span className="sk-tag sk-on">RECIPE</span>
      <span className="sk-tag">USES 47</span>
      <span className="sk-tag">DROPS</span>
      <span className="sk-tag">WIKI</span>
    </Row>
    <Row style={{ justifyContent: "space-between" }}>
      <span className="sk-h-sm">steel.plate</span>
      <Row gap={3}><span className="sk-tag">‹ 1/3</span><span className="sk-tag">›</span></Row>
    </Row>
    <div className="sk-box sk-thick" style={{ padding: 10 }}>
      <Row gap={10} style={{ alignItems: "center", justifyContent: "center" }}>
        <Col gap={2} style={{ alignItems: "center" }}>
          <span className="sk-mono-xs">INPUTS</span>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(3,1fr)", gap: 2 }}>
            {Array.from({ length: 9 }).map((_, i) => (
              <Slot key={i} style={{ width: 28, height: 28 }} filled={[0,1,2,4].includes(i)} icon={[0,1,2].includes(i) ? "◆" : i === 4 ? "◉" : null} qty={[0,1,2].includes(i) ? 1 : i === 4 ? 1 : null}/>
            ))}
          </div>
        </Col>
        <Col gap={1} style={{ alignItems: "center" }}>
          <span className="sk-arrow">⇒</span>
          <span className="sk-mono-xs">12.0s · 480 EU/t</span>
        </Col>
        <Col gap={2} style={{ alignItems: "center" }}>
          <span className="sk-mono-xs">OUTPUT</span>
          <Slot style={{ width: 44, height: 44 }} filled icon="▤" qty={4}/>
          <span className="sk-mono-xs">×4 · 1.6 kg</span>
        </Col>
      </Row>
    </div>
    <Row gap={3}>
      <button className="sk-btn sk-accent" style={{ flex: 1, justifyContent: "center" }}>▶ auto-craft</button>
      <button className="sk-btn">⊕ bm</button>
      <button className="sk-btn">★</button>
    </Row>
    <hr className="sk-div"/>
    <span className="sk-h-xs">USED IN · 47</span>
    <div className="sk-box" style={{ padding: 4 }}>
      {[
        ["frame.steel","CRAFT",4],
        ["pipe.steel","ROLL",2],
        ["plate.armor","PRESS",1],
        ["bolt.steel","LATHE",16],
        ["rail.heavy","CRAFT",6],
        ["gear.steel","MOLD",1],
      ].map((r, i) => (
        <Row key={i} gap={3} style={{ padding: "1px 0", borderBottom: "1px dashed var(--ink-faint)" }}>
          <Slot style={{ width: 14, height: 14 }} filled icon="·"/>
          <span className="sk-mono-sm" style={{ flex: 1 }}>{r[0]}</span>
          <span className="sk-tag" style={{ fontSize: 7, padding: "0 2px" }}>{r[1]}</span>
          <span className="sk-mono-xs">×{r[2]}</span>
        </Row>
      ))}
    </div>
  </Col>
);

window.IntegratedTerminal = IntegratedTerminal;
