# Infinifactory Market Note

> Date reviewed: 2026-05-20  
> Subject: [Infinifactory](https://store.steampowered.com/app/300570/Infinifactory/) by Zachtronics  
> Purpose: summarize player reception, compare against Exergon's design, and extract practical lessons.

## Snapshot

Infinifactory is a first-person 3D engineering puzzle game about building factories that assemble products for alien overseers. It combines spatial construction, conveyor logic, optimization histograms, story flavor, and workshop support.

At the time of review, Steam listed Infinifactory as `Overwhelmingly Positive`, with all-time reviews at 95% positive across more than 1,700 Steam purchaser reviews.

Useful source pages:

- Steam store: <https://store.steampowered.com/app/300570/Infinifactory/>
- Steam reviews: <https://steamcommunity.com/app/300570/reviews/>

## What Players Like

### 3D Spatial Puzzles

Players like solving production problems in a physical 3D space. Layout, gravity, timing, movement, and product assembly all matter.

### Open-Ended Engineering

As with other Zachtronics games, multiple solutions are valid. Players can optimize for footprint, cycles, or block count.

### Embodied Construction

First-person presence makes machines feel spatial and physical rather than purely abstract.

### Clear Testing Loop

Players build, run the machine, watch it fail, and revise. The feedback loop is fast and visual.

### Workshop And Custom Puzzles

User-created puzzles extend the game and support a technical community.

## What Players Dislike

### Difficulty And Spatial Reasoning Barrier

3D engineering puzzles can be hard for players who are comfortable with 2D logic but not spatial timing.

### Puzzle-Box Limits

Players who want large continuous factories may find discrete puzzles less satisfying.

### Camera And Construction Friction

Any 3D placement/camera friction is amplified when precision matters.

### Less Emotional Scale Than Factory Sandboxes

The game is elegant, but it does not offer the long expansion fantasy of Factorio, Satisfactory, or DSP.

## Comparison To Exergon

| Axis | Infinifactory | Exergon |
|---|---|---|
| Core fantasy | Build 3D assembly machines for puzzles | Build a run-scale factory from discovered science |
| Structure | Discrete 3D puzzles | Procedural campaign runs |
| Space | Puzzle chambers | 3D worlds, outposts, domains |
| Success | Produce target object | Escape by mastering the graph |
| Tooling | Test/replay, histograms, workshop | Planning, graph analysis, diagnostics, photo/run sharing |
| Main lesson | 3D factories need fast visual testing | Exergon's 3D factory must be easy to inspect and revise |

Infinifactory is useful because it shows how satisfying 3D production can be when the testing loop is fast.

## Lessons For Exergon

### 1. 3D Placement Must Be Excellent

If Exergon's factory layer is 3D, camera, snapping, rotation, alignment, and deconstruction need to be solid from the start.

### 2. Test Runs Should Be Cheap

Players need to simulate, preview, or diagnose production without waiting through long cycles.

### 3. Physicality Helps Understanding

Large machines, conduits, drones, and artifacts should make graph decisions feel physically present.

### 4. Avoid Turning The Whole Game Into Spatial Puzzle Solving

The GDD says spatial optimization is not the primary challenge. Infinifactory is a reminder that 3D spatial puzzles are a distinct audience. Exergon should support elegant layouts without requiring them.

### 5. Shareable Builds Matter

Workshop-style sharing may be post-MVP, but screenshots, blueprints, and run reports should be designed for.

## Store And Demo Implications

Show the player moving through a 3D factory, inspecting a machine, and using a planning tool to fix it. The pitch should clarify that 3D presence supports systems thinking, not manual placement busywork.

Potential positioning:

> A physical 3D factory world driven by a changing scientific puzzle.

## Open Questions For Exergon

- What placement operations must be single-click or drag-based?
- Can players preview throughput before committing construction?
- How does the camera support large machines and vertical routing?
- How do blueprints preserve 3D layouts across different run ratios?

