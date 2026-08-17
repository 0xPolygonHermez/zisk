## Keccakf `lanes_per_row` matrix (N=2^18)

|                           | **LPR=25** (current)            | **LPR=5**                       | **LPR=1**                      |
| ------------------------- | ------------------------------- | ------------------------------- | ------------------------------ |
| Trace shape               | 29 rows x 1600 cols x 2 Keccakf | 145 rows x 320 cols x 2 Keccakf | 725 rows x 64 cols x 2 Keccakf |
| Fixed                     | 2                               | 3                               | 3                              |
| Stage1                    | 1,925                           | 453                             | 146                            |
| Stage2                    | 721                             | 184                             | 121                            |
| **Total cols**            | **2,652**                       | **643**                         | **273**                        |
| Constraints               | 5,074                           | 1,099                           | 306                            |
| Max degree                | 3                               | 3                               | 3                              |
| Opening points            | 32                              | 151                             | 755                            |
| nEvals                    | 8,608                           | 4,275                           | 4,134                          |
| **Prover mem / instance** | **12.78 GB**                    | **3.12 GB**                     | **1.21 GB**                    |
| **Cells / Keccakf**       | 2652x29/2 = **38,454**          | 643x145/2 = **46,618** (+21%)   | 273x725/2 = **98,963** (+157%) |
| **Throughput / instance** | 2¹⁸/29x2 = **18,078**           | 2¹⁸/145x2 = **3,614**           | 2¹⁸/725x2 = **722**            |
