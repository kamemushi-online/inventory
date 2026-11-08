1. 正確な解では大きさが保たれる部分

baseの状態方程式は次のとおりです。

$$
\dot\psi_d = v_d - R_s i_d + \omega_e\psi_q,\qquad
\dot\psi_q = v_q - R_s i_q - \omega_e\psi_d
$$

このうち速度起電力の項だけを取り出すと、次の形になります。

$$
\dot{\boldsymbol\psi} = \omega_e A\boldsymbol\psi,\qquad
A=\begin{bmatrix}0&1\\-1&0\end{bmatrix}
$$
$A$ は歪対称行列なので、正確な解は磁束ベクトルを回転させるだけで、大きさ $|\boldsymbol\psi|$ を変えません。
この項がやり取りする電力 $\omega_e(\psi_q i_d - \psi_d i_q)$ は、ちょうど機械出力 $-T_e\omega_m$ に等しくなります。
つまり、エネルギーを生みも消しもせず、電気と機械の間で受け渡すだけの項です。
2. 前進Eulerでは円の接線方向に進む

1ステップの更新は次のとおりです。

$$\boldsymbol\psi_{k+1} = (I + \omega_e T_s A)\,\boldsymbol\psi_k$$

固有値は $1 \pm j\omega_e T_s$ で、その絶対値は次のようになります。

$$\sqrt{1+(\omega_e T_s)^2} \approx 1 + \tfrac{1}{2}(\omega_e T_s)^2 > 1$$

幾何学的には、円弧に沿って回るべきところを、接線方向へ長さ $\omega_e T_s|\boldsymbol\psi|$ だけまっすぐ進むことに当たります。そのため半径が毎ステップ $(\omega_e T_s)^2/2$ の割合で外へ膨らみます。

3. 偽の電圧と偽の電力として現れる

この膨らみを1ステップあたりの変化率に直すと、磁束と同じ向きに次の電圧を余分に加えたのと同じです。

$$\boldsymbol v_{\text{err}} \approx \frac{\omega_e^2 T_s}{2}\,\boldsymbol\psi$$

電力不変dqでは電力が $\boldsymbol v\cdot\boldsymbol i$ なので、巻線に注入される偽の電力は次のとおりです。

$$P_{\text{err}} \approx \frac{\omega_e^2 T_s}{2}\,(\boldsymbol\psi\cdot\boldsymbol i)$$

この電力は、端子電圧にも銅損にも機械出力にも含まれていません。そのため「入力電力 ＝ 銅損 ＋ 機械出力 ＋ 磁気エネルギーの変化」の収支が合わなくなります。

4. 符号と大きさ

線形モデルでは、$\boldsymbol\psi\cdot\boldsymbol i = \psi_f i_d + L_d i_d^2 + L_q i_q^2$ です。

状態	$\boldsymbol\psi\cdot\boldsymbol i$	現れ方
iqが主体（通常のトルク運転）	正	エネルギーを生み出す（負の抵抗のように働き、銅損を過小評価する）
弱め磁束の途中（例：id = −50 A）	負	エネルギーを吸い込む（余分な減衰のように働く）
磁束をほぼ打ち消した点（ψ ≈ 0、ASCに近い点）	ほぼ0	ほとんど影響しない

大きさについての要点は次のとおりです。

誤差の係数 $\omega_e^2 T_s/2$ は速度の2乗と刻み幅に比例します。速度が2倍なら4倍、10倍なら100倍です。
10000 rpm（ωe ≈ 4190 rad/s）、Ts = 1 µs では、係数は約8.8 s⁻¹ です。
これは物理的な減衰率 Rs/Lq ≈ 16 s⁻¹、Rs/Ld ≈ 51 s⁻¹ と同程度の大きさです。そのため、放電の要である銅損に対して無視できない割合になります。先の「id = −50 Aで銅損の約2割」という見積もりはこの比率から来ています。
刻みを大きくして数値上の増幅が物理的な減衰を上回ると、計算そのものが発散します。

なお、これらは線形モデルでの見積もりです。実際の大きさは、エネルギー収支のテストで確かめる必要があります。

5. 対策（baseは変えず、feature側で選べるもの）
刻みを小さくする: 誤差は $T_s$ に比例して小さくなりますが、計算時間が増えます。
RK2（Heun）: 増幅率が $\sqrt{1+(\omega_e T_s)^4/4}$ になり、1ステップあたりの誤差は約 $(\omega_e T_s)^4/8$ と、ほぼ無視できる大きさになります。
回転を厳密に解く: 回転項は回転行列 $e^{\omega_e T_s A}$ で正確に回し、抵抗と印加電圧の項だけをEulerで積分します。
静止αβ座標で磁束を積分する: $\dot{\boldsymbol\psi}{\alpha\beta} = \boldsymbol v{\alpha\beta} - R_s\boldsymbol i_{\alpha\beta}$ には回転項がないので、この誤差は生じません。電流はθでdqに回してからマップで引きます。
