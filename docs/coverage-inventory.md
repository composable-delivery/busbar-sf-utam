# Coverage inventory & honest scoping

The Salesforce live tests (`utam-runtime/tests/salesforce_live.rs`) discover
every page object whose root selector matches the live DOM and exercise all of
its methods and elements. To keep that coverage **real** — no silent passes,
no fabricated failures — we derive a *coverage inventory* from the upstream
compiled UTAM client and use it to scope what we assert.

## Why

The vendored `salesforce-pageobjects/**/*.utam.json` describe selectors and
element trees, but the **compiled** `salesforce-pageobjects` npm package (the
UTAM JS client) is the authority on how to *interact* with each page object:

- method signatures with real argument **names** + types
  (e.g. `getSearch(searchTerm: string)`, `getDockablePanel(apiName: string)`),
- **nullability**, encoded in the return type (`Promise<… | null>`), and
- a human description + root selector in each class' JSDoc.

We mine that, classify each root page object, and commit a single derived
artifact rather than vendoring ~1400 `.d.ts` files.

## Regenerating the inventory

```sh
python scripts/build_coverage_inventory.py        # uses the version in MANIFEST.json
python scripts/build_coverage_inventory.py -v 11.0.0
```

This pulls the compiled client with `npm pack`, pairs each root `.utam.json`
with its compiled `.d.ts`, and writes
`salesforce-pageobjects/coverage-inventory.json`:

```jsonc
{
  "version": "11.0.0",
  "summary": { "root_page_objects": 401, "standard": 380, "gated": 21, … },
  "objects": {
    "global/header": { "standard": true, "methods": [ … ], … },
    "devops/center/baseComponent": {
      "standard": false,
      "gated_reason": "DevOps Center managed package (not in a standard scratch org)"
    }
  }
}
```

## How the harness uses it

A page object is only *discovered* when its root **and** at least one
non-nullable element resolve, so we only ever test objects that are genuinely
present. Failures are therefore specific missing members. The runner records
three honest, distinct outcomes (`utam-runtime/tests/sf_live/`):

1. **Out-of-scope object → `Skipped`** with the capability it requires. Objects
   the inventory marks `gated` (DevOps Center, Experience Cloud `dxp_*` /
   `experience_*`, CMS `mcontent_*`) can't render in a standard scratch org.
   See `inventory::gated_reason`.
2. **Parameterized member with no curated value → `Skipped`** with the arg(s)
   we still owe. Calling `getUtilityBarItem("")` resolves `[data-id='']`, which
   never matches — that isn't a real test, so we skip it instead of fabricating
   an empty string. Supply real values via `synth::override_args` /
   `override_element_args` to actually exercise it.
3. **Feature-gated member of a standard object → `Skipped`** (e.g.
   `global/header::waitAndClickCoPilot` needs Einstein Copilot). See
   `synth::member_skip_reason`.

Everything else is asserted for real: a present, non-parameterized,
non-nullable member that fails to resolve is a genuine failure.

Skips are marked `known` so Allure shows them as intentional, with the reason —
never a green that hides a gap.

## Baseline metadata: growing standard coverage

Honest scoping is the prerequisite; the next lever is enriching the scratch org
so more standard members render in their intended state. We seed that as
committed source metadata under `force-app/` and deploy it into the shared
scratch org from CI.

### What's seeded

- **`UTAM_Console`** — a Lightning **console** app (`CustomApplication`,
  `navType=Console`) with a **utility bar**
  (`UTAM_Console_Utility_Bar.flexipage`, `type=UtilityBar`). Opening this app
  renders `div[class*=oneUtilityBarContainer]`, so `global/utilityBarContainer`
  is discovered and its `dockablePanel` / `utilityBarItems` /
  `utilityBarItemPanelHeader` elements are exercised for real (the runner opens
  a panel first — see `runner::open_utility_panel`). The console chrome also
  surfaces `navex/*` console tabs and the header's console-only
  `stageLeftToggle`.
- **`UTAM_Console_Access`** — a `PermissionSet` granting the live-test user
  access to the app, assigned by the deploy workflow so the app is visible.

### How it's deployed

`.github/workflows/deploy-baseline-metadata.yml` is a reusable workflow
(`workflow_call` + `workflow_dispatch`). The Salesforce Integration workflow
calls it after `provision` and before `live`, so each run drives an org that
contains the seeded surfaces. It can also be run by hand (`workflow_dispatch`)
to push metadata into the current shared org without re-provisioning. Deploy
failure is **fatal** — a malformed baseline must surface loudly rather than
letting the live tests silently fall back to "surface absent" skips.

The live harness adds a **`console`** page context
(`d_console_coverage`) that opens `/lightning/app/UTAM_Console`. If the deploy
was skipped (e.g. a hand-run against an un-seeded org), the app simply doesn't
load and discovery records honest absence rather than fabricating coverage.

### Curating arguments to widen execution coverage

A parameterized member is only worth curating when its page object is actually
**discovered**. Discovery anchors on a *fixed* (non-parameterized,
non-nullable) public element — `discovery::is_verification_anchor` rejects any
selector containing `%s`/`%d`. So a PO whose only public members are
parameterized (e.g. `navex/appNavMenu`, both of whose elements are
`button[title='%s']` / `a[data-label='%s']`) can **never** match the DOM, and
curating args for it is a no-op.

Filtering to root POs that have a fixed anchor *and* expose parameterized
members leaves a small, honest target set on the desktop surfaces we visit:

| Page object | Parameterized member(s) | Status |
| --- | --- | --- |
| `global/globalCreate` | `globalCreateMenuItem(titleString)` | curated → `"New Contact"` |
| `setup/setupNavTree` | `navTreeNodeByName(ariaLabel)` | curated → `"Users"` |
| `global/utilityBarContainer` | `dockablePanel(apiName)`, `utilityBarItem(name)` | unblocked by the seeded utility bar above |
| `setup/setupAlohaPage` | `*ByName(name)` inputs | classic "Aloha" setup pages only — not the Lightning setup context |
| `force/multiAdd`, `force/multiEdit` | action buttons | modal/record-action POs, present only mid-interaction |

The takeaway: there is **no large backlog of easy arg-curation wins** on the
contexts we currently load — the reliably-discovered ones are already curated.
The real levers for widening *execution* coverage are therefore (a) **seeding
metadata** (the utility bar above, which unblocks `utilityBarContainer`'s
parameterized elements) and (b) **adding page contexts** that load the surfaces
where the remaining parameterized POs render (Aloha setup pages, record-action
modals), then curating their args against state we control.

### Still ahead

Notification/favorite state for the header, list-view + related-list surfaces,
and record pages whose subheader layout matches the record-home templates'
stale `beforeLoad`.
