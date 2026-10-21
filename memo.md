文献を調べると、**「DTC のトルク応答が良い」は、主にトルク指令のステップ変化に対して、短い到達時間と小さいオーバーシュートを両立しやすい、という意味です。** ただし、比較する電流制御の調整、サンプリング周期、スイッチング頻度によって結果が変わります。IPM についても、DTC の無条件な優位性とは解釈できません。

今回の調査で、直接比較の中心に据えられる IEEE 論文と、Sim-no8 で比較する際の重要な条件が見つかりました。

**最も参考になるのは、Niu らの IPM 実機を含む比較です。**

F. Niu et al., *Comparative Evaluation of Direct Torque Control Strategies for Permanent Magnet Synchronous Machines*, IEEE Transactions on Power Electronics, 31(2), 1408–1424, 2016。DTC、予測 DTC、デューティ比を調整する DTC、FOC を比較しています。[DOI](https://doi.org/10.1109/TPEL.2015.2421321)／[著者公開本文](https://www.researchgate.net/publication/283007915_Comparative_Evaluation_of_Direct_Torque_Control_Strategies_for_Permanent_Magnet_Synchronous_Machines)

本文の Section IV-E、Fig. 15 では、次の結果が示されています。

- DTC 系は、トルク指令に **1 ms 未満で到達**。
- FOC も、PI の調整によって同程度に速くできるが、オーバーシュートが発生。
- オーバーシュートを抑えた FOC の調整では、応答が遅くなる。

したがって、この論文から支持されるのは、**「比較した PI 調整に対して、DTC 系が速さとオーバーシュート抑制を両立した」**という主張です。「FOC は速くできない」という結果ではありません。また、この 1 ms は指令到達時間であり、10–90%立上り時間や整定時間と同一ではありません。軸トルクセンサによる 1 ms の直接計測とも断定できません。[本文 Section IV-E](https://www.researchgate.net/publication/283007915_Comparative_Evaluation_of_Direct_Torque_Control_Strategies_for_Permanent_Magnet_Synchronous_Machines)

**まず、MTPA と DTC は併用できるものです。**

MTPA は「要求トルクを少ない電流で発生させる動作点」を決めます。FOC／DTC は「その動作点やトルクへ、どの制御量を使って追従させるか」を決めます。DTC でも、トルク指令に対応した磁束指令を与えて MTPA を実現できます。IPM 向けに、その磁束指令を補正する研究もあります。[Shinohara et al., 2017：DTC と MTPA の併用](https://www.jstage.jst.go.jp/article/ieejjia/6/1/6_12/_article)

今回の比較は、より正確には **「MTPA＋電流制御」と「MTPA を考慮した DTC」**です。MTPA のテーブル参照そのものが応答を遅くする、と考える必要はありません。

制御構造の違いは、次のように整理できます。

| 方式 | トルク指令からインバータまでの主な経路 | 応答を考える際の特徴 |
|---|---|---|
| 現行に近い FOC | トルク → dq電流指令 → 電流PI＋FF → PWM | 電流ループの調整、電圧制限、PWM反映遅れが効く |
| 古典的 DTC | トルク・磁束誤差 → ヒステリシス判定 → スイッチ状態選択 | 電流PIと通常のPWM変調器を介さず、電圧ベクトルを選ぶ |
| DTC-SVM | トルク・磁束誤差 → 電圧指令 → SVM/PWM | 周波数とリプルを扱いやすくするが、変調・更新遅れも評価対象になる |

古典的 DTC が速いと説明される主要因は、電流制御器に伴う遅れを省くことです。一方、固定周波数化した DTC の性能は、その電圧指令の作り方まで含めて評価する必要があります。[Rahman et al., 2004](https://dro.deakin.edu.au/articles/journal_contribution/Problems_associated_with_the_direct_torque_control_of_an_interior_permanent-magnet_synchronous_motor_drive_and_their_remedies/20883970)、[Tang et al., 2004](https://doi.org/10.1109/TPEL.2003.823170)

ここからの技術的な解釈として、**DTC の利点は、トルクを変える電圧の選択と反映を素早く行えること**です。DTC でも磁束・電流が変化する物理過程は必要なので、同じ DC 電圧・電流制約の下で、無制限に速くなるわけではありません。

**DTC のうれしさには、引き換えになる条件があります。**

| 期待する利点 | 併せて確認すること |
|---|---|
| トルク指令への速い追従 | オーバーシュート、過渡電流ピーク |
| トルクと固定子磁束を直接扱える | 推定磁束・推定トルクの誤差 |
| 古典方式では電流PIを省ける | トルクリプル、電流歪み、可変スイッチング頻度 |
| 磁束を使って電圧限界を扱える | 弱め界磁、電流制限、MTPVまで含む制御設計 |

特に、DTC の「直接」は、通常は**推定した電磁トルクのフィードバック**です。電流センサが不要という意味でも、磁気モデルの影響がなくなるという意味でもありません。電流オフセット、固定子抵抗の変化、初期ロータ位置は、IPM DTC の実装課題として実験を含めて検討されています。[Rahman et al., IEEE TIE, 2004：大学公開抄録を確認](https://dro.deakin.edu.au/articles/journal_contribution/Problems_associated_with_the_direct_torque_control_of_an_interior_permanent-magnet_synchronous_motor_drive_and_their_remedies/20883970)

また、磁束を直接扱う方式の利点は、高速域の電圧制約でも現れます。Pellegrino らは IPM の弱め界磁・MTPV・DC リンク変動への対応を実験しています。ただし、この **DFVC は磁束と横軸電流を制御する方式で、古典的 DTC とは区別すべき**です。[Pellegrino et al., IEEE TIE, 2012：著者機関公開本文](https://iris.polito.it/retrieve/handle/11583/2460421/55146/Direct%20flux%20vector%20control%20of%20IPM%20motor%20drives.pdf)

**比較条件の読み方についても、重要な文献がありました。**

| 文献 | 今回確認できた内容と注意点 |
|---|---|
| Casadei et al., *FOC and DTC: Two Viable Schemes for Induction Motors Torque Control*, IEEE TPEL, 2002 | 同じ平均スイッチング頻度で、DTC の整定時間・オーバーシュートの利点を比較。ただし、**誘導機**であり、FOC／DTC の制御周期は 160／40 µs。IPM に数値を直接転用できない。[論文](https://doi.org/10.1109/TPEL.2002.802183) |
| Gashtil et al., *Comparative Evaluation … in Field Weakening Regions for Interior Permanent Magnet Machines*, IEEE CPE-POWERENG, 2019 | IPM の比較で DTC のリプルが小さいが、FOC は固定 10 kHz、DTC はそれより速く更新できる条件。本文自身が DTC のスイッチング損失増を説明している。**同一損失条件での優位性の証拠にはならない。**[大学公開本文](https://www.staff.ncl.ac.uk/damian.giaouris/pdf/Papers/paper5.pdf) |
| Rahman & Zhong, *Comparison of Torque Responses of the Interior Permanent Magnet Motor Under PWM Current and Direct Torque Controls*, IECON’99 | 今回の問いに直接対応する先行論文。書誌・抄録まで確認できたが本文は未確認なので、定量的結論には使っていない。[論文](https://doi.org/10.1109/IECON.1999.819440) |

このため、「DTC の応答が良い」という文献を読む際には、**何の応答時間か、電流制御をどう調整したか、どの周期・スイッチング条件で比べたか**をセットで読む必要があります。

**Sim-no8 では、方式差を測る前に区別すべき遅れが見つかりました。**

現行経路は、おおむね次の構成です。

`トルク指令 → 最大トルク制限 → 10 ms LPF → MTPA電流指令表 → 電流PI＋FF → PWM`

- **トルク指令 LPF の時定数が 10 ms。** 初期化後のステップに対する、このフィルタ単独の理想的な10–90%立上り時間は約22 msです。これは計算値であり、Sim-no8 の実測トルク応答ではありません。[LPF仕様](/Users/sugusokothx/newspace/yamatoshi-ops/number-eight-old/sim_no8/docs/spec/ipm_control.md:129)
- **診断用 `Trq_est` は、3 ms の電流 LPF を通した推定値。** 応答評価には plant 真値の `Te` を使い、推定値の追従とは分ける必要があります。[推定経路](/Users/sugusokothx/newspace/yamatoshi-ops/number-eight-old/sim_no8/docs/spec/ipm_control.md:184)
- 通常は **5 kHz キャリア、100 µs 制御周期、次イベントでの duty 反映**です。古典的 DTC の直接スイッチングと比較する場合、この反映タイミングも評価条件になります。[時系列仕様](/Users/sugusokothx/newspace/yamatoshi-ops/number-eight-old/sim_no8/docs/spec/ipm_control.md:397)

したがって、DTC 側だけ指令 LPF を外して速くしても、DTC 固有の利点を示したことにはなりません。

Sim-no8 の別版で最初に答える問いは、次が適切だと考えます。

> **同一の IPM・DC 電圧・電流制約・定常 MTPA 動作点において、DTC は電流制御よりも、許容するオーバーシュートとリプルの範囲内でトルク応答を短縮できるか。**

最初は電圧余裕のある一定速度の1運転点で、小・大トルクステップを比較するだけでも、この問いの入口に答えられます。

| 比較対象 | 役割 |
|---|---|
| A：現行 Sim-no8 そのまま | 現行構成からの改善量を測る |
| B：指令整形を比較用に揃えた電流制御 | フィルタの影響を分離する |
| C：B と指令整形・運転条件を揃えた DTC | 制御方式による差を測る |

観測量は、`Te` の立上り時間・整定時間・オーバーシュートに加えて、**過渡電流ピーク、定常トルクリプル、平均スイッチング頻度**を揃えます。同じ平均スイッチング頻度でも損失が同じとは限らないため、初回の結果だけで効率優位までは主張しません。

方式としては、**古典的 DTC を速応性の原理確認に使い、DTC-SVM を固定周波数での実用候補として扱う**整理が有効です。初回からセンサレス化まで加えず、位置既知で制御方式の差を調べ、その後に低速・磁束推定誤差・電圧限界へ進めるのが、今回の問いに対して必要十分です。現時点の判断は **追加検証**であり、制御コードの変更やシミュレーションはまだ行っていません。
