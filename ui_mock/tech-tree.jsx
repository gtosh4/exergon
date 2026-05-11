// tech-tree.jsx — exergon/ui · procedural tech tree wireframes
//
// Five variations on a "fog-of-war" tech tree. All share:
//   - the same fictitious tech graph (TT data model)
//   - three knowledge tiers per node:
//       T1  KNOWN     — node exists, params hidden       (silhouette / redacted)
//       T2  PARTIAL   — broad params visible, ranges     (some text, some bars)
//       T3  REVEALED  — full recipe, buildable           (numbers, glyphs)
//   - milestone nodes that gate tiers
//   - a shared reveal overlay (variation 06)
//
// Tweaks read from window.__ttTweaks: density, vibe, fogStyle, milestoneStyle,
// showLockedEdges.

// ════════════════════════════════════════════════════════════════════════════
// DATA — fictitious "exergon" tech graph
// ════════════════════════════════════════════════════════════════════════════
const TT = (()=>{
  // tech glyphs are stylised, not drawn from any IRL franchise
  const T = (id, tier, name, tag, ms, glyph) => ({id, tier, name, tag, ms, glyph});
  const techs = [
    // ── T1 Landfall (8) — from tech-tree-design.md §6 ──────────────────────
    T("stone-furnace",  1, "Stone Furnace",      "smelt",    false, "▣"),
    T("ore-crusher",    1, "Ore Crusher",         "smelt",    false, "◇"),
    T("basic-miner",    1, "Basic Miner",         "extract",  false, "⌇"),
    T("combustion-gen", 1, "Combustion Gen",      "power",    false, "≋"),
    T("solar-array",    1, "Solar Array",         "power",    false, "◎"),
    T("field-analyzer", 1, "Field Analyzer",      "science",  false, "⌬"),
    T("net-node",       1, "Network Node",        "logistics",false, "⋮⋮"),
    T("land-drone",     1, "Land Drone Mk1",      "explore",  false, "▷"),

    // ── T2 Roots (12 = 11 + 1ms) ───────────────────────────────────────────
    T("amp-drill",      2, "Amp Drill",           "extract",  false, "⌖"),
    T("layer-drill",    2, "Layer Drill",         "extract",  false, "⌗"),
    T("ore-washer",     2, "Ore Washer",          "smelt",    false, "▤"),
    T("wire-draw",      2, "Wire Draw",           "smelt",    false, "〰"),
    T("thermal-boil",   2, "Thermal Boiler",      "power",    false, "≋"),
    T("chan-relay",     2, "Channel Relay",       "logistics",false, "⋯"),
    T("auto-sorter",    2, "Auto-Sorter",         "logistics",false, "⊞"),
    T("sample-lab",     2, "Sample Lab",          "science",  false, "⌽"),
    T("alien-sampler",  2, "Alien Sampler",       "science",  false, "◉"),
    T("electro-cell",   2, "Electro Cell",        "process",  false, "⊕"),
    T("amph-drone",     2, "Amphibious Drone",    "explore",  false, "▶"),
    T("t2-gate",        2, "Refined Output",      "smelt",    true,  "✦"),

    // ── T3 Contact (16 = 15 + 1ms) ─────────────────────────────────────────
    T("pressure-vessel",3, "Pressure Vessel",     "process",  false, "◈"),
    T("catalytic-bed",  3, "Catalytic Bed",       "process",  false, "⊗"),
    T("deep-drill",     3, "Deep Drill",          "extract",  false, "⌇"),
    T("alloy-forge",    3, "Alloy Forge",         "smelt",    false, "▥"),
    T("circuit-press",  3, "Circuit Press",       "fab",      false, "▦"),
    T("power-cell-t2",  3, "Power Cell T2",       "power",    false, "⚡"),
    T("fly-drone",      3, "Flying Drone",        "explore",  false, "△"),
    T("net-seg",        3, "Net Segment",         "logistics",false, "⊡"),
    T("alien-lab",      3, "Alien Lab",           "science",  false, "◊"),
    T("thermal-proc",   3, "Thermal Proc",        "process",  false, "⊙"),
    T("arc-smelter",    3, "Arc Smelter",         "smelt",    false, "▧"),
    T("sub-network",    3, "Sub-Network",         "logistics",false, "⊠"),
    T("alien-mat-t1",   3, "Alien Material T1",   "science",  false, "✧"),
    T("fab-bench",      3, "Fab Bench T1",        "fab",      false, "◐"),
    T("route-ctrl",     3, "Route Controller",    "logistics",false, "⊞"),
    T("t3-gate",        3, "Alien Contact",       "explore",  true,  "✺"),

    // ── T4 Reach (20 = 19 + 1ms) ───────────────────────────────────────────
    T("orbital-craft",  4, "Orbital Craft",       "fab",      false, "▩"),
    T("space-drone",    4, "Space Drone",         "explore",  false, "▲"),
    T("plasma-torch",   4, "Plasma Torch",        "process",  false, "☄"),
    T("reactor-t2",     4, "Reactor T2",          "power",    false, "⊟"),
    T("power-dist-t2",  4, "Power Dist T2",       "power",    false, "◎"),
    T("byproduct-sep",  4, "Byproduct Sep",       "process",  false, "⊘"),
    T("alloy-press",    4, "Alloy Press",         "smelt",    false, "▨"),
    T("crystal-grow",   4, "Crystal Grower",      "process",  false, "◑"),
    T("exo-scanner",    4, "Exo Scanner",         "science",  false, "◐"),
    T("alien-mat-t2",   4, "Alien Material T2",   "science",  false, "✧"),
    T("flux-conduit",   4, "Flux Conduit",        "process",  false, "〰"),
    T("fab-bench-t2",   4, "Fab Bench T2",        "fab",      false, "◒"),
    T("orbit-relay",    4, "Orbital Relay",       "logistics",false, "⊞"),
    T("deep-miner-t4",  4, "Deep Miner T4",       "extract",  false, "⌖"),
    T("sample-lab-t3",  4, "Sample Lab T3",       "science",  false, "⌽"),
    T("geothermal",     4, "Geothermal Tap",      "power",    false, "≋"),
    T("alien-circuit",  4, "Alien Circuit",       "fab",      false, "◓"),
    T("net-ctrl-t4",    4, "Net Ctrl T4",         "logistics",false, "⋯"),
    T("exo-refiner-t4", 4, "Exo Refiner T4",      "process",  false, "⊕"),
    T("t4-gate",        4, "Orbital Flight",      "explore",  true,  "✰"),

    // ── T5 Salvage (15 = 14 + 1ms) ─────────────────────────────────────────
    T("hull-press",     5, "Hull Press",          "fab",      false, "▩"),
    T("nav-comp",       5, "Nav Computer",        "fab",      false, "▦"),
    T("alien-fuel-syn", 5, "Alien Fuel Synth",    "process",  false, "⊛"),
    T("orbit-fab-t2",   5, "Orbital Fab T2",      "fab",      false, "◒"),
    T("net-arch-t5",    5, "Net Arch T5",         "logistics",false, "⊠"),
    T("deep-extr-t5",   5, "Deep Extract T5",     "extract",  false, "⌗"),
    T("power-t3",       5, "Power Array T3",      "power",    false, "⊟"),
    T("vessel-scan",    5, "Vessel Scanner",      "science",  false, "◉"),
    T("cryo-store",     5, "Cryo Storage",        "fab",      false, "◔"),
    T("ion-drive",      5, "Ion Drive T1",        "fab",      false, "✺"),
    T("life-support",   5, "Life Support",        "fab",      false, "◕"),
    T("alien-mat-t3",   5, "Alien Material T3",   "science",  false, "⌖"),
    T("fab-data",       5, "Fab Data Store",      "science",  false, "⌬"),
    T("exo-alloy-t5",   5, "Exo Alloy T5",        "smelt",    false, "▥"),
    T("t5-gate",        5, "Alien Vessel",        "fab",      true,  "✯"),

    // ── T6 Traverse (22 = 21 + 1ms) ────────────────────────────────────────
    T("outer-probe",    6, "Outer Probe",         "explore",  false, "▷"),
    T("exotic-chain",   6, "Exotic Chain T1",     "process",  false, "⊕"),
    T("ftl-prep-t6",    6, "FTL Prep T6",         "fab",      false, "◊"),
    T("deep-scan-t6",   6, "Deep Scanner T6",     "science",  false, "◉"),
    T("power-t4-a",     6, "Power T4 Array",      "power",    false, "⚡"),
    T("power-t4-b",     6, "Power T4 Store",      "power",    false, "◎"),
    T("exotic-mat-t4",  6, "Exotic Mat T4",       "science",  false, "✧"),
    T("alloy-t6",       6, "T6 Alloy",            "smelt",    false, "▨"),
    T("net-t6",         6, "Network T6",          "logistics",false, "⊞"),
    T("orbit-mine-t6",  6, "Orbital Mine T6",     "extract",  false, "⌖"),
    T("plasma-proc-t6", 6, "Plasma Proc T6",      "process",  false, "☄"),
    T("fab-bench-t3",   6, "Fab Bench T3",        "fab",      false, "▦"),
    T("exotic-fuel-t6", 6, "Exotic Fuel T6",      "process",  false, "◈"),
    T("relay-comp-t6",  6, "Relay Computer",      "fab",      false, "◓"),
    T("sample-lab-t5",  6, "Sample Lab T5",       "science",  false, "⌽"),
    T("deep-drone-t6",  6, "Deep Space Drone",    "explore",  false, "▲"),
    T("crystal-syn-t6", 6, "Crystal Synth T6",    "process",  false, "⊙"),
    T("power-dist-t6",  6, "Power Dist T6",       "power",    false, "⊟"),
    T("net-auto-t6",    6, "Net Auto T6",         "logistics",false, "⊡"),
    T("ext-rig-t6",     6, "Ext Rig T6",          "extract",  false, "⌗"),
    T("exotic-synth-t6",6, "Exotic Synth T6",     "process",  false, "⊗"),
    T("t6-gate",        6, "Outer System",        "explore",  true,  "✰"),

    // ── T7 Interface (16 = 15 + 1ms) ───────────────────────────────────────
    T("mega-scan",      7, "Megastruct Scan",     "science",  false, "◉"),
    T("relay-frag",     7, "Relay Fragments",     "fab",      false, "◊"),
    T("ftl-theory-t7",  7, "FTL Theory T7",       "science",  false, "⌬"),
    T("exotic-mat-t5",  7, "Exotic Mat T5",       "science",  false, "✧"),
    T("power-t5-a",     7, "Power T5 Grid",       "power",    false, "⚡"),
    T("power-t5-b",     7, "Power T5 Dist",       "power",    false, "◎"),
    T("fab-bench-t4",   7, "Fab Bench T4",        "fab",      false, "▦"),
    T("net-t7",         7, "Network T7",          "logistics",false, "⊞"),
    T("deep-mine-t7",   7, "Deep Mine T7",        "extract",  false, "⌇"),
    T("exotic-syn-t7",  7, "Exotic Synth T7",     "process",  false, "⊗"),
    T("archive-ext",    7, "Archive Extractor",   "science",  false, "⌖"),
    T("ftl-drive-t7",   7, "FTL Drive Comp",      "fab",      false, "✺"),
    T("exotic-alloy-t7",7, "Exotic Alloy T7",     "smelt",    false, "▩"),
    T("mega-drone",     7, "Megastruct Drone",    "explore",  false, "▲"),
    T("relay-power",    7, "Relay Power Sys",     "power",    false, "⊟"),
    T("t7-gate",        7, "Alien Interface",     "explore",  true,  "✯"),

    // ── T8 Revelation (24 = 23 + 1ms) ──────────────────────────────────────
    T("exotic-syn-t8a", 8, "Exotic Synth T8A",   "process",  false, "⊕"),
    T("exotic-syn-t8b", 8, "Exotic Synth T8B",   "process",  false, "◈"),
    T("exotic-syn-t8c", 8, "Exotic Synth T8C",   "process",  false, "⊙"),
    T("exotic-mat-t6a", 8, "Exotic Mat T6A",     "science",  false, "✧"),
    T("exotic-mat-t6b", 8, "Exotic Mat T6B",     "science",  false, "◊"),
    T("exotic-mat-t6c", 8, "Exotic Mat T6C",     "science",  false, "◉"),
    T("machine-t8a",    8, "Exotic Machine T8A", "fab",      false, "▦"),
    T("machine-t8b",    8, "Exotic Machine T8B", "fab",      false, "◓"),
    T("machine-t8c",    8, "Exotic Machine T8C", "fab",      false, "◒"),
    T("power-t6-a",     8, "Power T6 Gen",       "power",    false, "⚡"),
    T("power-t6-b",     8, "Power T6 Store",     "power",    false, "◎"),
    T("power-t6-c",     8, "Power T6 Dist",      "power",    false, "⊟"),
    T("net-t8",         8, "Network T8",         "logistics",false, "⊞"),
    T("deep-mine-t8",   8, "Deep Mine T8",       "extract",  false, "⌖"),
    T("exotic-alloy-t8",8, "Exotic Alloy T8",    "smelt",    false, "▥"),
    T("ftl-theory-t8",  8, "FTL Theory T8",      "science",  false, "⌬"),
    T("exotic-chain-t8",8, "Exotic Chain T8",    "process",  false, "⊗"),
    T("fab-bench-t5",   8, "Fab Bench T5",       "fab",      false, "◐"),
    T("survey-t8",      8, "Deep Survey T8",     "explore",  false, "△"),
    T("net-auto-t8",    8, "Net Auto T8",        "logistics",false, "⊡"),
    T("smelt-t8",       8, "Exotic Smelt T8",    "smelt",    false, "▧"),
    T("extract-t8",     8, "Deep Extract T8",    "extract",  false, "⌗"),
    T("crystal-t8",     8, "Crystal Lattice",    "process",  false, "◑"),
    T("t8-gate",        8, "Exotic Synthesis",   "process",  true,  "❈"),

    // ── T9 Forge (26 = 25 + 1ms) ───────────────────────────────────────────
    T("ftl-engine",     9, "FTL Engine Comp",    "fab",      false, "✺"),
    T("ftl-drive-core", 9, "FTL Drive Core",     "fab",      false, "◊"),
    T("ftl-field-gen",  9, "FTL Field Gen",      "fab",      false, "◓"),
    T("ftl-nav",        9, "FTL Nav Module",     "fab",      false, "▦"),
    T("ftl-mat-a",      9, "FTL Material A",     "process",  false, "⊕"),
    T("ftl-mat-b",      9, "FTL Material B",     "process",  false, "◈"),
    T("ftl-mat-c",      9, "FTL Material C",     "process",  false, "⊗"),
    T("power-ftl-a",    9, "FTL Power Gen",      "power",    false, "⚡"),
    T("power-ftl-b",    9, "FTL Power Store",    "power",    false, "◎"),
    T("power-ftl-c",    9, "FTL Power Dist",     "power",    false, "⊟"),
    T("power-ftl-d",    9, "FTL Power Grid",     "power",    false, "≋"),
    T("exotic-t9a",     9, "Exotic Proc T9A",    "process",  false, "⊙"),
    T("exotic-t9b",     9, "Exotic Proc T9B",    "process",  false, "◑"),
    T("exotic-t9c",     9, "Exotic Proc T9C",    "process",  false, "⊛"),
    T("exotic-mat-t7a", 9, "Exotic Mat T7A",     "science",  false, "✧"),
    T("exotic-mat-t7b", 9, "Exotic Mat T7B",     "science",  false, "◐"),
    T("forge-fab-t9a",  9, "Forge Fab T9A",      "fab",      false, "▩"),
    T("forge-fab-t9b",  9, "Forge Fab T9B",      "fab",      false, "◒"),
    T("net-t9",         9, "Network T9",         "logistics",false, "⊞"),
    T("net-auto-t9",    9, "Net Auto T9",        "logistics",false, "⊡"),
    T("mine-t9",        9, "Mine T9",            "extract",  false, "⌇"),
    T("smelt-t9",       9, "Forge Smelt T9",     "smelt",    false, "▥"),
    T("alloy-t9",       9, "FTL Alloy T9",       "smelt",    false, "▨"),
    T("science-t9",     9, "FTL Science T9",     "science",  false, "⌬"),
    T("survey-t9",      9, "Survey T9",          "explore",  false, "▷"),
    T("t9-gate",        9, "FTL Components",     "fab",      true,  "❉"),

    // ── T10 Transcendence (20 = 19 + 1ms) ──────────────────────────────────
    T("engine-t10",    10, "Ship Engine",        "fab",      false, "✺"),
    T("engine-sys-t10",10, "Engine Systems",     "fab",      false, "◊"),
    T("ftl-drive-t10", 10, "FTL Drive Core",     "fab",      false, "▦"),
    T("ftl-shell-t10", 10, "FTL Drive Shell",    "fab",      false, "◒"),
    T("reactor-t10",   10, "Ship Reactor",       "power",    false, "⚡"),
    T("reactor-core",  10, "Reactor Core",       "power",    false, "◎"),
    T("shield-t10",    10, "Shielding T10",      "fab",      false, "◓"),
    T("shield-matrix", 10, "Shield Matrix",      "fab",      false, "◐"),
    T("power-t10",     10, "Final Power Sys",    "power",    false, "≋"),
    T("proc-t10a",     10, "Trans Process A",    "process",  false, "⊕"),
    T("proc-t10b",     10, "Trans Process B",    "process",  false, "◈"),
    T("science-t10",   10, "Trans Science",      "science",  false, "✧"),
    T("net-t10",       10, "Network T10",        "logistics",false, "⊞"),
    T("mine-t10",      10, "Mine T10",           "extract",  false, "⌖"),
    T("smelt-t10",     10, "Trans Smelt",        "smelt",    false, "▥"),
    T("survey-t10",    10, "Final Survey",       "explore",  false, "▲"),
    T("assemble-a",    10, "Assembly A",         "fab",      false, "▩"),
    T("assemble-b",    10, "Assembly B",         "fab",      false, "▨"),
    T("assemble-final",10, "Final Assembly",     "fab",      false, "✦"),
    T("t10-gate",      10, "Transcendence",      "fab",      true,  "❊"),
  ];

  // edges (prerequisite → unlocks)
  // cross-tier edges to gate nodes show as exit/entry bridge cards
  // cross-tier non-gate edges show as port stubs at page margins
  const edges = [
    // T1 internal (from design doc §6)
    ["stone-furnace","ore-crusher"], ["stone-furnace","combustion-gen"],
    ["stone-furnace","net-node"],    ["basic-miner","land-drone"],
    // T1 → T2 gate (shows as exit card on T1 page)
    ["land-drone","t2-gate"], ["net-node","t2-gate"], ["field-analyzer","t2-gate"],
    // T2 gate → T2 nodes (shows as entry paths on T2 page)
    ["t2-gate","amp-drill"], ["t2-gate","alien-sampler"], ["t2-gate","electro-cell"],
    // T2 internal
    ["amp-drill","layer-drill"], ["electro-cell","layer-drill"],
    ["alien-sampler","sample-lab"],
    // T1→T2 cross-tier stubs
    ["ore-crusher","ore-washer"],   ["stone-furnace","wire-draw"],
    ["combustion-gen","thermal-boil"], ["net-node","chan-relay"],
    ["net-node","auto-sorter"],     ["field-analyzer","sample-lab"],
    ["land-drone","amph-drone"],
    // T2 → T3 gate
    ["alien-sampler","t3-gate"], ["amph-drone","t3-gate"], ["sample-lab","t3-gate"],
    // T3 gate → T3 nodes
    ["t3-gate","pressure-vessel"], ["t3-gate","fly-drone"], ["t3-gate","alien-lab"],
    // T3 internal
    ["pressure-vessel","catalytic-bed"], ["catalytic-bed","thermal-proc"],
    ["alloy-forge","arc-smelter"],  ["circuit-press","fab-bench"],
    ["sub-network","route-ctrl"],   ["sub-network","net-seg"],
    ["alien-lab","alien-mat-t1"],   ["fly-drone","deep-drill"],
    // T2→T3 cross-tier stubs
    ["layer-drill","deep-drill"],   ["wire-draw","alloy-forge"],
    ["thermal-boil","power-cell-t2"], ["chan-relay","sub-network"],
    ["auto-sorter","net-seg"],
    // T3 → T4 gate
    ["alien-mat-t1","t4-gate"], ["fly-drone","t4-gate"], ["arc-smelter","t4-gate"],
    // T4 gate → T4 nodes
    ["t4-gate","space-drone"], ["t4-gate","orbital-craft"], ["t4-gate","plasma-torch"],
    // T4 internal
    ["space-drone","exo-scanner"],  ["orbital-craft","orbit-relay"],
    ["plasma-torch","byproduct-sep"], ["byproduct-sep","crystal-grow"],
    ["reactor-t2","power-dist-t2"], ["fab-bench-t2","alien-circuit"],
    // T3→T4 cross-tier stubs
    ["circuit-press","fab-bench-t2"], ["alien-mat-t1","alien-mat-t2"],
    ["power-cell-t2","reactor-t2"], ["route-ctrl","net-ctrl-t4"],
    ["deep-drill","deep-miner-t4"],
    // T4 → T5 gate
    ["orbital-craft","t5-gate"], ["space-drone","t5-gate"], ["reactor-t2","t5-gate"],
    // T5 gate → T5 nodes
    ["t5-gate","hull-press"], ["t5-gate","nav-comp"], ["t5-gate","alien-fuel-syn"],
    // T5 internal
    ["hull-press","orbit-fab-t2"],  ["nav-comp","cryo-store"],
    ["alien-fuel-syn","ion-drive"], ["ion-drive","life-support"],
    // T4→T5 cross-tier stubs
    ["alien-circuit","nav-comp"],   ["exo-refiner-t4","alien-fuel-syn"],
    ["alien-mat-t2","alien-mat-t3"], ["orbit-relay","net-arch-t5"],
    // T5 → T6 gate
    ["ion-drive","t6-gate"], ["orbit-fab-t2","t6-gate"], ["vessel-scan","t6-gate"],
    // T6 gate → T6 nodes
    ["t6-gate","outer-probe"], ["t6-gate","exotic-chain"],
    // T6 internal
    ["outer-probe","deep-drone-t6"], ["exotic-chain","exotic-fuel-t6"],
    ["plasma-proc-t6","crystal-syn-t6"],
    // T6 → T7 gate
    ["outer-probe","t7-gate"], ["exotic-fuel-t6","t7-gate"],
    // T7 gate → T7 nodes
    ["t7-gate","mega-scan"], ["t7-gate","relay-frag"],
    // T7 internal
    ["mega-scan","ftl-theory-t7"],  ["relay-frag","ftl-drive-t7"],
    ["archive-ext","ftl-theory-t7"],
    // T7 → T8 gate
    ["ftl-theory-t7","t8-gate"], ["exotic-mat-t5","t8-gate"],
    // T8 gate → T8 nodes
    ["t8-gate","exotic-syn-t8a"], ["t8-gate","machine-t8a"],
    // T8 internal
    ["exotic-syn-t8a","exotic-syn-t8b"], ["exotic-syn-t8b","exotic-syn-t8c"],
    ["machine-t8a","machine-t8b"],
    // T8 → T9 gate
    ["exotic-syn-t8c","t9-gate"], ["ftl-theory-t8","t9-gate"],
    // T9 gate → T9 nodes
    ["t9-gate","ftl-engine"], ["t9-gate","ftl-drive-core"],
    // T9 internal
    ["ftl-engine","ftl-field-gen"], ["ftl-drive-core","ftl-nav"],
    ["ftl-mat-a","ftl-mat-b"],
    // T9 → T10 gate
    ["ftl-engine","t10-gate"], ["ftl-drive-core","t10-gate"], ["power-ftl-a","t10-gate"],
    // T10 gate → T10 nodes
    ["t10-gate","engine-t10"], ["t10-gate","ftl-drive-t10"],
    ["t10-gate","reactor-t10"], ["t10-gate","shield-t10"],
    // T10 internal
    ["engine-t10","engine-sys-t10"],     ["engine-sys-t10","assemble-a"],
    ["ftl-drive-t10","ftl-shell-t10"],   ["ftl-shell-t10","assemble-b"],
    ["reactor-t10","reactor-core"],
    ["shield-t10","shield-matrix"],
    ["assemble-a","assemble-b"],         ["assemble-b","assemble-final"],

    // Additional in-page edges — extend chains for graph depth variation
    ["ore-washer","wire-draw"],          // T2: ore wash → wire draw chain
    ["amph-drone","alien-sampler"],      // T2: drone access enables alien sampling
    ["alloy-forge","circuit-press"],     // T3: alloy materials → circuit fab
    ["deep-miner-t4","alloy-press"],     // T4: deep ore → alloy production
    ["geothermal","reactor-t2"],         // T4: geothermal powers reactor upgrade
    ["alien-mat-t2","alien-circuit"],    // T4: alien material → alien circuit
    ["exo-alloy-t5","hull-press"],       // T5: exotic alloy into hull construction
    ["alien-mat-t3","alien-fuel-syn"],   // T5: alien material → fuel synthesis
    ["orbit-mine-t6","alloy-t6"],        // T6: orbital mining feeds T6 alloy
    ["alloy-t6","fab-bench-t3"],         // T6: T6 alloy → fab bench
    ["deep-scan-t6","exotic-mat-t4"],    // T6: deep scanner discovers exotic mat
    ["exotic-mat-t4","exotic-fuel-t6"],  // T6: exotic mat → exotic fuel
    ["power-t4-a","power-t4-b"],         // T6: power gen → power storage
    ["net-t6","net-auto-t6"],            // T6: basic → automated network
    ["mega-scan","archive-ext"],         // T7: megastruct scan → archive extraction
    ["exotic-mat-t5","exotic-syn-t7"],   // T7: exotic mat → exotic synthesis
    ["power-t5-a","power-t5-b"],         // T7: power gen → power dist
    ["relay-frag","relay-power"],        // T7: relay fragments → relay power sys
    ["exotic-mat-t6a","exotic-syn-t8a"], // T8: exotic mat feeds synth chain A
    ["exotic-mat-t6b","exotic-syn-t8b"], // T8: exotic mat feeds synth chain B
    ["machine-t8b","machine-t8c"],       // T8: machine upgrade chain
    ["ftl-theory-t8","exotic-chain-t8"], // T8: FTL theory guides exotic chain
    ["power-t6-a","power-t6-b"],         // T8: power gen → power storage
    ["power-t6-b","power-t6-c"],         // T8: power storage → power dist
    ["exotic-alloy-t8","fab-bench-t5"],  // T8: exotic alloy → fab bench
    ["deep-mine-t8","extract-t8"],       // T8: deep mine → extraction chain
    ["net-t8","net-auto-t8"],            // T8: basic → automated network
    ["ftl-mat-a","ftl-engine"],          // T9: FTL mat A → engine component
    ["ftl-mat-a","ftl-mat-c"],           // T9: mat A enables mat C
    ["ftl-mat-b","ftl-drive-core"],      // T9: FTL mat B → drive core
    ["exotic-t9a","exotic-t9b"],         // T9: exotic processing chain
    ["exotic-t9b","exotic-t9c"],
    ["forge-fab-t9a","forge-fab-t9b"],   // T9: forge fabrication chain
    ["power-ftl-a","power-ftl-b"],       // T9: FTL power chain
    ["power-ftl-b","power-ftl-c"],
    ["power-ftl-c","power-ftl-d"],
    ["science-t9","exotic-mat-t7a"],     // T9: science → exotic materials
    ["exotic-mat-t7a","exotic-mat-t7b"],
    ["net-t9","net-auto-t9"],            // T9: basic → automated network
    ["mine-t9","smelt-t9"],              // T9: mining → forge smelting chain
    ["smelt-t9","alloy-t9"],
    ["science-t10","proc-t10a"],         // T10: science drives processing
    ["proc-t10a","proc-t10b"],
    ["proc-t10a","engine-t10"],          // T10: processing feeds engine
    ["proc-t10b","ftl-drive-t10"],       // T10: processing feeds FTL drive
    ["mine-t10","smelt-t10"],            // T10: mining → smelting
    ["smelt-t10","assemble-a"],          // T10: smelted materials → assembly

    // T1: create deeper chain through power and science nodes
    ["stone-furnace","solar-array"],     // stone smelting → solar panel materials
    ["solar-array","field-analyzer"],    // solar power enables field analysis
    // T2: chain logistics/process orphans into a sequence
    ["ore-washer","thermal-boil"],       // ore washing generates thermal byproduct
    ["thermal-boil","chan-relay"],        // thermal power feeds channel relay
    ["chan-relay","auto-sorter"],         // relay enables automated sorting
    // T4: chain orphan nodes behind existing depth-1/2 nodes
    ["deep-miner-t4","exo-refiner-t4"], // deep miner feeds refiner
    ["reactor-t2","flux-conduit"],       // reactor enables flux conduit
    ["orbit-relay","net-ctrl-t4"],       // orbital relay → network control
    ["exo-scanner","sample-lab-t3"],     // scanner data → sample lab analysis
    // T5: spread orphan nodes into deeper columns
    ["exo-alloy-t5","deep-extr-t5"],    // exo alloy data drives deep extraction
    ["nav-comp","fab-data"],             // nav systems → fabrication data
    ["orbit-fab-t2","vessel-scan"],      // orbital fab → vessel scanner
    ["orbit-fab-t2","net-arch-t5"],      // orbital fab → net architecture
    ["deep-extr-t5","power-t3"],         // deep extraction → power array
    // T6: break up the 12-node depth=0 cluster
    ["exotic-chain","exotic-synth-t6"], // exotic chain drives exotic synth
    ["power-t4-b","power-dist-t6"],      // power storage → power distribution
    ["net-auto-t6","relay-comp-t6"],     // automated net → relay computer
    ["deep-scan-t6","sample-lab-t5"],    // deep scanner → sample lab T5
    ["fab-bench-t3","ftl-prep-t6"],      // T3 fab bench → FTL prep
    ["orbit-mine-t6","ext-rig-t6"],      // orbital mine → extraction rig
    ["exotic-synth-t6","plasma-proc-t6"],// exotic synth → plasma processing
    // T8: chain 12 orphan depth=0 nodes into structured sequences
    ["exotic-mat-t6a","exotic-mat-t6b"], // exotic material progression chain
    ["exotic-mat-t6b","exotic-mat-t6c"],
    ["deep-mine-t8","smelt-t8"],          // deep mine ore feeds smelting
    ["smelt-t8","exotic-alloy-t8"],       // smelted ore → exotic alloy
    ["ftl-theory-t8","survey-t8"],        // FTL theory guides deep survey
    ["survey-t8","crystal-t8"],           // survey discovers crystal structures
    // T7: chain depth=0 orphans into the mega-scan / relay-frag trees
    ["mega-scan","mega-drone"],          // megastruct scan → mega drone
    ["mega-drone","deep-mine-t7"],       // mega drone enables deep mining
    ["exotic-syn-t7","exotic-alloy-t7"],// exotic synth → exotic alloy
    ["relay-frag","fab-bench-t4"],       // relay frag → fab bench T4
    ["power-t5-b","net-t7"],             // power dist → network T7
  ];

  // simulated "current run" knowledge state — T1 complete, T2 nearly done, T3 in progress
  const knowledge = {
    // T1 — fully revealed
    "stone-furnace":3,"ore-crusher":3,"basic-miner":3,"combustion-gen":3,
    "solar-array":3,"field-analyzer":3,"net-node":3,"land-drone":3,
    // T2 — almost complete
    "t2-gate":3,"amp-drill":3,"layer-drill":3,"ore-washer":3,"wire-draw":2,
    "thermal-boil":2,"chan-relay":2,"auto-sorter":2,"sample-lab":3,
    "alien-sampler":3,"electro-cell":2,"amph-drone":1,
    // T3 — in progress
    "t3-gate":2,"pressure-vessel":2,"catalytic-bed":2,"alloy-forge":2,
    "fly-drone":2,"alien-lab":2,"alien-mat-t1":2,
    "deep-drill":1,"circuit-press":1,"power-cell-t2":1,"net-seg":1,
    "thermal-proc":1,"arc-smelter":1,"sub-network":1,"fab-bench":1,"route-ctrl":1,
    // T4 — a few known
    "t4-gate":1,"orbital-craft":1,"space-drone":1,"plasma-torch":1,"reactor-t2":1,
  };

  // wishlist (player marked for next reveal)
  const wishlist = new Set(["alien-mat-t1","fly-drone"]);

  // research economy — cost scales with tier
  const cost = (t, toTier) => {
    const base = {1:5,2:12,3:30,4:75,5:160,6:350,7:700,8:1500,9:3000,10:6000}[t.tier] || 10;
    return toTier === 3 ? Math.round(base * 2.2) : base;
  };

  const byId = Object.fromEntries(techs.map(t=>[t.id,t]));
  return { techs, edges, knowledge, wishlist, cost, byId };
})();

// ════════════════════════════════════════════════════════════════════════════
// SHARED HELPERS
// ════════════════════════════════════════════════════════════════════════════
function ttTweaks(){ return (typeof window!=="undefined" && window.__ttTweaks) || {}; }

// Render a node's name/glyph with the right fog-of-war treatment for its tier.
function FogText({ tier, fogStyle, glyph, name, short=false }){
  // tier 3 — full reveal
  if (tier === 3){
    return (
      <span style={{ display:"inline-flex", alignItems:"center", gap:6 }}>
        <span style={{ fontFamily:"var(--font-hand)", fontSize:14, lineHeight:1 }}>{glyph}</span>
        <span>{name}</span>
      </span>
    );
  }
  // tier 2 — partial: name visible, params hidden elsewhere
  if (tier === 2){
    return (
      <span style={{ display:"inline-flex", alignItems:"center", gap:6 }}>
        <span style={{ fontFamily:"var(--font-hand)", fontSize:14, lineHeight:1, opacity:0.85 }}>{glyph}</span>
        <span style={{ opacity: 0.85 }}>{name}</span>
      </span>
    );
  }
  // tier 1 — known to exist; the fog style controls how we mask it
  switch (fogStyle){
    case "redact": return (
      <span style={{ display:"inline-flex", alignItems:"center", gap:6 }}>
        <span className="tt-redact" style={{ width: short?16:18 }}/>
        <span className="tt-redact" style={{ width: short?40:64 }}/>
      </span>
    );
    case "sketchy": return (
      <span style={{ display:"inline-flex", alignItems:"center", gap:6, fontFamily:"var(--font-hand)", fontSize:13, color:"var(--ink-faint)" }}>
        <span>?</span><span>{"~".repeat(short?5:8)}</span>
      </span>
    );
    case "polaroid": return (
      <span style={{ display:"inline-flex", alignItems:"center", gap:6, opacity:0.35, filter:"blur(0.6px)" }}>
        <span>{glyph}</span>
        <span>{name.replace(/[a-z]/gi,"·")}</span>
      </span>
    );
    case "microfilm": return (
      <span style={{ display:"inline-flex", alignItems:"center", gap:6, color:"var(--ink-faint)" }}>
        <span style={{ fontFamily:"var(--font-mono)" }}>fragment</span>
        <span style={{ fontFamily:"var(--font-mono)", fontSize:9 }}>#{Math.abs(hash(name))%9999}</span>
      </span>
    );
    case "silhouette":
    default: return (
      <span style={{ display:"inline-flex", alignItems:"center", gap:6 }}>
        <span className="tt-fog-glyph">{glyph}</span>
        <span className="tt-fog-glyph tt-fog-soft">{name}</span>
      </span>
    );
  }
}

// tiny string-hash for stable mock IDs
function hash(s){ let h=0; for(let i=0;i<s.length;i++) h=((h<<5)-h+s.charCodeAt(i))|0; return h; }

function ttClass(tech, kn){
  const t = kn[tech.id] ?? 0;
  return `tt-t${Math.max(t,1)}` + (tech.ms?" tt-milestone":"") + (TT.wishlist.has(tech.id)?" tt-wishlist":"");
}

// pretty range for partial info
function ttRange(rate){
  const lo = Math.max(1, Math.round(rate*0.6));
  const hi = Math.round(rate*1.4);
  return `~${lo}–${hi}`;
}


// ════════════════════════════════════════════════════════════════════════════
// 00 · NORTH STAR — reading guide (V6-only)
// ════════════════════════════════════════════════════════════════════════════
function TTNorthStar(){
  return (
    <div className="paper" style={{ height:"100%", padding:24, display:"flex", flexDirection:"column", gap:18 }}>
      <div>
        <div className="sk-h">tech tree — tier-paged questbook</div>
        <div className="sk-mono-sm" style={{ color:"var(--ink-soft)", marginTop:6, maxWidth:820 }}>
          a procedural run gives the player ~179 nodes split across ten tiers. each starts as a <span className="sk-squig">silhouette</span>,
          becomes <span className="sk-squig">partial</span> with rough numbers, then <span className="sk-squig">fully revealed</span> and buildable.
          the tree reads as a <b>questbook</b>: each tier is its own page · subway-style research lines colour the within-tier flow ·
          milestones bridge adjacent pages · cross-tier dependencies become labeled port stubs at the page margins.
        </div>
      </div>
      <div className="sk-div"/>
      <div style={{ display:"grid", gridTemplateColumns:"repeat(3, 1fr)", gap:14, flex:1 }}>
        {[
          { tag:"PAGES",      t:"tier = page",       blurb:"T0…T4 each their own tab. a player on T2 sees only the electric age stratum — no scrolling past 120 nodes." },
          { tag:"BRIDGES",    t:"milestones span",   blurb:"the milestone gating into a tier is the right-edge card on the previous page AND the left-edge card on the next. always know where you came from." },
          { tag:"LINES",      t:"research colours",  blurb:"smelt · refine · chem · electric · logic · power. each tag is a swim-lane with a colour. within-line edges use that colour; cross-line edges go dashed." },
          { tag:"PORTS",      t:"cross-tier stubs",  blurb:"a node on T2 that depends on something from T1 shows a coloured stub at the left margin labelled with the source · click to jump pages." },
          { tag:"FOG",        t:"3 knowledge tiers", blurb:"T1 known (silhouette) · T2 partial (ranges) · T3 revealed (buildable). pick the fog metaphor in tweaks." },
          { tag:"REVEAL",     t:"the action",        blurb:"clicking any node opens the shared reveal panel · cost · tier ladder · prereq chain · before/after diff." },
        ].map(c=>(
          <div key={c.tag} className="sk-box" style={{ padding:14, display:"flex", flexDirection:"column", gap:8 }}>
            <div style={{ display:"flex", alignItems:"center", gap:8 }}>
              <span className="sk-tag sk-on">{c.tag}</span>
              <span className="sk-h sk-h-sm">{c.t}</span>
            </div>
            <div className="sk-mono-sm" style={{ lineHeight:1.55, color:"var(--ink-soft)" }}>
              {c.blurb}
            </div>
          </div>
        ))}
      </div>
      <div className="sk-mono-xs" style={{ color:"var(--ink-faint)" }}>
        always-on: search (tag · tier · "reveals X") · wishlist stars · click any node for the reveal panel · locked edges optional via tweak.
      </div>
    </div>
  );
}

// ════════════════════════════════════════════════════════════════════════════
// SHARED CHROME — topbar, search, sidebar, etc.
// ════════════════════════════════════════════════════════════════════════════
function TTTopbar({ mode, research=128, frontier="exergon core" }){
  return (
    <div style={{
      borderBottom:"1.5px solid var(--ink)", padding:"6px 12px",
      display:"flex", alignItems:"center", gap:10, flexShrink:0, background:"var(--paper)"
    }}>
      <span className="sk-tag sk-on">{mode}</span>
      <span className="sk-mono" style={{ color:"var(--ink-soft)" }}>research:</span>
      <span className="sk-tag sk-accent">{research} R</span>
      <span className="sk-mono-sm" style={{ color:"var(--ink-soft)" }}>frontier · {frontier}</span>
      <div style={{ flex:1 }}/>
      <button className="sk-btn">search</button>
      <button className="sk-btn">wishlist (2)</button>
      <button className="sk-btn">filter</button>
      <button className="sk-btn sk-on">reveal queue</button>
    </div>
  );
}

function TTSearchBar(){
  return (
    <div style={{ display:"flex", alignItems:"center", gap:6, padding:8, borderBottom:"1.5px dashed var(--ink-soft)" }}>
      <span className="sk-mono-xs" style={{ color:"var(--ink-soft)" }}>FIND</span>
      <input className="tt-search" placeholder="tag:smelt · tier:3 · reveals:plate · or any name…"/>
      <span className="tt-chip tt-on">extract</span>
      <span className="tt-chip">smelt</span>
      <span className="tt-chip">process</span>
      <span className="tt-chip">power</span>
      <span className="tt-chip">logistics</span>
      <span className="tt-chip">science</span>
      <span className="tt-chip">explore</span>
      <span className="tt-chip">fab</span>
      <span className="tt-chip tt-accent">unlocks recipe</span>
    </div>
  );
}

function TTLeftRail(){
  return (
    <div style={{
      borderRight:"1.5px solid var(--ink)", padding:"8px 4px",
      display:"flex", flexDirection:"column", alignItems:"center", gap:8,
      background:"var(--paper-2)"
    }}>
      {["⌂","⚙","⌬","▦","✺","⋯"].map((g,i)=>(
        <div key={i} className="sk-box" style={{ width:36, height:36, display:"flex", alignItems:"center", justifyContent:"center" }}>
          <span style={{ fontFamily:"var(--font-hand)", fontSize:18 }}>{g}</span>
        </div>
      ))}
    </div>
  );
}

function TTRightRail({ children }){
  return (
    <div style={{ borderLeft:"1.5px solid var(--ink)", padding:"10px 12px", background:"var(--paper)", overflow:"auto" }}>
      {children}
    </div>
  );
}

// inspector — the right rail content; shows what the player knows about a tech.
function TTInspector({ tech, knTier }){
  const t = tech;
  const fogStyle = ttTweaks().fogStyle || "silhouette";
  return (
    <div style={{ display:"flex", flexDirection:"column", gap:10 }}>
      <div className="sk-mono-xs" style={{ color:"var(--ink-faint)", textTransform:"uppercase", letterSpacing:0.6 }}>
        selected · tier {t.tier} · knowledge T{knTier}
      </div>
      <div className="sk-h sk-h-sm">
        <FogText tier={knTier} fogStyle={fogStyle} glyph={t.glyph} name={t.name}/>
      </div>
      <div className="sk-div"/>
      <div className="sk-mono-xs" style={{ color:"var(--ink-soft)" }}>tag</div>
      <div><span className="tt-chip tt-on">{t.tag}</span> {t.ms && <span className="tt-chip tt-accent">milestone</span>}</div>

      <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:6 }}>inputs</div>
      <div style={{ display:"flex", gap:4 }}>
        {[0,1,2].map(i=>{
          if (knTier === 3) return <div key={i} className="sk-slot sk-filled"><span className="sk-icon">{["◇","≈","◍"][i]}</span><span className="sk-qty">{[2,1,3][i]}</span></div>;
          if (knTier === 2) return <div key={i} className="sk-slot"><span className="sk-icon" style={{ opacity:0.4 }}>?</span></div>;
          return <div key={i} className="sk-slot" style={{ background:"repeating-linear-gradient(135deg, var(--paper) 0 4px, var(--paper-2) 4px 8px)", borderStyle:"dashed" }}/>;
        })}
      </div>

      <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:6 }}>output rate</div>
      {knTier === 3 && <div className="sk-mono">12.0/s · ferro-laminate</div>}
      {knTier === 2 && <div className="sk-mono" style={{ color:"var(--ink-soft)" }}>{ttRange(12)}/s · plate-class</div>}
      {knTier === 1 && <div><span className="tt-redact" style={{ width: 110 }}/></div>}

      <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:6 }}>flavour</div>
      <div className="sk-mono-sm" style={{ color:"var(--ink-soft)", lineHeight:1.5 }}>
        {knTier >= 2
          ? "rolled and re-rolled, the laminate holds form even when red-hot."
          : "scattered references in the field journal."}
      </div>

      <div className="sk-div" style={{ marginTop:10 }}/>
      <button className="sk-btn sk-accent" style={{ justifyContent:"center" }}>
        reveal → T{Math.min(3, knTier+1)} · {TT.cost(t, knTier+1)} R
      </button>
      <button className="sk-btn" style={{ justifyContent:"center" }}>
        ★ {TT.wishlist.has(t.id) ? "wishlisted" : "add to wishlist"}
      </button>
    </div>
  );
}


// ════════════════════════════════════════════════════════════════════════════
// 06 · REVEAL OVERLAY — shared modal for tier 1 → 2 → 3
// ════════════════════════════════════════════════════════════════════════════
function TTRevealOverlay(){
  // we draw the constellation behind, dimmed, with the modal floating
  return (
    <div className="paper" style={{ height:"100%", position:"relative" }}>
      {/* faint paper backdrop */}
      <div style={{ position:"absolute", inset:0, opacity:0.35, pointerEvents:"none", background:"repeating-linear-gradient(135deg, var(--paper) 0 6px, var(--paper-2) 6px 8px)" }}/>
      {/* modal */}
      <div style={{
        position:"absolute", inset:0, background:"rgba(26,26,26,0.18)",
        display:"flex", alignItems:"center", justifyContent:"center"
      }}>
        <div className="sk-box sk-thick" style={{ width:1100, height:740, padding:0, display:"grid", gridTemplateColumns:"380px 1fr", background:"var(--paper)" }}>
          {/* LEFT — the focus card */}
          <div style={{ borderRight:"1.5px solid var(--ink)", padding:18, display:"flex", flexDirection:"column", gap:10 }}>
            <div style={{ display:"flex", justifyContent:"space-between", alignItems:"center" }}>
              <span className="sk-tag sk-on">REVEAL</span>
              <span className="sk-mono-xs" style={{ color:"var(--ink-faint)" }}>esc · close</span>
            </div>

            <div className="sk-box" style={{ padding:14, background:"var(--paper)" }}>
              <div className="sk-mono-xs" style={{ color:"var(--ink-faint)" }}>FILE · T3 · #4821</div>
              <div className="sk-h sk-h-sm" style={{ marginTop:4 }}>
                <span style={{ fontFamily:"var(--font-hand)", fontSize:18, marginRight:6 }}>▦</span>
                control chip
              </div>
              <div style={{ display:"flex", gap:6, marginTop:6 }}>
                <span className="tt-chip">logic</span>
                <span className="tt-chip">recipe</span>
                <span className="tt-chip tt-accent">★ wishlist</span>
              </div>
            </div>

            {/* tier ladder */}
            <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:6 }}>knowledge ladder</div>
            <div style={{ display:"flex", flexDirection:"column", gap:8 }}>
              <TierRow tier={1} state="done"
                cost="—"
                title="known to exist"
                blurb="appears on tree, no params."/>
              <TierRow tier={2} state="done"
                cost="30 R"
                title="partial revealed"
                blurb="approx inputs, output range, machine class."/>
              <TierRow tier={3} state="next"
                cost="75 R"
                title="fully revealed"
                blurb="exact recipe, all params, buildable."
                accent/>
            </div>

            <div className="sk-div"/>
            <button className="sk-btn sk-accent" style={{ justifyContent:"center", padding:"8px 12px" }}>
              <span style={{ fontFamily:"var(--font-hand)", fontSize:18 }}>↧</span> REVEAL → T3 · 75 R
            </button>
            <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", textAlign:"center" }}>
              you have <b>128 R</b> · 53 R remaining after
            </div>
            <button className="sk-btn" style={{ justifyContent:"center", marginTop:4 }}>
              ★ keep on wishlist · queue for next
            </button>
          </div>

          {/* RIGHT — what reveal will give you */}
          <div style={{ padding:18, display:"flex", flexDirection:"column", gap:14, overflow:"hidden" }}>
            <div className="sk-h">what you'll learn</div>
            <div className="sk-div"/>

            <div style={{ display:"grid", gridTemplateColumns:"1fr 1fr", gap:12 }}>
              {/* before */}
              <div className="sk-box sk-dashed" style={{ padding:12 }}>
                <div className="sk-mono-xs" style={{ color:"var(--ink-faint)" }}>BEFORE · T2 partial</div>
                <div className="sk-mono-sm" style={{ marginTop:8, lineHeight:1.7 }}>
                  inputs   · <span className="tt-redact" style={{ width:60 }}/> + <span className="tt-redact" style={{ width:32 }}/><br/>
                  output   · ~7–17/s plate-class<br/>
                  machine  · bench-class T2<br/>
                  modules  · <span className="tt-redact" style={{ width:24 }}/> slots<br/>
                  flavour  · scattered references…
                </div>
              </div>
              {/* after */}
              <div className="sk-box sk-thick" style={{ padding:12 }}>
                <div className="sk-mono-xs">AFTER · T3 revealed</div>
                <div className="sk-mono-sm" style={{ marginTop:8, lineHeight:1.7 }}>
                  inputs   · 2× silica wafer · 1× copper wire<br/>
                  output   · 12.0/s control chip<br/>
                  machine  · assembly bench T3<br/>
                  modules  · 2 slots (P/S compatible)<br/>
                  flavour  · "the pattern is the thing."
                </div>
              </div>
            </div>

            {/* prereq chain */}
            <div className="sk-h sk-h-sm" style={{ marginTop:6 }}>prerequisite chain</div>
            <div style={{
              display:"flex", alignItems:"center", gap:6, padding:10,
              border:"1.5px solid var(--ink)", background:"var(--paper-2)", overflowX:"auto"
            }}>
              <ChainCard tier="T0" name="hearth smelt" state={3}/>
              <span className="sk-arrow">→</span>
              <ChainCard tier="T1" name="reverberatory" state={3}/>
              <span className="sk-arrow">→</span>
              <ChainCard tier="T1" name="steam vessel" state={3} ms/>
              <span className="sk-arrow">→</span>
              <ChainCard tier="T2" name="dynamo array" state={2} ms/>
              <span className="sk-arrow">→</span>
              <ChainCard tier="T3" name="silica wafer" state={1}/>
              <span className="sk-arrow">→</span>
              <ChainCard tier="T3" name="control chip" state={2} self/>
            </div>
            <div className="sk-mono-xs" style={{ color:"var(--ink-soft)" }}>
              chain shows current knowledge · click any link to focus that node and reveal it instead.
            </div>

            <div className="sk-h sk-h-sm" style={{ marginTop:6 }}>also unlocked when revealed</div>
            <div style={{ display:"grid", gridTemplateColumns:"repeat(3, 1fr)", gap:8 }}>
              {[
                ["recipe","control chip recipe","2× silica · 1× wire → 1× chip"],
                ["item","control chip","item slot in item index"],
                ["machine","assembly bench T3","new placeable in the build menu"],
                ["edge","→ exergon core","prereq edge to milestone T4"],
              ].map((row,i)=>(
                <div key={i} className="sk-box" style={{ padding:8, fontSize:10, lineHeight:1.4 }}>
                  <div style={{ display:"flex", justifyContent:"space-between" }}>
                    <span className="tt-chip tt-on">{row[0]}</span>
                    <span style={{ fontFamily:"var(--font-hand)", fontSize:14 }}>+</span>
                  </div>
                  <div style={{ marginTop:4, fontWeight:600 }}>{row[1]}</div>
                  <div style={{ marginTop:2, color:"var(--ink-soft)" }}>{row[2]}</div>
                </div>
              ))}
            </div>

            <div className="sk-annot" style={{ position:"static", marginTop:6, color:"var(--ink-soft)" }}>
              this same panel handles T1→T2 and T2→T3 · the "next" row is the live action
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

// little helper components for the reveal panel
function TierRow({ tier, state, cost, title, blurb, accent=false }){
  // state: done | next | locked
  return (
    <div className="sk-box" style={{
      padding:8, display:"grid", gridTemplateColumns:"40px 1fr auto", gap:8, alignItems:"center",
      background: accent ? "var(--accent)" : (state==="done" ? "var(--paper-2)" : "var(--paper)"),
      borderStyle: state==="locked" ? "dashed" : "solid",
      opacity: state==="locked" ? 0.55 : 1,
    }}>
      <div className="sk-h" style={{ textAlign:"center" }}>T{tier}</div>
      <div>
        <div className="sk-mono" style={{ fontWeight:600 }}>{title}</div>
        <div className="sk-mono-xs" style={{ color:"var(--ink-soft)", marginTop:2 }}>{blurb}</div>
      </div>
      <div className="sk-mono-xs" style={{ textAlign:"right" }}>
        <div style={{ fontWeight:700 }}>{cost}</div>
        <div style={{ color:"var(--ink-soft)" }}>{state==="done"?"✓ owned":state==="next"?"available":"locked"}</div>
      </div>
    </div>
  );
}
function ChainCard({ tier, name, state, ms=false, self=false }){
  return (
    <div className="sk-box" style={{
      padding:6, minWidth:120, position:"relative",
      background: ms ? "var(--accent)" : "var(--paper)",
      outline: self ? "2px dashed var(--ink)" : "none",
      outlineOffset: 2,
      borderStyle: state===1 ? "dashed" : "solid"
    }}>
      <div className="sk-mono-xs" style={{ color:"var(--ink-faint)" }}>{tier} {ms?"· MS":""}</div>
      <div className="sk-mono" style={{ marginTop:2, fontWeight:600,
        opacity: state===1 ? 0.5 : 1
      }}>
        {state===1 ? <span className="tt-redact" style={{ width:80 }}/> : name}
      </div>
      <div className="sk-mono-xs" style={{ marginTop:2, color:"var(--ink-soft)" }}>
        {state===3?"●revealed":state===2?"~partial":"?known"}
      </div>
    </div>
  );
}