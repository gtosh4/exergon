/* global React, Slot, Row, Col */

// ============================================================
// CRAFT MODAL — qty input → resolved execution plan → enqueue
// ============================================================
// Shown as a dialog overlay. Two phases:
//   1. Input  — item display + qty field + RESOLVE PLAN button
//   2. Plan   — read-only dependency tree + machine plan + ENQUEUE / BACK
//
// No decisions in either phase. Plan is fully resolved from current
// network state + machine priorities. Footer always notes this.
//
// See: docs/ui.md § Terminal > CRAFT button flow

const { useState: useCMState } = React;

// ─── mock data ────────────────────────────────────────────────
const CRAFT_ITEM = { id: "reactor.core", icon: "◉", tag: "COMP", kg: 4.0, have: 0 };

// Dependency tree rows.
// status: "ok" (stocked) | "craft" (will be made) | "missing" (not available, no recipe active)
// machine + cpu: set when status === "craft"
// dedupe: true when this item appears again above in the tree (shared dependency)
const buildPlan = (qty) => [
  { d: 0, id: "reactor.core",  icon: "◉", q: 1  * qty, status: "craft",   machine: "CRAFTER LV4 #M-02", cpu: "α" },
  { d: 1, id: "frame.steel",   icon: "▦", q: 4  * qty, status: "craft",   machine: "SMITH LV3 #M-07",   cpu: "α" },
  { d: 2, id: "steel.plate",   icon: "▤", q: 16 * qty, status: "ok",      have: 4096 },
  { d: 2, id: "bolt.steel",    icon: "◇", q: 32 * qty, status: "craft",   machine: "LATHE LV2 #M-11",   cpu: "β" },
  { d: 3, id: "steel.plate",   icon: "▤", q: 16 * qty, status: "ok",      have: 4096, dedupe: true },
  { d: 1, id: "circuit.adv",   icon: "◈", q: 2  * qty, status: "ok",      have: 128 },
  { d: 1, id: "coolant",       icon: "≈", q: 6  * qty, status: "missing" },
];

// ─── ROOT ────────────────────────────────────────────────────
const CraftModal = () => {
  const [qty, setQty] = useCMState(1);
  const [phase, setPhase] = useCMState("input"); // "input" | "plan"
  const plan = buildPlan(qty);
  const missing = plan.filter(r => r.status === "missing");

  return (
    <div style={{
      position: "absolute", inset: 0,
      background: "rgba(26,26,26,0.42)",
      display: "flex", alignItems: "center", justifyContent: "center",
      fontFamily: "var(--font-mono)",
    }}>
      <div className="paper sk-box sk-thick" style={{
        width: phase === "input" ? 440 : 920,
        maxHeight: "88vh",
        padding: 0, display: "flex", flexDirection: "column",
        overflow: "hidden",
      }}>

        {/* MODAL HEADER */}
        <Row style={{
          padding: "10px 16px", borderBottom: "2px solid var(--ink)",
          background: "var(--paper-2)", justifyContent: "space-between",
        }}>
          <Row gap={8}>
            <span className="sk-h-sm">CRAFT</span>
            <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>
              {phase === "input" ? "— set quantity" : "— execution plan"}
            </span>
            {phase === "plan" && missing.length > 0 && (
              <span className="sk-tag sk-accent" style={{ fontSize: 9 }}>⚠ {missing.length} missing</span>
            )}
          </Row>
          <span className="sk-tag" style={{ cursor: "pointer", fontSize: 9 }}>ESC close</span>
        </Row>

        {phase === "input"
          ? <CMInputPhase qty={qty} setQty={setQty} onResolve={() => setPhase("plan")} />
          : <CMPlanPhase plan={plan} missing={missing} qty={qty} onBack={() => setPhase("input")} />
        }

      </div>
    </div>
  );
};


// ─── PHASE 1: QTY INPUT ──────────────────────────────────────
const CMInputPhase = ({ qty, setQty, onResolve }) => (
  <Col gap={0} style={{ flex: 1 }}>

    {/* item card */}
    <Row gap={14} style={{ padding: "20px 20px 16px", borderBottom: "1.5px solid var(--ink)" }}>
      <Slot filled style={{ width: 56, height: 56 }} icon={CRAFT_ITEM.icon} />
      <Col gap={3}>
        <span className="sk-h">{CRAFT_ITEM.id}</span>
        <Row gap={4}>
          <span className="sk-tag" style={{ fontSize: 9 }}>{CRAFT_ITEM.tag}</span>
          <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>{CRAFT_ITEM.kg} kg each</span>
        </Row>
        <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>
          in storage: <b style={{ color: "#9a1a1a" }}>{CRAFT_ITEM.have}</b>
        </span>
      </Col>
    </Row>

    {/* qty controls */}
    <Col gap={12} style={{ padding: "20px 20px 8px" }}>
      <span className="sk-mono-xs" style={{ letterSpacing: 2, color: "var(--ink-soft)" }}>QUANTITY</span>
      <Row gap={10} style={{ alignItems: "center" }}>
        <button className="sk-btn" style={{ width: 34, height: 34, justifyContent: "center", fontSize: 20, lineHeight: 1 }}
                onClick={() => setQty(Math.max(1, qty - 1))}>−</button>
        <div style={{
          border: "2px solid var(--ink)", padding: "6px 0",
          fontFamily: "var(--font-mono)", fontSize: 22, fontWeight: 900,
          width: 90, textAlign: "center",
        }}>{qty}</div>
        <button className="sk-btn" style={{ width: 34, height: 34, justifyContent: "center", fontSize: 20, lineHeight: 1 }}
                onClick={() => setQty(qty + 1)}>+</button>
      </Row>
      <Row gap={4} style={{ marginTop: -4 }}>
        {[1, 4, 16, 64, 256].map(n => (
          <button key={n} className={`sk-btn ${qty === n ? "sk-on" : ""}`}
                  style={{ padding: "2px 8px", fontSize: 10 }}
                  onClick={() => setQty(n)}>{n}</button>
        ))}
      </Row>
      <Row style={{ justifyContent: "space-between" }}>
        <span className="sk-mono-xs" style={{ color: "var(--ink-faint)" }}>est. mass added</span>
        <span className="sk-mono-sm"><b>+{(qty * CRAFT_ITEM.kg).toFixed(1)} kg</b></span>
      </Row>
    </Col>

    <div style={{ flex: 1 }} />

    {/* footer */}
    <Col gap={6} style={{ padding: "12px 20px", borderTop: "1.5px solid var(--ink)", background: "var(--paper-2)" }}>
      <span className="sk-mono-xs" style={{ color: "var(--ink-faint)", fontStyle: "italic" }}>
        plan resolves from current network state + machine priorities
      </span>
      <Row gap={8}>
        <button className="sk-btn sk-accent" style={{ flex: 1, justifyContent: "center", padding: "7px 12px" }}
                onClick={onResolve}>RESOLVE PLAN →</button>
        <button className="sk-btn" style={{ padding: "7px 14px" }}>CANCEL</button>
      </Row>
    </Col>
  </Col>
);


// ─── PHASE 2: RESOLVED PLAN ──────────────────────────────────
const CMPlanPhase = ({ plan, missing, qty, onBack }) => {
  const STATUS_COLOR = { ok: "#1a6b1a", craft: "var(--ink)", missing: "#9a1a1a" };
  const STATUS_LABEL = { ok: "✓ stocked", craft: "→ craft", missing: "✗ MISSING" };

  // unique machine assignments (deduplicate by machine id)
  const machines = plan
    .filter(r => r.machine)
    .reduce((acc, r) => {
      if (!acc.find(m => m.machine === r.machine)) acc.push(r);
      return acc;
    }, []);

  return (
    <Row gap={0} style={{ flex: 1, overflow: "hidden" }}>

      {/* LEFT: dependency tree */}
      <Col gap={0} style={{ flex: 1, overflow: "hidden" }}>
        <Row style={{
          padding: "6px 12px", borderBottom: "1.5px solid var(--ink)",
          background: "var(--paper-2)", justifyContent: "space-between",
          flexShrink: 0,
        }}>
          <span className="sk-mono-xs" style={{ color: "var(--ink-soft)", letterSpacing: 1 }}>
            DEPENDENCY TREE · read-only
          </span>
          <Row gap={8}>
            {[["#1a6b1a","stocked"],["var(--ink)","to craft"],["#9a1a1a","missing"]].map(([col, lbl]) => (
              <Row key={lbl} gap={2}>
                <span style={{ width: 7, height: 7, background: col, display: "inline-block", border: "1px solid var(--ink)" }} />
                <span className="sk-mono-xs" style={{ fontSize: 8, color: "var(--ink-faint)" }}>{lbl}</span>
              </Row>
            ))}
          </Row>
        </Row>

        <div style={{ flex: 1, overflow: "auto" }}>
          {plan.map((r, i) => {
            const sc = STATUS_COLOR[r.status];
            const sl = STATUS_LABEL[r.status];
            return (
              <Row key={i} gap={3} style={{
                padding: `3px 10px 3px ${r.d * 18 + 10}px`,
                borderBottom: "1px dashed var(--ink-faint)",
                background: r.status === "missing" ? "rgba(154,26,26,0.07)" : "transparent",
                opacity: r.dedupe ? 0.45 : 1,
              }}>
                {r.d > 0 && (
                  <span className="sk-mono-xs" style={{ color: "var(--ink-faint)", minWidth: 10, fontSize: 9 }}>└</span>
                )}
                <span style={{ width: 7, height: 7, background: sc, border: "1px solid var(--ink)", flexShrink: 0, marginTop: 1 }} />
                <Slot filled style={{ width: 16, height: 16 }} icon={r.icon} />
                <span className="sk-mono-sm" style={{ flex: 1, fontWeight: r.d === 0 ? 800 : 500, minWidth: 0 }}>
                  {r.id}
                  {r.dedupe && (
                    <span style={{ color: "var(--ink-faint)", fontWeight: 400, fontSize: 9 }}> (reuse)</span>
                  )}
                </span>
                <span className="sk-mono-xs" style={{ color: "var(--ink-faint)", minWidth: 40, textAlign: "right" }}>×{r.q}</span>
                <span className="sk-mono-xs" style={{
                  color: sc, minWidth: 70, textAlign: "right",
                  fontWeight: r.status === "missing" ? 700 : 400,
                }}>
                  {sl}{r.have && !r.dedupe ? ` (${r.have.toLocaleString()})` : ""}
                </span>
                {r.cpu && (
                  <span className="sk-tag sk-on" style={{ fontSize: 7, padding: "0 3px", marginLeft: 4 }}>
                    CPU-{r.cpu}
                  </span>
                )}
              </Row>
            );
          })}
        </div>
      </Col>

      {/* RIGHT: summary + machine plan + actions */}
      <Col gap={0} style={{ width: 275, borderLeft: "2px solid var(--ink)", background: "var(--paper-2)" }}>

        {/* plan stats */}
        <div style={{ padding: "10px 14px", borderBottom: "1.5px solid var(--ink)" }}>
          <span className="sk-mono-xs" style={{ color: "var(--ink-soft)", letterSpacing: 1 }}>PLAN SUMMARY</span>
          <Col gap={3} style={{ marginTop: 8 }}>
            {[
              ["target",     `${CRAFT_ITEM.id} ×${qty}`],
              ["est. time",  "14m 22s"],
              ["peak power", "8.4 MW"],
              ["mass added", `+${(qty * CRAFT_ITEM.kg).toFixed(1)} kg`],
              ["craft jobs", `${plan.filter(r => r.status === "craft" && !r.dedupe).length} steps`],
            ].map(([k, v]) => (
              <Row key={k} style={{ justifyContent: "space-between" }}>
                <span className="sk-mono-xs" style={{ color: "var(--ink-soft)" }}>{k}</span>
                <span className="sk-mono-sm" style={{ fontWeight: 700 }}>{v}</span>
              </Row>
            ))}
          </Col>
        </div>

        {/* missing warning */}
        {missing.length > 0 && (
          <div style={{ padding: "8px 14px", borderBottom: "1.5px solid var(--ink)", background: "rgba(154,26,26,0.06)" }}>
            <span className="sk-mono-xs" style={{ color: "#9a1a1a", fontWeight: 700 }}>⚠ MISSING INPUTS</span>
            <Col gap={2} style={{ marginTop: 6 }}>
              {missing.map((r, i) => (
                <Row key={i} gap={4}>
                  <Slot filled style={{ width: 14, height: 14 }} icon={r.icon} />
                  <span className="sk-mono-xs" style={{ color: "#9a1a1a", flex: 1 }}>{r.id}</span>
                  <span className="sk-mono-xs" style={{ color: "#9a1a1a" }}>×{r.q}</span>
                </Row>
              ))}
              <span className="sk-mono-xs" style={{ color: "var(--ink-faint)", marginTop: 2 }}>
                plan will stall until supplied
              </span>
            </Col>
          </div>
        )}

        {/* machine assignments */}
        <div style={{ padding: "10px 14px", flex: 1, overflow: "auto", borderBottom: "1.5px solid var(--ink)" }}>
          <span className="sk-mono-xs" style={{ color: "var(--ink-soft)", letterSpacing: 1 }}>MACHINE PLAN</span>
          <Col gap={4} style={{ marginTop: 8 }}>
            {machines.map((m, i) => (
              <Row key={i} gap={5} style={{
                padding: "4px 6px",
                border: "1px dashed var(--ink-faint)",
                background: "var(--paper)",
              }}>
                <span className="sk-tag sk-on" style={{ fontSize: 8, padding: "0 4px" }}>CPU-{m.cpu}</span>
                <Col gap={0} style={{ flex: 1, minWidth: 0 }}>
                  <span className="sk-mono-xs" style={{ fontWeight: 700, fontSize: 9 }}>{m.machine}</span>
                  <span className="sk-mono-xs" style={{ color: "var(--ink-faint)", fontSize: 8 }}>→ {m.id}</span>
                </Col>
              </Row>
            ))}
          </Col>
        </div>

        {/* actions footer */}
        <Col gap={6} style={{ padding: "10px 14px" }}>
          <span className="sk-mono-xs" style={{ color: "var(--ink-faint)", fontStyle: "italic", fontSize: 9 }}>
            based on current machine priorities
          </span>
          <button className="sk-btn sk-accent" style={{ justifyContent: "center", padding: "7px 12px" }}>
            {missing.length > 0 ? "⚠ ENQUEUE · will stall" : "▶ ENQUEUE CRAFT"}
          </button>
          <button className="sk-btn" style={{ justifyContent: "center" }} onClick={onBack}>← BACK</button>
        </Col>
      </Col>
    </Row>
  );
};


// CraftModalPlanOnly — starts on plan phase for wireframe artboard clarity
const CraftModalPlanOnly = () => {
  const qty = 1;
  const plan = buildPlan(qty);
  const missing = plan.filter(r => r.status === "missing");
  return (
    <div style={{ position:"absolute", inset:0, background:"rgba(26,26,26,0.42)",
                  display:"flex", alignItems:"center", justifyContent:"center",
                  fontFamily:"var(--font-mono)" }}>
      <div className="paper sk-box sk-thick" style={{ width:900, maxHeight:"88vh",
                                                       padding:0, display:"flex", flexDirection:"column",
                                                       overflow:"hidden" }}>
        <Row style={{ padding:"10px 16px", borderBottom:"2px solid var(--ink)",
                      background:"var(--paper-2)", justifyContent:"space-between" }}>
          <Row gap={8}>
            <span className="sk-h-sm">CRAFT</span>
            <span className="sk-mono-xs" style={{ color:"var(--ink-faint)" }}>— execution plan</span>
            {missing.length > 0 && <span className="sk-tag sk-accent" style={{ fontSize:9 }}>⚠ {missing.length} missing</span>}
          </Row>
          <span className="sk-tag" style={{ cursor:"pointer", fontSize:9 }}>ESC close</span>
        </Row>
        <CMPlanPhase plan={plan} missing={missing} qty={qty} onBack={()=>{}}/>
      </div>
    </div>
  );
};

window.CraftModal = CraftModal;
window.CraftModalPlanOnly = CraftModalPlanOnly;
