# Core placement (research)

Silicera-owned placement experiments compare:

- `os_default` — no affinity
- `compact` — first N logical CPUs
- `spread` — stepped logical CPUs

## Constraints

- Does **not** change global OS scheduling configuration
- Affinity applies only to Silicera worker threads for the experiment duration
- Windows: `SetThreadAffinityMask` on those threads
- Other OS: affinity currently a no-op (still measures OS default)

## Run

```bash
silicera measure placement --threads 4 --iterations 15
```

Do not assume compact beats spread. Publish measured medians only.
