# Network

This file contains information about the engine's networks and the data used to train them.

All data used for training is self-play data, previously generated using internal datagen and now using OpenBench datagen.

Network training schedules can be found in the project's history in [tools/trainer](https://github.com/tcheran-chess/tcheran/tree/master/tools/trainer). Schedules prior to that being moved in-tree can be found [the tcheran-archive branch of tcheran-chess/bullet](https://github.com/tcheran-chess/bullet/tree/tcheran-archive).

| #  | Architecture                       | SPRT                | Notes                |
| -- | --------------------               | ------------------- | -------------------- |
| -  | `(768->128)x2->1`                  | `-17.77 +- 9.74`    | Hello world! First attempt, trained with 'legacy' bullet with an initial self-gen dataset of only ~4M FENs at depth 8. These FENs were also generated without persistent TT and with no TBs. Clearly much too large a network for such a small dataset |
| 0  | `(768->16)x2->1`                   | `38.11 +- 13.69`    | Same setup as above, but reducing the hidden layer nodes to only 16 to account for the tiny dataset. Already a big improvement! |
| 1  | `(768->256)x2->1`                  | `223.69 +- 33.98`   | First proper dataset of ~100M FENs. Added TTs and used 5-man TBs. Also trained with 'legacy' bullet |
| 2  | `(768->256)x2->1`                  | `48.88 +- 14.81`    | Trained with bullet@main |
| 3  | `(768->256)x2->1`                  | `17.29 +- 8.50`     | Re-shuffled data before training |
| 4  | `(768->256)x2->1`                  | `89.86 +- 20.11`    | New dataset of ~460M FENs (including previous data) |
| -  | `(768->384)x2->1`                  | `-29.02 +- 26.87`   | Trying some different training params. Realised that WDL is broken because of a bug in adjudication. |
| 5  | `(768->256)x2->1`                  | `108.74 +- 20.12`   | Fresh dataset of ~125M FENs generated from scratch using net #4. Same setup as previous datasets, but fixing a critical bug that inverted WDL scores |
| 6  | `(768->256)x2->1`                  | `53.33 +- 15.20`    | Same data as net #5, but increasing WDL proportion and lowering LR |
| 7  | `(768->384)x2->1`                  | `62.36 +- 16.50`    | Moved to 384 nodes in the hidden layer, and used a fresh dataset generated using net #6. Had to re-label this dataset after generation due to a bug that caused all adjudicated games to be registered as a win for black. First dataset stored and filtered using viriformat |
| 8  | `(768->512)x2->1`                  | `27.07 +- 9.95`     | Moved to 512 nodes in the hidden layer, using a dataset of ~1B unfiltered positions, including positions used for net #8 |
| -  | `(768->512)x2->1`                  | `-3.88 +- 14.48`    | Same setup as net #8, with a full ~1B dataset (after filtering) |
| -  | `(768->768)x2->1`                  | `-6.57 +- 5.52`     | Attempt to up to 768 HL nodes in case we're overfitting |
| -  | `(768->768)x2->1`                  | `20.83 +- 9.53 at 40k fixed nodes`    | Test of the above at fixed nodes |
| 9  | `(768->768)x2->1`                  | `27.05 +- 9.85`     | Increasing WDL to 0.4 with 768 HL nodes  |
| -  | `(768->1024)x2->1`                 | `-2.34 +- 3.81`     | Attempted to increase HL to 1024 without changing any other training params |
| 10 | `(768->1024)x2->1`                 | `9.37 +- 5.40`      | Same dataset, increased training superbatches, upped hidden layer size to 1024 |
| 11 | `(768->1024)x2->1`                 | `23.37 +- 8.61`     | Entirely new dataset using 5k soft nodes and no tablebase adjudication, no other changes |
| 12 | `(768->1024)x2->8`                 | `9.89 +- 5.56`      | Added 8 output buckets, using the same dataset as net 11 |
| 13 | `(768hm->1024)x2->8`               | `7.49 +- 4.59`      | Same architecture and data as net 12, but with horizontally mirrored features |
| 14 | `(768hm->1024)x2->8`               | `5.00 +- 3.60`      | Same architecture and data as net 13, but with an additional 40 batch fine tuning stage at 0.6 WDL |
| -  | `(768hm->1024)x2->8`               | `-6.96 +- 4.58`     | Attempting a 0.4 -> 0.6 WDL first stage for training |
| 15 | `(768hm->1024)x2->8`               | `19.22 +- 7.60`     | Same as net 14, but with an additional ~416M positions at 20ksn |
| 16 | `(768x4hm->1024)x2->8`             | `19.04 +- 7.72`     | Added 4 input buckets, trained with an additional ~2.1B positions |
| 17 | `(768x4hm->1024)x2->8`             | `91.09 +- 18.98` DFRC     | Same as net 16 but with ~185M DFRC positions |
| 18 | `(768x4hm->1024)x2->8`             | `14.19 +- 6.56`     | Same as net 17 but with a new, much longer training schedule about half way between my existing one and Hobbes' |
| 19 | `(768x8hm->1024)x2->8`             | `2.95 +- 2.37`      | Ups the number of input buckets from 4 to 8 |
| 20 | `(768x8hm->1024)x2->8`             | `3.54 +- 2.80`      | Trained with Ranger instead of AdamW |
| 21 | `(768x8hm->1024)x2->(16->32->1)x8` | `16.47 +- 7.08`     | Switched to a multi-layer net |
| 22 | `(768x8hm->1024)x2->(16->32->1)x8` | `4.47 +- 3.26`      | Switch back to training the multi-layer net with Ranger |
| 23 | `(768x8hm->1024)x2->(16->32->1)x8` | `7.21 +- 4.45`      | Use dual activation for L2 |
