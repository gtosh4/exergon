/* global React, Slot, Row, Col */

// =====================================================================
// MACHINE UI — shared atoms
// =====================================================================
const MachIcon = ({ glyph = "▦", size = 18, dashed }) => (
  <div className={`sk-slot ${dashed ? "" : "sk-filled"}`}
       style={{ width: size, height: size, fontSize: size * 0.55 }}>
    <span className="sk-icon" style={{ fontSize: size * 0.6 }}>{glyph}</span>
  </div>
);

// I/O port pill — used on every variation
const Port = ({ dir = "in", label, item, qty, rate, alarm }) => (
  <div className="sk-box" style={{
    padding: "4px 6px",
    display: "flex", gap: 5, alignItems: "center",
    background: alarm ? "rgba(245,197,24,0.22)" : "var(--paper)",
    borderStyle: dir === "in" ? "solid" : "solid",
  }}>
    <span className="sk-mono-xs" style={{ width: 14, color: "var(--ink-faint)" }}>
      {dir === "in" ? "▸" : "◂"}
    </span>
    <Slot filled style={{ width: 22, height: 22 }} icon={item} qty={qty} />
    <Col gap={0} style={{ minWidth: 0, lineHeight: 1.05 }}>
      <span className="sk-mono-sm" style={{ fontWeight: 700, whiteSpace: "nowrap" }}>{label}</span>
      <span className="sk-mono-xs" style={{ color: alarm ? "#b88a00" : "var(--ink-faint)" }}>
        {rate}
      </span>
    </Col>
  </div>
);

// Progress / fill bar (re-export look of sk-bar, vertical option)
const VBar = ({ pct = 0, label, h = 90, hatched }) => (
  <Col gap={2} style={{ alignItems: "center" }}>
    <div style={{
      width: 14, height: h, border: "1.5px solid var(--ink)",
      background: "var(--paper)", position: "relative",
    }}>
      <i style={{
        position: "absolute", left: 0, right: 0, bottom: 0,
        height: `${pct}%`,
        background: hatched
          ? "repeating-linear-gradient(45deg, var(--ink) 0 2px, transparent 2px 5px)"
          : "var(--ink)"
      }}/>
    </div>
    <span className="sk-mono-xs">{label}</span>
  </Col>
);

// Stamp-style status badge
const Stamp = ({ kind = "ok", children }) => {
  const map = {
    ok:    { bg: "transparent",         color: "var(--ink)",  rot: -3 },
    warn:  { bg: "var(--accent)",       color: "var(--ink)",  rot: 4  },
    err:   { bg: "rgba(163,25,25,0.1)", color: "#a31919",     rot: -2 },
    idle:  { bg: "var(--paper-2)",      color: "var(--ink-faint)", rot: 1 },
  };
  const s = map[kind] || map.ok;
  return (
    <span style={{
      display: "inline-block",
      transform: `rotate(${s.rot}deg)`,
      border: `2px solid ${s.color}`,
      color: s.color, background: s.bg,
      padding: "1px 6px",
      fontFamily: "var(--font-label)",
      fontSize: 10, letterSpacing: "1px", textTransform: "uppercase",
    }}>{children}</span>
  );
};

// Mode pill — passive/autocraft toggle
const ModePill = ({ mode = "passive", onChange }) => (
  <Row gap={0} style={{ border: "1.5px solid var(--ink)" }}>
    {["passive", "autocraft", "off"].map(m => (
      <button key={m}
        onClick={() => onChange?.(m)}
        className="sk-mono-xs"
        style={{
          padding: "3px 8px", border: 0, cursor: "pointer",
          background: mode === m ? "var(--ink)" : "var(--paper)",
          color: mode === m ? "var(--paper)" : "var(--ink)",
          fontFamily: "var(--font-mono)", textTransform: "uppercase",
          letterSpacing: "0.6px", fontWeight: 700,
        }}>{m}</button>
    ))}
  </Row>
);

// Hand-written annotation pinned absolutely
const Annot = ({ x, y, w, children, accent }) => (
  <div className={`sk-annot ${accent ? "sk-accent-text" : ""}`}
       style={{ left: x, top: y, width: w, transform: "rotate(-2deg)" }}>
    {children}
  </div>
);

// =====================================================================
// VARIATION 1 — TABBED ASSEMBLER (single-recipe, conventional)
// "the by-the-book one" — 4 tabs across top, recipe is centerpiece
// =====================================================================
const MachineV1 = () => {
  const [tab, setTab] = React.useState("RECIPE");
  return (
    <div className="paper" style={{ padding: 16, height: "100%", display: "flex", flexDirection: "column", gap: 10 }}>
      {/* HEADER */}
      <Row style={{ justifyContent: "space-between", alignItems: "flex-end" }}>
        <Row gap={10} style={{ alignItems: "flex-end" }}>
          <MachIcon glyph="▦" size={36}/>
          <Col gap={1}>
            <Row gap={6} style={{ alignItems: "baseline" }}>
              <span className="sk-h">ASSEMBLER · LV3</span>
              <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>#A-04 · sector γ</span>
            </Row>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>
              single-recipe · 4 input · 1 output · 4 modules
            </span>
          </Col>
        </Row>
        <Row gap={6}>
          <Stamp kind="ok">RUNNING</Stamp>
          <ModePill mode="passive"/>
        </Row>
      </Row>

      {/* TABS */}
      <Row gap={0} style={{ borderBottom: "1.5px solid var(--ink)" }}>
        {["RECIPE", "I/O", "MODULES", "TELEMETRY"].map(t => (
          <button key={t}
            onClick={() => setTab(t)}
            className={`sk-btn ${t === tab ? "sk-on" : ""}`}
            style={{
              borderBottom: 0, boxShadow: "none",
              borderRadius: 0, marginRight: -1.5,
              padding: "5px 14px", letterSpacing: "0.8px",
            }}>{t}</button>
        ))}
        <span className="sk-mono-xs" style={{ marginLeft: "auto", alignSelf: "center", color: "var(--ink-faint)" }}>
          12.0s/cycle · 480 EU/t
        </span>
      </Row>

      {/* BODY — recipe canvas */}
      <Row gap={14} style={{ flex: 1, alignItems: "stretch" }}>
        {/* LEFT — input ports + recipe in center */}
        <Col gap={6} style={{ flex: 1 }}>
          <span className="sk-h-sm">RECIPE · steel.plate ×4</span>
          <div className="sk-box" style={{ flex: 1, padding: 24, position: "relative" }}>
            <Row gap={26} style={{ alignItems: "center", justifyContent: "center", height: "100%" }}>
              {/* Inputs */}
              <Col gap={3} style={{ alignItems: "center" }}>
                <span className="sk-mono-xs">INPUTS</span>
                <div style={{ display: "grid", gridTemplateColumns: "repeat(2,1fr)", gap: 3 }}>
                  {[
                    ["◆", 3, "iron"],
                    ["◉", 1, "coal"],
                    ["▢", 0, null],
                    ["▢", 0, null],
                  ].map((it, i) => (
                    <Slot key={i} filled={!!it[2]} style={{ width: 44, height: 44 }}
                          icon={it[2] ? it[0] : null} qty={it[2] ? it[1] : null}/>
                  ))}
                </div>
                <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>3 iron + 1 coal</span>
              </Col>

              <span className="sk-arrow">⇒</span>

              {/* Process */}
              <Col gap={3} style={{ alignItems: "center" }}>
                <span className="sk-mono-xs">PROCESS · 8.4s left</span>
                <div className="sk-bar" style={{ width: 130 }}>
                  <i style={{ width: "62%" }}/>
                </div>
                <span className="sk-mono-xs">cycle 47 / ∞</span>
                <div style={{ position: "relative", marginTop: 6 }}>
                  <div className="sk-box" style={{ padding: "3px 10px" }}>
                    <span className="sk-h-sm">62%</span>
                  </div>
                </div>
              </Col>

              <span className="sk-arrow">⇒</span>

              {/* Output */}
              <Col gap={3} style={{ alignItems: "center" }}>
                <span className="sk-mono-xs">OUTPUT</span>
                <Slot filled style={{ width: 60, height: 60 }} icon="▤" qty={4}/>
                <span className="sk-mono-xs">steel.plate</span>
              </Col>
            </Row>
            <Annot x={310} y={70} w={140}>← byproduct slot is empty</Annot>
          </div>

          {/* MODE STRIP */}
          <Row gap={10} style={{ alignItems: "center" }}>
            <Col gap={2}>
              <span className="sk-mono-xs">MODE</span>
              <ModePill mode="passive"/>
            </Col>
            <Col gap={2} style={{ flex: 1 }}>
              <Row style={{ justifyContent: "space-between" }}>
                <span className="sk-mono-xs">PASSIVE LIMIT — pause when output ≥</span>
                <span className="sk-mono-sm" style={{ fontWeight: 700 }}>500</span>
              </Row>
              <Row gap={6} style={{ alignItems: "center" }}>
                <div className="sk-bar" style={{ flex: 1, height: 14 }}>
                  <i style={{ width: "38%" }}/>
                  <span style={{
                    position: "absolute", left: "calc(38% - 4px)", top: -3, height: 18,
                    width: 8, background: "var(--accent)", border: "1.5px solid var(--ink)"
                  }}/>
                </div>
                <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>192 / 500</span>
              </Row>
            </Col>
          </Row>
        </Col>

        {/* RIGHT — sidebar: ports + modules + power */}
        <Col gap={8} style={{ width: 280 }}>
          <Col gap={3}>
            <span className="sk-h-sm">PORTS</span>
            <Port dir="in" label="iron" item="◆" qty={64} rate="48/min from belt-N"/>
            <Port dir="in" label="coal" item="◉" qty={22} rate="16/min from belt-W" alarm/>
            <Port dir="out" label="steel.plate" item="▤" qty={192} rate="20/min → bus-S"/>
          </Col>

          <Col gap={3}>
            <Row style={{ justifyContent: "space-between", alignItems: "baseline" }}>
              <span className="sk-h-sm">MODULES · 3/4</span>
              <span className="sk-mono-xs sk-squig">configure →</span>
            </Row>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(4,1fr)", gap: 4 }}>
              {[
                { g: "✦", label: "speed II" },
                { g: "◈", label: "eff I"  },
                { g: "◇", label: "yield I" },
                { g: "+", label: null, dashed: true },
              ].map((m, i) => (
                <div key={i} className={`sk-slot ${m.dashed ? "" : "sk-filled"}`}
                     style={{ width: "100%", height: 44, flexDirection: "column", padding: 2 }}>
                  <span className="sk-icon" style={{ fontSize: 16 }}>{m.g}</span>
                  <span className="sk-mono-xs" style={{ fontSize: 7 }}>{m.label || "empty"}</span>
                </div>
              ))}
            </div>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>
              +50% speed · −15% energy · +10% yield
            </span>
          </Col>

          <Col gap={3}>
            <span className="sk-h-sm">POWER</span>
            <Row gap={6} style={{ alignItems: "center" }}>
              <VBar pct={68} h={48} label="" hatched/>
              <Col gap={0} style={{ flex: 1 }}>
                <span className="sk-mono-sm" style={{ fontWeight: 700 }}>480 / 720 EU/t</span>
                <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>buffer 68%</span>
                <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>grid: stable</span>
              </Col>
            </Row>
          </Col>
        </Col>
      </Row>
    </div>
  );
};

// =====================================================================
// VARIATION 2 — SCHEMATIC / EVERYTHING-VISIBLE (multi-recipe crafter)
// machine in the middle, ports + modules + recipe queue radiating out
// =====================================================================
const MachineV2 = () => {
  const recipes = [
    { name: "circuit.basic ×4",   pri: 8, mode: "auto",    job: "claim", time: "6.0s" },
    { name: "circuit.adv ×1",     pri: 5, mode: "auto",    job: "1 / 4", time: "32.0s" },
    { name: "wire.copper ×8",     pri: 3, mode: "passive", job: null,    time: "1.2s",  thresh: "≤ 256" },
    { name: "gear.bronze ×2",     pri: 2, mode: "passive", job: null,    time: "8.0s",  thresh: "≤ 64" },
    { name: "plate.tin ×4",       pri: 1, mode: "off",     job: null,    time: "4.0s" },
    { name: "rod.steel ×1",       pri: 1, mode: "off",     job: null,    time: "2.5s" },
  ];
  return (
    <div className="paper" style={{ padding: 16, height: "100%", position: "relative" }}>
      {/* faint grid subtitle stamp */}
      <div style={{ position: "absolute", top: 12, right: 16, transform: "rotate(2deg)", opacity: 0.6 }}>
        <Stamp kind="warn">SCHEMATIC · v.2</Stamp>
      </div>

      {/* HEADER */}
      <Row style={{ justifyContent: "space-between", alignItems: "flex-end", marginBottom: 8 }}>
        <Col gap={1}>
          <Row gap={8} style={{ alignItems: "baseline" }}>
            <span className="sk-h">ASSEMBLY MATRIX · LV4</span>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>#M-12 · multi-pattern · 32 recipes</span>
          </Row>
          <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>
            current job <b>circuit.basic ×4</b> from CPU-α · 47% · 3.2s left
          </span>
        </Col>
        <Stamp kind="ok">CLAIMED · CPU-α</Stamp>
      </Row>
      <hr className="sk-div"/>

      {/* MAIN LAYOUT: 3 columns — left ports, center machine, right recipes */}
      <Row gap={12} style={{ height: "calc(100% - 60px)" }}>
        {/* LEFT — input ports stack */}
        <Col gap={4} style={{ width: 230 }}>
          <span className="sk-h-sm">▸ INPUTS</span>
          {[
            ["copper",       "◆", 256, "32/min"],
            ["silicon",      "◇", 88,  "8/min"],
            ["solder",       "·", 12,  "low!", true],
            ["plastic",      "▣", 144, "16/min"],
            ["circuit.base", "▦", 24,  "internal"],
          ].map((p, i) => (
            <Port key={i} dir="in" label={p[0]} item={p[1]} qty={p[2]} rate={p[3]} alarm={p[4]}/>
          ))}
          <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>+ 3 unfiltered ports</span>
        </Col>

        {/* CENTER — machine schematic + module ring + power */}
        <Col gap={6} style={{ flex: 1, alignItems: "stretch" }}>
          <div className="sk-box" style={{ flex: 1, padding: 14, position: "relative" }}>
            {/* faint cross-hair grid behind */}
            <div style={{
              position: "absolute", inset: 0,
              backgroundImage: "linear-gradient(rgba(26,26,26,0.05) 1px, transparent 1px), linear-gradient(90deg, rgba(26,26,26,0.05) 1px, transparent 1px)",
              backgroundSize: "24px 24px", pointerEvents: "none",
            }}/>

            {/* central machine block */}
            <div style={{
              position: "absolute", left: "50%", top: "50%",
              transform: "translate(-50%, -50%)",
              border: "2.5px solid var(--ink)",
              background: "var(--paper)",
              padding: "18px 22px",
              boxShadow: "3px 3px 0 var(--ink)",
              minWidth: 240,
            }}>
              <Col gap={4} style={{ alignItems: "center" }}>
                <span className="sk-h-sm">[ MATRIX CORE ]</span>
                <Row gap={6} style={{ alignItems: "center" }}>
                  <Slot filled style={{ width: 40, height: 40 }} icon="▦" qty={4}/>
                  <span className="sk-arrow">⇒</span>
                  <Slot filled style={{ width: 40, height: 40 }} icon="▩" qty={4}/>
                </Row>
                <span className="sk-mono-xs">cycle 47 · 3.2s left</span>
                <div className="sk-bar" style={{ width: 200 }}>
                  <i style={{ width: "47%" }}/>
                </div>
                <Row gap={4}>
                  <span className="sk-tag">480 EU/t</span>
                  <span className="sk-tag">6.0 s</span>
                  <span className="sk-tag sk-on">CPU-α</span>
                </Row>
              </Col>
            </div>

            {/* modules pinned at corners */}
            {[
              { x: 20,  y: 20,  g: "✦", l: "speed II"  },
              { x: 20,  y: "calc(100% - 64px)", g: "◈", l: "eff I" },
              { x: "calc(100% - 80px)", y: 20, g: "◇", l: "yield I" },
              { x: "calc(100% - 80px)", y: "calc(100% - 64px)", g: "+", l: "empty", dashed: true },
            ].map((m, i) => (
              <div key={i} style={{ position: "absolute", left: m.x, top: m.y }}>
                <div className={`sk-slot ${m.dashed ? "" : "sk-filled"}`}
                     style={{ width: 60, height: 44, flexDirection: "column" }}>
                  <span className="sk-icon">{m.g}</span>
                  <span className="sk-mono-xs" style={{ fontSize: 7 }}>{m.l}</span>
                </div>
              </div>
            ))}

            {/* sketched module bus lines */}
            <svg style={{ position: "absolute", inset: 0, width: "100%", height: "100%", pointerEvents: "none" }}>
              <path d="M70,40 C150,80 200,140 260,180" stroke="var(--ink-faint)" strokeWidth="1" strokeDasharray="3 3" fill="none"/>
              <path d="M70,calc(100%-40px) C150,calc(100%-80px) 200,calc(100%-140px) 260,calc(100%-180px)" stroke="var(--ink-faint)" strokeWidth="1" strokeDasharray="3 3" fill="none"/>
            </svg>

            <Annot x={20} y="calc(100% - 110px)" w={200} accent>
              ↑ modules attach to physical sockets
            </Annot>
          </div>

          {/* power + summary strip */}
          <Row gap={10} style={{ alignItems: "center" }}>
            <Col gap={1}>
              <span className="sk-mono-xs">POWER</span>
              <Row gap={4}>
                <div className="sk-bar" style={{ width: 120 }}><i style={{ width: "78%" }}/></div>
                <span className="sk-mono-sm">560/720 EU/t</span>
              </Row>
            </Col>
            <Col gap={1}>
              <span className="sk-mono-xs">UPTIME</span>
              <span className="sk-mono-sm" style={{ fontWeight: 700 }}>92.4%  · last 30m</span>
            </Col>
            <Col gap={1}>
              <span className="sk-mono-xs">BACKLOG</span>
              <span className="sk-mono-sm" style={{ fontWeight: 700 }}>2 jobs claimed</span>
            </Col>
            <button className="sk-btn sk-accent" style={{ marginLeft: "auto" }}>+ assign recipe</button>
          </Row>
        </Col>

        {/* RIGHT — recipe queue */}
        <Col gap={4} style={{ width: 290 }}>
          <Row style={{ justifyContent: "space-between", alignItems: "baseline" }}>
            <span className="sk-h-sm">RECIPES · 6 / 32</span>
            <Row gap={3}>
              <span className="sk-tag sk-on">all</span>
              <span className="sk-tag">auto</span>
              <span className="sk-tag">passive</span>
            </Row>
          </Row>
          {recipes.map((r, i) => (
            <div key={i} className={`sk-box ${r.mode === "off" ? "sk-dashed" : ""}`}
                 style={{ padding: 6, opacity: r.mode === "off" ? 0.5 : 1 }}>
              <Row style={{ justifyContent: "space-between" }}>
                <Row gap={5}>
                  <Slot filled style={{ width: 18, height: 18 }} icon={["▦","▧","▨","◈","▢","▣"][i]}/>
                  <span className="sk-mono-sm" style={{ fontWeight: 700 }}>{r.name}</span>
                </Row>
                <Row gap={2}>
                  {[1,2,3,4,5,6,7,8].map(p => (
                    <span key={p} style={{
                      width: 4, height: 8,
                      background: p <= r.pri ? "var(--ink)" : "var(--paper-2)",
                      border: "0.5px solid var(--ink)"
                    }}/>
                  ))}
                </Row>
              </Row>
              <Row gap={6} style={{ marginTop: 3 }}>
                <span className="sk-tag" style={{
                  background: r.mode === "auto" ? "var(--ink)" : r.mode === "passive" ? "var(--accent)" : "var(--paper)",
                  color: r.mode === "auto" ? "var(--paper)" : "var(--ink)",
                  fontSize: 7,
                }}>{r.mode}</span>
                <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>{r.time}</span>
                {r.thresh && <span className="sk-mono-xs">{r.thresh}</span>}
                {r.job && <span className="sk-mono-xs sk-squig" style={{ marginLeft: "auto" }}>job {r.job}</span>}
              </Row>
            </div>
          ))}
        </Col>
      </Row>
    </div>
  );
};

// =====================================================================
// VARIATION 3 — SIDE-RAIL TERMINAL (dense / AE2-ish)
// vertical machine status rail on left, big recipe table on right
// recipes are a fixed list (always present) · mode = passive ☐ + craft ☐
// (independent flags) · limit configures inline when passive is on
// =====================================================================
const MachineV3 = ({ portsSlot = null }) => {
  // mode flags: P = passive, C = autocraft. Either, neither, both.
  // priority: integer (click to edit). 0 = lowest. higher wins ties.
  // limit: only meaningful when P=true.
  const rowsRaw = [
    { icon: "▦", inputs: [["◆",3],["◉",1]],          out: ["▦",4],  P: false, C: true,  pri: 80, limit: null, rate: "3.2s",  claim: { state: "running", from: "CPU-α", pct: 47 }, name: "circuit.basic" },
    { icon: "▧", inputs: [["▦",2],["◇",1],["·",1]],  out: ["▧",1],  P: false, C: true,  pri: 60, limit: null, rate: "12s",   claim: { state: "queued",  from: "CPU-α" }, name: "circuit.adv" },
    { icon: "◉", inputs: [["▤",1],["◆",2]],          out: ["◉",16], P: true,  C: true,  pri: 50, limit: 1024, rate: "0.8s",  claim: { state: "queued",  from: "CPU-β" }, name: "rivet.steel" },
    { icon: "◈", inputs: [["◆",2],["▤",1]],          out: ["◈",2],  P: true,  C: false, pri: 40, limit: 64,   rate: "2.4s",  name: "gear.bronze" },
    { icon: "▨", inputs: [["◆",1]],                  out: ["▨",8],  P: true,  C: false, pri: 30, limit: 256,  rate: "1.6s",  name: "wire.copper" },
    { icon: "◇", inputs: [["◆",1]],                  out: ["◇",4],  P: true,  C: false, pri: 20, limit: 32,   rate: "1.2s",  name: "spring.bronze" },
    { icon: "▢", inputs: [["·",1]],                  out: ["▢",4],  P: false, C: false, pri: 10, limit: null, rate: "0.4s",  name: "plate.tin" },
    { icon: "▣", inputs: [["▤",1]],                  out: ["▣",1],  P: false, C: false, pri: 0,  limit: null, rate: "0.6s",  name: "rod.steel" },
  ];
  const rows = [...rowsRaw].sort((a, b) => b.pri - a.pri);
  const activeIdx = rows.findIndex(r => r.claim?.state === "running");
  const active = rows[activeIdx];

  // mode-flag cell — order: C P [▾ limit] · all on one row
  const ModeFlags = ({ P, C, limit }) => (
    <Row gap={4} style={{ justifyContent: "flex-start", alignItems: "center" }}>
      <span title="auto-craft" style={{
        display: "inline-flex", alignItems: "center", justifyContent: "center",
        width: 18, height: 18, border: "1.5px solid var(--ink)",
        background: C ? "var(--ink)" : "var(--paper)",
        color: C ? "var(--paper)" : "var(--ink)",
        fontFamily: "var(--font-mono)", fontSize: 11, fontWeight: 700,
        cursor: "pointer", lineHeight: 1,
      }}>{C ? "C" : ""}</span>
      <span title="passive" style={{
        display: "inline-flex", alignItems: "center", justifyContent: "center",
        width: 18, height: 18, border: "1.5px solid var(--ink)",
        background: P ? "var(--accent)" : "var(--paper)",
        fontFamily: "var(--font-mono)", fontSize: 11, fontWeight: 700,
        color: "var(--ink)", cursor: "pointer", lineHeight: 1,
      }}>{P ? "P" : ""}</span>
      {P ? (
        <Row gap={2} style={{ alignItems: "center" }}>
          <span style={{ fontFamily: "var(--font-mono)", fontSize: 10, color: "var(--ink-faint)", lineHeight: 1 }}>⌄</span>
          <span style={{
            padding: "1px 5px",
            border: "1.5px dashed var(--ink)",
            background: "var(--paper)",
            fontFamily: "var(--font-mono)", fontSize: 11, fontWeight: 700,
            cursor: "text", lineHeight: 1.1,
          }}>{limit ?? "—"}</span>
        </Row>
      ) : null}
    </Row>
  );

  return (
    <div className="paper" style={{ padding: 0, height: "100%", display: "flex" }}>
      {/* LEFT RAIL — machine identity & live status */}
      <Col gap={0} style={{
        width: 260, padding: 14, gap: 10,
        borderRight: "1.5px solid var(--ink)",
        background: "var(--paper-2)",
      }}>
        <Row gap={8} style={{ alignItems: "flex-start" }}>
          <MachIcon glyph="▦" size={48}/>
          <Col gap={1}>
            <span className="sk-h">CRAFTER</span>
            <span className="sk-h-sm" style={{ color: "var(--ink-faint)" }}>LV4 · #M-12</span>
          </Col>
        </Row>

        <hr className="sk-div"/>

        {/* CURRENT CRAFT — status, recipe, progress bar all merged */}
        <Col gap={4}>
          <Row style={{ justifyContent: "space-between", alignItems: "baseline" }}>
            <span className="sk-mono-xs">CURRENT CRAFT</span>
            <Stamp kind="ok">RUNNING</Stamp>
          </Row>
          <div className="sk-box sk-thick" style={{ padding: 8, position: "relative", overflow: "hidden" }}>
            {/* progress fill behind content */}
            <div style={{
              position: "absolute", left: 0, top: 0, bottom: 0,
              width: `${active.claim.pct}%`,
              background: "repeating-linear-gradient(45deg, rgba(245,197,24,0.35) 0 3px, transparent 3px 6px)",
              pointerEvents: "none",
            }}/>
            <Row gap={6} style={{ position: "relative" }}>
              <Slot filled style={{ width: 32, height: 32 }} icon={active.icon} qty={active.out[1]}/>
              <Col gap={0} style={{ minWidth: 0, flex: 1 }}>
                <span className="sk-mono-sm" style={{ fontWeight: 700 }}>{active.name} ×{active.out[1]}</span>
                <Row style={{ justifyContent: "space-between" }}>
                  <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>
                    {active.claim.from} · cycle 47
                  </span>
                  <span className="sk-mono-sm" style={{ fontWeight: 700 }}>{active.claim.pct}%</span>
                </Row>
              </Col>
            </Row>
            <span className="sk-mono-xs" style={{ position: "relative", display: "block", marginTop: 2, color: "var(--ink-faint)" }}>
              3.2s remaining · job #0421
            </span>
          </div>
        </Col>

        <Col gap={3}>
          <span className="sk-mono-xs">POWER · 560/720 EU/t</span>
          <Row gap={4} style={{ alignItems: "flex-end" }}>
            <VBar pct={78} h={56} label="in"/>
            <VBar pct={68} h={56} label="buf" hatched/>
            <Col gap={1} style={{ flex: 1, fontSize: 10, fontFamily: "var(--font-mono)" }}>
              <span>peak 612</span>
              <span style={{ color: "var(--ink-faint)" }}>idle 24</span>
              <span style={{ color: "var(--ink-faint)" }}>grid OK</span>
            </Col>
          </Row>
        </Col>

        <Col gap={3}>
          <span className="sk-mono-xs">MODULES · 3 attached</span>
          <Row gap={3}>
            {[
              { g: "✦", l: "+50% spd"  },
              { g: "◈", l: "−15% nrg"  },
              { g: "◇", l: "+10% yld" },
            ].map((m, i) => (
              <div key={i} className="sk-slot sk-filled"
                   style={{ width: 48, height: 38, flexDirection: "column", padding: 2 }}>
                <span className="sk-icon" style={{ fontSize: 14 }}>{m.g}</span>
                <span style={{ fontSize: 6, fontFamily: "var(--font-mono)" }}>{m.l}</span>
              </div>
            ))}
          </Row>
        </Col>

        <div style={{ marginTop: "auto" }}>
          {portsSlot || (
            <Col gap={3}>
              <span className="sk-mono-xs">PORTS</span>
              <Row style={{ justifyContent: "space-between" }}>
                <span className="sk-mono-xs">▸ inputs</span>
                <span className="sk-mono-sm">5 / 8</span>
              </Row>
              <Row style={{ justifyContent: "space-between" }}>
                <span className="sk-mono-xs">◂ outputs</span>
                <span className="sk-mono-sm">2 / 4</span>
              </Row>
              <button className="sk-btn">configure ports →</button>
            </Col>
          )}
        </div>
      </Col>

      {/* RIGHT — big recipe table */}
      <Col gap={8} style={{ flex: 1, padding: 14 }}>
        <Row style={{ justifyContent: "space-between", alignItems: "flex-end" }}>
          <Col gap={1}>
            <span className="sk-h">RECIPES</span>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>
              {rows.length} available · sorted by priority ↓
            </span>
          </Col>
          <div className="sk-box" style={{ padding: "8px 12px", minWidth: 280 }}>
            <Row gap={6} style={{ alignItems: "center" }}>
              <span className="sk-mono" style={{ fontSize: 14 }}>⌕</span>
              <span className="sk-mono" style={{ fontSize: 13, color: "var(--ink-faint)" }}>filter recipes…</span>
            </Row>
          </div>
        </Row>

        {/* BULK SELECT TOOLBAR — appears when ≥1 row selected */}
        <div className="sk-box" style={{
          padding: "5px 8px",
          background: "rgba(245,197,24,0.15)",
          borderStyle: "dashed",
        }}>
          <Row gap={8} style={{ alignItems: "center", flexWrap: "wrap" }}>
            <span className="sk-mono-xs" style={{ fontWeight: 700 }}>
              ☑ 3 selected
            </span>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>bulk:</span>
            <Row gap={3}>
              <span className="sk-tag">+ passive</span>
              <span className="sk-tag">− passive</span>
              <span className="sk-tag">+ craft</span>
              <span className="sk-tag">− craft</span>
            </Row>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)", marginLeft: 4 }}>priority:</span>
            <Row gap={3}>
              <span className="sk-tag">↑ +10</span>
              <span className="sk-tag">↓ −10</span>
              <span className="sk-tag">set…</span>
            </Row>
            <span className="sk-mono-xs sk-squig" style={{ marginLeft: "auto" }}>clear ✕</span>
          </Row>
        </div>

        <table style={{
          width: "100%", borderCollapse: "collapse",
          fontFamily: "var(--font-mono)", fontSize: 11,
        }}>
          <thead>
            <tr style={{ borderBottom: "1.5px solid var(--ink)" }}>
              <th style={{ padding: "5px 8px", width: 18, textAlign: "left" }}>
                <span title="select all" style={{
                  display: "inline-block", width: 12, height: 12,
                  border: "1.5px solid var(--ink)",
                  background: "repeating-linear-gradient(45deg, var(--ink) 0 2px, transparent 2px 4px)",
                  cursor: "pointer",
                }}/>
              </th>
              {["recipe", "inputs → outputs", "rate", "mode", "pri", ""].map((h, i) => (
                <th key={i} style={{
                  textAlign: i < 2 ? "left" : "center",
                  padding: "5px 8px", fontFamily: "var(--font-label)",
                  fontWeight: 400, letterSpacing: "0.4px",
                  color: "var(--ink-soft)",
                }}>{h}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((r, i) => {
              const isActive = r === active;
              const dim = !r.P && !r.C;
              return (
                <React.Fragment key={i}>
                  <tr style={{
                    borderBottom: "1px dashed var(--ink-faint)",
                    background: isActive ? "rgba(245,197,24,0.18)" : "transparent",
                    opacity: dim ? 0.5 : 1,
                  }}>
                    {/* row select checkbox */}
                    <td style={{ padding: "4px 8px", width: 18 }}>
                      <span style={{
                        display: "inline-block", width: 12, height: 12,
                        border: "1.5px solid var(--ink)",
                        background: [0,2,4].includes(i) ? "var(--ink)" : "var(--paper)",
                      }}/>
                    </td>
                    {/* name */}
                    <td style={{ padding: "4px 8px", fontWeight: 700 }}>{r.name}</td>
                    {/* inputs → outputs as icons w/ qty */}
                    <td style={{ padding: "4px 8px" }}>
                      <Row gap={3} style={{ alignItems: "center" }}>
                        {r.inputs.map((inp, k) => (
                          <Slot key={k} filled style={{ width: 22, height: 22 }} icon={inp[0]} qty={inp[1]}/>
                        ))}
                        <span className="sk-arrow" style={{ fontSize: 14, marginLeft: 2, marginRight: 2 }}>⇒</span>
                        <Slot filled style={{ width: 24, height: 24 }} icon={r.out[0]} qty={r.out[1]}/>
                      </Row>
                    </td>
                    {/* rate — how fast machine processes this recipe */}
                    <td style={{ padding: "4px 8px", textAlign: "center" }}>
                      <span className="sk-mono-sm" style={{
                        fontFamily: "var(--font-mono)", fontSize: 11,
                        fontWeight: 700, color: "var(--ink-soft)",
                      }}>{r.rate}</span>
                    </td>
                    {/* mode = two independent flags + inline passive limit */}
                    <td style={{ padding: "4px 8px" }}>
                      <ModeFlags P={r.P} C={r.C} limit={r.limit}/>
                    </td>
                    {/* priority — number, click to edit */}
                    <td style={{ padding: "4px 8px", textAlign: "center" }}>
                      <span style={{
                        display: "inline-block",
                        minWidth: 32,
                        padding: "1px 6px",
                        border: "1.5px dashed var(--ink-faint)",
                        background: "var(--paper)",
                        fontFamily: "var(--font-mono)", fontSize: 12, fontWeight: 700,
                        cursor: "text",
                      }}>{r.pri}</span>
                    </td>
                    {/* claim/state hint */}
                    <td style={{ padding: "4px 8px", textAlign: "right", color: "var(--ink-faint)", whiteSpace: "nowrap" }}>
                      {r.claim?.state === "running" && <span className="sk-mono-xs" style={{ color: "var(--ink)", fontWeight: 700 }}>▶ {r.claim.pct}%</span>}
                      {r.claim?.state === "queued"  && <span className="sk-mono-xs">queued · {r.claim.from}</span>}
                      {!r.claim && <span className="sk-mono-xs">⋯</span>}
                    </td>
                  </tr>
                </React.Fragment>
              );
            })}
          </tbody>
        </table>

        <Row gap={8} style={{ marginTop: "auto", alignItems: "center" }}>
          <Row gap={4} style={{ alignItems: "center" }}>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>legend:</span>
            <span style={{
              display: "inline-flex", alignItems: "center", justifyContent: "center",
              width: 16, height: 16, border: "1.5px solid var(--ink)",
              background: "var(--accent)", fontSize: 10, fontWeight: 700,
            }}>P</span>
            <span className="sk-mono-xs">passive</span>
            <span style={{
              display: "inline-flex", alignItems: "center", justifyContent: "center",
              width: 16, height: 16, border: "1.5px solid var(--ink)",
              background: "var(--ink)", color: "var(--paper)", fontSize: 10, fontWeight: 700,
            }}>C</span>
            <span className="sk-mono-xs">auto-craft</span>
          </Row>
          <span className="sk-mono-xs" style={{ marginLeft: "auto", color: "var(--ink-faint)" }}>
            higher pri wins ties · click pri to edit
          </span>
        </Row>
      </Col>
    </div>
  );
};

// =====================================================================
// VARIATION 4 — BLUEPRINT MULTIBLOCK (fluid + item, many ports)
// scaled top-down schematic of a multiblock w/ port labels around it
// =====================================================================
const MachineV4 = () => (
  <div className="paper" style={{ padding: 16, height: "100%", display: "flex", flexDirection: "column", gap: 10 }}>
    <Row style={{ justifyContent: "space-between", alignItems: "flex-end" }}>
      <Col gap={1}>
        <Row gap={8} style={{ alignItems: "baseline" }}>
          <span className="sk-h">CHEM REACTOR · 3×3×4</span>
          <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>multiblock · #C-01 · sector δ</span>
        </Row>
        <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>
          fluid + item · 6 input ports · 4 output ports · 6 module sockets
        </span>
      </Col>
      <Row gap={8}>
        <Stamp kind="warn">UNDERFED · solder low</Stamp>
        <ModePill mode="autocraft"/>
      </Row>
    </Row>
    <hr className="sk-div"/>

    <Row gap={14} style={{ flex: 1 }}>
      {/* LEFT — ports column */}
      <Col gap={5} style={{ width: 220 }}>
        <span className="sk-h-sm">▸ INPUTS · 6</span>
        <Port dir="in" label="naphtha"   item="≈" qty={2400} rate="80 mB/t"/>
        <Port dir="in" label="hydrogen"  item="∽" qty={1200} rate="40 mB/t"/>
        <Port dir="in" label="catalyst"  item="◆" qty={4}    rate="− idle"/>
        <Port dir="in" label="solder"    item="·" qty={2}    rate="!! low"  alarm/>
        <Port dir="in" label="—"         item="?" qty={0}    rate="unfilt."/>
        <Port dir="in" label="—"         item="?" qty={0}    rate="unfilt."/>
      </Col>

      {/* CENTER — blueprint schematic */}
      <Col gap={4} style={{ flex: 1, alignItems: "stretch" }}>
        <div className="sk-box sk-thick" style={{
          flex: 1, position: "relative", padding: 12,
          background: "var(--paper)",
        }}>
          {/* dashed crosshair grid */}
          <div style={{
            position: "absolute", inset: 12,
            backgroundImage: "linear-gradient(rgba(26,26,26,0.06) 1px, transparent 1px), linear-gradient(90deg, rgba(26,26,26,0.06) 1px, transparent 1px)",
            backgroundSize: "32px 32px", pointerEvents: "none",
          }}/>

          {/* multiblock 3x3 footprint (top-down) */}
          <div style={{
            position: "absolute", left: "50%", top: "50%",
            transform: "translate(-50%, -50%)",
            display: "grid", gridTemplateColumns: "repeat(3, 72px)",
            gridTemplateRows: "repeat(3, 72px)", gap: 0,
          }}>
            {[
              "I", "C", "I",
              "I", "X", "O",
              "M", "C", "O",
            ].map((c, i) => {
              const isCore = c === "X";
              return (
                <div key={i} style={{
                  border: "1.5px solid var(--ink)",
                  background: isCore ? "var(--accent)" : "var(--paper)",
                  margin: -0.75,
                  display: "flex", alignItems: "center", justifyContent: "center",
                  flexDirection: "column",
                  fontFamily: "var(--font-mono)", fontSize: 10,
                  fontWeight: isCore ? 700 : 400,
                  position: "relative",
                }}>
                  <span style={{ fontFamily: "var(--font-hand)", fontSize: isCore ? 28 : 16 }}>
                    {isCore ? "✦" : c === "C" ? "◇" : c === "M" ? "✚" : c === "I" ? "▸" : "◂"}
                  </span>
                  <span style={{ fontSize: 8, opacity: 0.6 }}>
                    {isCore ? "core" : c === "C" ? "casing" : c === "M" ? "mod" : c === "I" ? "in" : "out"}
                  </span>
                </div>
              );
            })}
          </div>

          {/* ruler / dimensions */}
          <Annot x="calc(50% - 110px)" y={"20%"}>↤ 3 blocks ↦</Annot>
          <Annot x="calc(50% + 130px)" y="35%">↥ 3 ↧</Annot>
          <Annot x="20" y="20" accent>BLUEPRINT · top-down view</Annot>
          <Annot x="20" y="calc(100% - 50px)">
            <span className="sk-mono-xs">⊟ side view →</span>
          </Annot>

          {/* port leader-lines (decorative) */}
          <svg style={{ position: "absolute", inset: 0, width: "100%", height: "100%", pointerEvents: "none" }}>
            <path d="M0,80 L40,80 L60,150" stroke="var(--ink)" strokeWidth="1" strokeDasharray="2 3" fill="none"/>
            <path d="M100%,160 L80%,160 L70%,200" stroke="var(--ink)" strokeWidth="1" strokeDasharray="2 3" fill="none"/>
          </svg>
        </div>

        {/* recipe + progress strip below schematic */}
        <Row gap={10} style={{ alignItems: "center" }}>
          <Col gap={1}>
            <span className="sk-mono-xs">CURRENT RECIPE</span>
            <Row gap={4} style={{ alignItems: "center" }}>
              <Slot filled style={{ width: 22, height: 22 }} icon="≈"/>
              <Slot filled style={{ width: 22, height: 22 }} icon="∽"/>
              <span className="sk-arrow" style={{ fontSize: 18 }}>⇒</span>
              <Slot filled style={{ width: 28, height: 28 }} icon="▩" qty={4}/>
              <span className="sk-mono-sm" style={{ fontWeight: 700, marginLeft: 6 }}>plastic.sheet ×4</span>
            </Row>
          </Col>
          <Col gap={1} style={{ flex: 1 }}>
            <Row style={{ justifyContent: "space-between" }}>
              <span className="sk-mono-xs">CYCLE 12 · UNDERFED</span>
              <span className="sk-mono-xs">stalled · waiting solder</span>
            </Row>
            <div className="sk-bar" style={{ height: 14 }}>
              <i style={{ width: "82%" }}/>
            </div>
          </Col>
        </Row>
      </Col>

      {/* RIGHT — outputs + modules + telemetry */}
      <Col gap={10} style={{ width: 240 }}>
        <Col gap={5}>
          <span className="sk-h-sm">◂ OUTPUTS · 4</span>
          <Port dir="out" label="plastic"   item="▩" qty={88}  rate="32/min"/>
          <Port dir="out" label="benzene"   item="≈" qty={400} rate="20 mB/t"/>
          <Port dir="out" label="slag"      item="·" qty={4}   rate="0.4/min"/>
          <Port dir="out" label="—"         item="?" qty={0}   rate="unbound"/>
        </Col>

        <Col gap={3}>
          <span className="sk-h-sm">MODULES · 4 / 6</span>
          <div style={{ display: "grid", gridTemplateColumns: "repeat(3, 1fr)", gap: 3 }}>
            {[
              { g: "✦", l: "ovr II"  },
              { g: "◈", l: "ins I"   },
              { g: "◆", l: "cat I"   },
              { g: "❅", l: "cool I"  },
              { g: "+", l: "—", dashed: true },
              { g: "+", l: "—", dashed: true },
            ].map((m, i) => (
              <div key={i} className={`sk-slot ${m.dashed ? "" : "sk-filled"}`}
                   style={{ width: "100%", height: 38, flexDirection: "column", padding: 2 }}>
                <span className="sk-icon" style={{ fontSize: 14 }}>{m.g}</span>
                <span style={{ fontSize: 6, fontFamily: "var(--font-mono)" }}>{m.l}</span>
              </div>
            ))}
          </div>
          {/* config detail for selected module */}
          <div className="sk-box sk-dashed" style={{ padding: 6, marginTop: 4 }}>
            <Row style={{ justifyContent: "space-between" }}>
              <span className="sk-mono-xs" style={{ fontWeight: 700 }}>OVR II · overclock</span>
              <span className="sk-mono-xs sk-squig">edit</span>
            </Row>
            <Row gap={3} style={{ marginTop: 3 }}>
              <span className="sk-mono-xs">×</span>
              {[1,2,4,8,16].map(n => (
                <span key={n} className={`sk-tag ${n===4?"sk-on":""}`}>{n}</span>
              ))}
            </Row>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>4× speed · 16× energy</span>
          </div>
        </Col>

        <Col gap={3}>
          <span className="sk-mono-xs">POWER · 7.6 / 12 MW</span>
          <div className="sk-bar"><i style={{ width: "63%" }}/></div>
          <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>buffer 4.2 MJ · stable</span>
        </Col>
      </Col>
    </Row>
  </div>
);

// =====================================================================
// VARIATION 5 — JOB CLAIM / AUTOCRAFT CONTRACT VIEW
// the "what is this machine willing to claim" viewpoint
// big priority dial + claim-from-network panel
// =====================================================================
const MachineV5 = () => {
  const claims = [
    { name: "circuit.basic ×4", from: "CPU-α", state: "running",  progress: 47, eta: "3.2s",  pri: 7 },
    { name: "circuit.adv ×1",   from: "CPU-α", state: "queued",   progress: 0,  eta: "32s",   pri: 5 },
    { name: "rivet.steel ×16",  from: "CPU-β", state: "queued",   progress: 0,  eta: "20s",   pri: 4 },
  ];
  const offered = [
    { name: "gear.bronze ×8",   from: "CPU-β",  pri: 3, lost: "lost to #M-09 (pri 4)" },
    { name: "spring.bronze ×4", from: "CPU-γ",  pri: 6, lost: "available · taking…" },
  ];
  return (
    <div className="paper" style={{ padding: 16, height: "100%", display: "flex", flexDirection: "column", gap: 10 }}>
      {/* HEADER */}
      <Row style={{ justifyContent: "space-between", alignItems: "flex-end" }}>
        <Col gap={1}>
          <Row gap={8} style={{ alignItems: "baseline" }}>
            <span className="sk-h">CRAFTER · #M-12</span>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>
              CONTRACT VIEW · what jobs will I claim
            </span>
          </Row>
          <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>
            registered with logistic network <b>net.0</b> · 3 active claims · 12 offers seen / min
          </span>
        </Col>
        <Row gap={6}>
          <Stamp kind="ok">CLAIMING</Stamp>
          <ModePill mode="autocraft"/>
        </Row>
      </Row>
      <hr className="sk-div"/>

      <Row gap={14} style={{ flex: 1 }}>
        {/* LEFT — priority dial + offered */}
        <Col gap={10} style={{ width: 320 }}>
          <Col gap={4}>
            <span className="sk-h-sm">PRIORITY · external</span>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>
              vs. other machines bidding on the same job
            </span>
            {/* big dial */}
            <div className="sk-box" style={{ padding: 12, position: "relative" }}>
              <Row gap={2} style={{ alignItems: "flex-end", justifyContent: "center" }}>
                {[1,2,3,4,5,6,7,8,9,10].map(n => (
                  <Col key={n} gap={2} style={{ alignItems: "center" }}>
                    <div style={{
                      width: 14, height: 8 + n * 4,
                      background: n <= 7 ? "var(--ink)" : "var(--paper-2)",
                      border: "1.5px solid var(--ink)",
                    }}/>
                    <span className="sk-mono-xs" style={{
                      fontSize: 8,
                      color: n === 7 ? "var(--ink)" : "var(--ink-faint)",
                      fontWeight: n === 7 ? 700 : 400,
                    }}>{n}</span>
                  </Col>
                ))}
              </Row>
              <Row style={{ marginTop: 8, justifyContent: "space-between" }}>
                <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>low · backup</span>
                <Stamp kind="warn">7 · normal+</Stamp>
                <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>urgent · 10</span>
              </Row>
            </div>
          </Col>

          <Col gap={4}>
            <span className="sk-h-sm">OFFERS · seen on network</span>
            {offered.map((o, i) => (
              <div key={i} className="sk-box sk-dashed" style={{ padding: 6 }}>
                <Row style={{ justifyContent: "space-between" }}>
                  <span className="sk-mono-sm" style={{ fontWeight: 700 }}>{o.name}</span>
                  <span className="sk-tag">pri {o.pri}</span>
                </Row>
                <Row style={{ justifyContent: "space-between", marginTop: 2 }}>
                  <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>from {o.from}</span>
                  <span className="sk-mono-xs"
                        style={{ color: o.lost.startsWith("lost") ? "#a31919" : "var(--ink)" }}>
                    {o.lost}
                  </span>
                </Row>
              </div>
            ))}
          </Col>

          <Col gap={3} style={{ marginTop: "auto" }}>
            <span className="sk-mono-xs">CLAIM POLICY</span>
            <Row gap={3} style={{ flexWrap: "wrap" }}>
              <span className="sk-tag sk-on">match recipes</span>
              <span className="sk-tag sk-on">skip if busy</span>
              <span className="sk-tag">prefer near</span>
              <span className="sk-tag">refuse if power &lt; 80%</span>
            </Row>
          </Col>
        </Col>

        {/* CENTER — recipe priority list (internal ordering) */}
        <Col gap={4} style={{ flex: 1 }}>
          <Row style={{ justifyContent: "space-between", alignItems: "baseline" }}>
            <span className="sk-h-sm">RECIPE PRIORITY · internal</span>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>drag to reorder · top wins ties</span>
          </Row>

          <div className="sk-box" style={{ padding: 8, flex: 1 }}>
            {[
              { n: "circuit.basic",  out: "×4",  pri: 8, mode: "auto",    note: "running now" },
              { n: "circuit.adv",    out: "×1",  pri: 5, mode: "auto",    note: "1 of 4 in queue" },
              { n: "rivet.steel",    out: "×16", pri: 4, mode: "auto",    note: "queued" },
              { n: "wire.copper",    out: "×8",  pri: 3, mode: "passive", note: "stop @ 256" },
              { n: "gear.bronze",    out: "×2",  pri: 2, mode: "passive", note: "stop @ 64" },
              { n: "plate.tin",      out: "×4",  pri: 1, mode: "off",     note: "—" },
            ].map((r, i) => (
              <Row key={i} gap={6} style={{
                padding: "5px 4px",
                borderBottom: "1px dashed var(--ink-faint)",
                opacity: r.mode === "off" ? 0.5 : 1,
              }}>
                <span className="sk-mono-xs" style={{ width: 14, color: "var(--ink-faint)" }}>⋮⋮</span>
                <span className="sk-mono-sm" style={{ width: 22, fontWeight: 700, textAlign: "right" }}>{i+1}</span>
                <Slot filled style={{ width: 22, height: 22 }} icon={["▦","▧","◉","▨","◈","▢"][i]}/>
                <span className="sk-mono-sm" style={{ flex: 1, fontWeight: 700 }}>{r.n}</span>
                <span className="sk-mono-xs" style={{ width: 30, textAlign: "right" }}>{r.out}</span>
                <span className={`sk-tag ${r.mode==="auto"?"sk-on":r.mode==="passive"?"sk-accent":""}`}>{r.mode}</span>
                <Row gap={1}>
                  {[1,2,3,4,5,6,7,8].map(p => (
                    <span key={p} style={{
                      width: 4, height: 10,
                      background: p <= r.pri ? "var(--ink)" : "transparent",
                      border: "0.5px solid var(--ink)"
                    }}/>
                  ))}
                </Row>
                <span className="sk-mono-xs" style={{ width: 130, color: "var(--ink-faint)" }}>{r.note}</span>
              </Row>
            ))}
          </div>

          <Row gap={6}>
            <button className="sk-btn">+ recipe</button>
            <button className="sk-btn">bulk priority…</button>
            <span className="sk-mono-xs" style={{ marginLeft: "auto", color: "var(--ink-faint)" }}>
              tie-break order: priority ↓ · arrival time ↑
            </span>
          </Row>
        </Col>

        {/* RIGHT — active claims */}
        <Col gap={4} style={{ width: 280 }}>
          <Row style={{ justifyContent: "space-between", alignItems: "baseline" }}>
            <span className="sk-h-sm">ACTIVE CLAIMS · 3</span>
            <span className="sk-mono-xs sk-squig">history →</span>
          </Row>

          {claims.map((c, i) => (
            <div key={i} className={`sk-box ${c.state === "running" ? "sk-thick" : ""}`}
                 style={{ padding: 8 }}>
              <Row style={{ justifyContent: "space-between" }}>
                <span className="sk-mono-sm" style={{ fontWeight: 700 }}>{c.name}</span>
                <Stamp kind={c.state === "running" ? "ok" : "idle"}>{c.state}</Stamp>
              </Row>
              <Row gap={4} style={{ marginTop: 4 }}>
                <span className="sk-tag">{c.from}</span>
                <span className="sk-tag">pri {c.pri}</span>
                <span className="sk-mono-xs" style={{ marginLeft: "auto", color: "var(--ink-faint)" }}>
                  eta {c.eta}
                </span>
              </Row>
              {c.state === "running" && (
                <div className="sk-bar" style={{ marginTop: 6 }}>
                  <i style={{ width: `${c.progress}%` }}/>
                </div>
              )}
              <Row gap={4} style={{ marginTop: 4 }}>
                <span className="sk-mono-xs sk-squig">cancel</span>
                <span className="sk-mono-xs sk-squig" style={{ marginLeft: "auto" }}>jump to CPU</span>
              </Row>
            </div>
          ))}

          <div className="sk-box sk-dashed" style={{ padding: 6, marginTop: "auto" }}>
            <span className="sk-mono-xs">⓪ this machine has refused 4 jobs in last 5m</span>
            <br/>
            <span className="sk-mono-xs sk-squig">why? →</span>
          </div>
        </Col>
      </Row>
    </div>
  );
};

window.MachineV1 = MachineV1;
window.MachineV2 = MachineV2;
window.MachineV3 = MachineV3;
window.MachineV4 = MachineV4;
window.MachineV5 = MachineV5;

// =====================================================================
// PORT CONFIGURATION OPTIONS — fit in the V3 left rail (≈232px wide)
// All three swap into the place where the simple "PORTS 5/8 · 2/4" block is.
//
// Model: ports are filtered logistic-net endpoints. Not voxel faces.
// A port = (direction, network, item-filter, rate). A machine can bind
// to several nets to source different items or route outputs.
//   { dir, net, items:[...], rate, alarm }
// =====================================================================
const NETS = [
  { id: "ore",  label: "ore.net",   tone: "#7a5cff" },
  { id: "fab",  label: "fab.net",   tone: "#188a4a" },
  { id: "mat",  label: "mat.net",   tone: "#b88a00" },
  { id: "byp",  label: "byp.net",   tone: "#a31919" },
];

// Legacy directional model — kept for B & C below.
const PORTS_DATA = [
  { dir: "in",  net: "ore",  items: ["◆"],         label: "iron",          rate: 48, unit: "/min", alarm: false },
  { dir: "in",  net: "ore",  items: ["◉"],         label: "coal",          rate: 16, unit: "/min", alarm: true  },
  { dir: "in",  net: "mat",  items: ["·","◆"],     label: "solder + flux", rate: 4,  unit: "/min", alarm: false },
  { dir: "out", net: "fab",  items: ["▦","▧"],     label: "circuits",      rate: 22, unit: "/min", alarm: false },
  { dir: "out", net: "byp",  items: ["·"],         label: "slag",          rate: 2,  unit: "/min", alarm: false },
];

// Bidirectional port model — one port = one logistic-net binding.
// Each item in the filter has a policy: in / out / both / none.
// `defaultPolicy` applies to items not explicitly listed.
// `overrides` are explicit per-item rules.
//   policies: in (▸) / out (◂) / both (⇆) / none (⊘)
const PORTS_BIDIR = [
  {
    net: "ore",
    name: "ore feed",
    defaultPolicy: "none",
    overrides: [
      { item: "◆", label: "iron",   policy: "in",   alarm: false },
      { item: "◉", label: "coal",   policy: "in",   alarm: true  },
      { item: "▤", label: "tin",    policy: "in",   alarm: false },
    ],
  },
  {
    net: "mat",
    name: "mat bus",
    defaultPolicy: "in",
    overrides: [
      { item: "·", label: "solder", policy: "in",   alarm: false },
      { item: "◆", label: "flux",   policy: "both", alarm: false },
      { item: "▦", label: "circ.b", policy: "none", alarm: false },
    ],
  },
  {
    net: "fab",
    name: "fab return",
    defaultPolicy: "out",
    overrides: [
      { item: "▦", label: "circ.b", policy: "out",  alarm: false },
      { item: "▧", label: "circ.a", policy: "out",  alarm: false },
    ],
  },
  {
    net: "byp",
    name: "slag drop",
    defaultPolicy: "none",
    overrides: [
      { item: "·", label: "slag",   policy: "out",  alarm: false },
    ],
  },
];

// Policy glyph + color used everywhere in port A
const POLICIES = {
  in:   { glyph: "▸",  label: "in",   tone: "#188a4a", desc: "machine pulls from net" },
  out:  { glyph: "◂",  label: "out",  tone: "#7a5cff", desc: "machine pushes to net"  },
  both: { glyph: "⇆", label: "both", tone: "#1a1a1a", desc: "bidirectional"           },
  none: { glyph: "⊘", label: "none", tone: "#999",    desc: "blocked"                 },
};

const PolicyChip = ({ policy, size = 10 }) => {
  const p = POLICIES[policy];
  return (
    <span title={p.desc} style={{
      display: "inline-flex", alignItems: "center", justifyContent: "center",
      width: size + 6, height: size + 6,
      border: "1.5px solid var(--ink)",
      background: policy === "none" ? "var(--paper-2)" : p.tone,
      color: policy === "none" ? "var(--ink-faint)" : "var(--paper)",
      fontFamily: "var(--font-mono)", fontSize: size, fontWeight: 700,
      lineHeight: 1, cursor: "pointer",
    }}>{p.glyph}</span>
  );
};

const NetTag = ({ net, size = 10 }) => {
  const n = NETS.find(x => x.id === net) || { label: net, tone: "var(--ink)" };
  return (
    <span style={{
      display: "inline-flex", alignItems: "center", gap: 3,
      border: `1.5px solid var(--ink)`, padding: "0 4px",
      fontFamily: "var(--font-mono)", fontSize: size, fontWeight: 700,
      letterSpacing: "0.4px", textTransform: "lowercase",
      background: "var(--paper)",
    }}>
      <span style={{
        width: 6, height: 6, background: n.tone, borderRadius: "50%",
        border: "1px solid var(--ink)",
      }}/>
      {n.label}
    </span>
  );
};

// ── A · BIDIRECTIONAL NET PORTS ─────────────────────────────────────
// Each port = one logistic-net binding. No direction. Items in the
// filter carry a per-item policy (in / out / both / none); a default
// policy governs unlisted items. Click an item to cycle its policy,
// click × to remove the override (revert to default).
const PortConfigA = () => {
  return (
    <Col gap={6}>
      <Row style={{ justifyContent: "space-between", alignItems: "baseline" }}>
        <span className="sk-mono-xs">PORTS · {PORTS_BIDIR.length} nets bound</span>
      </Row>

      {PORTS_BIDIR.map((port) => {
        const tone = (NETS.find(n => n.id === port.net) || {}).tone || "var(--ink)";
        const def  = POLICIES[port.defaultPolicy];
        return (
          <div key={port.net} className="sk-box" style={{
            padding: 0, borderColor: "var(--ink)", borderWidth: 1.5,
            background: "var(--paper)",
          }}>
            {/* PORT HEADER — net + name + default policy */}
            <Row gap={5} style={{
              alignItems: "center",
              padding: "5px 6px",
              borderBottom: "1.5px solid var(--ink)",
              background: "var(--paper-2)",
              position: "relative",
            }}>
              {/* colored bar at left = net tone */}
              <div style={{
                position: "absolute", left: 0, top: 0, bottom: 0,
                width: 3, background: tone,
              }}/>
              <NetTag net={port.net} size={9}/>
              <span className="sk-mono-xs" style={{ color: "var(--ink-faint)", fontStyle: "italic" }}>
                {port.name}
              </span>
              <span className="sk-mono-xs sk-squig" style={{ marginLeft: "auto", cursor: "pointer" }}>⋯</span>
            </Row>

            {/* DEFAULT POLICY — applies to unlisted items */}
            <Row gap={5} style={{
              alignItems: "center",
              padding: "4px 6px",
              borderBottom: "1px dashed var(--ink-faint)",
              background: "rgba(245,197,24,0.08)",
            }}>
              <span className="sk-mono-xs" style={{ color: "var(--ink-soft)", flex: 1 }}>
                default <span style={{ color: "var(--ink-faint)" }}>(any other item)</span>
              </span>
              <Row gap={2}>
                {["in","out","both","none"].map(k => (
                  <span key={k} title={POLICIES[k].desc} style={{
                    display: "inline-flex", alignItems: "center", justifyContent: "center",
                    width: 16, height: 16,
                    border: "1.5px solid var(--ink)",
                    background: port.defaultPolicy === k
                      ? (k === "none" ? "var(--paper-2)" : POLICIES[k].tone)
                      : "var(--paper)",
                    color: port.defaultPolicy === k && k !== "none"
                      ? "var(--paper)"
                      : "var(--ink)",
                    fontFamily: "var(--font-mono)", fontSize: 9, fontWeight: 700,
                    lineHeight: 1, cursor: "pointer",
                    opacity: port.defaultPolicy === k ? 1 : 0.4,
                  }}>{POLICIES[k].glyph}</span>
                ))}
              </Row>
            </Row>

            {/* PER-ITEM OVERRIDES */}
            <Col gap={0}>
              {port.overrides.map((o, i) => (
                <Row key={i} gap={5} style={{
                  alignItems: "center",
                  padding: "3px 6px",
                  borderBottom: i === port.overrides.length - 1 ? "none" : "1px dashed var(--ink-faint)",
                  background: o.alarm ? "rgba(245,197,24,0.18)" : "transparent",
                }}>
                  <PolicyChip policy={o.policy} size={9}/>
                  <Slot filled style={{ width: 18, height: 18 }} icon={o.item}/>
                  <span className="sk-mono-sm" style={{ fontWeight: 700, fontSize: 10, flex: 1, minWidth: 0 }}>
                    {o.label}
                  </span>
                  {o.alarm && (
                    <span className="sk-mono-xs" style={{
                      fontSize: 9, color: "#b88a00", fontWeight: 700,
                    }}>low!</span>
                  )}
                  <span title="remove override" style={{
                    fontFamily: "var(--font-mono)", fontSize: 11,
                    color: "var(--ink-faint)", cursor: "pointer",
                    width: 12, textAlign: "center",
                  }}>×</span>
                </Row>
              ))}
              {/* + add item — uses default policy */}
              <Row gap={5} style={{
                alignItems: "center",
                padding: "3px 6px",
                borderTop: port.overrides.length ? "1px dashed var(--ink-faint)" : "none",
                background: "var(--paper)",
                cursor: "pointer",
              }}>
                <span style={{
                  width: 15, height: 15,
                  border: "1.5px dashed var(--ink-faint)",
                  display: "inline-flex", alignItems: "center", justifyContent: "center",
                  fontFamily: "var(--font-mono)", fontSize: 10, fontWeight: 700,
                  color: "var(--ink-faint)",
                }}>+</span>
                <span className="sk-mono-xs" style={{ flex: 1, color: "var(--ink-faint)", fontSize: 9 }}>
                  add item
                </span>
                <span className="sk-mono-xs" style={{ color: "var(--ink-faint)", fontSize: 8 }}>
                  starts as <span style={{ color: "var(--ink)", fontWeight: 700 }}>{def.glyph} {def.label}</span>
                </span>
              </Row>
            </Col>
          </div>
        );
      })}

      {/* LEGEND — what the four glyphs mean */}
      <Row gap={4} style={{ flexWrap: "wrap", marginTop: 2 }}>
        {["in","out","both","none"].map(k => (
          <Row key={k} gap={2} style={{ alignItems: "center" }}>
            <PolicyChip policy={k} size={8}/>
            <span className="sk-mono-xs" style={{ fontSize: 8, color: "var(--ink-faint)" }}>
              {POLICIES[k].label}
            </span>
          </Row>
        ))}
      </Row>
    </Col>
  );
};

// ── B · FILTER SLOT GRID ────────────────────────────────────────────
// Fixed slot grid (8 ports). Each port = a card with dir/net/filter/rate.
// Empty ports are dashed placeholders, click to configure.
const PortConfigB = () => {
  const slots = [...PORTS_DATA, null, null, null];
  return (
    <Col gap={4}>
      <Row style={{ justifyContent: "space-between", alignItems: "baseline" }}>
        <span className="sk-mono-xs">PORTS · {PORTS_DATA.length} / 8</span>
        <Row gap={3}>
          <span className="sk-tag sk-on" style={{ fontSize: 7, padding: "0 3px" }}>all</span>
          <span className="sk-tag" style={{ fontSize: 7, padding: "0 3px" }}>in</span>
          <span className="sk-tag" style={{ fontSize: 7, padding: "0 3px" }}>out</span>
        </Row>
      </Row>
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 3 }}>
        {slots.map((p, i) => {
          if (!p) return (
            <div key={i} className="sk-slot" style={{
              width: "100%", height: 56, flexDirection: "column",
              padding: 2, color: "var(--ink-faint)",
            }}>
              <span style={{ fontFamily: "var(--font-hand)", fontSize: 14 }}>+</span>
              <span style={{ fontSize: 7 }}>port {i+1}</span>
            </div>
          );
          const tone = (NETS.find(n=>n.id===p.net) || {}).tone || "var(--ink)";
          return (
            <div key={i} style={{
              border: `1.5px ${p.dir === "in" ? "solid" : "dashed"} var(--ink)`,
              background: p.alarm ? "rgba(245,197,24,0.22)" : "var(--paper)",
              padding: "3px 4px",
              minHeight: 56,
              display: "flex", flexDirection: "column", gap: 2,
              position: "relative",
            }}>
              {/* dir corner tag */}
              <span style={{
                position: "absolute", top: -1, left: -1,
                background: p.dir === "in" ? "var(--ink)" : "var(--paper)",
                color: p.dir === "in" ? "var(--paper)" : "var(--ink)",
                border: "1.5px solid var(--ink)",
                fontFamily: "var(--font-mono)", fontSize: 7, fontWeight: 700,
                padding: "0 3px", letterSpacing: "0.4px",
              }}>{p.dir === "in" ? "IN" : "OUT"}</span>
              <Row gap={3} style={{ marginTop: 8, alignItems: "center" }}>
                <Row gap={1}>
                  {p.items.map((it, j) => (
                    <Slot key={j} filled style={{ width: 16, height: 16 }} icon={it}/>
                  ))}
                </Row>
                <span className="sk-mono-sm" style={{ fontSize: 10, fontWeight: 700, flex: 1, minWidth: 0, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                  {p.label}
                </span>
              </Row>
              <Row style={{ justifyContent: "space-between", alignItems: "center" }}>
                <span style={{
                  display: "inline-flex", alignItems: "center", gap: 2,
                  fontFamily: "var(--font-mono)", fontSize: 8, fontWeight: 700,
                }}>
                  <span style={{
                    width: 5, height: 5, background: tone, borderRadius: "50%",
                    border: "0.5px solid var(--ink)",
                  }}/>
                  {p.net}
                </span>
                <span style={{
                  fontFamily: "var(--font-mono)", fontSize: 8,
                  color: p.alarm ? "#b88a00" : "var(--ink-faint)",
                  fontWeight: p.alarm ? 700 : 400,
                }}>
                  {p.alarm ? "low!" : `${p.rate}${p.unit}`}
                </span>
              </Row>
            </div>
          );
        })}
      </div>
      <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>
        click slot to bind net + filter
      </span>
    </Col>
  );
};

// ── C · FILTER TABLE (dir / net / filter / rate) ────────────────────
// Densest. Sortable / scannable. Filter column shows item glyph stack.
const PortConfigC = () => (
  <Col gap={4}>
    <Row style={{ justifyContent: "space-between", alignItems: "baseline" }}>
      <span className="sk-mono-xs">PORTS · filter table</span>
      <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>5 of 8</span>
    </Row>
    <div className="sk-box" style={{ padding: 0 }}>
      <table style={{ width: "100%", borderCollapse: "collapse", fontFamily: "var(--font-mono)", fontSize: 10 }}>
        <thead>
          <tr style={{ borderBottom: "1.5px solid var(--ink)", background: "var(--paper-2)" }}>
            <th style={{ padding: "3px 4px", textAlign: "center", width: 22, fontFamily: "var(--font-label)", fontWeight: 400 }}>↕</th>
            <th style={{ padding: "3px 4px", textAlign: "left",  fontFamily: "var(--font-label)", fontWeight: 400 }}>net</th>
            <th style={{ padding: "3px 4px", textAlign: "left",  fontFamily: "var(--font-label)", fontWeight: 400 }}>filter</th>
            <th style={{ padding: "3px 4px", textAlign: "right", fontFamily: "var(--font-label)", fontWeight: 400 }}>rate</th>
          </tr>
        </thead>
        <tbody>
          {PORTS_DATA.map((p, i) => {
            const tone = (NETS.find(n => n.id === p.net) || {}).tone || "var(--ink)";
            return (
              <tr key={i} style={{
                borderBottom: "1px dashed var(--ink-faint)",
                background: p.alarm ? "rgba(245,197,24,0.18)" : "transparent",
              }}>
                <td style={{ padding: "3px 4px", textAlign: "center" }}>
                  <span style={{
                    display: "inline-block", padding: "0 3px",
                    border: "1px solid var(--ink)",
                    background: p.dir === "in" ? "var(--paper)" : "var(--ink)",
                    color: p.dir === "in" ? "var(--ink)" : "var(--paper)",
                    fontSize: 8, fontWeight: 700,
                  }}>{p.dir === "in" ? "IN" : "OUT"}</span>
                </td>
                <td style={{ padding: "3px 4px" }}>
                  <Row gap={3} style={{ alignItems: "center" }}>
                    <span style={{
                      width: 6, height: 6, background: tone, borderRadius: "50%",
                      border: "0.5px solid var(--ink)",
                    }}/>
                    <span style={{ fontWeight: 700 }}>{p.net}</span>
                  </Row>
                </td>
                <td style={{ padding: "3px 4px" }}>
                  <Row gap={2} style={{ alignItems: "center" }}>
                    {p.items.map((it, j) => (
                      <Slot key={j} filled style={{ width: 14, height: 14 }} icon={it}/>
                    ))}
                    <span style={{ fontSize: 9, color: "var(--ink-faint)" }}>{p.label}</span>
                  </Row>
                </td>
                <td style={{
                  padding: "3px 4px", textAlign: "right",
                  color: p.alarm ? "#b88a00" : "var(--ink-faint)",
                  fontWeight: p.alarm ? 700 : 400,
                }}>{p.alarm ? "low!" : `${p.rate}${p.unit}`}</td>
              </tr>
            );
          })}
          {/* empty rows */}
          {[0,1,2].map(k => (
            <tr key={"e"+k} style={{ borderBottom: k === 2 ? "none" : "1px dashed var(--ink-faint)", opacity: 0.5 }}>
              <td colSpan={4} style={{ padding: "3px 4px", textAlign: "center", color: "var(--ink-faint)", fontStyle: "italic" }}>
                + bind port
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
    <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>
      port = (dir, net, filter) · sort by net to group
    </span>
  </Col>
);

window.PortConfigA = PortConfigA;
window.PortConfigB = PortConfigB;
window.PortConfigC = PortConfigC;
