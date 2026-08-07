import os
import numpy as np
from PIL import Image
import pillow_heif
pillow_heif.register_heif_opener()
from photutils.detection import DAOStarFinder
from astropy.stats import sigma_clipped_stats
import twirl
from validate_plate_solving import find_file, load_local_catalog, fit_radial_distortion_lmfit, fit_sip_distortion_lmfit

def run_heic_fit():
    heic_path = find_file('IMG_8556.HEIC')
    if not heic_path:
        heic_path = find_file('data/IMG_8556.HEIC')
    if not heic_path or not os.path.exists(heic_path):
        print("IMG_8556.HEIC not found.")
        return

    print(f"=== Loading HEIC File: {heic_path} ===")
    img = Image.open(heic_path).convert('L')
    data = np.array(img, dtype=float)
    height, width = data.shape
    print(f"Image Resolution: {width} x {height} px")

    mean, median, std = sigma_clipped_stats(data, sigma=3.0)
    daofind = DAOStarFinder(fwhm=3.5, threshold=3.5 * std)
    sources = daofind(data - median)
    
    if sources is None or len(sources) == 0:
        print("No stars detected in HEIC file.")
        return

    x_col = 'x_centroid' if 'x_centroid' in sources.colnames else 'xcentroid'
    y_col = 'y_centroid' if 'y_centroid' in sources.colnames else 'ycentroid'
    stars_xy = np.array([sources[x_col], sources[y_col]]).T
    
    if 'peak' in sources.colnames:
        idx = np.argsort(sources['peak'])[::-1]
        stars_xy = stars_xy[idx]

    print(f"Detected {len(stars_xy)} stars from HEIC image.")
    print(f"Top 10 detected star pixel positions (x, y):\n{stars_xy[:10]}")

    cat_radecs, cat_names, cat_vmags = load_local_catalog()
    # Filter catalog around target sky region (RA=335 deg, Dec=42 deg)
    ra_diff = np.abs(cat_radecs[:, 0] - 335.0)
    dec_diff = np.abs(cat_radecs[:, 1] - 42.0)
    region_mask = (ra_diff < 35.0) & (dec_diff < 35.0)
    cat_search_radecs = cat_radecs[region_mask]
    if len(cat_search_radecs) > 25:
        cat_search_radecs = cat_search_radecs[:25]

    wcs = twirl.compute_wcs(stars_xy[:12], cat_search_radecs, tolerance=25)
    if wcs is None:
        print("Twirl WCS solve failed with local catalog fallback.")
        return

    crval_ra, crval_dec = wcs.wcs.crval
    print(f"SUCCESS: HEIC WCS Solved! Center RA={crval_ra:.4f}°, Dec={crval_dec:.4f}°")

    cat_pixels = np.array(wcs.world_to_pixel_values(cat_search_radecs))
    match_threshold_px = 35.0
    
    det_matched_xy = []
    cat_matched_xy = []
    dx_list = []
    dy_list = []
    dist_list = []

    for s_xy in stars_xy:
        dists = np.linalg.norm(cat_pixels - s_xy, axis=1)
        min_idx = np.argmin(dists)
        min_dist = dists[min_idx]
        if min_dist <= match_threshold_px:
            cat_xy = cat_pixels[min_idx]
            det_matched_xy.append(s_xy)
            cat_matched_xy.append(cat_xy)
            dx_list.append(s_xy[0] - cat_xy[0])
            dy_list.append(s_xy[1] - cat_xy[1])
            dist_list.append(min_dist)

    matched_count = len(det_matched_xy)
    print(f"\nMatched {matched_count} star catalog positions for HEIC file.")

    if matched_count > 3:
        center_x, center_y = width / 2.0, height / 2.0
        max_radius = np.hypot(center_x, center_y)
        det_arr = np.array(det_matched_xy)
        cat_arr = np.array(cat_matched_xy)

        u_det = det_arr[:, 0] - center_x
        v_det = det_arr[:, 1] - center_y
        u_cat = cat_arr[:, 0] - center_x
        v_cat = cat_arr[:, 1] - center_y

        du_data = np.array(dx_list)
        dv_data = np.array(dy_list)

        norm_r = np.hypot(u_det, v_det) / max_radius
        dr_pixels = np.array(dist_list)

        print(f"\n========================================================")
        print(f"✦ lmfit DISTORTION FITTING ON HEIC STAR POSITIONS ✦")
        print(f"========================================================")
        fit_radial_distortion_lmfit(norm_r, dr_pixels, max_radius)
        fit_sip_distortion_lmfit(u_cat, v_cat, du_data, dv_data, order=3)

if __name__ == '__main__':
    run_heic_fit()
