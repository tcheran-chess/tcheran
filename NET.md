# Network

This file contains information about the engine's networks and the data used to train them.

## Networks and experiments

Parameters in the following table are noted only where they differ from Bullet's defaults, which [can be found here](https://github.com/jgilchrist/bullet/blob/e1d5ced0916dbbc0c1e603e67542cbe99d2e05b7/src/main.rs).

All data used for training is self-play data generated using the datagen code from this repository.

| Network | Architecture       | Parameters                     | SPRT              | Notes                |
| ------- | ------------------ | ------------------------------ | ----------------- | -------------------- |
| -       | (768->128)x2->1    | WDL 0.1, LR 0.01               | -17.77 +- 9.74    | Hello world! First attempt, trained with 'legacy' bullet with an initial self-gen dataset of only ~4M FENs at depth 8. These FENs were also generated without persistent TT and with no TBs. Clearly much too large a network for such a small dataset |
| 0       | (768->16)x2->1     | WDL 0.1, LR 0.01, 20 batches   | 38.11 +- 13.69    | Same setup as above, but reducing the hidden layer nodes to only 16 to account for the tiny dataset. Already a big improvement! |
| 1       | (768->256)x2->1    | WDL 0.1, LR 0.01, 20 batches   | 223.69 +- 33.98   | First proper dataset of ~100M FENs. Added TTs and used 5-man TBs. Also trained with 'legacy' bullet |
| 2       | (768->256)x2->1    | WDL 0.1, LR 0.01, 40 batches   | 48.88 +- 14.81    | Trained with bullet@main |
| 3       | (768->256)x2->1    | WDL 0.1, LR 0.01, 40 batches   | 17.29 +- 8.50     | Re-shuffled data before training |
| 4       | (768->256)x2->1    | WDL 0.1, LR 0.01, 40 batches   | 89.86 +- 20.11    | New dataset of ~460M FENs (including previous data) |
| -       | (768->384)x2->1    | WDL 0.3, LR 0.001, 40 batches  | -29.02 +- 26.87   | Trying some different training params. Realised that WDL is broken because of a bug in adjudication. |
| 5       | (768->256)x2->1    | WDL 0.1, LR 0.01, 40 batches   | 108.74 +- 20.12   | Fresh dataset of ~125M FENs generated from scratch using net #4. Same setup as previous datasets, but fixing a critical bug that inverted WDL scores |
| 6       | (768->256)x2->1    | WDL 0.3, LR 0.001, 40 batches  | 53.33 +- 15.20    | Same data as net #5, but increasing WDL proportion and lowering LR |
| 7       | (768->384)x2->1    | WDL 0.3, LR 0.001, 40 batches  | 62.36 +- 16.50    | Moved to 384 nodes in the hidden layer, and used a fresh dataset generated using net #6. Had to re-label this dataset after generation due to a bug that caused all adjudicated games to be registered as a win for black. First dataset stored and filtered using viriformat |
