# Milestones
Current status: pre-Vertical Slice

## [Vertical Slice](./vertical_slice.md)
The vertical slice should not try to prove the full MVP. It should prove whether the core premise works before scope expands.

Marketing and outreach:
- None. Internal validation only.

Assets and narrative:
- Blockout or placeholder quality acceptable throughout
- Machines must be visually distinct enough to identify type at a glance from inside the 3D space
- Network connections and cable routing must be legible in 3D — no final art required, but topology cannot be ambiguous
- Diegetic field computer framing stubbed with placeholder text — persona and voice not required, but the delivery surface must exist
- No story content required

The slice exists to answer five questions:

1. **First-hour insight:** Does a new player reliably have a real "I figured out this planet" moment in the first 30-60 minutes?
2. **Repeat-run discovery:** After multiple runs, does discovery still feel like discovery, or does it collapse into recognition and parameter lookup?
3. **Remote mode feel:** Does drone exploration feel tense, useful, and distinct, or does it feel like slow remote clicking?
4. **3D factory readability:** Can players understand their own factory topology, routing, and bottlenecks in 3D without fighting the camera or walking every connection?
5. **Standard-length pacing:** Does a longer run shape sustain planning and decision-making, or does the middle collapse into passive factory management?

If the slice cannot answer these, it is too broad, too shallow, or focused on the wrong features.

Gate conditions:
- Insight Run playable end-to-end on at least one curated seed
- Standard Probe playable for a 3-5 hour pacing test on at least one curated seed
- At least 5 curated or generated test seeds available for repeat-run comparison
- Lightweight development telemetry records the events and derived metrics defined in the vertical slice spec
- Save/resume works for split Standard Probe sessions
- First-time player, repeat-run player, and Standard Probe playtest protocols completed with written observations

**What the slice will not answer:**

- **Run-stakes tension.** The slice does not test whether failure (slow escape, resource lock-out, forced restart) feels meaningful. At slice run lengths, non-destructive failure is the correct model. Whether the absence of real failure creates a tension deficit is a positioning question to answer before marketing, not before shipping the slice. The game should be framed internally as a procedural campaign, not a roguelite, until permadeath or equivalent stakes are validated.
- **Procedural graph validity at scale.** Curated seeds bypass the generator. Slice results do not prove that procedural generation produces solvable, balanced runs.
- **Discovery-to-recognition beyond run 3.** The slice tests early runs. Collapse at run 5–10 requires a longer study.

## Alpha

The Alpha proves that procedural generation can produce valid, playable Initiation runs and Standard Probe runs — and that vertical slice signals hold under generated seeds, not just curated ones. Alpha must nail down the core game shape before the Demo becomes a public commercial artifact.

The Alpha exists to answer four questions:

1. **Procedural validity:** Does the generator produce Initiation runs that are solvable, balanced, and non-pathological without hand-tuning?
2. **Standard core validity:** Do generated Standard Probe runs preserve the core decision shape: mid-run planning, at least one meaningful power transition, and second-site or expansion pressure?
3. **Loop completeness:** Can a player complete a full Initiation run end-to-end without developer assistance?
4. **Iteration stability:** Is the codebase stable enough for rapid iteration as full Standard development begins?

Gate conditions:
- Procedural graph validator passing:
  - Smoke gate: minimum 10 generated Initiation seeds and 10 generated Standard Probe seeds pass reachability and difficulty bounds checks
  - Scale gate before Demo: larger generator validation target defined and passing for Standard-length content, enough to catch pathological graphs before external playtesting
- Internal testers can complete Initiation runs without external guidance
- Standard Probe tests under generated seeds preserve the vertical slice pacing, readability, and repeat-run signals
- Development telemetry is enabled for all other-player Alpha sessions, recording run progression, blocked states, Remote mode usage, discovery timing, factory pacing, and completion outcomes
- Run save/load is stable enough for other-player sessions: players can save, quit, reload, and continue in-progress Initiation and Standard Probe runs without losing world, factory, tech-tree, research, drone, or telemetry state
- Meta save writes are validated for Alpha scope: run completion updates completion history and any enabled codex or unlock data without corrupting future runs

Marketing and outreach:
- Begin capturing gameplay footage and screenshots for future press use
- Draft presskit (not published): concept, target audience, key features
- Identify target communities: factory game players, GTNH community, Factorio and DSP player bases
- No public-facing communication required

Assets and narrative:
- Blockout or placeholder quality acceptable throughout
- Planet surface must produce a visually distinct identity per seed — not final art, but distinct enough that two different seeds feel like different worlds
- No audio required
- No narrative content required

What Alpha does not need:
- Full Standard difficulty
- Meta-progression
- Polished UI or performance optimization
- External playtesting
- Final art, audio, or narrative

---

## Demo (MVP)

The Demo proves that Standard difficulty is commercially viable and that Initiation functions as an onboarding funnel. Standard is the commercial anchor — the experience the store page, demo, and early access should center. The Demo ships when Standard is ready, not before.

The Demo exists to answer four questions:

1. **Standard viability:** Does a Standard run sustain active decision-making through the middle run, across multiple seeds?
2. **Onboarding funnel:** Does Initiation reliably deliver a satisfying first completion and leave the player wanting Standard?
3. **Run-stakes positioning:** Is non-punishing failure an acceptable commercial model, or does the Demo require a stakes hook before public exposure?
4. **Meta-progression hook:** Does first-pass meta-progression (codex, biome unlocks) give players reason to start a second run immediately after completion?

Gate conditions:
- Standard difficulty playable end-to-end
- External playtesting confirms Standard pacing holds across seeds
- Run-stakes tension decision locked before public exposure
- First meta-progression loop in place: codex entries populate, first biome unlock earned
- Development telemetry is enabled for external Demo playtests and press/creator demo builds, with opt-out or disclosure handled appropriately for the audience
- User-facing save/load flow complete for Initiation and Standard: continue, manual save, autosave, load, abandon run, and completed-run revisit all work without developer tools
- Cloud saves configured and validated for the public Demo or Early Access build on the target store platform
- Initial achievement set implemented and validated for Demo-visible progression, completion, discovery, and meta-progression beats

Marketing and outreach:
- Steam page live: store art, description, screenshots, announcement trailer
- Wishlist campaign active before or at Demo launch
- Demo available for press and content creators
- Early access date announced or early access live
- Community hub live (Discord or forum)
- Press and streamer outreach for first-wave coverage
- The Demo is the commercial pitch — Standard difficulty must be the centerpiece, not Initiation

Assets and narrative:
- Commercial-quality models and materials for all machines, structures, and environments in Initiation and Standard
- At least two visually distinct biomes with legible environmental identity
- Discovery and escape events require visual payoff — these are the game's best screenshots
- Core sound design complete: machine operation, power state changes, discovery events, escape sequence
- Arrival-to-escape narrative arc for Initiation — the player understands who they are, why they are here, and what leaving means
- Field computer has a defined persona and consistent voice; all in-run prompts and tutorial interventions use it
- Run completion screen delivers a narrative beat — not just a condition-met summary
- Codex first entries populated with lore flavor for all encountered types in Initiation and Standard

What the Demo does not need:
- Advanced or Pinnacle difficulty
- Full meta-progression depth
- Mod schema
- Permadeath variants
- Narrative for Advanced/Pinnacle tiers or alien civilization arc beyond Initiation hooks

---

## Release

Release proves the full difficulty ladder is complete, meta-progression is deep enough to sustain long-term play, and the official content pack is complete for 1.0.

Gate conditions:
- All four difficulties playable end-to-end: Initiation, Standard, Advanced, Pinnacle
- Run length targets validated via playtesting for all difficulties
- Full meta-progression shipped: biomes, codex, blueprint slots, run modifiers, starting conditions pool
- Permadeath variant(s) shipped or explicitly deferred to post-release roadmap
- Save/load compatibility policy defined for 1.0 saves and tested across upgrade scenarios
- Cloud saves validated for all launch platforms that support them
- Full 1.0 achievement set implemented, tested, and matched to launch platform metadata
- Performance and stability targets met for launch platforms

Marketing and outreach:
- Launch trailer featuring escape climax visuals for each difficulty tier
- Press review copies distributed before launch
- Streamer and creator outreach coordinated with launch window
- Launch day community management plan in place
- 1.0 store page updated with full difficulty ladder and meta-progression content

Assets and narrative:
- Full visual and audio polish for all four difficulties and all shipped biomes
- Each escape type (gateway, derelict ship, relay, spacecraft) has a dramatic, screenshot-worthy climax — a visible, in-world event, not a condition-met screen
- Full audio mix: music, ambient, reactive audio responding to factory state and world reactivity
- Meta-progression narrative arc complete across all four difficulties — the alien civilization trail emerges coherently across many runs
- Alien civilization lore distributed across run completion screens, artifact discoveries, and codex entries
- Codex entries carry narrative flavor for all types encountered across all difficulties

What Release does not need:
- Mod schema publication or external modding support; modding is explicitly post-release
- Additional content packs beyond the official one
- Platform ports beyond launch targets

---

## Post-release

Post-release expands the game's possibility space without modifying its core loop.

Marketing and outreach:
- Content update announcements for new biomes, scenarios, and mod tooling releases
- Sale campaigns coordinated with major store events
- Community seed showcases and challenge runs as recurring engagement
- Modder spotlight and community content promotion once mod tooling ships

Content and systems:
- Additional biome packs expanding the world generation pool
- New escape artifact scenarios and run modifier types
- Mod schema stable, documented, and externally published; official content pack readable as modder reference
- Official mod tooling (content editor, run validator, balance checker) shipped to modders
- Permadeath variants, if deferred from Release
- Platform ports
- Community content pipeline support
- Additional biome visual identities and audio environments
- Narrative content packs expanding the alien civilization arc
- Photo mode and base-sharing tooling if not shipped at Release
