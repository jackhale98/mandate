# Concerns: `politics.vote.salience_base` 1.0 → 1.5 (mutes local priorities by abo

**Date:** 2026-11-18
**Area:** Concerns

## Parameter and change
`politics.vote.salience_base` 1.0 → 1.5 (mutes local priorities by about a third)

## Evidence
`bots --seeds 32 --grid politics.vote.salience_base=1.5,2.0,3.0`: idle 12.5% at each; 1.5 keeps the strongest local effect

## Effect
16 seeds: mixed 81%, legislate 75%, proxy 75%, canvass 69%, media 19%, idle 6% (before: mixed 81%, canvass 75%, legislate 69%, proxy 69%, media 19%, idle 6%); best − median 0.06. 32 seeds: mixed 81%, legislate 78%, canvass 75%, proxy 75%, media 19%, idle 12.5%. CI sweep and all five scenario sweeps pass
