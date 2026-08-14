# Hardware-Native Execution (HNE)

**HNE** denotes the research idea: execution behavior is specialized using an explicit hardware-native profile rather than compile-host ISA alone.

Components:

1. **Identity** — fingerprint class  
2. **Measurement** — tournaments with statistical honesty  
3. **Profile** — HNEP artifact  
4. **Dispatch** — cheap selection + safe fallback  

HNE is broader than any single compiler pass. Phase I realizes the profile/runtime loop; compiler integration is Phase II.
