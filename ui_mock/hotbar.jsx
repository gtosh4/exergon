/* global React */
const { useState } = React;

// Reusable atoms
const Slot = ({ filled, active, label, qty, icon, style }) => (
  <div className={`sk-slot ${filled ? "sk-filled" : ""} ${active ? "sk-active" : ""}`} style={style}>
    {icon && <span className="sk-icon">{icon}</span>}
    {!icon && filled && <span className="sk-mono-xs" style={{opacity:0.5}}>{label||""}</span>}
    {qty && <span className="sk-qty">{qty}</span>}
  </div>
);

const Row = ({ children, gap = 4, style }) => (
  <div style={{ display: "flex", gap, alignItems: "center", ...style }}>{children}</div>
);

const Col = ({ children, gap = 4, style }) => (
  <div style={{ display: "flex", flexDirection: "column", gap, ...style }}>{children}</div>
);

// =====================================================================
// HOTBAR VARIATION 1 — Classic 9-slot, centered (Minecraft baseline)
// =====================================================================
const HotbarV1 = () => (
  <div className="paper" style={{ padding: 24, height: "100%", position: "relative", display: "flex", flexDirection: "column", justifyContent: "flex-end" }}>
    <div className="sk-annot" style={{ top: 16, left: 24 }}>centered, world view behind ↓</div>
    <div className="sk-img" style={{ position: "absolute", inset: 50, opacity: 0.35 }}>
      <span>first-person world</span>
    </div>
    <div style={{ position: "relative", display: "flex", justifyContent: "center", marginBottom: 8 }}>
      <Row gap={2}>
        {[0,1,2,3,4,5,6,7,8].map(i => (
          <Slot key={i}
            filled={[0,1,3,5,8].includes(i)}
            active={i===2}
            icon={["⛏","◇","","✦","","◉","","","▣"][i]}
            qty={[16,1,null,null,32,null,null,null,4][i]}
          />
        ))}
      </Row>
    </div>
    <Row gap={6} style={{ justifyContent: "center" }}>
      {[1,2,3,4,5,6,7,8,9].map(n => (
        <span key={n} className="sk-mono-xs" style={{ width: 38, textAlign: "center", color: "var(--ink-faint)" }}>{n}</span>
      ))}
    </Row>
    <div className="sk-annot" style={{ bottom: 16, right: 24 }}>1-9 keys</div>
  </div>
);

// =====================================================================
// HOTBAR VARIATION 2 — Radial / wheel (hold-key reveals)
// =====================================================================
const HotbarV2 = () => {
  const radius = 90;
  const slots = 8;
  return (
    <div className="paper" style={{ padding: 24, height: "100%", position: "relative", display: "flex", alignItems: "center", justifyContent: "center" }}>
      <div className="sk-annot" style={{ top: 16, left: 24 }}>hold TAB → wheel</div>
      <div style={{ position: "relative", width: 240, height: 240 }}>
        {/* outer ring */}
        <div style={{ position: "absolute", inset: 0, border: "1.5px dashed var(--ink)", borderRadius: "50%" }} />
        <div style={{ position: "absolute", inset: 60, border: "1.5px solid var(--ink)", borderRadius: "50%", background: "var(--paper)" }}>
          <div style={{ display: "flex", alignItems: "center", justifyContent: "center", height: "100%", flexDirection: "column", gap: 2 }}>
            <span className="sk-h-xs">SELECTED</span>
            <span className="sk-mono">pickaxe.iron</span>
            <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>×1 · dura 234/250</span>
          </div>
        </div>
        {Array.from({length: slots}).map((_, i) => {
          const angle = (i / slots) * Math.PI * 2 - Math.PI / 2;
          const x = 120 + Math.cos(angle) * radius - 19;
          const y = 120 + Math.sin(angle) * radius - 19;
          return (
            <div key={i} style={{ position: "absolute", left: x, top: y }}>
              <Slot filled={i!==4} active={i===0} icon={["⛏","◇","✦","◉","","▣","◈","✕"][i]} qty={[1,1,32,null,null,4,1,null][i]} />
            </div>
          );
        })}
      </div>
      <div className="sk-annot" style={{ bottom: 16, right: 24, textAlign: "right" }}>scroll/mouse<br/>to pick</div>
    </div>
  );
};

// =====================================================================
// HOTBAR VARIATION 3 — Multi-bank (3 layers, swap with shift+wheel)
// =====================================================================
const HotbarV3 = () => (
  <div className="paper" style={{ padding: 24, height: "100%", position: "relative", display: "flex", flexDirection: "column", justifyContent: "flex-end", gap: 8 }}>
    <div className="sk-annot" style={{ top: 16, left: 24 }}>3 banks · shift+wheel swaps</div>
    {[
      { tag: "A · TOOLS", active: true },
      { tag: "B · BUILD", active: false },
      { tag: "C · COMBAT", active: false },
    ].map((bank, bi) => (
      <Row key={bi} gap={8} style={{ opacity: bank.active ? 1 : 0.45, justifyContent: "center" }}>
        <Col gap={2} style={{ alignItems: "flex-end", width: 80 }}>
          <span className={`sk-tag ${bank.active ? "sk-on" : ""}`}>{bank.tag}</span>
          {bank.active && <span className="sk-mono-xs" style={{color:"var(--ink-faint)"}}>active</span>}
        </Col>
        <Row gap={2}>
          {Array.from({length:9}).map((_,i) => (
            <Slot key={i}
              filled={Math.random() > 0.3 || (bi===0 && [0,1,3,5,8].includes(i))}
              active={bank.active && i === 2}
              icon={bi===0 ? ["⛏","◇","◆","✦","","◉","","","▣"][i] : bi===1 ? ["▦","▧","","▨","▩","","","▤","▥"][i] : ["✕","◈","","✦","","","◉","",""][i]}
            />
          ))}
        </Row>
      </Row>
    ))}
    <div className="sk-annot" style={{ bottom: 16, right: 24 }}>27 slots reachable</div>
  </div>
);

// =====================================================================
// HOTBAR VARIATION 4 — Contextual (changes by tool: build, mine, fight)
// =====================================================================
const HotbarV4 = () => (
  <div className="paper" style={{ padding: 24, height: "100%", position: "relative", display: "flex", flexDirection: "column", justifyContent: "flex-end" }}>
    <div className="sk-annot" style={{ top: 16, left: 24 }}>contextual · adapts to held tool</div>
    {/* mode strip */}
    <Row gap={6} style={{ justifyContent: "center", marginBottom: 10 }}>
      <span className="sk-tag sk-on">⛏ MINE</span>
      <span className="sk-tag">▦ BUILD</span>
      <span className="sk-tag">✕ FIGHT</span>
      <span className="sk-tag">◇ SCAN</span>
    </Row>
    {/* primary 6 slots, larger */}
    <Row gap={4} style={{ justifyContent: "center", marginBottom: 6 }}>
      {[0,1,2,3,4,5].map(i => (
        <Slot key={i} style={{ width: 48, height: 48 }}
          filled={i!==4}
          active={i===1}
          icon={["⛏","◈","◆","✦","","▣"][i]}
          qty={[1,1,null,32,null,4][i]}
        />
      ))}
    </Row>
    {/* contextual quick-access (ammo / blocks / mods) */}
    <Row gap={4} style={{ justifyContent: "center" }}>
      <span className="sk-mono-xs" style={{width:60, textAlign:"right", color:"var(--ink-faint)"}}>QUICK →</span>
      {[0,1,2,3,4,5,6,7].map(i => (
        <Slot key={i} style={{ width: 24, height: 24 }} filled={i<5} icon="·" />
      ))}
    </Row>
    <div className="sk-annot" style={{ bottom: 16, right: 24, textAlign:"right" }}>quick row =<br/>tool-specific</div>
  </div>
);

window.HotbarV1 = HotbarV1;
window.HotbarV2 = HotbarV2;
window.HotbarV3 = HotbarV3;
window.HotbarV4 = HotbarV4;
window.Slot = Slot;
window.Row = Row;
window.Col = Col;
