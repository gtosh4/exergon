# Balance State

> **Generated** by `scenario balance --emit`. Do not hand-edit — rerun to refresh.
>
> Each difficulty's canonical scenario, swept across 8 seeds (worldgen varies per seed; the step list is fixed). Deterministic: same seeds → same numbers.

## Initiation  (`scenarios/initiation.ron`)

- base seed: `0xe7e60007`, 8 runs
- outcome: **8/8 finished**
- victory: mean **1.43h** (min 1.34h, max 1.56h)

| seed | victory | slowest tier | flags |
|---|---|---|---|
| `0x00000000e7e60007` | 1.56h | t1 57m | idle:discovery idle:synthesis |
| `0x00000000e7e60008` | 1.45h | t1 53m | idle:discovery idle:synthesis |
| `0x00000000e7e60009` | 1.34h | t1 50m | idle:discovery idle:synthesis |
| `0x00000000e7e6000a` | 1.39h | t1 51m | idle:discovery idle:synthesis |
| `0x00000000e7e6000b` | 1.42h | t1 50m | idle:discovery idle:synthesis |
| `0x00000000e7e6000c` | 1.48h | t1 55m | idle:discovery idle:synthesis |
| `0x00000000e7e6000d` | 1.41h | t1 51m | idle:discovery idle:synthesis |
| `0x00000000e7e6000e` | 1.40h | t1 51m | idle:discovery idle:synthesis |

<details>
<summary><strong>Fastest seed</strong> <code>0x00000000e7e60009</code> — 1.34h (full timeline &amp; stats)</summary>

```text
  outcome: VICTORY   virtual time: 4829s (1.34h)

  ── tier progression ──
    tier 1  5/7 nodes          3s →     2985s  (0.00h → 0.83h)
    tier 2  2/12 nodes       3386s →     3909s  (0.94h → 1.09h)
    tier 3  3/5 nodes       3607s →     4442s  (1.00h → 1.23h)

  ── research currency curve ──
      secs    material  engineer  discover  synthesis
          3          0         0         0         0
        303          0         0         0         0
        603          0         0         0         0
        903          0         0         0         0
       1203          0         0         0         0
       1503         10         0         0         0
       1803         50         0         0         0
       2103         60         0         0         0
       2403         60         0         0         0
       2703         60         0         0         0
       3003         70         0         0         0
       3303         70       760         0         0
       3603         70       760         0         0
       3903        130       760         0         0
       4203         80       760         0         0
       4503         10       360         0         0
       4803         10       360         0         0

  ── raw ore extracted ──
    copper_ore               1146
    coal                      429

  ── node unlocks (10) ──
           3s  basic_smelting
           3s  science_basics
           3s  power_basics
        1492s  ore_extraction
        2985s  basic_processing
        3386s  ore_crusher
        3607s  ore_washer
        3909s  silicon_refining
        4442s  steel_alloying
        4442s  minimal_successor

  ── stage checks ──
    [x] kit miner latched deposit
    [x] kit machines share network
    [x] analysis station ran
    [x] smelter ran (power-gated)
    [x] assembler ran make_circuit
    [x] solar generator charged
    [x] atmospheric oxygen revealed
    [x] atmospheric pressure revealed
    [ ] geological activity revealed
    [ ] xalite site discovered
    [x] build list enqueued
    [x] launch_site ran launch
```

</details>

<details>
<summary><strong>Slowest seed</strong> <code>0x00000000e7e60007</code> — 1.56h (full timeline &amp; stats)</summary>

```text
  outcome: VICTORY   virtual time: 5615s (1.56h)

  ── tier progression ──
    tier 1  5/7 nodes          3s →     3449s  (0.00h → 0.96h)
    tier 2  2/12 nodes       3850s →     4809s  (1.07h → 1.34h)
    tier 3  3/5 nodes       4124s →     5228s  (1.15h → 1.45h)

  ── research currency curve ──
      secs    material  engineer  discover  synthesis
          3          0         0         0         0
        303          0         0         0         0
        603          0         0         0         0
        903          0         0         0         0
       1203          0         0         0         0
       1503          0         0         0         0
       1803         50         0         0         0
       2103         50         0         0         0
       2403         50         0         0         0
       2703         50         0         0         0
       3003         50         0         0         0
       3303        130         0         0         0
       3603         60       240         0         0
       3903         60       760         0         0
       4203         60       760         0         0
       4503        120       760         0         0
       4803        140       760         0         0
       5103         70       760         0         0
       5403          0       360         0         0

  ── raw ore extracted ──
    copper_ore               1130
    coal                      657

  ── node unlocks (10) ──
           3s  basic_smelting
           3s  power_basics
           3s  science_basics
        1671s  ore_extraction
        3449s  basic_processing
        3850s  ore_crusher
        4124s  ore_washer
        4809s  silicon_refining
        5228s  steel_alloying
        5228s  minimal_successor

  ── stage checks ──
    [x] kit miner latched deposit
    [x] kit machines share network
    [x] analysis station ran
    [x] smelter ran (power-gated)
    [x] assembler ran make_circuit
    [x] solar generator charged
    [x] atmospheric oxygen revealed
    [x] atmospheric pressure revealed
    [ ] geological activity revealed
    [ ] xalite site discovered
    [x] build list enqueued
    [x] launch_site ran launch
```

</details>

## Standard  (`scenarios/standard.ron`)

- base seed: `0xe7e60007`, 8 runs
- outcome: **8/8 finished**
- victory: mean **2.46h** (min 2.27h, max 2.68h)

| seed | victory | slowest tier | flags |
|---|---|---|---|
| `0x00000000e7e60007` | 2.68h | t2 62m | — |
| `0x00000000e7e60008` | 2.34h | t1 53m | — |
| `0x00000000e7e60009` | 2.27h | t2 50m | — |
| `0x00000000e7e6000a` | 2.40h | t2 53m | — |
| `0x00000000e7e6000b` | 2.41h | t2 57m | — |
| `0x00000000e7e6000c` | 2.44h | t2 55m | — |
| `0x00000000e7e6000d` | 2.55h | t2 58m | — |
| `0x00000000e7e6000e` | 2.57h | t2 62m | — |

<details>
<summary><strong>Fastest seed</strong> <code>0x00000000e7e60009</code> — 2.27h (full timeline &amp; stats)</summary>

```text
  outcome: VICTORY   virtual time: 8174s (2.27h)

  ── tier progression ──
    tier 1  5/7 nodes          3s →     2749s  (0.00h → 0.76h)
    tier 2  6/12 nodes       3250s →     6236s  (0.90h → 1.73h)
    tier 3  4/5 nodes       4108s →     6236s  (1.14h → 1.73h)
    tier 4  10/11 nodes       4108s →     6455s  (1.14h → 1.79h)
    tier 5  10/11 nodes       5733s →     6997s  (1.59h → 1.94h)

  ── research currency curve ──
      secs    material  engineer  discover  synthesis
          3          0         0         0         0
        303          0         0         0         0
        603          0         0         0         0
        903          0         0         0         0
       1203          0         0         0         0
       1503         10         0         0         0
       1803         50         0         0         0
       2104         50         0         0         0
       2404         50         0         0         0
       2704        130         0         0         0
       3004         60       160         0         0
       3304         70       780         0         0
       3604         70       780         0         0
       3904         70       560        36         0
       4204         70       780       180         0
       4504          0       780       162         0
       4804         80       780       142       360
       5104        260       780       274       760
       5404        420       780        94       760
       5704        580       780       286       760
       6004        580       780       238       760
       6304         80       180       382       760
       6604         80       400       586       130
       6904        280       780       646        50
       7204        280       780       706       210
       7504        280       780       706       210
       7804        280       780       706       210
       8104        280       780       706       210

  ── raw ore extracted ──
    copper_ore               3063
    coal                      749
    resonite_shard            853
    fluxite_shard             454
    cryophase_shard           966
    aluminum_ore              428
    titanium_ore              325

  ── node unlocks (35) ──
           3s  basic_smelting
           3s  power_basics
           3s  science_basics
        1492s  ore_extraction
        2749s  basic_processing
        3250s  ore_crusher
        3861s  drone_recon
        3862s  advanced_processing
        3862s  exotic_materials
        4108s  ore_washer
        4108s  plate_roller
        4234s  silicon_refining
        4304s  precursor_survey
        4305s  fluxite_studies
        4366s  exotic_processing
        4692s  synthesis_lab
        5138s  space_scanner
        5733s  cryophase_prospecting
        5734s  cryophase_extraction
        6017s  aluminum_extraction
        6235s  titanium_forming
        6236s  steel_alloying
        6236s  resonite_engineering
        6256s  advanced_assembler
        6349s  provisioning_module
        6349s  vitreite_synthesis
        6350s  coolant_reclaim
        6455s  fluxite_coil
        6455s  successor_core
        6456s  successor_chassis
        6557s  successor_drive
        6671s  successor_sensor
        6775s  exotic_fuel_refining
        6893s  launch_site_assembly
        6997s  launch_successor

  ── stage checks ──
    [x] kit miner latched deposit
    [x] kit machines share network
    [x] analysis station ran
    [x] smelter ran (power-gated)
    [x] assembler ran make_circuit
    [x] solar generator charged
    [x] atmospheric oxygen revealed
    [x] atmospheric pressure revealed
    [x] geological activity revealed
    [x] xalite site discovered
    [x] build list enqueued
    [x] launch_site ran launch
```

</details>

<details>
<summary><strong>Slowest seed</strong> <code>0x00000000e7e60007</code> — 2.68h (full timeline &amp; stats)</summary>

```text
  outcome: VICTORY   virtual time: 9637s (2.68h)

  ── tier progression ──
    tier 1  5/7 nodes          3s →     3245s  (0.00h → 0.90h)
    tier 2  6/12 nodes       3751s →     7489s  (1.04h → 2.08h)
    tier 3  4/5 nodes       4773s →     7489s  (1.33h → 2.08h)
    tier 4  10/11 nodes       4773s →     7696s  (1.33h → 2.14h)
    tier 5  10/11 nodes       7085s →     8460s  (1.97h → 2.35h)

  ── research currency curve ──
      secs    material  engineer  discover  synthesis
          3          0         0         0         0
        303          0         0         0         0
        603          0         0         0         0
        903          0         0         0         0
       1203          0         0         0         0
       1503          0         0         0         0
       1803         50         0         0         0
       2103         50         0         0         0
       2403         50         0         0         0
       2703         50         0         0         0
       3003         50         0         0         0
       3303         60         0         0         0
       3603         60       360         0         0
       3903         70       780         0         0
       4203         70       780         0         0
       4503         70       780         0         0
       4803         70       780       216         0
       5103          0       780       288         0
       5403          0       780        90         0
       5703        110       780       198         0
       6003        260       780       166       740
       6303        420       780        10       800
       6603        500       780        82       800
       6903        660       780       262       800
       7203        660       780        34       800
       7503        160       340       178       800
       7803        160       300       298        90
       8103        240       760       346       310
       8403        350       760       454       330
       8703        400       760       550       150
       9003        400       760       550       170
       9303        400       760       550       170
       9603        400       760       550       170

  ── raw ore extracted ──
    copper_ore               3070
    coal                     1157
    resonite_shard            767
    fluxite_shard             526
    cryophase_shard           942
    aluminum_ore              458
    titanium_ore              439

  ── node unlocks (35) ──
           3s  power_basics
           3s  basic_smelting
           3s  science_basics
        1671s  ore_extraction
        3245s  basic_processing
        3751s  ore_crusher
        4526s  drone_recon
        4527s  advanced_processing
        4527s  exotic_materials
        4773s  ore_washer
        4773s  plate_roller
        5064s  silicon_refining
        5134s  precursor_survey
        5135s  exotic_processing
        5135s  fluxite_studies
        5748s  synthesis_lab
        6223s  space_scanner
        7085s  cryophase_prospecting
        7086s  cryophase_extraction
        7281s  aluminum_extraction
        7489s  titanium_forming
        7489s  steel_alloying
        7489s  resonite_engineering
        7510s  advanced_assembler
        7603s  provisioning_module
        7603s  vitreite_synthesis
        7603s  coolant_reclaim
        7696s  fluxite_coil
        7696s  successor_core
        7696s  successor_chassis
        7767s  successor_drive
        7857s  successor_sensor
        7953s  exotic_fuel_refining
        8232s  launch_site_assembly
        8460s  launch_successor

  ── stage checks ──
    [x] kit miner latched deposit
    [x] kit machines share network
    [x] analysis station ran
    [x] smelter ran (power-gated)
    [x] assembler ran make_circuit
    [x] solar generator charged
    [x] atmospheric oxygen revealed
    [x] atmospheric pressure revealed
    [x] geological activity revealed
    [x] xalite site discovered
    [x] build list enqueued
    [x] launch_site ran launch
```

</details>

