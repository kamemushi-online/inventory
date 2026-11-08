I have read all 11 pages of the paper. Next, I will examine the plant configuration on the Simulink side.

I have reviewed the full paper and the plant configurations in both Rust and Simulink. I will add the conclusions and estimates to the study memo.

The existing base can be used. The motor electrical model (Eq. (1) in the paper) is already available in Rust and is more detailed than the SPM model used in the paper, as it also accounts for magnetic saturation in an IPM motor. The only missing elements are rotor inertia and capacitor voltage dynamics. Both can be implemented in the feature-specific Rust program by extending the shared Rust components (`sim-core`) without modifying the base.

The additional findings are summarized in §7 of `features/dclink_discharge/design/plant_model_study_v1.md`, and `progress.md` has also been updated to reflect the new assumptions. These changes have not yet been committed.

## Correspondence Between the Paper's Power Balance Model and the Current Base

| Equation in Paper | Description | Current Base | Required Action |
|---|---|---|---|
| (1) | dq-axis voltage equations and torque | `IpmMotorModel::step(vd, vq, ωe)`. IPM model using the Prius flux maps. An SPM can also be represented by using a linear model with Ld = Lq. | Use as is |
| (2) | J·dωm/dt = Te − Bm·ωm | Not implemented (speed is an external input) | Add on the feature side. Since `step` receives ωe at every call, the calculated speed can be passed as a state variable. |
| (3) | C·dVdc/dt = i_dc | Not implemented (Vdc is constant) | Add on the feature side |
| (4) | Vdc·i_dc = −(vd·id + vq·iq) − P_con | Not implemented | Calculate directly from the equation in the average-value model. In the switching model, calculate it from the conduction states of the inverter legs (which agrees with Eq. (4) under lossless conditions). |

The paper uses an amplitude-invariant transformation (with a factor of 3/2), whereas the base uses a power-invariant transformation. To reproduce the paper, ψf and the currents must be multiplied by √(3/2).

## Proposed Configuration

- **A. Average-Value Model (Power Balance Model from the Paper)**
  - Add the voltage limit |v| ≤ Vdc/√2, Eqs. (2), (3), and (4), and the auxiliary power supply load to the IPM model.
  - Since simulating deceleration from 10,000 rpm over several tens of seconds would be computationally expensive, this model will be used first to design the control law and ESO.

- **B. Switching Model**
  - Add the existing PWM and inverter components to Model A, and update Vdc every 1 µs.
  - Use this model to evaluate voltage errors and current ripple when Vdc is low.

- **Simulink**
  - The configuration is feasible. Since `three_phase_inverter.m` accepts Vdc as an input signal and the motor block accepts ωe as an input signal, the system can be closed by adding two integrators.
  - However, simulating several tens of seconds with a 1 µs time step is computationally expensive, and execution is limited to MATLAB on Windows. I recommend postponing this until the stage where the embedded C code is integrated.

## Estimates Under the New Assumptions (Prius Parameters, Linear Model)

1. **The Prius motor qualifies as a "low back-EMF motor."**
   - At 10,000 rpm, the no-load back-EMF (line-to-line peak value) is approximately 545 V, which is below 600 V.
   - The d-axis current required to cancel the magnet flux (characteristic current ψf/Ld) is approximately 107 A, which is within the range of the flux maps.

2. **Reducing the voltage below 60 V within 3 s is straightforward from an energy perspective.**
   - The energy required to discharge a 600 µF capacitor from 600 V to 55 V is approximately 107 J.
   - With copper loss of approximately 500 W at around 107 A, this would take roughly 0.2 s.
   - The main challenges are voltage overshoot and robustness.

3. **The operating point for maintaining 55 V at 10,000 rpm is almost identical to ASC.**
   - The allowable flux linkage is approximately 9.3 mWb, only about one-tenth of ψf.
   - Therefore, control must operate within a narrow range of id ≈ −107 ± 11 A and |iq| ≲ 3 A.

4. **Voltage holding can be sustained autonomously using the rotor's kinetic energy.**
   - The power supplied to the auxiliary power supply and capacitor is provided by the reduction in the rotor's kinetic energy. This is the key difference from a method that maintains the ASC state.
   - The back-EMF falls below 60 V at approximately 1,100 rpm or lower.
   - The holding duration depends on J. For example, if J = 0.03 kg·m², the holding time is approximately 30 s.

5. **For an IPM motor, the ESO gain α from the paper cannot be used directly.**
   - At id = −107 A, the reluctance term causes α to become approximately three times the value predicted by the SPM equation in the paper.
   - The equation must therefore be replaced with an IPM-specific formulation.

## Considerations for Numerical Integration

The forward Euler method used in the base
