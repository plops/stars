# Code Review: SIP Distortion & Plate Solving Enhancement
**Datum:** 2026-08-07 · **Scope:** 10 Commits, 24 Dateien, ~11.100 Zeilen

---

## Gesamtbewertung

> [!IMPORTANT]
> **Architektonisch sauber, aber mit 3 echten Bugs in der Kernmathematik.** Die Commit-Struktur ist vorbildlich, die Modularisierung ist gut, und die Dokumentation ist vollständig. Allerdings gibt es in `sip.rs` und `astrometry/mod.rs` mathematische Fehler, die die Plate-Solve-Qualität signifikant beeinträchtigen können.

### Scorecard

| Dimension | Bewertung | Anmerkung |
|-----------|-----------|-----------|
| **Korrektheit** | ⭐⭐⭐ | 3 schwerwiegende Bugs in der Kernmathematik |
| **Code-Qualität** | ⭐⭐⭐⭐ | Sauber, gut benannt, `solve_plate` etwas lang |
| **Testabdeckung** | ⭐⭐⭐ | Basistests OK, Bugs nicht durch Tests abgefangen |
| **Dokumentation** | ⭐⭐⭐⭐⭐ | Plan, Task, Walkthrough, README — alles aktuell |
| **Commit-Struktur** | ⭐⭐⭐⭐½ | Conventional Commits, atomare Schritte, aber Commit-Bodies fehlen |

---

## 🔴 Schwerwiegende Bugs (müssen behoben werden)

### Bug 1: SIP-Fitting verwendet skalaren Residual statt direktionaler Fehler
**Datei:** [`sip.rs:107-108`](file:///workspace/src/stars/src/astrometry/sip.rs#L107-L108)

```rust
let du = m_star.residual_pixels.min(50.0);  // ← BUG
let dv = m_star.residual_pixels.min(50.0);  // ← BUG
```

`residual_pixels` ist der **skalare Abstand** (Betrag), nicht die direktionale Komponente. Beide Achsen erhalten identische Werte. Die SIP-A- und B-Koeffizienten werden dadurch identisch gefittet, was die Distortionskorrektur effektiv nutzlos macht.

**Fix:** Verwende die Richtungskomponenten:
```rust
let du = m_star.dx_pixels.clamp(-50.0, 50.0);
let dv = m_star.dy_pixels.clamp(-50.0, 50.0);
```

### Bug 2: RA-Mittelung ohne Wrap-Around-Behandlung
**Datei:** [`astrometry/mod.rs:497-498`](file:///workspace/src/stars/src/astrometry/mod.rs#L497-L498)

```rust
let ra_sum: f64 = matches.iter().map(|m| m.catalog_ra).sum();
(ra_sum / matches.len() as f64 + 360.0) % 360.0
```

Lineares Mitteln von Winkelwerten versagt am 0°/360°-Übergang. Wenn Sterne bei 359° und 1° liegen, ergibt der Mittelwert 180° statt 0°.

**Fix:** Vector-Averaging verwenden:
```rust
let (sin_sum, cos_sum): (f64, f64) = matches.iter()
    .map(|m| m.catalog_ra.to_radians())
    .fold((0.0, 0.0), |(s, c), ra| (s + ra.sin(), c + ra.cos()));
let mean_ra = sin_sum.atan2(cos_sum).to_degrees().rem_euclid(360.0);
```

### Bug 3: Quad-Hash auf unsortierte Katalogdaten
**Datei:** [`astrometry/mod.rs:382-399`](file:///workspace/src/stars/src/astrometry/mod.rs#L382-L399)

```rust
let projected_cat: Vec<(f64, f64)> = catalog.iter()  // ← unsortiert
    .filter_map(|cat| { ... })
    .collect();
// ...
for i in 0..projected_cat.len().min(12) {  // ← nimmt die ersten 12
```

Der Katalog ist nach RA sortiert (CSV-Reihenfolge). Die "ersten 12" projizierten Sterne sind daher **räumlich geclustert** und nicht die hellsten. Die Quad-Hashes basieren auf einer willkürlichen Teilmenge und können dadurch fehlschlagen.

**Fix:** `projected_cat` nach Helligkeit sortieren (dafür muss `vmag` mitgeführt werden):
```rust
let mut projected_cat: Vec<(f64, f64, f64)> = catalog.iter()
    .filter_map(|cat| {
        let (alt, az) = radec_to_altaz(...);
        altaz_to_pixel(...).map(|(x, y)| (x, y, cat.vmag))
    })
    .collect();
projected_cat.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
```

> [!WARNING]
> **Bug 1 und Bug 3 zusammen** erklären, warum der SIP-Fit möglicherweise keine Verbesserung gegenüber dem Basis-Solve bringt. Der Quad-Hash-Match funktioniert suboptimal (Bug 3), und die resultierende Distortionskorrektur ist effektiv ein No-Op (Bug 1).

---

## 🟠 Mittelschwere Issues (sollten behoben werden)

### 4. SIP Inverse: Stille Nicht-Konvergenz
**Datei:** [`sip.rs`](file:///workspace/src/stars/src/astrometry/sip.rs) ~L80-100

`apply_inverse()` gibt bei Nicht-Konvergenz (nach 20+ Iterationen) still die letzte Schätzung zurück. Empfehlung: `Result` oder `log::warn!`.

### 5. Refraktion bei Horizont (alt = 0°)
**Datei:** [`aberration/mod.rs`](file:///workspace/src/stars/src/aberration/mod.rs) ~L48

`atmospheric_refraction_correction(0.0)` gibt `0.0` zurück statt ~0.57° (34 Bogenminuten). Bedingung `alt_deg <= 0.0` → `alt_deg < 0.0`.

### 6. Zeitdrift ignoriert Deklination
**Datei:** [`validation/mod.rs:33-40`](file:///workspace/src/stars/src/validation/mod.rs#L33-L40)

```rust
heading_error_deg = (mean_dx * pixel_scale).clamp(-10.0, 10.0);
time_drift_seconds = heading_error_deg * 240.0;
```

Der Faktor 240 s/° (= 15°/h Erdrotation) gilt nur am Äquator. Bei δ=60° ist der Effekt doppelt so groß → `time_drift = heading_error * 240.0 / cos(dec)`.

### 7. Integrationstests mit stiller Skip-Logik
**Datei:** [`integration_tests.rs`](file:///workspace/src/stars/tests/integration_tests.rs)

Die Real-Image-Tests prüfen `if !img_path.exists() { return; }` — in CI werden sie still übersprungen und melden trotzdem "pass". Das gibt falsches Vertrauen. Verwende `#[ignore]` stattdessen.

---

## 🟡 Kleinere Issues (Verbesserungsvorschläge)

| # | Issue | Datei |
|---|-------|-------|
| 8 | Kein Logging bei Catalog-Fallback | [`catalog.rs`](file:///workspace/src/stars/src/astrometry/catalog.rs) L43-51 |
| 9 | `solve_plate` ~200 Zeilen monolithisch | [`astrometry/mod.rs`](file:///workspace/src/stars/src/astrometry/mod.rs) |
| 10 | Hardcoded Thresholds (2.0px, 6 Matches, 80px, 0.08 Quad) | [`astrometry/mod.rs`](file:///workspace/src/stars/src/astrometry/mod.rs) |
| 11 | Keine HTML-Escaping in Web-Templates | [`web/mod.rs`](file:///workspace/src/stars/src/web/mod.rs) |
| 12 | Catalog wird bei jedem Aufruf neu geladen (kein Cache) | [`catalog.rs`](file:///workspace/src/stars/src/astrometry/catalog.rs) |
| 13 | `fetch_tle()` ist ein Stub — TLE-Daten veralten in ~2 Wochen | [`satellites/mod.rs`](file:///workspace/src/stars/src/satellites/mod.rs) |
| 14 | Commit-Bodies fehlen (Plan verlangt "detailed body") | Git-Log |

---

## 🟢 Test-Coverage-Lücken

| Bereich | Fehlender Test |
|---------|----------------|
| `sip.rs` | `fit_from_residuals` komplett ungetestet (enthält Bug 1) |
| `sip.rs` | Nicht-Konvergenz der inversen Iteration |
| `astrometry/mod.rs` | Quad-Hash mit helligkeitssortiertem Katalog |
| `astrometry/mod.rs` | RA-Mittelung über 360°-Grenze |
| `catalog.rs` | Malformed CSV Input |
| `aberration/mod.rs` | Negative Höhe (unter Horizont) |
| `validation/mod.rs` | Hohe Deklination (cos(dec)-Korrektur) |
| `web/mod.rs` | Axum-Endpoint-Responses |
| `satellites/mod.rs` | TLE-Parsing gegen bekannte Orbitalelemente |

---

## ✅ Highlights

- **Commit-Qualität**: 10 Commits im Conventional-Commit-Format mit klaren Scopes. Logische Reihenfolge: Katalog → Algorithmus → Integration → Tests → Docs.
- **Dual-Loading-Strategie** (Runtime-Datei + `include_bytes!` Fallback) ist elegant und robust.
- **Fixed-Point SIP-Inversion** konvergiert in <5 Iterationen auf Sub-0.001px — guter Ansatz.
- **Dokumentationskette**: Plan → Task → Implementation → Walkthrough ist vollständig und konsistent.
- **8.785 Hipparcos-Sterne** korrekt exportiert mit Proper Motions und Parallaxen.

---

## ⚠️ Walkthrough-Korrekturen

1. "15 unit tests" → tatsächlich **17 Testzeilen** aufgelistet (Zeilen 54-71)
2. `csv` Version: `deps.md` sagt "1.4", ggf. gegen `Cargo.toml`/`Cargo.lock` prüfen

---

## Build-Verifikation

```
✅ cargo test         — Alle Tests bestanden
✅ cargo clippy       — 0 Warnungen
✅ cargo fmt --check  — Saubere Formatierung
```

---

## Empfohlene Priorität

1. **Bug 1 fixen** (SIP `dx_pixels`/`dy_pixels`) — trivial, 2 Zeilen
2. **Bug 3 fixen** (Katalog nach Helligkeit sortieren) — moderat
3. **Bug 2 fixen** (RA Vector-Averaging) — moderat
4. **Tests für `fit_from_residuals` und RA-Wrap** schreiben
5. Issues 4-7 in separatem Cleanup-PR
