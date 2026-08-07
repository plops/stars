 #17235 — gemini-3.6-flash (cost: $0.002652)

Abstract

Die vorliegenden Dokumente beschreiben die Simple Imaging Polynomial (SIP) Konvention zur Kodierung nichtlinearer optischer Verzeichnungen in FITS-Bildheadern sowie die Prinzipien, mathematischen Grundlagen und Softwaretools der astrometrischen Kalibrierung (Plate Solving). Die SIP-Konvention erweitert den WCS-Standard (World Coordinate System) durch das Anhängen des Suffixes -SIP an die CTYPEi-Keywords und speichert Verzeichnungskoeffizienten zweidimensionaler Polynome höherer Ordnung ($A_{p,q}$ und $B_{p,q}$ bis zur 9. Ordnung) für die Transformation zwischen Pixelkoordinaten und Zwischen-Weltkoordinaten. Ursprünglich für die variablen Instrumentenoptiken des Spitzer-Weltraumteleskops entwickelt, findet die SIP-Konvention breite Anwendung bei Weltraumobservatorien (z. B. Hubble ACS) sowie in wissenschaftlichen und amate астроometrischen Auswertungspipelines. Das astrometrische Solving identifiziert Sternmuster (Triangeln oder Quads) mittels geometrischer Hashcodes aus Präzisionskatalogen wie Gaia, um Transformationsparameter, Feldmittelpunkte, Bildskalierungen und Verzeichnungsmodelle automatisiert zu berechnen.

Wichtigste Punkte

    SIP-Konvention & WCS-Erweiterung: Das Simple Imaging Polynomial (SIP) erweitert FITS-WCS-Header um das Suffix -SIP im CTYPEi-Keyword (z. B. RA---TAN-SIP), um optische Bildverzeichnungen mathematisch eindeutig und selbsterklärend im Datei-Header abzubilden.
    Mathematische Formulierung: Optische Verzeichnungen werden über Polynome höherer Ordnung $f(u,v)$ und $g(u,v)$ mit den Koeffizienten $A_{p,q}$ und $B_{p,q}$ definiert ($p+q \le \text{A_ORDER/B_ORDER}$, Ordnung 2 bis 9), kombiniert mit einer $CD_{i,j}$-Matrix für Skalierung, Rotation und Scherung (Skew).
    Inverse Transformation (Fast Inversion): Zur schnellen Umrechnung von Himmelskoordinaten in originale Pixelpositionen ohne iterative Inversion können optionale Umkehrkoeffizienten ($AP_{p,q}, BP_{p,q}$) gespeichert werden; diese stellen jedoch Approximationen dar (z. B. Abweichung von ca. 0,014 Pixeln bei Spitzer IRAC Kanal 4).
    Anwendung bei Weltraumteleskopen: Entwickelt für Spitzer (MIPS und IRAC), wird SIP auch für das Hubble-Weltraumteleskop (ACS Wide Field Channel und WFC3) eingesetzt, wo optische Verzeichnungen an den Bildrändern über 50 Pixel betragen können.
    Unterstützendes Software-Ökosystem: Breiter Support für SIP existiert in astronomischen Reduktions- und Anzeigeprogrammen wie MOPEX, WCSTools, SAOImage DS9, Montage, IDL ASTROLIB, Drizzle, GAIA (über die AST-Bibliothek) sowie Astrometry-dot-net.
    Funktionsweise des Astrometric Solving: Software-Solver extrahieren Sternpositionen ($x,y$), bilden geometrische Gruppen (3-Stern-Triangeln oder 4-Stern-Quads) und vergleichen deren mathematische Hash-Codes mit Referenzdatenbanken (wie dem Gaia-Katalog), um Bildmittelpunkt, Ausrichtung und Verzeichnung zu bestimmen.
    Blinde Kalibrierung (Blind Solving): Moderne Solver-Engines wie Astrometric Solving / Astrometry-dot-net oder ASTAP benötigen keine initialen Schätzwerte (Initial Guess) für Position oder Bildmaßstab und können beliebige Himmelsaufnahmen über das gesamte 360°-Sichtfeld auflösen.
    Solver-Landschaft & Plattformen: Das Spektrum reicht von quelloffenen Engines (Astrometry-dot-net, ASTAP, StellarSolver, Siril) bis hin zu proprietären Systemen (PinPoint, PlateSolve2/3, PixInsight, TheSkyX), die CLI-, API-, DLL- oder Cloud-Schnittstellen für Linux, Windows und macOS bereitstellen.
    Präzisions- und Rechenanforderungen: Zur Vermeidung von Genauigkeitsverlusten durch Stellenauslöschung bei hohen Polynomgraden und großen Pixelkoordinaten ist die Verwendung von 64-Bit-Gleitkommazahlen (Double Precision) zwingend erforderlich.

Glossar

    SIP (Simple Imaging Polynomial): Eine standardisierte FITS-Konvention zur Repräsentation geometrischer, optischer Bildverzeichnungen mittels mathematischer Polynomkoeffizienten in Bildheadern.
    FITS (Flexible Image Transport System): Das internationale Standard-Dateiformat der Astronomie zur Speicherung von Bilddaten, Spektren und wissenschaftlichen Metadaten.
    WCS (World Coordinate System): Standardisierte FITS-Spezifikation zur Umrechnung von Pixelkoordinaten ($x,y$) eines Bildes in astronomische Himmelskoordinaten (Rektaszension und Deklination).
    Astrometrisches Solving / Plate Solving: Ein rechnergestütztes Verfahren, das durch Abgleich extrahierter Sternmuster mit Referenzkatalogen die exakten astronomischen Koordinaten, Ausrichtung und Verzeichnung einer Aufnahme ermittelt.
    CTYPEi: FITS-Header-Schlüsselwort zur Festlegung der Koordinatenachse $i$ und des verwendeten Projektionstyps (z. B. RA---TAN für tangentiale gnomonische Projektion).
    CD-Matrix ($CD_{i,j}$): Eine 2x2-Transformationsmatrix im FITS-Header, die Maßstab, Orientierung und Scherung der Bildachsen beschreibt.
    Gaia-Katalog: Hochpräziser astrometrischer Sternenkatalog der Europäischen Weltraumorganisation (ESA), der als primäre Referenzdatenbank für modernes Plate Solving dient.
    Blind Solving: Astrometrisches Auflösungsverfahren, das eine Zuordnung von Bilddaten ohne jegliche Vorinformationen über Teleskopposition oder Brennweite durchführen kann.
    BCD (Basic Calibrated Data): Automatisch kalibrierte wissenschaftliche Primärdatenprodukte der Instrumente des Spitzer-Weltraumteleskops.
    ACS (Advanced Camera for Surveys) / WFC: Ein hochauflösendes Abbildungsinstrument an Bord des Hubble-Weltraumteleskops mit starker feldwinkelabhängiger Verzeichnung.

