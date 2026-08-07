# Analyse: Warum die Residuen so hoch sind

## Das Problem in Zahlen

| Metrik | Ist-Wert | Erwarteter Wert | Faktor |
|--------|----------|-----------------|--------|
| Median Residual | **29.0 px** | 2-5 px | ~6-15× zu hoch |
| RMSE | **38.3 px** | 3-8 px | ~5-13× zu hoch |
| Mean Residual | **33.1 px** | 2-5 px | ~7-17× zu hoch |
| Range | 0.86 - 78.7 px | 0.5 - 10 px | — |
| Gute Matches (<10px) | **9 von 81** (11%) | >80% | — |

## Diagnose: Es gibt **nicht ein Problem, sondern fünf**

### 1. 🔴 Match-Schwellenwert 80px ist viel zu hoch → Falsche Zuordnungen

```rust
// astrometry/mod.rs:258 und :348
if nearest.distance.sqrt() < 80.0 {  // ← WAY TOO LOOSE
```

**80 Pixel** Matching-Radius ist enorm. Bei einem ~65° FoV auf einem 4032px-Sensor entspricht das **~1.3° am Himmel**. Das bedeutet:
- Der Algorithmus akzeptiert fast jede Zuordnung, auch wenn der nächste Katalogstern gar nicht der richtige ist
- Sterne werden dem falschen Katalogstern zugeordnet → hohe Residuen
- Die 9 guten Matches (<10px) sind die zufällig richtigen Zuordnungen

**Evidenz:** HIP 102158 hat nur Δ=0.8° vom Feldzentrum, aber 34.9 px Residual. Wenn die Projektion richtig wäre, müsste ein Stern so nah am Zentrum <5px Residual haben. → Er ist wahrscheinlich dem **falschen detektierten Stern** zugeordnet.

### 2. 🔴 Gnomonic Projektion verwendet `x * f`, nicht `tan(x) * f` → Systematischer Skalenfehler

```rust
// astrometry/mod.rs:217
let scale_factor = (width as f64) * (focal_len_35mm / 36.0);
```

Die Zeile berechnet `scale_factor = width * f/36`. Dann wird `x_proj` (eine tangentiale Projektion in Radian) direkt mit diesem Faktor multipliziert. Das Problem:

Die gnomonic projection (`x_proj / cos_c`) ist bereits in **Radian** und liefert `tan(θ)` für den Winkelabstand θ. Aber die Pixel-Skalierung über `focal_len_35mm / 36.0` nimmt implizit an, dass `x_proj ≈ θ` (kleine Winkel), was bei einem **65° FoV** grob falsch ist.

Am Bildrand (32° vom Zentrum): `tan(32°) = 0.625` vs. `rad(32°) = 0.559` → **12% Fehler** → bei 2000px vom Zentrum = **~25px Fehler**. Das passt perfekt zu den beobachteten Residuen!

**Aber:** Die Projektion verwendet schon `/ cos_c` was korrekt ist für gnomonic. Das Problem ist subtiler — die `scale_factor`-Berechnung ist eine Linearisierung, die bei 26mm Brennweite (ultraweit) nicht gilt.

### 3. 🔴 Keine iterative Lösung — nur ein Single-Pass Forward-Match

```rust
// solve_plate: nur ein einziger Matching-Pass
for cat in &catalog {
    // project → find nearest → accept if < 80px
}
```

Moderne Plate Solver machen:
1. **Initial Match** (grob)
2. **Affine/TPS Transformation** aus den Matches berechnen
3. **Re-Projekt** mit der neuen Transformation
4. **Erneut matchen** mit engerem Schwellenwert
5. Wiederholen bis Konvergenz

Hier passiert nur Schritt 1. Es gibt keine Transformation, die die initialen Matchfehler korrigiert.

### 4. 🔴 Keine Outlier-Rejection (RANSAC, Sigma-Clipping)

Alle 81 Matches werden akzeptiert, auch offensichtlich falsche. Keine:
- Sigma-Clipping (z.B. Matches >3σ verwerfen)
- RANSAC-basierte robuste Schätzung
- Konsistenzprüfung (z.B. ob die Matches eine kohärente Transformation ergeben)

### 5. 🟡 Proper Motions werden nicht angewendet

Der Katalog hat `pmra_mas` und `pmdec_mas`, aber in `solve_plate()` werden die Positionen direkt aus `cat.ra_deg` und `cat.dec_deg` verwendet — ohne Korrektur für die Proper Motion zum Beobachtungszeitpunkt.

Für einen Stern wie HIP 107315 (Fomalhaut, mag 2.38) mit hoher Eigenbewegung kann das mehrere Pixel ausmachen. Aber im Vergleich zu den anderen Problemen ist das sekundär (~0.5-2 px Fehler).

---

## Warum es trotzdem "funktioniert"

Die 9 guten Matches (<10px) reichen aus, damit `is_solved = matches.len() >= 3` true wird. Der Algorithmus meldet einen erfolgreichen Plate Solve mit RMSE=38px, was astronomisch gesehen **kein** erfolgreicher Solve ist.

---

## Empfohlener Fix-Plan

### Phase 1: Sofort-Fixes (großer Impact, kleine Änderungen)

| # | Fix | Erwarteter Impact |
|---|-----|------------------|
| A | **Match-Schwellenwert 80px → 15px** reduzieren | Eliminiert ~50% der falschen Matches |
| B | **Sigma-Clipping** nach initialem Match (3σ) | Entfernt verbleibende Outlier |
| C | **Minimum-Match-Qualität**: `is_solved` an RMSE koppeln (z.B. `rmse < 15.0`) | Verhindert falsch-positive Solves |

### Phase 2: Algorithmische Verbesserungen

| # | Fix | Erwarteter Impact |
|---|-----|------------------|
| D | **2-Pass Matching**: Nach erstem Match eine 6-Parameter affine Transformation fitten, dann re-projizieren und erneut matchen mit 5px Schwellenwert | RMSE ~3-5px |
| E | **Proper Motion Korrektur** auf Beobachtungszeitpunkt | ~0.5-2px Verbesserung |

### Phase 3: Robuste Lösung

| # | Fix | Erwarteter Impact |
|---|-----|------------------|
| F | **RANSAC** für die affine Transformation | Robuster gegen falsche Matches |
| G | **Iterative Focal-Length-Refinement**: Brennweite als freien Parameter mitlösen | Beseitigt systematischen Skalenfehler |

---

## Soll ich die Fixes implementieren?

Phase 1 (A+B+C) ist schnell umsetzbar und sollte die Residuen von ~33px auf ~10-15px reduzieren.  
Phase 2 (D+E) bringt sie auf ~3-5px (professionelle Qualität).
