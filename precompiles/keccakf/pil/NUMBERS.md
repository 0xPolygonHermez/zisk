## Keccakf `lanes_per_row` matrix (N=2^18)

|                           | **LPR=25**                      | **LPR=5** (current)             | **LPR=1**                      |
| ------------------------- | ------------------------------- | ------------------------------- | ------------------------------ |
| Trace shape               | 27 rows x 1600 cols x 2 Keccakf | 135 rows x 320 cols x 2 Keccakf | 675 rows x 64 cols x 2 Keccakf |
| Fixed                     | 2                               | 3                               | 3                              |
| Stage1                    | 1,923                           | 467                             | 148                            |
| Stage2                    | 682                             | 142                             | 112                            |
| **Total cols**            | **2,611**                       | **615**                         | **266**                        |
| Constraints               | 1,859                           | 459                             | 177                            |
| Max degree                | 3                               | 3                               | 3                              |
| Opening points            | 30                              | 140                             | 704                            |
| nEvals                    | 5,392                           | 3,628                           | 3,958                          |
| Expressions               | 79,578                          | 22,926                          | 15,056                         |
| **Prover mem / instance** | **10.30 GB**                    | **2.51 GB**                     | **1.15 GB**                    |
| **Cells / Keccakf**       | 2611x27/2 = **35,249**          | 615x135/2 = **41,513** (+18%)   | 266x675/2 = **89,775** (+155%) |
| **Throughput / instance** | 2¹⁸/27x2 = **19,418**           | 2¹⁸/135x2 = **3,882**           | 2¹⁸/675x2 = **776**            |

`proofman-setup stats`, blowup 1. Prover memory is the GPU prover buffer (`mapTotalN`).
