/* global React */

// ============================================================
// IN-WORLD HUD — LOCAL (body/build) · REMOTE (drone) modes
// ============================================================
// LOCAL  — body mode: O₂+SAT vitals, research pool in top bar
//          no HP (body safe from harm in build mode)
// REMOTE — drone mode: INT+PWR+SIG vitals, no research pool
//
// Floating panels — world shows through between elements.
// Three bottom areas: vitals (left, remote only) · hotbar (center) · minimap (right)
//
// See: docs/ui.md § In-World HUD

const { useState: useHUDState } = React;

// ─── DATA ────────────────────────────────────────────────────
const RESEARCH_TYPES = [
  { id: "mat.sci", color: "#3a6ea8", label: "material science" },
  { id: "field",   color: "#3d8b6b", label: "field research"   },
  { id: "eng",     color: "#b88a00", label: "engineering"      },
  { id: "disc",    color: "#7a3d8b", label: "discovery"        },
];
const RESEARCH_AMOUNTS = { "mat.sci": 47, field: 0, eng: 0, disc: 0 };

const BANKS = [
  {
    tag: "A", label: "TOOLS",
    slots: [
      { icon: "⛏", qty: 1 }, { icon: "◇" }, { icon: "◆" },
      { icon: "✦", qty: 1 }, { icon: "◉" }, null,
      null, { icon: "▣", qty: 1 }, null,
    ],
  },
  {
    tag: "B", label: "BUILD",
    slots: [
      { icon: "▦", qty: 1 }, { icon: "▧", qty: 1 }, null,
      { icon: "▨", qty: 1 }, { icon: "▩", qty: 1 }, null,
      null, { icon: "▤", qty: 1 }, { icon: "▥", qty: 1 },
    ],
  },
  {
    tag: "C", label: "COMBAT",
    slots: [
      { icon: "✕", qty: 64 }, { icon: "◈" }, null,
      { icon: "✦", qty: 1 }, null, null,
      { icon: "◉", qty: 1 }, null, null,
    ],
  },
];

const ALERTS = [
  { kind: "err",  icon: "▦", text: "CRAFTER #M-14 — gear.bronze blocked: missing input" },
  { kind: "warn", icon: "◍", text: "REACTOR #M-03 — power draw near grid cap (92%)" },
];

const PANEL_BG = "rgba(245,240,225,0.92)";

// ─── LOCAL HUD ───────────────────────────────────────────────
// Body/build mode. No HP. O₂+SAT vitals. Research pool in top bar.
const LocalHUD = ({ initialAlertsOpen = false }) => {
  const [activeBank, setActiveBank] = useHUDState(0);
  const [activeSlot, setActiveSlot] = useHUDState(0);
  const [alertsOpen, setAlertsOpen] = useHUDState(initialAlertsOpen);

  return (
    <div style={{
      position: "relative", width: "100%", height: "100%",
      overflow: "hidden", userSelect: "none",
    }}>
      <WorldBackground />
      <TopBar mode="local" alertsOpen={alertsOpen} setAlertsOpen={setAlertsOpen} />
      {alertsOpen && <AlertsPanel alerts={ALERTS} onClose={() => setAlertsOpen(false)} />}
      <HotbarPanel
        activeBank={activeBank} setActiveBank={setActiveBank}
        activeSlot={activeSlot} setActiveSlot={setActiveSlot}
      />
      <Minimap mode="local" />
    </div>
  );
};

// ─── REMOTE HUD ──────────────────────────────────────────────
// Drone control mode. INT+PWR+SIG vitals. No research pool.
const RemoteHUD = ({ initialAlertsOpen = false }) => {
  const [activeBank, setActiveBank] = useHUDState(0);
  const [activeSlot, setActiveSlot] = useHUDState(0);
  const [alertsOpen, setAlertsOpen] = useHUDState(initialAlertsOpen);

  return (
    <div style={{
      position: "relative", width: "100%", height: "100%",
      overflow: "hidden", userSelect: "none",
    }}>
      <WorldBackground drone />
      <TopBar mode="remote" alertsOpen={alertsOpen} setAlertsOpen={setAlertsOpen} />
      {alertsOpen && <AlertsPanel alerts={ALERTS} onClose={() => setAlertsOpen(false)} />}
      <DroneVitals />
      <HotbarPanel
        activeBank={activeBank} setActiveBank={setActiveBank}
        activeSlot={activeSlot} setActiveSlot={setActiveSlot}
      />
      <Minimap mode="remote" />
    </div>
  );
};

// ─── WORLD BACKGROUND ────────────────────────────────────────
const WorldBackground = ({ drone = false }) => (
  <div style={{ position: "absolute", inset: 0, overflow: "hidden" }}>
    {drone && (
      <div style={{
        position: "absolute", inset: 0, zIndex: 1,
        background: "rgba(10,20,45,0.10)",
        backgroundImage: "repeating-linear-gradient(0deg, transparent, transparent 3px, rgba(58,110,168,0.04) 3px, rgba(58,110,168,0.04) 4px)",
      }} />
    )}
    <div style={{
      position: "absolute", inset: 0,
      backgroundImage: `
        linear-gradient(var(--ink) 1px, transparent 1px),
        linear-gradient(90deg, var(--ink) 1px, transparent 1px)
      `,
      backgroundSize: "40px 40px",
      opacity: drone ? 0.10 : 0.06,
    }} />
    {[
      { g: "▦", x: "28%", y: "38%" }, { g: "⚙", x: "34%", y: "42%" },
      { g: "▦", x: "40%", y: "36%" }, { g: "◍", x: "46%", y: "44%" },
      { g: "▦", x: "52%", y: "40%" }, { g: "▦", x: "58%", y: "35%" },
      { g: "⚙", x: "64%", y: "43%" }, { g: "▦", x: "70%", y: "38%" },
    ].map((m, i) => (
      <span key={i} style={{
        position: "absolute", left: m.x, top: m.y,
        fontFamily: "var(--font-hand)", fontSize: 22,
        opacity: 0.14, color: "var(--ink)",
        transform: "translate(-50%,-50%)",
      }}>{m.g}</span>
    ))}
    <div style={{
      position: "absolute", left: "50%", top: "52%",
      transform: "translate(-50%,-50%)",
    }}>
      <div style={{
        width: 20, height: 20,
        border: `2px solid ${drone ? "#3a6ea8" : "var(--ink)"}`,
        borderRadius: drone ? 2 : "50%",
        background: drone ? "rgba(58,110,168,0.18)" : "var(--paper-2)",
        display: "flex", alignItems: "center", justifyContent: "center",
        fontSize: 10, opacity: 0.8,
        color: drone ? "#3a6ea8" : "var(--ink)",
      }}>{drone ? "✦" : "◎"}</div>
    </div>
    <div className="sk-annot" style={{ left: "50%", top: "60%", transform: "translateX(-50%)", opacity: 0.5 }}>
      {drone ? "drone view · remote control active" : "3D world view · cursor + WASD"}
    </div>
  </div>
);

// ─── TOP BAR ─────────────────────────────────────────────────
// local: research pool | remote: drone status widget
const TopBar = ({ mode, alertsOpen, setAlertsOpen }) => {
  const menuItems = [
    { key: "T", label: "TERMINAL" },
    { key: "I", label: "INDEX"    },
    { key: "P", label: "PLANNER"  },
    { key: "Y", label: "TECH"     },
  ];

  return (
    <div style={{
      position: "absolute", top: 0, left: 0, right: 0,
      display: "flex", alignItems: "center", gap: 6,
      padding: "5px 12px",
      background: PANEL_BG,
      borderBottom: "1.5px solid var(--ink)",
      zIndex: 10,
      fontFamily: "var(--font-mono)",
    }}>
      {/* mode badge */}
      <div style={{
        border: "1.5px solid var(--ink)",
        padding: "2px 7px",
        background: mode === "remote" ? "rgba(58,110,168,0.15)" : "var(--paper-2)",
        fontSize: 8, fontWeight: 700, letterSpacing: 1,
        fontFamily: "var(--font-label)",
        color: mode === "remote" ? "#3a6ea8" : "var(--ink)",
      }}>{mode === "remote" ? "DRONE" : "LOCAL"}</div>

      <div style={{ display: "flex", gap: 2 }}>
        {menuItems.map(m => (
          <div key={m.key} style={{
            border: "1.5px solid var(--ink)",
            padding: "2px 8px",
            display: "flex", gap: 5, alignItems: "center",
            cursor: "pointer",
            background: "var(--paper-2)",
          }}>
            <span style={{ fontSize: 10, fontWeight: 900, fontFamily: "var(--font-label)", color: "var(--ink)" }}>{m.key}</span>
            <span style={{ fontSize: 8, color: "var(--ink-soft)" }}>{m.label}</span>
          </div>
        ))}
      </div>

      <div style={{ flex: 1 }} />

      {mode === "local" ? (
        <div style={{
          display: "flex", alignItems: "center", gap: 8,
          padding: "2px 10px",
          border: "1.5px solid var(--ink)",
          background: "var(--paper)",
        }}>
          <span style={{ fontSize: 8, letterSpacing: 1, color: "var(--ink-soft)", fontFamily: "var(--font-label)" }}>R</span>
          {RESEARCH_TYPES.map(rt => {
            const amt = RESEARCH_AMOUNTS[rt.id] || 0;
            const active = amt > 0;
            return (
              <div key={rt.id} title={rt.label} style={{
                display: "flex", gap: 3, alignItems: "center",
                opacity: active ? 1 : 0.35,
              }}>
                <span style={{
                  width: 6, height: 6,
                  background: active ? rt.color : "var(--ink-faint)",
                  border: "1px solid var(--ink)",
                  borderRadius: 1, flexShrink: 0,
                }} />
                <span style={{
                  fontSize: 10, fontFamily: "var(--font-mono)",
                  fontWeight: active ? 700 : 400,
                  color: active ? "var(--ink)" : "var(--ink-faint)",
                }}>{active ? amt : "—"}</span>
                <span style={{ fontSize: 7, color: "var(--ink-faint)" }}>{rt.id}</span>
              </div>
            );
          })}
        </div>
      ) : (
        <div style={{
          display: "flex", alignItems: "center", gap: 8,
          padding: "2px 10px",
          border: "1.5px solid #3a6ea8",
          background: "rgba(58,110,168,0.08)",
        }}>
          <span style={{ fontSize: 8, color: "#3a6ea8", fontFamily: "var(--font-label)", letterSpacing: 1 }}>DRONE-07</span>
          <span style={{ width: 6, height: 6, borderRadius: "50%", background: "#3d8b6b", flexShrink: 0 }} />
          <span style={{ fontSize: 8, color: "#3d8b6b", fontFamily: "var(--font-mono)", fontWeight: 700 }}>ACTIVE</span>
          <span style={{ fontSize: 7, color: "var(--ink-faint)" }}>ctrl+R → recall</span>
        </div>
      )}

      <div
        onClick={() => setAlertsOpen(!alertsOpen)}
        style={{
          border: "1.5px solid var(--ink)",
          padding: "2px 8px",
          display: "flex", gap: 4, alignItems: "center",
          cursor: "pointer",
          background: alertsOpen ? "var(--ink)" : "rgba(154,26,26,0.14)",
          color: alertsOpen ? "var(--paper)" : "#9a1a1a",
        }}>
        <span style={{ fontSize: 10, fontWeight: 700 }}>⚠</span>
        <span style={{ fontSize: 9, fontWeight: 700 }}>{ALERTS.length}</span>
        <span style={{ fontSize: 7 }}>alerts</span>
      </div>
    </div>
  );
};

// ─── ALERTS PANEL ────────────────────────────────────────────
const AlertsPanel = ({ alerts, onClose }) => (
  <div style={{
    position: "absolute", top: 34, right: 12,
    width: 360,
    border: "1.5px solid var(--ink)",
    background: "var(--paper)",
    zIndex: 20,
    fontFamily: "var(--font-mono)",
  }}>
    <div style={{
      padding: "4px 10px", borderBottom: "1.5px solid var(--ink)",
      background: "var(--paper-2)",
      display: "flex", justifyContent: "space-between", alignItems: "center",
    }}>
      <span style={{ fontSize: 9, color: "var(--ink-soft)", letterSpacing: 1 }}>ALERTS</span>
      <span style={{ fontSize: 9, cursor: "pointer", color: "var(--ink-faint)" }} onClick={onClose}>✕</span>
    </div>
    {alerts.map((a, i) => (
      <div key={i} style={{
        display: "flex", gap: 8, alignItems: "flex-start",
        padding: "7px 10px",
        borderBottom: i < alerts.length - 1 ? "1px dashed var(--ink-faint)" : "none",
        background: a.kind === "err" ? "rgba(154,26,26,0.05)" : "rgba(184,138,0,0.05)",
      }}>
        <span style={{ fontSize: 16, opacity: 0.7, marginTop: -1 }}>{a.icon}</span>
        <span style={{ fontSize: 10, lineHeight: 1.4, color: a.kind === "err" ? "#9a1a1a" : "#b88a00" }}>{a.text}</span>
      </div>
    ))}
    <div style={{ padding: "5px 10px", borderTop: "1.5px solid var(--ink)", background: "var(--paper-2)" }}>
      <span style={{ fontSize: 8, color: "var(--ink-faint)" }}>click machine name → jump to machine UI</span>
    </div>
  </div>
);

// ─── DRONE VITALS ────────────────────────────────────────────
// Integrity + power + signal
const DroneVitals = () => (
  <div style={{
    position: "absolute", bottom: 14, left: 14,
    background: PANEL_BG,
    border: "1.5px solid #3a6ea8",
    padding: "8px 12px",
    display: "flex", flexDirection: "column", gap: 5,
    fontFamily: "var(--font-mono)",
    zIndex: 10,
  }}>
    <VitalBar label="INT" value={78} max={100} color="#a31919" />
    <VitalBar label="PWR" value={65} max={100} color="#b88a00" />
    <VitalBar label="SIG" value={91} max={100} color="#3d8b6b" />
  </div>
);

// ─── HOTBAR PANEL ────────────────────────────────────────────
// Floating center-bottom. Bank selector + slots.
const HotbarPanel = ({ activeBank, setActiveBank, activeSlot, setActiveSlot }) => {
  const bank = BANKS[activeBank];
  return (
    <div style={{
      position: "absolute", bottom: 14,
      left: "50%", transform: "translateX(-50%)",
      background: PANEL_BG,
      border: "1.5px solid var(--ink)",
      padding: "6px 10px 4px",
      fontFamily: "var(--font-mono)",
      zIndex: 10,
      display: "flex", flexDirection: "column", gap: 3, alignItems: "center",
    }}>
      {/* bank tabs */}
      <div style={{ display: "flex", gap: 2 }}>
        {BANKS.map((b, i) => (
          <button key={i}
            onClick={() => { setActiveBank(i); setActiveSlot(0); }}
            style={{
              padding: "1px 8px",
              border: "1.5px solid var(--ink)",
              background: activeBank === i ? "var(--ink)" : "var(--paper-2)",
              color: activeBank === i ? "var(--paper)" : "var(--ink)",
              fontFamily: "var(--font-mono)", fontSize: 9, fontWeight: 700,
              cursor: "pointer",
            }}>{b.tag} · {b.label}</button>
        ))}
      </div>

      {/* slots */}
      <div style={{ display: "flex", gap: 3 }}>
        {bank.slots.map((slot, i) => {
          const active = i === activeSlot;
          return (
            <div key={i}
              onClick={() => setActiveSlot(i)}
              style={{
                width: 44, height: 44,
                border: active ? "2px solid var(--ink)" : "1.5px solid var(--ink)",
                background: active ? "var(--ink)" : slot ? "var(--paper-2)" : "var(--paper)",
                display: "flex", flexDirection: "column",
                alignItems: "center", justifyContent: "center",
                position: "relative",
                cursor: slot ? "pointer" : "default",
                opacity: slot ? 1 : 0.3,
              }}>
              {slot && (
                <span style={{
                  fontFamily: "var(--font-hand)", fontSize: 19,
                  color: active ? "var(--paper)" : "var(--ink)",
                }}>{slot.icon}</span>
              )}
              {slot?.qty != null && (
                <span style={{
                  position: "absolute", bottom: 1, right: 2,
                  fontSize: 7, fontFamily: "var(--font-mono)", fontWeight: 700,
                  color: active ? "rgba(245,240,225,0.8)" : "var(--ink-faint)",
                }}>{slot.qty}</span>
              )}
              <span style={{
                position: "absolute", top: 1, left: 2,
                fontSize: 6, fontFamily: "var(--font-mono)", lineHeight: 1,
                color: active ? "rgba(245,240,225,0.55)" : "var(--ink-faint)",
              }}>{i + 1}</span>
            </div>
          );
        })}
      </div>

      {/* slot number hints */}
      <div style={{ display: "flex", gap: 3 }}>
        {Array.from({ length: 9 }, (_, i) => (
          <span key={i} style={{
            width: 47, textAlign: "center",
            fontSize: 6, fontFamily: "var(--font-mono)", color: "var(--ink-faint)",
          }}>{i + 1}</span>
        ))}
      </div>
    </div>
  );
};

// ─── MINIMAP ─────────────────────────────────────────────────
const MINIMAP_MACHINES = [
  { x: 28, y: 38, color: "#b88a00" },
  { x: 52, y: 44, color: "#b88a00" },
  { x: 68, y: 30, color: "#b88a00" },
  { x: 38, y: 58, color: "#3a6ea8" },
  { x: 78, y: 52, color: "#a31919" },
  { x: 18, y: 62, color: "#b88a00" },
  { x: 85, y: 68, color: "#b88a00" },
];
const MINIMAP_PATCHES = [
  { x: 15, y: 20, w: 25, h: 18, color: "rgba(61,139,107,0.28)" },
  { x: 60, y: 15, w: 20, h: 30, color: "rgba(61,139,107,0.18)" },
  { x: 45, y: 65, w: 30, h: 20, color: "rgba(184,138,0,0.18)"  },
  { x:  5, y: 55, w: 18, h: 25, color: "rgba(58,110,168,0.22)" },
];

const Minimap = ({ mode }) => {
  const SIZE = 120;
  const playerColor = mode === "remote" ? "#3a6ea8" : "#a31919";

  return (
    <div style={{
      position: "absolute", bottom: 14, right: 14,
      background: PANEL_BG,
      border: "1.5px solid var(--ink)",
      padding: 6,
      display: "flex", flexDirection: "column", gap: 4,
      alignItems: "center",
      fontFamily: "var(--font-mono)",
      zIndex: 10,
    }}>
      <div style={{
        width: SIZE, height: SIZE,
        background: "rgba(18,16,12,0.85)",
        border: "1px solid var(--ink)",
        position: "relative",
        overflow: "hidden",
      }}>
        {/* faint grid */}
        <div style={{
          position: "absolute", inset: 0,
          backgroundImage: `
            linear-gradient(rgba(200,190,160,0.07) 1px, transparent 1px),
            linear-gradient(90deg, rgba(200,190,160,0.07) 1px, transparent 1px)
          `,
          backgroundSize: "20px 20px",
        }} />

        {/* terrain patches */}
        {MINIMAP_PATCHES.map((p, i) => (
          <div key={i} style={{
            position: "absolute",
            left: `${p.x}%`, top: `${p.y}%`,
            width: `${p.w}%`, height: `${p.h}%`,
            background: p.color, borderRadius: 2,
          }} />
        ))}

        {/* machine dots */}
        {MINIMAP_MACHINES.map((m, i) => (
          <div key={i} style={{
            position: "absolute",
            left: `${m.x}%`, top: `${m.y}%`,
            width: 4, height: 4,
            background: m.color,
            transform: "translate(-50%,-50%)",
            opacity: 0.85,
          }} />
        ))}

        {/* fog of unexplored edges */}
        <div style={{
          position: "absolute", inset: 0,
          background: "radial-gradient(ellipse at 45% 50%, transparent 52%, rgba(18,16,12,0.55) 100%)",
          pointerEvents: "none",
        }} />

        {/* player / drone marker */}
        <div style={{
          position: "absolute", left: "50%", top: "52%",
          transform: "translate(-50%,-50%)",
          width: 7, height: 7,
          borderRadius: "50%",
          background: playerColor,
          boxShadow: `0 0 5px ${playerColor}`,
          zIndex: 2,
        }} />

        {/* N compass */}
        <span style={{
          position: "absolute", top: 2, right: 3,
          fontSize: 7, color: "rgba(200,190,160,0.45)",
          fontFamily: "var(--font-mono)",
        }}>N</span>
      </div>

      <div style={{ display: "flex", justifyContent: "space-between", width: "100%" }}>
        <span style={{ fontSize: 7, color: "var(--ink-faint)" }}>x:+247 y:−83</span>
        <span style={{ fontSize: 7, color: "var(--ink-faint)" }}>100m</span>
      </div>
    </div>
  );
};

// ─── VITAL BAR ───────────────────────────────────────────────
const VitalBar = ({ label, value, max, color }) => (
  <div style={{ display: "flex", alignItems: "center", gap: 5 }}>
    <span style={{
      fontSize: 8, width: 24, fontFamily: "var(--font-mono)",
      color: "var(--ink-soft)", textAlign: "right",
    }}>{label}</span>
    <div style={{
      width: 104, height: 6,
      border: "1.5px solid var(--ink)", background: "var(--paper)",
      position: "relative",
    }}>
      <div style={{ width: `${(value / max) * 100}%`, height: "100%", background: color }} />
    </div>
    <span style={{
      fontSize: 8, fontFamily: "var(--font-mono)",
      fontWeight: 700, color: "var(--ink-soft)", minWidth: 24,
    }}>{value}</span>
  </div>
);

window.LocalHUD  = LocalHUD;
window.RemoteHUD = RemoteHUD;
window.InWorldHUD = LocalHUD; // backwards compat
