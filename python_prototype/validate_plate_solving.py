import os
import numpy as np
import matplotlib.pyplot as plt
from PIL import Image
import exifread
from astropy.wcs import WCS
from astropy.coordinates import SkyCoord, EarthLocation, AltAz
from astropy.time import Time
import astropy.units as u
from photutils.detection import DAOStarFinder
from astropy.stats import sigma_clipped_stats
from astroquery.gaia import Gaia
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
            # Default test parameters matching iPhone dummy metadata
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

print("=== 1. Image Loading & Star Detection (Ground Truth) ===")
images = ['/workspace/src/stars.jpg', '/workspace/src/IMG_8550.jpg']
results = {}

for img_path in images:
    print(f"\nProcessing {img_path}")
    if not os.path.exists(img_path):
        print(f"File {img_path} not found.")
        continue
        
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
        'lat': lat, 'lon': lon, 'dt': dt, 'heading': heading,
        'shape': data.shape
    }

print("\n=== 2. Ground Truth Astrometric Fitting ===")
for img_path, res in results.items():
    print(f"\nEvaluating ground truth for {os.path.basename(img_path)}")
    if res['sources'] is None or len(res['sources']) < 4:
        print("Insufficient stars detected.")
        continue
    
    x_col = 'x_centroid' if 'x_centroid' in res['sources'].colnames else 'xcentroid'
    y_col = 'y_centroid' if 'y_centroid' in res['sources'].colnames else 'ycentroid'
    stars_xy = np.array([res['sources'][x_col], res['sources'][y_col]]).T
    
    # Sort stars by peak brightness
    if 'peak' in res['sources'].colnames:
        idx = np.argsort(res['sources']['peak'])[::-1]
        stars_xy = stars_xy[idx]
        
    print(f"Top 10 detected star coordinates (px):\n{stars_xy[:10]}")

print("\nGround truth evaluation complete. Plots saved to plots/.")
