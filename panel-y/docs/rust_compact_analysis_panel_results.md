---
project: Panel_y
created: "2026-10-27"
status: 360px native UI and statistics horizontal-scroll checks passed; 240 tests passed
---

# 小さいウィンドウでの解析パネル操作性

1280×832のmacOSウィンドウで、3ソース・12チャンネルの統計とFFTを操作できるよう
解析欄を変更した。従来は初期高さ220pxを統計とFFTで常に分割し、FFTの対象一覧・設定・
実行・結果が小さなスクロール領域へ積み重なっていた。

## 変更した操作

- 初期高さを360pxとし、`Statistics`（既定）／`FFT`／`Split`を選択できる。
  単独表示は解析欄の高さを使用し、`Split`では従来の分割ハンドルを利用する。
- FFTの`Run FFT`／実行中の`Cancel`をスクロール領域の上に固定する。
  対象一覧、結果数値と標本情報、計算詳細は折りたたみ、窓・振幅表示・周波数設定へ直接アクセスできる。
  スペクトルと色付き凡例を結果数値より先に表示する。
- 大きなFFT区間の警告は計算詳細を閉じていても表示する。
- ウィンドウ縮小時は表示高だけを制限し、ユーザーが選んだパネル高は拡大後に戻す。
  パネル間の余白も波形入力の除外領域へ含め、解析欄のクリップ領域を明示する。
- 統計値の横スクロール領域を実際の表示幅へ制限し、右端のSlope値がクリップされても
  その全桁へスクロールできるようにする。
- 表示切替・開閉ではCursor A/Bと完了済みFFTを保持する。解析の数値計算、対象数上限、
  入力変更時の結果無効化、設定ファイルの保存範囲は変更していない。

## 検証

基準revisionは`829dd372875d7107d9afc4e1b333a41f77808747`、macOS 27.0.1。
入力は[前回のモータシミュレーション由来Parquet](rust_motor_log_continuous_validation_results.md)の3本を使用した。
実機実測ログではない。[環境記録](../study/diary/assets/2026-10-27-compact-analysis/environment.json)に
初期GUI検証版と最終版の実行ファイルhashを分けて保存している。

**確認状況:** 初期高さ360px・入力除外領域・警告表示を調整した版で、統計12行、FFTの設定・実行・
200 Hzスペクトル全体、Split、開閉と表示復元をネイティブで再確認した。
その後、統計右端のSlope値に桁切れを見つけ、横スクロール領域の幅制限を修正した。
画像01は最終行への到達、画像14・15は修正後の横移動の証跡とする。
幅制限を外すと失敗する回帰テストを追加し、全240自動テストとrelease buildが成功した。
最新版のネイティブでも横スクロールバーでSlope値の全桁を表示し、左端へ戻してA列を再表示できた。

| 確認 | 結果・証跡 |
|---|---|
| 統計の全行 | 1280×832のまま360px欄に12行全件を表示した。[最終表示高の画面](../study/diary/assets/2026-10-27-compact-analysis/08-final-statistics.jpg)。初期320px版の[画像01](../study/diary/assets/2026-10-27-compact-analysis/01-statistics-last-row.jpg)は最終行`Hi_a #3`への縦スクロールだけを示す |
| 統計の右端列 | 最新版1280×832で横スクロールバーを操作し、Slope値`288.466253`／`263.529631`等の全桁を確認。左端をクリックするとA列へ戻った。[右端](../study/diary/assets/2026-10-27-compact-analysis/14-horizontal-scroll-fixed.jpg)、[A列へ復帰](../study/diary/assets/2026-10-27-compact-analysis/15-horizontal-scroll-return.jpg) |
| FFT設定と再実行 | 初期版で対象のClear→First visible、実行・再実行・周波数設定・plot Hoverを確認。360px版でも設定・実行と200 Hzスペクトル全体を表示した。[最終ビルドのスペクトル](../study/diary/assets/2026-10-27-compact-analysis/16-final-build-fft.jpg) |
| 結果のスクロール | 結果数値へスクロールしても上部の実行操作へアクセスできた。[画面](../study/diary/assets/2026-10-27-compact-analysis/03-fft-results-scrolled.jpg) |
| 分割・開閉・表示復元 | 360px版でSplitへ切り替え、Collapse→FFTで再展開、Hide→表示を確認し、Cursorと完了済みFFTを保持した。[Split](../study/diary/assets/2026-10-27-compact-analysis/10-final-split.jpg)、[復元後](../study/diary/assets/2026-10-27-compact-analysis/12-final-restored-fft.jpg) |
| ウィンドウ拡大と復帰 | 1920×960へ拡大後、1280×832へ戻してCursorとFFT結果を保持した。[復帰後](../study/diary/assets/2026-10-27-compact-analysis/07-window-restored.jpg) |

自動テストでは次の実際の描画・入力経路を確認した。

- 1280×800の内容領域でタブ・Collapse・Hideをクリックし、解析状態を保持する。
- 1024×600を含む縮小・拡大で操作ボタンが表示され、保存したパネル高が失われない。
- 高さ112pxのFFT欄をホイールでスクロールし、固定位置のRun／Cancelをクリックできる。
- 計算詳細を閉じた状態でも大きなFFT区間の警告が表示される。
- 統計の右端まで横スクロールするとSlope値の全桁がクリップ領域内へ入る。
  明示的な幅制限を外すと失敗するため、今回の桁切れに対する回帰テストとなる。

ネイティブの今回のFFTは約10msで完了したため、実行中のCancel操作は上記の自動入力テストで確認した。
1024×600も自動描画テストの範囲であり、ネイティブでの操作合格と混同しない。
統計表のネイティブ横移動で成功した操作はスクロールバーであり、横ホイールによる移動は確認できなかった。

## 残る範囲

この確認はmacOSの上記ウィンドウ寸法と3ソース・12チャンネルに限定する。
高DPI設定を変えた比較、さらに小さい画面、他OS・配布版は未確認。
実機実測ログ、多数の大規模ソース、長時間の安定性、OS/GPUを含む提示遅延は別の残件として維持する。
