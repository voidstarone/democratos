# Democratos — Governance Model

> A Reddit-style forum where each community (a **demos**) is a self-governing polity.
> Citizenship (the right to vote) is *earned*, and the rules of citizenship are
> *themselves set by vote*. This document defines the governance engine.

## 1. Core concept

- The platform hosts many **demos** (communities).
- Within a demos, users hold a **tier**: `lurker → member → voter`.
- **Voters** are the citizens. They govern the demos.
- The **franchise** (becoming a voter) is *earned* by meeting **criteria**.
- The criteria — and the rules, leaders, and thresholds — are **set by the voters themselves**.

The interesting part is not the forum; it is the governance engine that lets a
community write and amend its own constitution without being captured.

## 2. The governing principle

> **Make takeover slow, not impossible.**

If takeover is *impossible*, there is no democracy — only entrenched founders.
If it is *instant*, there is mob rule. The thing that distinguishes a legitimate
shift in the community from a hostile flood is **time**: a brigade loses interest,
sybils age poorly, but a genuine change of heart in the demos persists.

Every defense in this document is therefore a **time tax**. None of them deny
anyone the franchise — they only slow the *rate* at which the electorate can flip.

### The three "floods" (they need different defenses)

| Threat | What it is | Beaten primarily by |
|---|---|---|
| **Sybils** | one person, many fake accounts | account age + earned contribution + identity friction |
| **Brigade** | real outsiders, coordinated, short attention span | dwell-time + enfranchisement rate cap + timelock |
| **Genuine shift** | the demos really did change its mind | *nothing — this must be allowed*, just gradually |

The third row is the discipline: a default that blocks a genuine majority is not
democratic. We target the *speed* of change, never the outcome.

## 3. The four defensive layers

A flood must beat **all four**, and each one costs the attacker weeks.

### Layer 1 — Earned franchise (the gate)
A member becomes a voter in a demos only when they meet **all** of:
- Account age ≥ **30 days** (global, across the platform)
- Member of *this* demos ≥ **14 days** (dwell before you decide)
- A minimum of contributions that **existing voters reacted positively to**
  (endorsement-weighted, not raw upvotes from anyone)
- No active sanctions

→ A flood of fresh accounts simply cannot vote for weeks. Defeats the naive attack outright.

### Layer 2 — Enfranchisement rate cap (the airlock)
**This is the layer that most directly answers "don't let a flood take over."**

The voter roll of a demos may grow by at most **+10% per 30 days**, with a floor of
**+5 voters** so small demos can still grow. If more users qualify than the cap allows,
they **queue by qualification date** (first-qualified, first-admitted).

→ Even 10,000 qualified newcomers cannot outnumber 100 established voters in one move.
The demos *digests* newcomers at a controlled rate. Everyone still gets in eventually —
one-person-one-vote is preserved; the electorate just cannot be flipped overnight.

### Layer 3 — Tiered decision thresholds (the constitution)
Not all decisions are equal:

| Decision class | Threshold |
|---|---|
| Routine moderation (remove post, resolve report) | simple majority, short window, low quorum |
| Bans / leader recall | **60%** + quorum |
| **Constitutional change** (franchise criteria, rules, *these thresholds*) | **2/3 of established voters + 50% quorum + 7-day timelock** |

Supermajority protects minorities from majorities — standard liberal-democratic
design, not oligarchy.

### Layer 4 — Timelock + recall hatch
A passed constitutional change does **not** activate immediately. During a cooling-off
window (default **7 days**) the rest of the demos can trigger a **recall vote**.
Even a faction that *wins* cannot make the change irreversible before the community reacts.

## 4. Why this is still democracy

- **One citizen, one vote** on every actual ballot. Tenure *never* weights a vote —
  it only gates *who may propose* amendments and *when* a qualified user is admitted.
- **Incumbents cannot exclude anyone.** The rate cap only *delays*; it never denies.
  Founders cannot bar a qualified user.
- **The community can lower its own walls.** An open demos amends toward openness; a
  besieged one tightens up. The system ships cautious and lets each demos tune itself.

### The honest tradeoff
Layers 2 and 3 tilt slightly toward incumbents — that is the price of flood resistance.
The tilt is a **bounded delay**, never a permanent veto, and it is community-adjustable.

## 5. Bootstrap: training wheels for small demos

**Problem:** small demos are where capture is *easiest* and the percentage math is
*weakest*. With 3 voters, a "2/3 supermajority" is just 2 people and a "+10% cap"
rounds to nothing.

**Default decision: (a) training wheels.** New demos run under platform-imposed rules
until they are large enough for self-governance math to be meaningful. This is a
*platform default*, chosen because it is the only option consistent with "no flood
takeover" — it is not a permanent constraint.

| Phase | Voter count | Governance |
|---|---|---|
| **Seed** | 1–9 | Founder is voter #1 and sets initial franchise criteria from a **platform menu** of sane presets. **No constitutional amendments** yet. Moderation by simple majority with a **platform backstop**. Franchise criteria fixed to the chosen preset. |
| **Chartering** | 10–24 | Amendments may be *proposed* but pass under **stricter** quorum/supermajority than steady-state. Rate-cap floor (+5) dominates, deliberately favoring stability while the demos finds its identity. |
| **Sovereign** | 25+ | Full self-governance. All four layers active; percentage-based math now works naturally. The demos owns its constitution. |

**Founder dilution:** the founder's outsized early influence decays automatically as
the demos crosses each phase boundary — there is no founder-for-life. The handoff is
structural, not discretionary.

## 6. Open questions (not yet decided)

- **Identity friction:** what raises sybil cost without demanding real-world ID?
  (phone, proof-of-personhood, vouching, cost-to-create?)
- **Contribution quality:** how exactly is "positively received by voters" measured,
  and how is *that* gamed?
- **Proposal rights:** exact tenure required to *propose* (vs. merely *vote on*)
  a constitutional amendment.
- **Inter-demos brigading:** defenses against coordinated cross-community campaigns.
- **Exit / forking:** can a losing minority fork a demos and take their content?
- **The numbers:** every threshold here (30d, 14d, 10%, 2/3, 7d, phase sizes) is a
  starting default to be validated, not a finding.

## 6a. Content, rules, and moderation

Beyond the franchise, a demos governs its **content** and **conduct**:

- **Community rules.** A demos votes its own rulebook in and out (`AddRule` /
  `RemoveRule`, decision class **RuleChange** — 60% + 30% quorum). Unlike
  franchise-criteria amendments, rule changes are **permitted even in Seed**, so a
  founding community can establish conduct from day one. Changing *who votes* is
  dangerous and stays locked; changing *the rulebook* is normal governance.
- **Posts & comments.** Text / image / video posts (media by URL reference) with a
  tree of comments.

### Automatic bot detection — "the machine accuses, the demos judges"

A pure, auditable heuristic scores accounts 0–100 on behavioural signals (account
age, posting cadence, duplicate content, cross-posting). Above the threshold the
system **files an automatic report** (`reporter = None`) — and stops there. It
**never auto-punishes**, because automated bans weaponise false positives. Every
accusation is decided by people.

### Trial by jury

A report (filed by a member, or automatically) can be put to a **trial by jury**:

- A jury of **existing members** is drawn at random — **deterministically seeded**
  by the report id, so the panel is reproducible and auditable — excluding the
  accused. Default size 7 (or all members, if fewer).
- Jurors vote guilty / not-guilty. **Conviction requires a 2/3 supermajority of the
  whole jury** — a deliberately high bar that protects the accused, mirroring
  "supermajority protects minorities".
- A guilty verdict **sanctions** the accused (which, by the existing rules, also
  *disqualifies them from the franchise*) and removes the offending content. An
  acquittal dismisses the report.

These defaults are, like everything else, community-votable over time.

## 7. Decisions locked so far

- Governance scope: voters govern **moderation, rules & criteria, leader election, and content ranking**.
- Defense philosophy: **make takeover slow, not impossible** (time as the filter).
- Four-layer defense: **earned franchise, enfranchisement rate cap, tiered thresholds, timelock + recall**.
- Small-demos default: **(a) training wheels** with Seed / Chartering / Sovereign phases.
