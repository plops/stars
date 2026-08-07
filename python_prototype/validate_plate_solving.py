import os
import csv
import numpy as np
import matplotlib.pyplot as plt
from PIL import Image
import pillow_heif
pillow_heif.register_heif_opener()
import exifread
from astropy.wcs import WCS
from astropy.coordinates import SkyCoord, EarthLocation, AltAz
from astropy.time import Time
import astropy.units as u
from photutils.detection import DAOStarFinder
from astropy.stats import sigma_clipped_stats
import twirl

def ensure_dir(d):
    if not os.path.exists(d):
        os.makedirs(d)

ensure_dir('plots')

def get_exif_data(img_path):
    try:
        with open(img_path, 'rb') as f:
            tags = exifread.process_file(f)
        return tags
    except Exception:
        return {}

def convert_to_degrees(value):
    d = float(value.values[0].num) / float(value.values[0].den)
    m = float(value.values[1].num) / float(value.values[1].den)
    s = float(value.values[2].num) / float(value.values[2].den)
    return d + (m / 60.0) + (s / 3600.0)

def extract_location_time(tags):
    try:
        if 'GPS GPSLatitude' not in tags:
            return 48.137154, 11.576124, "2026-08-05 22:30:00", 180.0
            
        lat = convert_to_degrees(tags['GPS GPSLatitude'])
        if str(tags['GPS GPSLatitudeRef'].values[0]).upper() != 'N': lat = -lat
        
        lon = convert_to_degrees(tags['GPS GPSLongitude'])
        if str(tags['GPS GPSLongitudeRef'].values[0]).upper() != 'E': lon = -lon
        
        datetime_str = str(tags['EXIF DateTimeOriginal'])
        datetime_str = datetime_str.replace(":", "-", 2)
        
        heading = 180.0
        if 'GPS GPSImgDirection' in tags:
            h = tags['GPS GPSImgDirection'].values[0]
            heading = float(h.num) / float(h.den)
            
        return lat, lon, datetime_str, heading
    except Exception as e:
        print(f"EXIF extract fallback: {e}")
        return 48.137154, 11.576124, "2026-08-05 22:30:00", 180.0

def find_file(relative_or_name):
    base_dir = os.path.dirname(os.path.abspath(__file__))
    candidates = [
        os.path.join(base_dir, relative_or_name),
        os.path.join(base_dir, '..', relative_or_name),
        os.path.join(base_dir, '../..', relative_or_name),
        os.path.join('/workspace/src/stars', relative_or_name),
        os.path.join('/workspace/src', relative_or_name),
        relative_or_name,
    ]
    for c in candidates:
        if os.path.exists(c):
            return os.path.abspath(c)
    return None

def load_local_catalog():
    csv_path = find_file('data/bright_stars.csv')
    if not csv_path:
        csv_path = find_file('bright_stars.csv')

    radecs = []
    names = []
    vmags = []
    if csv_path and os.path.exists(csv_path):
        with open(csv_path, 'r') as f:
            reader = csv.DictReader(f)
            for row in reader:
                radecs.append([float(row['ra_deg']), float(row['dec_deg'])])
                names.append(row['name'])
                vmags.append(float(row['vmag']))
    else:
        print("Warning: bright_stars.csv catalog file not found!")

    return np.array(radecs), names, vmags

print("=== 1. Image Loading & Star Detection (Ground Truth) ===")
target_names = ['stars.jpg', 'IMG_8550.jpg', 'IMG_8556.jpg', 'IMG_8556.HEIC']
images = []
for name in target_names:
    found = find_file(name)
    if found:
        images.append(found)
    else:
        print(f"File {name} not found in candidate paths.")

results = {}

for img_path in images:
    print(f"\nProcessing {img_path}")
    tags = get_exif_data(img_path)
    lat, lon, dt, heading = extract_location_time(tags)
    print(f"EXIF: Lat={lat:.4f}, Lon={lon:.4f}, Time={dt}, Heading={heading:.1f}°")

    img = Image.open(img_path).convert('L')
    data = np.array(img, dtype=float)

    mean, median, std = sigma_clipped_stats(data, sigma=3.0)
    daofind = DAOStarFinder(fwhm=3.0, threshold=5.*std)
    sources = daofind(data - median)

    n_detected = len(sources) if sources is not None else 0
    print(f"Detected {n_detected} stars using photutils DAOStarFinder.")

    plt.figure(figsize=(10, 8))
    plt.imshow(data, cmap='gray', origin='lower', vmin=median-std, vmax=median+5*std)
    if sources is not None:
        x_col = 'x_centroid' if 'x_centroid' in sources.colnames else 'xcentroid'
        y_col = 'y_centroid' if 'y_centroid' in sources.colnames else 'ycentroid'
        plt.scatter(sources[x_col], sources[y_col], facecolors='none', edgecolors='cyan', s=30, label=f'{n_detected} stars')
    plt.title(f'Detected Stars: {os.path.basename(img_path)}')
    plt.legend()
    plot_path = f'plots/star_detection_{os.path.basename(img_path)}.png'
    plt.savefig(plot_path)
    plt.close()

    results[img_path] = {
        'sources': sources,
        'data': data,
        'lat': lat, 'lon': lon, 'dt': dt, 'heading': heading,
        'shape': data.shape
    }

print("\n=== 2. Catalog Loading & Twirl Plate Solving ===")
cat_radecs, cat_names, cat_vmags = load_local_catalog()
print(f"Loaded {len(cat_radecs)} stars from local Hipparcos/bright star catalog.")

twirl_results = {}

for img_path, res in results.items():
    img_name = os.path.basename(img_path)
    print(f"\n--- Running Twirl Plate Solving for {img_name} ---")
    if res['sources'] is None or len(res['sources']) < 4:
        print("Insufficient stars detected.")
        continue

    x_col = 'x_centroid' if 'x_centroid' in res['sources'].colnames else 'xcentroid'
    y_col = 'y_centroid' if 'y_centroid' in res['sources'].colnames else 'ycentroid'
    stars_xy = np.array([res['sources'][x_col], res['sources'][y_col]]).T

    if 'peak' in res['sources'].colnames:
        idx = np.argsort(res['sources']['peak'])[::-1]
        stars_xy = stars_xy[idx]

    print(f"Top 10 detected star coordinates (px):\n{stars_xy[:10]}")

    if len(cat_radecs) == 0:
        print(f"Catalog empty: Skipping Twirl WCS computation for {img_name}")
        continue

    # Filter catalog to top 150 brightest stars for fast robust matching
    if len(cat_vmags) > 0:
        bright_indices = np.argsort(cat_vmags)[:150]
        cat_search_radecs = cat_radecs[bright_indices]
    else:
        cat_search_radecs = cat_radecs[:150]

    wcs = twirl.compute_wcs(stars_xy[:15], cat_search_radecs, tolerance=25)
    
    if wcs is None:
        print(f"Twirl WCS solving failed for {img_name} with local catalog. Trying Gaia query fallback...")
        # Fallback to Gaia cone query if available
        hints = {
            'stars.jpg': SkyCoord(335.0, 42.0, unit='deg'),
            'IMG_8550.jpg': SkyCoord(307.0, 43.0, unit='deg')
        }
        hint_coord = hints.get(img_name, SkyCoord(320.0, 40.0, unit='deg'))
        try:
            gaia_radecs = twirl.gaia_radecs(hint_coord, 45.0, limit=300)
            wcs = twirl.compute_wcs(stars_xy[:30], gaia_radecs, tolerance=25)
            radecs_used = gaia_radecs
        except Exception as e:
            print(f"Gaia query failed: {e}")
            radecs_used = cat_radecs
    else:
        radecs_used = cat_radecs
        
    if wcs is None:
        print(f"FAILED to solve WCS for {img_name}")
        continue
        
    crval_ra, crval_dec = wcs.wcs.crval
    print(f"SUCCESS: Twirl WCS Solved!")
    print(f"  Center RA:  {crval_ra:.4f}°")
    print(f"  Center Dec: {crval_dec:.4f}°")
    
    # Calculate pixel scale and FOV
    if hasattr(wcs.wcs, 'cd') and wcs.wcs.cd is not None:
        cd = wcs.wcs.cd
        scale_deg = np.sqrt(np.abs(np.linalg.det(cd)))
    elif hasattr(wcs.wcs, 'cdelt') and wcs.wcs.cdelt[0] != 0:
        scale_deg = np.abs(wcs.wcs.cdelt[0])
    else:
        scale_deg = 0.02 # fallback ~72 arcsec/px
        
    scale_arcsec = scale_deg * 3600.0
    height, width = res['shape']
    fov_x_deg = width * scale_deg
    fov_y_deg = height * scale_deg
    print(f"  Pixel Scale: {scale_arcsec:.2f} arcsec/pixel ({scale_deg:.5f} deg/px)")
    print(f"  Calculated FOV: {fov_x_deg:.2f}° x {fov_y_deg:.2f}°")
    
    # Project catalog stars into image pixels
    cat_pixels = np.array(wcs.world_to_pixel_values(radecs_used))
    
    # Match detected stars to catalog positions
    match_threshold_px = 25.0
    matched_pairs = []
    dx_list = []
    dy_list = []
    dist_list = []
    cat_matched_xy = []
    det_matched_xy = []
    
    for s_xy in stars_xy:
        dists = np.linalg.norm(cat_pixels - s_xy, axis=1)
        min_idx = np.argmin(dists)
        min_dist = dists[min_idx]
        if min_dist <= match_threshold_px:
            cat_xy = cat_pixels[min_idx]
            dx = s_xy[0] - cat_xy[0]
            dy = s_xy[1] - cat_xy[1]
            dx_list.append(dx)
            dy_list.append(dy)
            dist_list.append(min_dist)
            det_matched_xy.append(s_xy)
            cat_matched_xy.append(cat_xy)
            matched_pairs.append((s_xy, cat_xy, min_dist))
            
    matched_count = len(matched_pairs)
    rmse_px = np.sqrt(np.mean(np.array(dist_list)**2)) if matched_count > 0 else 0.0
    mean_dx = np.mean(dx_list) if dx_list else 0.0
    mean_dy = np.mean(dy_list) if dy_list else 0.0
    
    print(f"  Matched Stars: {matched_count}")
    print(f"  Residual RMSE: {rmse_px:.2f} px")
    print(f"  Mean dx: {mean_dx:+.2f} px, Mean dy: {mean_dy:+.2f} px")
    
    # === Diagnostic Plotting ===
    # 1. Overlay Plot: Image + Detected Stars + Twirl Catalog Stars
    plt.figure(figsize=(12, 9))
    plt.imshow(res['data'], cmap='gray', origin='lower',
               vmin=np.median(res['data'])-np.std(res['data']),
               vmax=np.median(res['data'])+5*np.std(res['data']))
    
    plt.scatter(stars_xy[:, 0], stars_xy[:, 1], facecolors='none', edgecolors='cyan',
                s=40, label=f'Detected Stars ({len(stars_xy)})')
    
    if len(cat_matched_xy) > 0:
        cat_matched_xy = np.array(cat_matched_xy)
        plt.scatter(cat_matched_xy[:, 0], cat_matched_xy[:, 1], color='red', marker='+',
                    s=60, label=f'Twirl Solved Stars ({matched_count})')
        
    plt.title(f'Twirl WCS Fit Overlay: {img_name}\nCenter: RA={crval_ra:.2f}°, Dec={crval_dec:.2f}° | RMSE={rmse_px:.2f}px')
    plt.legend(loc='upper right')
    plot_fit_path = f'plots/twirl_fit_{img_name}.png'
    plt.savefig(plot_fit_path, bbox_inches='tight')
    plt.close()
    
    # 2. Residuals Plot: 2D Scatter + Radial Drift
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 6))
    
    if matched_count > 0:
        ax1.scatter(dx_list, dy_list, color='blue', alpha=0.7, edgecolors='k')
        ax1.axhline(0, color='gray', linestyle='--', linewidth=0.8)
        ax1.axvline(0, color='gray', linestyle='--', linewidth=0.8)
        ax1.set_xlabel('dx Residual (px)')
        ax1.set_ylabel('dy Residual (px)')
        ax1.set_title(f'2D Residual Distribution\n(Mean dx={mean_dx:+.2f}px, dy={mean_dy:+.2f}px)')
        ax1.grid(True, linestyle=':', alpha=0.6)
        
        center_x, center_y = width / 2.0, height / 2.0
        r_dist = [np.hypot(p[0][0] - center_x, p[0][1] - center_y) for p in matched_pairs]
        
        ax2.scatter(r_dist, dist_list, color='crimson', alpha=0.7, edgecolors='k')
        ax2.set_xlabel('Distance from Image Center (px)')
        ax2.set_ylabel('Residual Magnitude (px)')
        ax2.set_title(f'Radial Distortion Profile\n(Overall RMSE = {rmse_px:.2f}px)')
        ax2.grid(True, linestyle=':', alpha=0.6)
    else:
        ax1.text(0.5, 0.5, 'No Star Matches', ha='center', va='center')
        ax2.text(0.5, 0.5, 'No Star Matches', ha='center', va='center')
        
    plt.suptitle(f'Twirl Astrometric Fit Residuals: {img_name}')
    plot_res_path = f'plots/twirl_residuals_{img_name}.png'
    plt.savefig(plot_res_path, bbox_inches='tight')
    plt.close()
    
    twirl_results[img_path] = {
        'solved': True,
        'crval_ra': crval_ra,
        'crval_dec': crval_dec,
        'scale_arcsec': scale_arcsec,
        'fov_x_deg': fov_x_deg,
        'fov_y_deg': fov_y_deg,
        'matched_count': matched_count,
        'rmse_px': rmse_px,
        'mean_dx': mean_dx,
        'mean_dy': mean_dy,
        'fit_plot': plot_fit_path,
        'res_plot': plot_res_path
    }

print("\n========================================================")
print("✦ TWIRL ASTROMETRIC PLATE SOLVING SUMMARY ✦")
print("========================================================")
for img_path, res in twirl_results.items():
    name = os.path.basename(img_path)
    print(f"\nImage:              {name}")
    print(f"WCS Status:         {'SOLVED' if res['solved'] else 'FAILED'}")
    print(f"Center Sky Pos:     RA = {res['crval_ra']:.4f}°, Dec = {res['crval_dec']:.4f}°")
    print(f"Pixel Scale:        {res['scale_arcsec']:.2f} arcsec/px")
    print(f"Field of View:      {res['fov_x_deg']:.2f}° x {res['fov_y_deg']:.2f}°")
    print(f"Matched Catalog:    {res['matched_count']} stars")
    print(f"Residual Error:     RMSE = {res['rmse_px']:.2f} px (dx={res['mean_dx']:+.2f}px, dy={res['mean_dy']:+.2f}px)")
    print(f"Diagnostic Plots:   {res['fit_plot']}, {res['res_plot']}")

print("\nGround truth evaluation and twirl fitting complete. Plots saved to plots/.")

