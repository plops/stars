import numpy as np
import lmfit
from validate_plate_solving import fit_radial_distortion_lmfit, fit_sip_distortion_lmfit

def test_lmfit_radial_distortion():
    np.random.seed(42)
    # Generate synthetic radial distortion data
    max_radius = 1000.0
    r_norm = np.linspace(0.05, 0.95, 50)
    true_k1 = 0.005
    true_k2 = -0.001
    
    noise = np.random.normal(0, 0.5, size=r_norm.shape)
    dr_pixels = (true_k1 * r_norm**3 + true_k2 * r_norm**5) * max_radius + noise
    
    res, param_errors = fit_radial_distortion_lmfit(r_norm, dr_pixels, max_radius)
    
    assert 'k1' in param_errors
    assert 'k2' in param_errors
    k1_val, k1_err = param_errors['k1']
    k2_val, k2_err = param_errors['k2']
    
    assert abs(k1_val - true_k1) < 0.002
    assert k1_err > 0.0
    assert k2_err > 0.0
    print(f"\n[Test Passed] Radial k1: {k1_val:.6e} ± {k1_err:.6e}")
    print(f"[Test Passed] Radial k2: {k2_val:.6e} ± {k2_err:.6e}")

def test_lmfit_sip_distortion():
    np.random.seed(42)
    x = np.linspace(-400, 400, 15)
    y = np.linspace(-400, 400, 15)
    u_grid, v_grid = np.meshgrid(x, y)
    u_cat = u_grid.flatten()
    v_cat = v_grid.flatten()
    
    # Synthetic SIP distortion: A_2_0 = 1e-5, B_0_2 = 2e-5
    du_data = 1e-5 * (u_cat**2) + np.random.normal(0, 0.01, size=u_cat.shape)
    dv_data = 2e-5 * (v_cat**2) + np.random.normal(0, 0.01, size=v_cat.shape)
    
    res, param_errors = fit_sip_distortion_lmfit(u_cat, v_cat, du_data, dv_data, order=2)
    
    assert 'A_2_0' in param_errors
    assert 'B_0_2' in param_errors
    a_2_0_val, a_2_0_err = param_errors['A_2_0']
    b_0_2_val, b_0_2_err = param_errors['B_0_2']
    
    assert abs(a_2_0_val - 1e-5) < 5e-6
    assert a_2_0_err > 0.0
    assert b_0_2_err > 0.0
    print(f"\n[Test Passed] SIP A_2_0: {a_2_0_val:.6e} ± {a_2_0_err:.6e}")
    print(f"[Test Passed] SIP B_0_2: {b_0_2_val:.6e} ± {b_0_2_err:.6e}")

if __name__ == "__main__":
    test_lmfit_radial_distortion()
    test_lmfit_sip_distortion()
    print("\nAll lmfit Python fitting tests completed successfully!")
