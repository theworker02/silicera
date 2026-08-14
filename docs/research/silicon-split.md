# Silicon Split — multi-machine strategy comparison



**Protocol id:** `silicera-silicon-split/1`  

**Question:** Can two machines benefit from different measured strategies?



## Method



1. Fix the protocol: same workload set, same candidate set, same measurement knobs (`warmup`, `iterations`, `min_improvement`).

2. **Machine A:** `silicera research silicon-split train --role A -o out/machine_a.hnep`

3. Export sanitized artifact: `out/machine_a.split.json` (auto-written on train).

4. Emit **Machine B placeholder** schema (`out/machine_b.placeholder.json`) — **no invented B numbers**.

5. **Machine B** (second Zen3/Zen4/Zen5 host): same Silicera revision → train `--role B` → copy artifacts back.

6. Compare: `silicera research compare out/machine_a.hnep out/machine_b.hnep`  

   or `silicera research silicon-split report --a out/machine_a.hnep --b out/machine_b.hnep`



## Verdicts



| Verdict | Meaning |

|---------|---------|

| `UNKNOWN` | Machine B not measured (placeholder). Default on a single Ryzen box. |

| `YES` | Shared targets diverged with usable confidence. |

| `NO` | Shared targets agree on winners. |

| `INCONCLUSIVE` | Noise / low confidence — do not claim divergence. |



## What is compared



- Strategy vectors (workload → winner, size-class → winner)

- Per-target divergence + optional median relative delta (cross-machine informational, not a speedup claim)

- Fingerprints (must differ for a real two-SKU study; equal fingerprints are same class)



## Honesty rules



- Never invent Machine B medians, winners, or fingerprints.

- Never copy Machine A winners into the B slot.

- Publish tables only next to environment snapshots from real runs.

- MockHardware tests compare **structure** across mock Zen4/Zen5 — they do not fabricate marketing physics for a second SKU.



## Site / publish path



Use `site/benchmarks.html` Silicon Split section + this doc. If only Machine A exists, publish A artifacts + placeholder instructions + verdict **UNKNOWN**.


