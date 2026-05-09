/* global React, Slot, Row, Col */

// =====================================================================
// MACHINE UI — shared atoms (V3 + PortConfigA only)
// =====================================================================
const MachIcon = ({ glyph = "▦", size = 18, dashed }) => (
  <div className={`sk-slot ${dashed ? "" : "sk-filled"}`}
       style={{ width: size, height: size, fontSize: size * 0.55 }}>
    <span className="sk-icon" style={{ fontSize: size * 0.6 }}>{glyph}</span>
  </div>
);

// Progress / fill bar (vertical)
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

// =====================================================================
// V3 — SIDE-RAIL TERMINAL (dense / AE2-ish)
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
  const active = rows.find(r => r.claim?.state === "running");

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

        {/* CURRENT CRAFT */}
        <Col gap={4}>
          <Row style={{ justifyContent: "space-between", alignItems: "baseline" }}>
            <span className="sk-mono-xs">CURRENT CRAFT</span>
            <Stamp kind="ok">RUNNING</Stamp>
          </Row>
          <div className="sk-box sk-thick" style={{ padding: 8, position: "relative", overflow: "hidden" }}>
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

        {/* BULK SELECT TOOLBAR */}
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
                <tr key={i} style={{
                  borderBottom: "1px dashed var(--ink-faint)",
                  background: isActive ? "rgba(245,197,24,0.18)" : "transparent",
                  opacity: dim ? 0.5 : 1,
                }}>
                  <td style={{ padding: "4px 8px", width: 18 }}>
                    <span style={{
                      display: "inline-block", width: 12, height: 12,
                      border: "1.5px solid var(--ink)",
                      background: [0,2,4].includes(i) ? "var(--ink)" : "var(--paper)",
                    }}/>
                  </td>
                  <td style={{ padding: "4px 8px", fontWeight: 700 }}>{r.name}</td>
                  <td style={{ padding: "4px 8px" }}>
                    <Row gap={3} style={{ alignItems: "center" }}>
                      {r.inputs.map((inp, k) => (
                        <Slot key={k} filled style={{ width: 22, height: 22 }} icon={inp[0]} qty={inp[1]}/>
                      ))}
                      <span className="sk-arrow" style={{ fontSize: 14, marginLeft: 2, marginRight: 2 }}>⇒</span>
                      <Slot filled style={{ width: 24, height: 24 }} icon={r.out[0]} qty={r.out[1]}/>
                    </Row>
                  </td>
                  <td style={{ padding: "4px 8px", textAlign: "center" }}>
                    <span className="sk-mono-sm" style={{
                      fontFamily: "var(--font-mono)", fontSize: 11,
                      fontWeight: 700, color: "var(--ink-soft)",
                    }}>{r.rate}</span>
                  </td>
                  <td style={{ padding: "4px 8px" }}>
                    <ModeFlags P={r.P} C={r.C} limit={r.limit}/>
                  </td>
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
                  <td style={{ padding: "4px 8px", textAlign: "right", color: "var(--ink-faint)", whiteSpace: "nowrap" }}>
                    {r.claim?.state === "running" && <span className="sk-mono-xs" style={{ color: "var(--ink)", fontWeight: 700 }}>▶ {r.claim.pct}%</span>}
                    {r.claim?.state === "queued"  && <span className="sk-mono-xs">queued · {r.claim.from}</span>}
                    {!r.claim && <span className="sk-mono-xs">⋯</span>}
                  </td>
                </tr>
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

window.MachineV3 = MachineV3;

// =====================================================================
// PORT CONFIGURATION — bidirectional logistic-net ports (PortConfigA)
// Each port = one logistic-net binding. Items in the filter carry a
// per-item policy (in / out / both / none). Default policy governs
// unlisted items.
// =====================================================================
const NETS = [
  { id: "ore",  label: "ore.net",   tone: "#7a5cff" },
  { id: "fab",  label: "fab.net",   tone: "#188a4a" },
  { id: "mat",  label: "mat.net",   tone: "#b88a00" },
  { id: "byp",  label: "byp.net",   tone: "#a31919" },
];

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

            {/* DEFAULT POLICY */}
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

      {/* LEGEND */}
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

window.PortConfigA = PortConfigA;
