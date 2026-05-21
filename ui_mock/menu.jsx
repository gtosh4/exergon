/* global React */

// ============================================================
// MENU SCREENS — main menu / new run wizard / load run /
//                settings / pause / save (checkpoint confirm)
// ============================================================
//
// Save model per docs/technical/save.md:
//   - one continuous primary save per run (autosave)
//   - auto checkpoints: tier unlock, escape construction start
//   - one manual "checkpoint" slot per run (overwritable)
//   - completed runs are read-only
//
// New-run flow per ui.md ask: full wizard
//   difficulty -> seed -> planet preview -> starting loadout
//
// See: docs/ui.md, docs/technical/save.md

const { useState: useM } = React;

// ─── COLOR TOKENS ────────────────────────────────────────────
const M = {
  bg:      "#1a1d21",
  p1:      "#22262b",
  p2:      "#2a2f36",
  p3:      "#333940",
  text:    "#c8cdd4",
  dim:     "#5a6370",
  soft:    "#909aa4",
  acc:     "#8a72aa",
  ok:      "#4a9e6a",
  warn:    "#c8b440",
  err:     "#b84a4a",
  border:  "rgba(255,255,255,0.07)",
};

// ─── REUSED PRIMITIVES ───────────────────────────────────────
const Wordmark = ({ size = 56 }) => (
  <div style={{
    fontFamily: "Inter, system-ui, sans-serif",
    fontWeight: 200, fontSize: size, letterSpacing: "0.42em",
    color: M.text, paddingLeft: "0.42em",
  }}>EXERGON</div>
);

const Tagline = ({ text }) => (
  <div style={{
    fontFamily: "JetBrains Mono, monospace", fontSize: 11,
    color: M.dim, letterSpacing: "0.3em", textTransform: "uppercase",
    marginTop: 4,
  }}>{text}</div>
);

const MenuRow = ({ label, kbd, hint, on, disabled, accent }) => (
  <div style={{
    display: "flex", alignItems: "center", gap: 16,
    padding: "10px 18px", cursor: disabled ? "not-allowed" : "pointer",
    borderLeft: `2px solid ${on ? M.acc : "transparent"}`,
    background: on ? "rgba(138,114,170,0.10)" : "transparent",
    opacity: disabled ? 0.35 : 1,
  }}>
    <div style={{
      fontFamily: "Inter, system-ui, sans-serif",
      fontWeight: on ? 600 : 400, fontSize: 17,
      color: accent ? M.acc : (on ? M.text : M.soft),
      letterSpacing: "0.08em", flex: 1, whiteSpace: "nowrap",
    }}>{label}</div>
    {hint && <div style={{
      fontFamily: "JetBrains Mono, monospace", fontSize: 10,
      color: M.dim,
    }}>{hint}</div>}
    {kbd && <div style={{
      fontFamily: "JetBrains Mono, monospace", fontSize: 10,
      color: M.dim, border: `1px solid ${M.border}`,
      padding: "1px 6px", borderRadius: 2,
    }}>{kbd}</div>}
  </div>
);

const Btn = ({ children, on, ghost, danger, style }) => (
  <button className={`sk-btn ${on ? "sk-on" : ""}`} style={{
    ...(ghost ? { background: "transparent" } : {}),
    ...(danger ? { color: M.err, borderColor: "rgba(184,74,74,0.35)" } : {}),
    ...style,
  }}>{children}</button>
);

const Tag = ({ children, color, on }) => (
  <span className={`sk-tag ${on ? "sk-on" : ""}`} style={
    color ? { color, borderColor: `${color}55`, background: `${color}11` } : {}
  }>{children}</span>
);

const Divider = () => <div className="sk-div" style={{ margin: "10px 0" }} />;

// Backdrop = subtle gradient + faint grid; evokes deep space
const Backdrop = ({ children, dim = false }) => (
  <div style={{
    position: "relative", width: "100%", height: "100%",
    background: `radial-gradient(ellipse at 70% 25%, #2b3142 0%, ${M.bg} 60%)`,
    overflow: "hidden",
  }}>
    <div style={{
      position: "absolute", inset: 0,
      backgroundImage: `linear-gradient(rgba(255,255,255,0.025) 1px, transparent 1px),
                        linear-gradient(90deg, rgba(255,255,255,0.025) 1px, transparent 1px)`,
      backgroundSize: "40px 40px",
      maskImage: "radial-gradient(ellipse at 50% 50%, #000 30%, transparent 80%)",
    }}/>
    {dim && <div style={{ position: "absolute", inset: 0, background: "rgba(0,0,0,0.55)" }}/>}
    {children}
  </div>
);

// Stylised planet glyph — placeholder art for previews
const PlanetGlyph = ({ size = 180, hue = 220, ring = false, atmo = true }) => (
  <div style={{
    position: "relative", width: size, height: size,
    display: "flex", alignItems: "center", justifyContent: "center",
  }}>
    {atmo && <div style={{
      position: "absolute", width: size + 24, height: size + 24,
      borderRadius: "50%",
      background: `radial-gradient(circle, hsla(${hue},45%,55%,0.18) 30%, transparent 65%)`,
    }}/>}
    <div style={{
      width: size, height: size, borderRadius: "50%",
      background: `radial-gradient(circle at 32% 32%,
        hsl(${hue},35%,55%) 0%,
        hsl(${hue},45%,32%) 45%,
        hsl(${hue+15},55%,15%) 100%)`,
      boxShadow: `inset -20px -10px 40px rgba(0,0,0,0.7)`,
      position: "relative", overflow: "hidden",
    }}>
      <div style={{
        position: "absolute", top: "18%", left: "12%", width: "30%", height: "14%",
        borderRadius: "50%", background: "rgba(255,255,255,0.07)", filter: "blur(8px)",
      }}/>
      <div style={{
        position: "absolute", bottom: "22%", right: "18%", width: "26%", height: "12%",
        borderRadius: "50%", background: "rgba(0,0,0,0.4)", filter: "blur(10px)",
      }}/>
    </div>
    {ring && <div style={{
      position: "absolute", width: size * 1.5, height: size * 0.32,
      borderRadius: "50%", border: `1px solid hsla(${hue},40%,60%,0.45)`,
      transform: "rotate(-18deg)", boxShadow: "inset 0 0 8px rgba(0,0,0,0.6)",
    }}/>}
  </div>
);

// ============================================================
// 01 · MAIN MENU
// ============================================================
const MainMenu = ({ variant = "default" }) => {
  // variant: "default" (with resume) | "noresume" | "firstrun"
  const hasResume = variant === "default";
  const isFirst = variant === "firstrun";
  return (
    <Backdrop>
      {/* Wordmark + tagline */}
      <div style={{
        position: "absolute", left: 80, top: 110,
      }}>
        <Wordmark/>
        <Tagline text="A run-based factory science campaign"/>
        <div style={{
          fontFamily: "JetBrains Mono, monospace", fontSize: 10,
          color: M.dim, marginTop: 28,
        }}>
          v0.1.0-vs · build 4a91c2 · 2026-05-20
        </div>
      </div>

      {/* Menu list */}
      <div style={{
        position: "absolute", left: 80, top: 280, width: 360,
      }}>
        {hasResume && (
          <>
            <MenuRow
              label="RESUME RUN"
              hint="kepler-9 · standard · T3 · 6h12m"
              on={true} accent
            />
            <div style={{ height: 4 }}/>
          </>
        )}
        <MenuRow label="NEW RUN" on={isFirst} accent={isFirst}/>
        <MenuRow label="LOAD RUN" disabled={isFirst}/>
        <MenuRow label="CODEX" hint={isFirst ? "" : "47 entries"}/>
        <MenuRow label="SETTINGS"/>
        <MenuRow label="QUIT"/>
      </div>

      {/* Right panel — run-at-a-glance for resume, or splash quote */}
      {hasResume ? (
        <div style={{
          position: "absolute", right: 80, top: 200, width: 380,
          border: `1px solid ${M.border}`, borderRadius: 4,
          background: M.p1, padding: 20,
        }}>
          <div style={{
            fontFamily: "JetBrains Mono, monospace", fontSize: 10,
            color: M.dim, letterSpacing: "0.2em",
          }}>LAST RUN · IN PROGRESS</div>
          <div style={{ display: "flex", alignItems: "center", gap: 16, marginTop: 12 }}>
            <PlanetGlyph size={88} hue={205}/>
            <div>
              <div className="sk-h">kepler-9</div>
              <div className="sk-mono" style={{ color: M.soft, marginTop: 2 }}>
                low-O₂ · high-pressure
              </div>
              <div className="sk-mono" style={{ color: M.dim, marginTop: 8 }}>
                STANDARD · TIER 3
              </div>
              <div className="sk-mono" style={{ color: M.dim }}>
                playtime 6h 12m
              </div>
            </div>
          </div>
          <Divider/>
          <div className="sk-label">objective</div>
          <div className="sk-mono" style={{ color: M.text, marginTop: 4 }}>
            unlock electrolysis chain → tier 4 gateway
          </div>
          <Divider/>
          <div className="sk-label">last checkpoint</div>
          <div className="sk-mono" style={{ color: M.text, marginTop: 4 }}>
            tier_3 · 47m ago (auto)
          </div>
        </div>
      ) : (
        <div style={{
          position: "absolute", right: 80, top: 240, width: 360,
          fontFamily: "Inter, system-ui, sans-serif", fontStyle: "italic",
          fontSize: 18, color: M.soft, lineHeight: 1.7,
        }}>
          “The reset must feel like a launch, not a loss.”
          <div style={{
            fontStyle: "normal", fontSize: 10, color: M.dim, marginTop: 12,
            fontFamily: "JetBrains Mono, monospace", letterSpacing: "0.2em",
          }}>— EXERGON DESIGN PILLAR · 02</div>
        </div>
      )}

      {/* Footer hints */}
      <div style={{
        position: "absolute", bottom: 24, left: 80, right: 80,
        display: "flex", justifyContent: "space-between",
        fontFamily: "JetBrains Mono, monospace", fontSize: 10, color: M.dim,
      }}>
        <div>↑↓ navigate · ↵ select · esc back</div>
        <div>gtosh4@gmail.com</div>
      </div>
    </Backdrop>
  );
};

// ============================================================
// 02 · NEW RUN WIZARD — 4 steps
// ============================================================
const StepBar = ({ step }) => {
  const labels = ["DIFFICULTY", "MODIFIERS", "PLANET"];
  return (
    <div style={{
      display: "flex", alignItems: "center", gap: 12,
      padding: "16px 32px", borderBottom: `1px solid ${M.border}`,
    }}>
      <div style={{
        fontFamily: "Inter, system-ui, sans-serif",
        fontWeight: 200, fontSize: 14, letterSpacing: "0.3em",
        color: M.text, marginRight: 12,
      }}>NEW RUN</div>
      {labels.map((l, i) => (
        <React.Fragment key={l}>
          <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
            <div style={{
              width: 22, height: 22, borderRadius: "50%",
              border: `1px solid ${i === step ? M.acc : i < step ? M.ok : M.border}`,
              background: i === step ? "rgba(138,114,170,0.18)" : i < step ? "rgba(74,158,106,0.15)" : "transparent",
              color: i === step ? M.acc : i < step ? M.ok : M.dim,
              display: "flex", alignItems: "center", justifyContent: "center",
              fontFamily: "JetBrains Mono, monospace", fontSize: 10,
            }}>{i < step ? "✓" : i + 1}</div>
            <div style={{
              fontFamily: "JetBrains Mono, monospace", fontSize: 10,
              color: i === step ? M.text : M.dim, letterSpacing: "0.15em",
            }}>{l}</div>
          </div>
          {i < labels.length - 1 && <div style={{
            flex: "0 0 24px", height: 1, background: i < step ? M.ok : M.border,
          }}/>}
        </React.Fragment>
      ))}
      <div style={{ flex: 1 }}/>
      <Btn ghost>CANCEL</Btn>
    </div>
  );
};

const DiffCard = ({ name, len, tiers, audience, selected, locked }) => (
  <div style={{
    flex: 1, padding: 20,
    border: `1px solid ${selected ? M.acc : M.border}`,
    background: selected ? "rgba(138,114,170,0.08)" : M.p1,
    borderRadius: 4, opacity: locked ? 0.4 : 1,
    position: "relative",
  }}>
    <div className="sk-h" style={{ color: selected ? M.acc : M.text }}>{name}</div>
    <Divider/>
    <div className="sk-label">approx run length</div>
    <div className="sk-mono" style={{ color: M.text, marginTop: 2 }}>{len}</div>
    <div className="sk-label" style={{ marginTop: 8 }}>tiers</div>
    <div className="sk-mono" style={{ color: M.text, marginTop: 2 }}>{tiers}</div>
    <div className="sk-label" style={{ marginTop: 8 }}>for</div>
    <div className="sk-mono" style={{ color: M.soft, marginTop: 2 }}>{audience}</div>
    {locked && (
      <div style={{
        position: "absolute", top: 16, right: 16,
        fontFamily: "JetBrains Mono, monospace", fontSize: 9,
        color: M.dim,
      }}>🔒 complete prev tier</div>
    )}
  </div>
);

const NewRunStep1 = () => (
  <div style={{ padding: 32 }}>
    <div className="sk-h">Select difficulty</div>
    <div className="sk-label" style={{ marginTop: 4 }}>
      Difficulty sets the depth of the tech tree and approximate run length.
      Higher tiers unlocked by completing the previous.
    </div>
    <div style={{ display: "flex", gap: 12, marginTop: 24 }}>
      <DiffCard name="INITIATION" len="3–6 h"  tiers="T0 – T3" audience="first run / tutorial"/>
      <DiffCard name="STANDARD"   len="10–15 h" tiers="T0 – T5" audience="experienced builders" selected/>
      <DiffCard name="ADVANCED"   len="20–30 h" tiers="T0 – T7" audience="depth-seekers" locked/>
      <DiffCard name="PINNACLE"   len="30–50 h" tiers="T0 – T10" audience="full graph mastery" locked/>
    </div>
  </div>
);

// Compact seed control — sits at top of modifiers step. De-emphasized:
// most players just roll. Power-users get input + paste.
const SeedStrip = ({ open, onToggle }) => (
  <div style={{
    padding: 12, border: `1px solid ${M.border}`,
    background: M.p1, borderRadius: 3,
  }}>
    <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
      <div className="sk-label" style={{ flex: "0 0 auto" }}>SEED</div>
      <div className="sk-mono" style={{
        flex: 1, color: M.text, letterSpacing: "0.05em",
      }}>kepler-9-aurelius</div>
      <div className="sk-mono-xs" style={{ color: M.dim }}>0x9f4ac2e1</div>
      <Btn>↻ ROLL</Btn>
      <Btn ghost>✎ EDIT</Btn>
    </div>
    <div className="sk-mono-xs" style={{ color: M.dim, marginTop: 6, fontStyle: "italic" }}>
      most players just roll · share a seed only to invite spoilers
    </div>
  </div>
);

const PlanetProp = ({ k, v, severity }) => (
  <div style={{ display: "flex", justifyContent: "space-between", padding: "5px 0" }}>
    <span className="sk-label">{k}</span>
    <span className="sk-mono" style={{
      color: severity === "warn" ? M.warn : severity === "err" ? M.err : M.text,
    }}>{v}</span>
  </div>
);

const NewRunStep3 = () => (
  <div style={{ padding: 32, display: "flex", gap: 32 }}>
    {/* Left — planet visual */}
    <div style={{ flex: "0 0 360px", display: "flex", flexDirection: "column", alignItems: "center" }}>
      <PlanetGlyph size={260} hue={28} atmo={true}/>
      <div style={{
        fontFamily: "Inter, system-ui, sans-serif", fontWeight: 200,
        fontSize: 28, letterSpacing: "0.15em", color: M.text, marginTop: 20,
      }}>KEPLER-9</div>
      <div className="sk-mono" style={{ color: M.dim, marginTop: 4 }}>
        the low-oxygen high-pressure world
      </div>
      <div style={{ display: "flex", gap: 6, marginTop: 12, flexWrap: "wrap", justifyContent: "center" }}>
        <Tag color="#8a72aa">aetherspark trace</Tag>
        <Tag color="#c8b440">low-O₂</Tag>
        <Tag color="#b84a4a">high-pressure</Tag>
        <Tag color="#4a82c8">geothermal active</Tag>
      </div>
    </div>

    {/* Right — properties */}
    <div style={{ flex: 1, display: "flex", flexDirection: "column", gap: 16 }}>
      <div>
        <div className="sk-h">Planet read · final review</div>
        <div className="sk-label" style={{ marginTop: 4 }}>
          Broad characteristics from orbit, with selected modifiers applied.
          Full property reveal requires scouting on landing.
        </div>
        <div style={{ display: "flex", gap: 6, marginTop: 8, flexWrap: "wrap" }}>
          <Tag color={M.err}>+2 challenge: tighter research</Tag>
          <Tag color={M.err}>+3 challenge: hardcore</Tag>
          <Tag color={M.ok}>−2 boon: ratio calc T1</Tag>
          <Tag color={M.ok}>−1 boon: starting cache</Tag>
          <Tag color={M.acc}>net +2</Tag>
        </div>
      </div>

      <div style={{ display: "flex", gap: 16 }}>
        <div style={{ flex: 1, padding: 16, border: `1px solid ${M.border}`, background: M.p1, borderRadius: 4 }}>
          <div className="sk-label">atmospheric</div>
          <Divider/>
          <PlanetProp k="oxygen"     v="≈ 7%"     severity="warn"/>
          <PlanetProp k="pressure"   v="3.4 atm"  severity="err"/>
          <PlanetProp k="temp range" v="-40 → 35 °C"/>
          <PlanetProp k="weather"    v="dust storms · frequent"/>
        </div>
        <div style={{ flex: 1, padding: 16, border: `1px solid ${M.border}`, background: M.p1, borderRadius: 4 }}>
          <div className="sk-label">resources</div>
          <Divider/>
          <PlanetProp k="solar yield" v="weak · diffuse"/>
          <PlanetProp k="geothermal"  v="abundant" severity="ok"/>
          <PlanetProp k="ores"        v="rich · iron, copper, ???"/>
          <PlanetProp k="fluids"      v="liquid CO₂ at depth"/>
        </div>
      </div>

      <div style={{ padding: 16, border: `1px solid ${M.border}`, background: M.p1, borderRadius: 4 }}>
        <div className="sk-label">precursor</div>
        <Divider/>
        <div className="sk-mono" style={{ color: M.text, lineHeight: 1.7 }}>
          GATEWAY-CLASS structure detected at coordinates [redacted].
          Inactive. Surrounded by partially intact processing arrays.
        </div>
        <div style={{ marginTop: 6 }}>
          <Tag color="#8a72aa">escape objective</Tag>
        </div>
      </div>

      <div style={{ flex: 1 }}/>
      <div style={{
        fontFamily: "JetBrains Mono, monospace", fontSize: 10,
        color: M.dim, fontStyle: "italic",
      }}>
        “Solar is hostile here — geothermal is your power story.
         The pressure forecast suggests an early need for sealed processing.”
      </div>
    </div>
  </div>
);

// ─── modifier point-buy row ──────────────────────────────────
// Per gdd §14: challenges award pts, boons cost pts, net must be ≥ 0.
// Tool-access boon = shifts an Engineering unlock window earlier.
const ModRow = ({ on, name, desc, pts, tags = [], locked }) => {
  const isChallenge = pts > 0;
  const color = isChallenge ? M.err : M.ok;
  return (
    <div style={{
      display: "flex", alignItems: "center", gap: 14,
      padding: "10px 12px",
      border: `1px solid ${on ? color : M.border}`,
      background: on ? `${color}11` : M.p2,
      borderRadius: 3, marginBottom: 6,
      opacity: locked ? 0.4 : 1,
    }}>
      <div style={{
        width: 16, height: 16, borderRadius: 2,
        border: `1px solid ${on ? color : M.border}`,
        background: on ? color : "transparent",
        display: "flex", alignItems: "center", justifyContent: "center",
        color: "#fff", fontSize: 10, flexShrink: 0,
      }}>{on ? "✓" : ""}</div>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <div className="sk-h-sm" style={{ color: M.text }}>{name}</div>
          {tags.map(t => <Tag key={t}>{t}</Tag>)}
          {locked && <Tag color={M.dim}>🔒 META-LOCKED</Tag>}
        </div>
        <div className="sk-mono-sm" style={{ color: M.soft, marginTop: 2 }}>{desc}</div>
      </div>
      <div style={{
        fontFamily: "JetBrains Mono, monospace", fontWeight: 700, fontSize: 14,
        color, minWidth: 60, textAlign: "right",
      }}>{isChallenge ? `+${pts}` : pts} pt</div>
    </div>
  );
};

const NewRunStep4 = () => {
  // Sample selection — illustrative, not interactive in a mock.
  // Challenges (active): low-research-budget (+2), hardcore (+3) → +5 pts
  // Boons (active): early-ratio-calculator (-2), starting-cache (-1) → -3 pts
  // Starting-conditions pool pick: "extra cable spool" (free, single choice)
  const net = 2; // +5 - 3
  return (
    <div style={{ padding: 28, display: "flex", flexDirection: "column", gap: 16, height: "calc(100% - 57px - 56px)", overflow: "hidden" }}>
      <SeedStrip/>
      <div style={{ display: "flex", gap: 24, flex: 1, minHeight: 0 }}>
      {/* Left — challenges */}
      <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
          <div className="sk-h">Challenges</div>
          <Tag color={M.err}>award points</Tag>
        </div>
        <div className="sk-label" style={{ marginTop: 4, marginBottom: 12 }}>
          Make the run harder along an explicit axis. Required to fund boons.
        </div>
        <div style={{ flex: 1, overflowY: "auto", paddingRight: 4 }}>
          <ModRow on name="tighter research budget" pts={2}
                  tags={["RESEARCH"]}
                  desc="research yield −25% across all four types"/>
          <ModRow on name="hardcore mode" pts={3}
                  tags={["SAVE"]}
                  desc="no checkpoints, no manual save — primary autosave only"/>
          <ModRow name="harder planet modifiers" pts={3}
                  tags={["PLANET"]}
                  desc="amplify physical penalties (atmospheric, thermal, pressure)"/>
          <ModRow name="elevated world reactivity" pts={2}
                  tags={["WORLD"]}
                  desc="biosphere and weather escalate faster against your footprint"/>
          <ModRow name="GT-style power punishment" pts={4}
                  tags={["POWER"]}
                  desc="overvoltage destroys machines; undervoltage stalls hard"/>
          <ModRow name="disable autocraft" pts={3}
                  tags={["LOGISTICS"]}
                  desc="terminal CRAFT modal disabled; all crafting goes through machine UI"/>
          <ModRow name="no blueprints this run" pts={1}
                  tags={["QOL"]}
                  desc="saved blueprints unavailable; layout from scratch"/>
        </div>
      </div>

      {/* Middle — point balance */}
      <div style={{ flex: "0 0 200px", display: "flex", flexDirection: "column", alignItems: "center", paddingTop: 24 }}>
        <div className="sk-label">net</div>
        <div style={{
          fontFamily: "Inter, system-ui, sans-serif", fontWeight: 200, fontSize: 72,
          color: net >= 0 ? M.ok : M.err, lineHeight: 1,
          margin: "8px 0 4px",
        }}>{net >= 0 ? `+${net}` : net}</div>
        <div className="sk-mono" style={{ color: M.dim }}>points</div>

        <div style={{ width: "100%", marginTop: 24, padding: 14, background: M.p1, border: `1px solid ${M.border}`, borderRadius: 4 }}>
          <div style={{ display: "flex", justifyContent: "space-between", padding: "2px 0" }}>
            <span className="sk-mono" style={{ color: M.err }}>challenges</span>
            <span className="sk-mono" style={{ color: M.err }}>+5</span>
          </div>
          <div style={{ display: "flex", justifyContent: "space-between", padding: "2px 0" }}>
            <span className="sk-mono" style={{ color: M.ok }}>boons</span>
            <span className="sk-mono" style={{ color: M.ok }}>−3</span>
          </div>
          <div className="sk-div" style={{ margin: "6px 0" }}/>
          <div style={{ display: "flex", justifyContent: "space-between", padding: "2px 0" }}>
            <span className="sk-mono" style={{ color: M.text }}>net</span>
            <span className="sk-mono" style={{ color: M.ok, fontWeight: 700 }}>+{net}</span>
          </div>
        </div>

        <div style={{ marginTop: 14, fontFamily: "JetBrains Mono, monospace", fontSize: 10, color: M.dim, textAlign: "center", lineHeight: 1.6 }}>
          Net must be <span style={{ color: M.text }}>≥ 0</span>.
          Earned challenge-pt balance: <span style={{ color: M.text }}>14</span>
        </div>
      </div>

      {/* Right — boons + starting pool */}
      <div style={{ flex: 1, display: "flex", flexDirection: "column", minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "baseline", gap: 8 }}>
          <div className="sk-h">Boons</div>
          <Tag color={M.ok}>cost points</Tag>
        </div>
        <div className="sk-label" style={{ marginTop: 4, marginBottom: 12 }}>
          Spend points to soften specific frictions. Capped by challenges this run.
        </div>
        <div style={{ flex: 1, overflowY: "auto", paddingRight: 4 }}>
          <ModRow on name="ratio calculator unlocked at T1" pts={-2}
                  tags={["QOL", "ENGINEERING"]}
                  desc="shift Engineering unlock window from T3 → T1"/>
          <ModRow on name="starting resource cache" pts={-1}
                  tags={["BOON"]}
                  desc="+200 iron, +100 copper, +50 sealed-canister"/>
          <ModRow name="auto-craft network unlocked at T2" pts={-5}
                  tags={["QOL", "ENGINEERING"]}
                  desc="shift Engineering unlock window from T4 → T2"/>
          <ModRow name="pre-research one alien node" pts={-3}
                  tags={["RESEARCH"]}
                  desc="reveal one random tier-2 alien-science node at landing"/>
          <ModRow name="extra blueprint slot" pts={-1}
                  tags={["QOL"]} locked
                  desc="+1 blueprint slot for this run"/>
          <ModRow name="upgraded starting drill" pts={-1}
                  tags={["TOOL"]}
                  desc="hand drill begins at T1 speed instead of T0"/>
        </div>

        <div className="sk-div" style={{ margin: "16px 0 12px" }}/>
        <div className="sk-label">starting condition · pick one (free)</div>
        <div style={{ display: "flex", gap: 6, marginTop: 8, flexWrap: "wrap" }}>
          <Btn>extra cable spool</Btn>
          <Btn on>geothermal sample</Btn>
          <Btn>field battery</Btn>
          <Btn>extended scanner</Btn>
        </div>
      </div>
      </div>
    </div>
  );
};

const NewRun = ({ step = 0 }) => (
  <Backdrop>
    <StepBar step={step}/>
    {step === 0 && <NewRunStep1/>}
    {step === 1 && <NewRunStep4/>}
    {step === 2 && <NewRunStep3/>}
    <div style={{
      position: "absolute", bottom: 0, left: 0, right: 0,
      padding: "12px 32px", borderTop: `1px solid ${M.border}`,
      display: "flex", justifyContent: "space-between", alignItems: "center",
      background: M.bg,
    }}>
      <div style={{ fontFamily: "JetBrains Mono, monospace", fontSize: 10, color: M.dim }}>
        step {step + 1} / 3
      </div>
      <div style={{ display: "flex", gap: 8 }}>
        <Btn ghost>← BACK</Btn>
        <Btn on>{step === 2 ? "LAND ON KEPLER-9 →" : "NEXT →"}</Btn>
      </div>
    </div>
  </Backdrop>
);

// ============================================================
// 03 · LOAD RUN
// ============================================================
const RunListItem = ({ seed, planet, diff, tier, time, status, selected }) => {
  const statusColor = status === "in-progress" ? M.acc :
                      status === "completed"   ? M.ok  :
                      M.dim;
  return (
    <div style={{
      display: "flex", alignItems: "center", gap: 14,
      padding: "12px 16px", cursor: "pointer",
      borderLeft: `2px solid ${selected ? M.acc : "transparent"}`,
      background: selected ? "rgba(138,114,170,0.10)" : "transparent",
      borderBottom: `1px solid ${M.border}`,
    }}>
      <PlanetGlyph size={42} hue={status === "completed" ? 130 : 205} atmo={false}/>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          <div className="sk-h-sm" style={{ color: M.text }}>{seed}</div>
          <Tag color={statusColor}>{status}</Tag>
        </div>
        <div className="sk-mono" style={{ color: M.soft, marginTop: 2 }}>
          {planet}
        </div>
        <div className="sk-mono" style={{ color: M.dim, marginTop: 2 }}>
          {diff} · {tier} · {time}
        </div>
      </div>
    </div>
  );
};

const CheckpointRow = ({ kind, label, age, selected, locked }) => (
  <div style={{
    display: "flex", alignItems: "center", gap: 10,
    padding: "10px 12px",
    border: `1px solid ${selected ? M.acc : M.border}`,
    background: selected ? "rgba(138,114,170,0.08)" : M.p2,
    borderRadius: 3, marginBottom: 6,
  }}>
    <div style={{
      width: 8, height: 8, borderRadius: "50%",
      background: kind === "primary" ? M.ok : kind === "manual" ? M.acc : M.warn,
    }}/>
    <div style={{ flex: 1 }}>
      <div className="sk-mono" style={{ color: M.text }}>{label}</div>
      <div className="sk-mono-xs" style={{ color: M.dim, marginTop: 1 }}>
        {kind === "primary" ? "primary save · overwritable" :
         kind === "manual"  ? "manual checkpoint · overwritable" :
                              "auto checkpoint · read-only"}
      </div>
    </div>
    <div className="sk-mono-xs" style={{ color: M.dim }}>{age}</div>
    {locked && <span style={{ color: M.dim, fontSize: 11 }}>🔒</span>}
  </div>
);

const LoadRun = () => (
  <Backdrop>
    {/* Top bar */}
    <div style={{
      display: "flex", alignItems: "center", padding: "16px 32px",
      borderBottom: `1px solid ${M.border}`,
    }}>
      <div style={{
        fontFamily: "Inter, system-ui, sans-serif", fontWeight: 200,
        fontSize: 14, letterSpacing: "0.3em", color: M.text,
      }}>LOAD RUN</div>
      <div style={{ flex: 1 }}/>
      <div style={{ display: "flex", gap: 8 }}>
        <Btn on>ALL</Btn>
        <Btn>IN PROGRESS</Btn>
        <Btn>COMPLETED</Btn>
      </div>
      <div style={{ flex: 1 }}/>
      <Btn ghost>← BACK</Btn>
    </div>

    <div style={{ display: "flex", height: "calc(100% - 57px)" }}>
      {/* Left — run list */}
      <div style={{
        flex: "0 0 420px", borderRight: `1px solid ${M.border}`,
        overflowY: "auto",
      }}>
        <RunListItem
          seed="kepler-9-aurelius" planet="low-O₂ high-pressure"
          diff="STANDARD" tier="T3 · 47% to T4" time="6h 12m"
          status="in-progress" selected
        />
        <RunListItem
          seed="delphi-rift" planet="ice shelf · tidal"
          diff="STANDARD" tier="T2" time="2h 04m"
          status="in-progress"
        />
        <RunListItem
          seed="3hf-burnoff" planet="volcanic · sulfurous"
          diff="INITIATION" tier="T3 · escape" time="4h 38m"
          status="completed"
        />
        <RunListItem
          seed="seed-4729" planet="terrestrial · standard"
          diff="STANDARD" tier="T5 · escape" time="11h 27m"
          status="completed"
        />
        <RunListItem
          seed="amber-cradle" planet="dense atmosphere"
          diff="INITIATION" tier="T1" time="0h 21m"
          status="in-progress"
        />
      </div>

      {/* Right — detail */}
      <div style={{ flex: 1, padding: 28, display: "flex", flexDirection: "column", gap: 18, overflowY: "auto" }}>
        <div style={{ display: "flex", gap: 20, alignItems: "flex-start" }}>
          <PlanetGlyph size={130} hue={205}/>
          <div style={{ flex: 1 }}>
            <div style={{
              fontFamily: "Inter, system-ui, sans-serif", fontWeight: 200,
              fontSize: 26, letterSpacing: "0.15em", color: M.text,
            }}>KEPLER-9-AURELIUS</div>
            <div className="sk-mono" style={{ color: M.soft, marginTop: 4 }}>
              the low-oxygen high-pressure world
            </div>
            <div style={{ display: "flex", gap: 6, marginTop: 10, flexWrap: "wrap" }}>
              <Tag>STANDARD</Tag>
              <Tag color={M.acc}>IN PROGRESS</Tag>
              <Tag color={M.warn}>low-O₂</Tag>
              <Tag color={M.err}>high-pressure</Tag>
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "auto 1fr", gap: "4px 16px", marginTop: 16, fontFamily: "JetBrains Mono, monospace", fontSize: 11 }}>
              <span style={{ color: M.dim }}>run id</span>
              <span style={{ color: M.text }}>aZ3kP9mNqR</span>
              <span style={{ color: M.dim }}>started</span>
              <span style={{ color: M.text }}>2026-05-15 · 5 days ago</span>
              <span style={{ color: M.dim }}>last save</span>
              <span style={{ color: M.text }}>47 minutes ago</span>
              <span style={{ color: M.dim }}>playtime</span>
              <span style={{ color: M.text }}>6h 12m</span>
              <span style={{ color: M.dim }}>tier</span>
              <span style={{ color: M.text }}>T3 (47% toward T4)</span>
            </div>
          </div>
        </div>

        <Divider/>

        <div>
          <div className="sk-label">restore from</div>
          <div style={{ marginTop: 8 }}>
            <CheckpointRow kind="primary" label="primary save (continue)"
                           age="47m ago" selected/>
            <CheckpointRow kind="manual" label="manual · &quot;before refinery rebuild&quot;"
                           age="2h ago"/>
            <CheckpointRow kind="auto"   label="tier_3 · auto"
                           age="3h ago" locked/>
            <CheckpointRow kind="auto"   label="tier_2 · auto"
                           age="1d ago" locked/>
            <CheckpointRow kind="auto"   label="tier_1 · auto"
                           age="2d ago" locked/>
          </div>
        </div>

        <div style={{ flex: 1 }}/>

        <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
          <Btn danger>🗑 DELETE RUN</Btn>
          <div style={{ display: "flex", gap: 8 }}>
            <Btn>EXPORT SEED</Btn>
            <Btn on>LOAD →</Btn>
          </div>
        </div>
      </div>
    </div>
  </Backdrop>
);

// ============================================================
// 04 · SETTINGS — Graphics / Audio / Controls / Gameplay
// ============================================================
const SettingsTab = ({ label, on }) => (
  <div style={{
    padding: "10px 18px", cursor: "pointer",
    borderLeft: `2px solid ${on ? M.acc : "transparent"}`,
    background: on ? "rgba(138,114,170,0.10)" : "transparent",
    fontFamily: "Inter, system-ui, sans-serif",
    fontSize: 13, color: on ? M.text : M.soft,
    fontWeight: on ? 600 : 400, letterSpacing: "0.08em",
  }}>{label}</div>
);

const SettingsRow = ({ label, hint, control }) => (
  <div style={{
    display: "grid", gridTemplateColumns: "260px 1fr",
    gap: 24, padding: "12px 0", alignItems: "center",
    borderBottom: `1px solid ${M.border}`,
  }}>
    <div>
      <div style={{ fontFamily: "Inter, system-ui, sans-serif", fontSize: 13, color: M.text }}>
        {label}
      </div>
      {hint && <div className="sk-mono-xs" style={{ color: M.dim, marginTop: 2 }}>{hint}</div>}
    </div>
    <div>{control}</div>
  </div>
);

const Select = ({ value, options = [value, "..."] }) => (
  <select value={value} readOnly style={{
    background: M.p2, color: M.text, border: `1px solid ${M.border}`,
    padding: "6px 10px", fontFamily: "JetBrains Mono, monospace", fontSize: 11,
    borderRadius: 3, minWidth: 200,
  }}>
    {options.map(o => <option key={o}>{o}</option>)}
  </select>
);

const Slider = ({ pct = 50, val }) => (
  <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
    <div style={{ flex: 1, position: "relative", height: 4, background: M.p3, borderRadius: 2 }}>
      <div style={{ position: "absolute", left: 0, top: 0, height: "100%", width: `${pct}%`, background: M.acc, borderRadius: 2 }}/>
      <div style={{ position: "absolute", left: `calc(${pct}% - 6px)`, top: -4, width: 12, height: 12, background: M.acc, borderRadius: "50%" }}/>
    </div>
    <div className="sk-mono" style={{ color: M.text, minWidth: 40, textAlign: "right" }}>{val || `${pct}%`}</div>
  </div>
);

const Toggle = ({ on }) => (
  <div style={{
    width: 32, height: 18, borderRadius: 9,
    background: on ? M.acc : M.p3, position: "relative",
    border: `1px solid ${M.border}`,
  }}>
    <div style={{
      position: "absolute", top: 1, left: on ? 15 : 1,
      width: 14, height: 14, borderRadius: "50%", background: "#fff",
    }}/>
  </div>
);

const KbdBinding = ({ keys, action, scope, conflict }) => (
  <div style={{
    display: "grid", gridTemplateColumns: "1fr auto auto 80px",
    gap: 14, alignItems: "center", padding: "8px 0",
    borderBottom: `1px solid ${M.border}`,
  }}>
    <div style={{ fontFamily: "Inter, system-ui, sans-serif", fontSize: 12, color: M.text }}>
      {action}
    </div>
    <Tag>{scope}</Tag>
    {conflict ? <Tag color={M.err}>⚠ CONFLICT</Tag> : <span/>}
    <div style={{
      fontFamily: "JetBrains Mono, monospace", fontSize: 11,
      padding: "4px 10px", border: `1px solid ${M.border}`,
      background: M.p2, color: M.text, borderRadius: 3, textAlign: "center",
    }}>{keys}</div>
  </div>
);

const SettingsGraphics = () => (
  <>
    <SettingsRow label="Display mode" control={<Select value="Borderless" options={["Borderless", "Fullscreen", "Windowed"]}/>}/>
    <SettingsRow label="Resolution"   control={<Select value="2560 × 1440" options={["3840 × 2160","2560 × 1440","1920 × 1080"]}/>}/>
    <SettingsRow label="Quality preset" hint="custom overrides any preset below" control={<Select value="High" options={["Ultra","High","Medium","Low","Custom"]}/>}/>
    <SettingsRow label="V-Sync"       control={<Toggle on/>}/>
    <SettingsRow label="Frame limit"  control={<Select value="144" options={["unlimited","240","144","60","30"]}/>}/>
    <SettingsRow label="Field of view" hint="default 80°" control={<Slider pct={62} val="78°"/>}/>
    <SettingsRow label="UI scale"     control={<Slider pct={50} val="1.00×"/>}/>
    <SettingsRow label="HDR"          hint="display must support HDR10" control={<Toggle/>}/>
    <SettingsRow label="Render scale" hint="lower = better perf, softer image" control={<Slider pct={100} val="100%"/>}/>
  </>
);

const SettingsAudio = () => (
  <>
    <SettingsRow label="Master volume" control={<Slider pct={80}/>}/>
    <SettingsRow label="Music"         control={<Slider pct={55}/>}/>
    <SettingsRow label="SFX"           control={<Slider pct={80}/>}/>
    <SettingsRow label="UI"            control={<Slider pct={70}/>}/>
    <SettingsRow label="Ambient"       hint="planetary weather, machine hum" control={<Slider pct={60}/>}/>
    <SettingsRow label="Voice / log"   control={<Slider pct={90}/>}/>
    <SettingsRow label="Mute when unfocused" control={<Toggle on/>}/>
    <SettingsRow label="Output device" control={<Select value="System default" options={["System default","Speakers","Headphones"]}/>}/>
  </>
);

const SettingsControls = () => (
  <>
    <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 12 }}>
      <input placeholder="search bindings…" style={{
        flex: 1, background: M.p2, border: `1px solid ${M.border}`,
        color: M.text, padding: "6px 10px",
        fontFamily: "JetBrains Mono, monospace", fontSize: 11, borderRadius: 3,
      }}/>
      <Btn>RESET ALL</Btn>
    </div>
    <div className="sk-label" style={{ marginTop: 4, marginBottom: 6 }}>MOVEMENT</div>
    <KbdBinding action="forward / back / strafe" scope="local" keys="W A S D"/>
    <KbdBinding action="jump"                    scope="local" keys="SPACE"/>
    <KbdBinding action="sprint"                  scope="local" keys="SHIFT"/>
    <div className="sk-label" style={{ marginTop: 16, marginBottom: 6 }}>MENUS</div>
    <KbdBinding action="terminal"  scope="global" keys="T"/>
    <KbdBinding action="index"     scope="global" keys="I"/>
    <KbdBinding action="planner"   scope="global" keys="TAB"/>
    <KbdBinding action="tech tree" scope="global" keys="Y"/>
    <div className="sk-label" style={{ marginTop: 16, marginBottom: 6 }}>BUILD</div>
    <KbdBinding action="rotate"           scope="build" keys="R / scroll"/>
    <KbdBinding action="reset rotation"   scope="build" keys="SHIFT + R"/>
    <KbdBinding action="remove (hold)"    scope="build" keys="hold R"  conflict/>
    <KbdBinding action="hotbar bank swap" scope="local" keys="ALT + 1-9"/>
  </>
);

const SettingsGameplay = () => (
  <>
    <SettingsRow label="Language" control={<Select value="English (US)" options={["English (US)","Deutsch","Français","日本語","简体中文"]}/>}/>
    <SettingsRow label="Autosave interval" hint="primary save trigger cadence" control={<Slider pct={50} val="60 s"/>}/>
    <SettingsRow label="Pause on focus loss" control={<Toggle on/>}/>
    <SettingsRow label="Tooltip delay" control={<Slider pct={25} val="250 ms"/>}/>
    <SettingsRow label="Colorblind mode" control={<Select value="off" options={["off","deuteranopia","protanopia","tritanopia"]}/>}/>
    <SettingsRow label="Show research pool in HUD" control={<Toggle on/>}/>
    <SettingsRow label="Show machine error toasts" hint="alerts dropdown is always available" control={<Toggle on/>}/>
    <SettingsRow label="Camera shake" control={<Slider pct={75}/>}/>
    <SettingsRow label="Telemetry" hint="anonymous run data, helps tune difficulty" control={<Toggle/>}/>
  </>
);

const Settings = ({ tab = "graphics" }) => (
  <Backdrop>
    <div style={{
      display: "flex", alignItems: "center", padding: "16px 32px",
      borderBottom: `1px solid ${M.border}`,
    }}>
      <div style={{
        fontFamily: "Inter, system-ui, sans-serif", fontWeight: 200,
        fontSize: 14, letterSpacing: "0.3em", color: M.text,
      }}>SETTINGS</div>
      <div style={{ flex: 1 }}/>
      <div className="sk-mono" style={{ color: M.dim }}>changes saved on apply</div>
      <div style={{ width: 16 }}/>
      <Btn ghost>← BACK</Btn>
      <div style={{ width: 8 }}/>
      <Btn on>APPLY</Btn>
    </div>

    <div style={{ display: "flex", height: "calc(100% - 57px)" }}>
      <div style={{
        flex: "0 0 200px", borderRight: `1px solid ${M.border}`,
        padding: "16px 0",
      }}>
        <SettingsTab label="GRAPHICS"  on={tab === "graphics"}/>
        <SettingsTab label="AUDIO"     on={tab === "audio"}/>
        <SettingsTab label="CONTROLS"  on={tab === "controls"}/>
        <SettingsTab label="GAMEPLAY"  on={tab === "gameplay"}/>
      </div>
      <div style={{ flex: 1, padding: 32, overflowY: "auto" }}>
        {tab === "graphics" && <SettingsGraphics/>}
        {tab === "audio"    && <SettingsAudio/>}
        {tab === "controls" && <SettingsControls/>}
        {tab === "gameplay" && <SettingsGameplay/>}
      </div>
    </div>
  </Backdrop>
);

// ============================================================
// 05 · PAUSE MENU — overlay on dimmed game scene
// ============================================================
const FakeWorld = () => (
  // Cheap stand-in for the paused 3D scene under the overlay.
  <div style={{
    position: "absolute", inset: 0,
    background:
      "linear-gradient(180deg, #2c3a4a 0%, #4a5642 55%, #6b5832 100%)",
  }}>
    {/* horizon stripes */}
    <div style={{ position: "absolute", left: 0, right: 0, top: "55%", height: 1, background: "rgba(255,255,255,0.05)" }}/>
    {/* machines */}
    {[
      [180, 360], [320, 380], [440, 360], [540, 400], [820, 350], [960, 380],
    ].map(([x, y], i) => (
      <div key={i} style={{
        position: "absolute", left: x, top: y, width: 70, height: 90,
        background: "#3a4250", border: "1px solid #1a1d21", borderRadius: 2,
        boxShadow: `0 10px 14px rgba(0,0,0,0.4)`,
      }}>
        <div style={{ position: "absolute", top: 6, left: 6, right: 6, height: 14, background: "#5a6370", borderRadius: 1 }}/>
        <div style={{ position: "absolute", bottom: 8, left: 8, right: 8, height: 4, background: i % 2 ? M.ok : M.acc, opacity: 0.7 }}/>
      </div>
    ))}
    {/* cables */}
    <svg style={{ position: "absolute", inset: 0, width: "100%", height: "100%" }}>
      <path d="M 215 450 L 355 470 L 475 450 L 575 490 L 855 440 L 995 470"
            stroke="#3a4250" strokeWidth="3" fill="none"/>
    </svg>
  </div>
);

const Pause = ({ confirm = false }) => (
  <Backdrop dim>
    <FakeWorld/>
    <div style={{ position: "absolute", inset: 0, background: "rgba(20,22,26,0.65)" }}/>

    {/* PAUSED label */}
    <div style={{
      position: "absolute", top: 80, left: 0, right: 0, textAlign: "center",
      fontFamily: "Inter, system-ui, sans-serif", fontWeight: 200,
      fontSize: 12, letterSpacing: "0.5em", color: M.dim,
    }}>· PAUSED ·</div>

    {/* center column */}
    <div style={{
      position: "absolute", left: "50%", top: "50%",
      transform: "translate(-50%, -50%)", width: 360,
    }}>
      <MenuRow label="RESUME"            kbd="ESC" on accent/>
      <MenuRow label="CHECKPOINT"        hint="manual save slot"/>
      <MenuRow label="SAVE & QUIT TO MENU"/>
      <Divider/>
      <MenuRow label="RUN SUMMARY"       hint="objectives · planet"/>
      <MenuRow label="SETTINGS"/>
      <Divider/>
      <MenuRow label="QUIT TO DESKTOP"/>
    </div>

    {/* right side — run at a glance */}
    <div style={{
      position: "absolute", right: 60, top: "50%", transform: "translateY(-50%)",
      width: 340, padding: 20,
      border: `1px solid ${M.border}`, background: "rgba(34,38,43,0.92)",
      borderRadius: 4,
    }}>
      <div className="sk-label">run</div>
      <div className="sk-h" style={{ marginTop: 4 }}>kepler-9-aurelius</div>
      <div className="sk-mono" style={{ color: M.soft, marginTop: 2 }}>
        STANDARD · T3 → T4 · 6h 12m
      </div>
      <Divider/>
      <div className="sk-label">current objective</div>
      <div className="sk-mono" style={{ color: M.text, marginTop: 4, lineHeight: 1.6 }}>
        unlock <span style={{ color: M.acc }}>electrolysis chain</span>
      </div>
      <div className="sk-mono" style={{ color: M.dim, marginTop: 2 }}>
        progress 47% · 3 of 7 prerequisites
      </div>
      <Divider/>
      <div className="sk-label">last save</div>
      <div className="sk-mono" style={{ color: M.text, marginTop: 4 }}>
        primary · 47s ago (auto)
      </div>
      <div className="sk-mono" style={{ color: M.dim }}>
        manual · 2h ago
      </div>
      <Divider/>
      <div className="sk-label">alerts</div>
      <div style={{ display: "flex", gap: 6, marginTop: 4 }}>
        <Tag color={M.err}>1 err</Tag>
        <Tag color={M.warn}>1 warn</Tag>
      </div>
    </div>

    {/* footer */}
    <div style={{
      position: "absolute", bottom: 24, left: 0, right: 0, textAlign: "center",
      fontFamily: "JetBrains Mono, monospace", fontSize: 10, color: M.dim,
    }}>↑↓ navigate · ↵ select · ESC resume</div>

    {confirm && <CheckpointConfirm/>}
  </Backdrop>
);

// ============================================================
// 06 · SAVE — checkpoint confirm modal (overwrite the one slot)
// ============================================================
const CheckpointConfirm = () => (
  <div style={{
    position: "absolute", inset: 0,
    background: "rgba(0,0,0,0.55)",
    display: "flex", alignItems: "center", justifyContent: "center",
  }}>
    <div style={{
      width: 460, padding: 24, background: M.p1,
      border: `1px solid ${M.border}`, borderRadius: 4,
      boxShadow: "0 20px 60px rgba(0,0,0,0.55)",
    }}>
      <div className="sk-label">manual checkpoint</div>
      <div className="sk-h" style={{ marginTop: 4 }}>Overwrite checkpoint slot?</div>

      <div style={{
        marginTop: 16, padding: 12, background: M.p2,
        border: `1px solid ${M.border}`, borderRadius: 3,
      }}>
        <div className="sk-mono-xs" style={{ color: M.dim }}>EXISTING</div>
        <div className="sk-mono" style={{ color: M.text, marginTop: 2 }}>
          "before refinery rebuild"
        </div>
        <div className="sk-mono-xs" style={{ color: M.dim, marginTop: 2 }}>
          2h ago · tier 3 · 5h 42m playtime
        </div>
      </div>
      <div style={{ display: "flex", justifyContent: "center", color: M.dim, padding: "6px 0" }}>↓</div>
      <div style={{
        padding: 12, background: M.p2,
        border: `1px solid ${M.acc}`, borderRadius: 3,
      }}>
        <div className="sk-mono-xs" style={{ color: M.acc }}>NEW</div>
        <input placeholder="label (optional)" style={{
          marginTop: 2, width: "100%", background: "transparent",
          border: "none", outline: "none", color: M.text,
          fontFamily: "JetBrains Mono, monospace", fontSize: 12,
        }} defaultValue="after T3 unlock"/>
        <div className="sk-mono-xs" style={{ color: M.dim, marginTop: 2 }}>
          now · tier 3 · 6h 12m playtime
        </div>
      </div>

      <div style={{
        marginTop: 14, padding: 10,
        border: `1px dashed ${M.warn}55`, borderRadius: 3,
        fontFamily: "JetBrains Mono, monospace", fontSize: 10,
        color: M.soft, lineHeight: 1.5,
      }}>
        ⓘ One manual slot per run. The previous label cannot be recovered.
        Auto-checkpoints (tier_3, tier_2, tier_1) are unaffected.
      </div>

      <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 18 }}>
        <Btn ghost>CANCEL</Btn>
        <Btn on>OVERWRITE</Btn>
      </div>
    </div>
  </div>
);

// "Saved" toast variant — shown briefly after a save completes
const SaveToast = () => (
  <Backdrop dim>
    <FakeWorld/>
    <div style={{ position: "absolute", inset: 0, background: "rgba(20,22,26,0.4)" }}/>
    <div style={{
      position: "absolute", top: 80, left: "50%", transform: "translateX(-50%)",
      padding: "10px 20px", background: M.p1,
      border: `1px solid ${M.ok}`, borderRadius: 3,
      display: "flex", alignItems: "center", gap: 12,
      boxShadow: "0 10px 30px rgba(0,0,0,0.6)",
    }}>
      <div style={{
        width: 22, height: 22, borderRadius: "50%",
        background: "rgba(74,158,106,0.15)", color: M.ok,
        display: "flex", alignItems: "center", justifyContent: "center",
        fontSize: 12,
      }}>✓</div>
      <div>
        <div className="sk-mono" style={{ color: M.text }}>
          checkpoint saved · "after T3 unlock"
        </div>
        <div className="sk-mono-xs" style={{ color: M.dim }}>
          manual slot · 6h 12m playtime · 384 KB
        </div>
      </div>
    </div>
  </Backdrop>
);
