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

## Roadmap: growing standard coverage

Honest scoping is the prerequisite; the next step is enriching the scratch org
so more standard members render in their intended state (a Lightning app with a
utility bar for `utilityBarContainer`, notification/favorite state for the
header, etc.) and curating real arguments for the parameterized standard
members the skip reasons now enumerate.
