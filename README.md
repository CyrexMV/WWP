# Zento

**Die Zentrale für deine Kunden und jeden Cent.**

Kunden, Aufträge, Angebote, Rechnungen und Finanzen an einem Ort – **komplett in Rust geschrieben**
(WebAssembly) und als **Android-APK** sowie installierbare Web-App verfügbar. Optimiert für das
**Galaxy Tab S12 Ultra** (Quer- und Hochformat, Touch, helles und dunkles Design).

👉 Web-App: **https://cyrexmv.github.io/WWP/** · APK: **https://cyrexmv.github.io/WWP/wwp.apk**
(sobald GitHub Pages eingerichtet ist, siehe unten – die APK liegt außerdem bei jedem Build unter
*Actions → Lauf → Artifacts → wwp-apk*).

## Funktionen

**Assistent & Suche**
- **Heute zu tun** auf der Übersicht: überfällige Rechnungen und Mahnungen, Bestätigungen ohne Rechnung,
  Termine heute/morgen, Ausgaben ohne Beleg und Backup-Erinnerung (alle 14 Tage) – jeweils mit direktem Knopf
- **Globale Suche & Schnellbefehle** (Strg+K, „/“ oder Lupe): Kunden, Aufträge, Dokumente (auch nach Betrag),
  Buchungen, Termine, Leistungen und Befehle wie „Neue Rechnung“ in einem Feld, mit Pfeiltasten bedienbar

**Cloud & Team (Supabase)**
- **Konto** mit E-Mail und Passwort (Einstellungen → *Konto & Team*); Daten in der Cloud, Server in Frankfurt
- **Betrieb mit Team**: der Anlegende ist Chef; Monteure und weitere Chef-Geräte treten mit einem
  **Einladungscode** bei (8 Zeichen, 7 Tage gültig, einmal verwendbar)
- **Offline-fähiger Abgleich**: die App arbeitet lokal weiter und gleicht im Hintergrund ab (nach Änderungen,
  jede Minute, sobald wieder Internet da ist); gleichzeitige Änderungen: die zuletzt hochgeladene gewinnt,
  Nummernkreise werden zusammengeführt; jedes Gerät hat einen eigenen ID-Bereich
- **Rollen werden vom Server durchgesetzt** (Row Level Security): Monteure erhalten nur Kunden, Anlagen,
  Wartungen, Termine und Firmendaten – keine Rechnungen, Buchungen oder Preise; ein Monteur-Konto startet
  automatisch im Monteur-Modus
- Dateien (Logo, Belege, Typenschild-Fotos) im privaten Speicher; Schema unter `supabase/migrations/`

**Anlagen & Wartung (für Heizungsbetriebe)**
- **Anlagen beim Kunden**: Gas-Brennwert, Öl, Wärmepumpe, Pellet … mit Hersteller, Modell, Seriennummer, Baujahr,
  Leistung, Standort, Notizen und **Foto vom Typenschild**; Wartungsintervall → nächste Fälligkeit automatisch
- **Digitales Wartungsprotokoll**: Checkliste passend zur Anlagenart (OK / Mangel / entfällt, Notiz je Mangel),
  Messwerte (Abgas, CO2, O2, CO, Drücke … bzw. Vor-/Rücklauf bei Wärmepumpen), Arbeiten und Empfehlungen,
  **Unterschrift des Kunden** per S-Pen, PDF im Briefbogen, per E-Mail an den Kunden; beim Abschließen wird die
  nächste Wartung vorgemerkt; mit einem Tipp die Rechnung (Preis aus dem Leistungskatalog)
- Bereich **Wartungen**: fällige Anlagen (30 Tage), alle Anlagen, letzte Protokolle; „Heute zu tun“ meldet fällige
  und noch nicht abgerechnete Wartungen
- **Monteur-Modus** (pro Gerät, Einstellungen → Darstellung): nur *Mein Tag*, Kunden, Wartungen und Kalender –
  keine Preise, Rechnungen oder Finanzen; Navigation zum Kunden; Beenden optional nur mit PIN

**Einfacher Ablauf: Kunde → Bestätigung → Rechnung**
- Seitenleiste: Übersicht, Kunden, Rechnungen (mit Reitern *Rechnungen · Bestätigungen · Mahnwesen*), Kalender,
  Finanzen, Leistungskatalog
- Beim Kunden **+ Bestätigung** (Auftragsbestätigung, auf Wunsch vor Ort unterschreiben lassen) oder direkt
  **+ Rechnung**; aus der Bestätigung wird mit einem Tipp die Rechnung (Kunde und Positionen werden übernommen)
- Übersicht: „Bestätigt – noch ohne Rechnung“ mit Knopf zur Rechnung
- Kunden mit Kundennummer (K-1001 …), Anschrift, Ansprechpartner, E-Mail, Telefon, USt-IdNr., Notizen;
  Umsatz und offene Beträge je Kunde, alle Dokumente des Kunden auf einen Blick
- Angebote, Aufträge (Board) und Zeiterfassung sind ausgeblendet; früher angelegte Daten bleiben erhalten und
  sind über Kunde und Suche erreichbar

**Angebots- und Rechnungsgenerator**
- Fertige PDFs im DIN-5008-Briefbogen: **dein Logo** (PNG/JPG), Absenderzeile, Anschriftfeld, Infoblock
  (Nummer, Datum, Kundennr., Leistungsdatum, Zahlungsziel), Positionstabelle, Summen mit Steuerausweis,
  Fußzeile mit Kontakt, Bankverbindung, Steuernummer/USt-IdNr., Falzmarken, Seitenumbruch mit Seitenzahlen
- Einfach ausfüllen: Kunde wählen → Anschrift und Briefanrede („Sehr geehrte Frau Schön,“) werden übernommen;
  Positionen mit Menge, Einheit, Einzelpreis, USt. (19/7/0 %) – Summen rechnen live mit
- **Leistungskatalog**: häufige Leistungen speichern und mit einem Tipp einfügen; **Vorlage Heizungsbau** mit rund
  50 Leistungen (Arbeitszeit, Wartung, Material, Heizkörper/Fußbodenheizung, Brennwert, Wärmepumpe) zum Laden;
  Kategorien zum Auf- und Zuklappen mit Anzahl und Preisspanne
- **Per E-Mail senden**: PDF wird angehängt, Empfänger, Betreff und Text sind vorausgefüllt (Gmail, Outlook,
  Samsung E-Mail …) – oder an WhatsApp/Drive teilen; PDF ansehen oder in *Download/Zento* speichern
- Fortlaufende Nummern je Jahr (AB-2026-0001, RE-2026-0001), Präfixe einstellbar
- **Ablauf auf einen Blick**: Auftragsbestätigung → Rechnung. Im Dokument zeigt eine Schrittleiste, was schon da
  ist und was als Nächstes kommt; „Erstellen“ übernimmt Kunde, Anschrift und Positionen. Duplizieren unter
  „Weitere Aktionen“
- **Unterschrift per S-Pen**: Der Kunde unterschreibt direkt auf dem Tablet (Druckstufen, Handballen wird
  ignoriert) – Auftrag bestätigt; die Unterschrift erscheint als Vektor-Tinte mit Ort, Datum und Name im PDF
- **E-Rechnung**: Rechnungs-PDFs sind PDF/A-3 mit eingebetteten Rechnungsdaten nach EN 16931 (**ZUGFeRD/Factur-X**);
  zusätzlich **XRechnung 3.0** als XML-Datei (z. B. für Behörden mit Leitweg-ID). Vor dem Versand werden die
  Pflichtangaben geprüft; die Ausgabe ist gegen das offizielle XSD und die EN-16931-/XRechnung-Prüfregeln getestet
- **Positionsarten**: Titel mit Zwischensumme (Nummern 1, 1.1, 1.2 …), Hinweistexte ohne Preis, optionale
  Positionen (werden angezeigt, aber nicht summiert); Positionen per Ziehen am Griff umsortieren
- **Rabatt & Skonto**: Positionsrabatt in %, Gesamtrabatt in % oder €, Skonto mit Frist (Hinweis auf der Rechnung,
  beim Zahlungseingang mit einem Tipp – Umsatz und Umsatzsteuer werden anteilig gemindert); Standardwerte je Kunde
- **Leistungskatalog** mit Einkaufspreis, Marge, Kategorien und Suche
- **GiroCode** (EPC-QR-Code) auf Rechnungen und Mahnungen: Banking-App scannen, Empfänger, IBAN, Betrag und
  Verwendungszweck sind ausgefüllt; IBAN-Prüfung in den Einstellungen
- **Mahnwesen**: Zahlungserinnerung → 1. Mahnung → 2. Mahnung mit eigenen Fristen, Gebühren und Texten,
  optional Verzugszinsen (Basiszinssatz + 5/9 Prozentpunkte); Mahnschreiben als PDF mit Rechnungskopie,
  per E-Mail senden, Mahnhistorie an der Rechnung, Filter „Zu mahnen“ und Hinweis auf der Startseite
- **Teilzahlungen**: Zahlungseingänge mit beliebigem Betrag erfassen, Restbetrag wird weitergeführt; Zahlungen
  werden nach Steuersätzen aufgeteilt gebucht, Mahngebühren/Zinsen als nicht steuerbare sonstige Einnahme
- Versendete Rechnungen werden **festgeschrieben** (GoBD); Korrektur per **Stornorechnung**
- **Kleinunternehmer** (§ 19 UStG): keine Umsatzsteuer, Hinweis auf der Rechnung
- Standardtexte (Einleitung, Schluss, E-Mail) mit Platzhaltern wie `{betrag}`, `{faellig}`, `{nummer}`
- „Briefbogen-Vorschau“ in den Einstellungen zeigt eine Musterrechnung mit deinen Daten

**Kalender**
- Termine mit Uhrzeit, mehrtägig oder ganztägig, Ort, Auftrag und Kunde; Wochen- und Monatsansicht,
  Fristen der Aufträge werden mit angezeigt, nächste Termine auf der Übersicht, beim Auftrag und beim Kunden
- Termin mit einem Tipp in die Kalender-App des Tablets übernehmen (im Browser als .ics), Route in Google Maps

**Einnahmen- und Ausgaben-Tracker**
- „Als bezahlt markieren“ bucht die Einnahme automatisch (bei gemischten Steuersätzen getrennt)
- Ausgaben mit Kategorie (angelehnt an die Anlage EÜR), Zahlart, Lieferant und **Beleg** (Foto per Kamera oder PDF)
- Jahresübersicht: Einnahmen, Ausgaben, Gewinn, Monatsdiagramm, Kategorien
- **Für die Steuer**: Umsatzsteuer, Vorsteuer und Zahllast je Quartal (für die Voranmeldung), Hinweis auf
  Ausgaben ohne Beleg, Export als **CSV** (Buchungsliste, Rechnungsausgangsbuch – für Excel/Steuerberater)
  und **PDF-Jahresübersicht**
- Privatentnahmen, Steuerzahlungen ans Finanzamt u. Ä. sind „gewinnneutral“ und verfälschen den Gewinn nicht

**Bedienung**
- Seitenleiste mit eigenem Menüpunkt für jeden Bereich: Übersicht, Kunden, Aufträge, Kalender, Angebote,
  Auftragsbestätigungen, Rechnungen, Mahnwesen, Finanzen, Leistungskatalog
- Reihenfolge der Seitenleiste selbst festlegen (am Griff ziehen oder Pfeile) und Menüpunkte ausblenden –
  über das Griff-Symbol neben „Einstellungen“ oder Einstellungen → Darstellung
- Mahnwesen-Seite mit allen überfälligen Rechnungen, nächster Mahnstufe und Verlauf aller Mahnungen

**Daten**
- Alles wird automatisch gespeichert – nur auf dem Gerät (IndexedDB), ohne Konto und ohne Cloud
- **Rücklage & Prognose**: empfohlene Steuerrücklage (USt.-Zahllast des Quartals + einstellbarer Anteil vom
  Gewinn), Hochrechnung aufs Jahresende, Vergleich mit dem Vorjahr bzw. Kleinunternehmergrenze
- **Komplett-Backup** (inkl. Logo und Belege) als eine `.wwpbackup`-Datei speichern oder teilen
  (z. B. in Google Drive) und wiederherstellen
- Beispieldaten zum Ausprobieren (Startbildschirm oder Einstellungen)

> Hinweis: Die Finanzübersicht ist eine vereinfachte Einnahmen-Überschuss-Sicht nach Zahlungsdatum und ersetzt
> keine Steuerberatung (Abschreibungen, Fahrtenbuch u. Ä. sind nicht enthalten).

## Installation auf dem Tablet

**APK:** Link oben öffnen → Installieren (beim ersten Mal „Unbekannte Apps installieren“ erlauben).
**Web-App:** Seite in Samsung Internet/Chrome öffnen → „Zum Startbildschirm hinzufügen“.

### Einmalig im Repository einrichten

1. **GitHub Pages:** *Settings → Pages → Build and deployment → Source: „Deploy from a branch“*,
   Branch **`gh-pages`** / `(root)` wählen. (Der Branch entsteht beim ersten erfolgreichen Build.)
2. **Signatur (wichtig für Updates):** Android installiert ein Update nur, wenn es mit demselben Schlüssel
   signiert ist. *Settings → Secrets and variables → Actions → New repository secret*:
   Name **`WWP_SIGNING_PASSWORD`**, Wert: ein langes, eigenes Passwort (mindestens 12, besser 20+ Zeichen) –
   im Passwort-Manager aufbewahren und **nie ändern**. Beim nächsten Build erzeugt GitHub den Schlüssel und legt
   ihn **nur verschlüsselt** (AES-256) im Branch `signing` ab. Ab dann ist jede APK gleich signiert.

Ohne dieses Secret wird mit einem temporären Schlüssel signiert (dann vor jedem Update neu installieren –
**vorher ein Backup erstellen**, da beim Deinstallieren die App-Daten gelöscht werden).
Alternativ kann ein eigener Keystore als `WWP_KEYSTORE_B64` (+ `WWP_KEYSTORE_PASSWORD`, `WWP_KEY_ALIAS`,
`WWP_KEY_PASSWORD`) hinterlegt werden.

## Selbst bauen

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130
./build.sh                                   # Web-App nach web/
python3 -m http.server -d web 8080           # → http://localhost:8080
cd android && ./gradlew assembleRelease      # APK (Android SDK + JDK 17)
cargo test                                   # Tests der Geschäftslogik
cargo run --example muster -- /tmp           # Muster-Rechnung und -Angebot als PDF
```

Jeder Push baut über `.github/workflows/build.yml` Tests, Web-App und APK und veröffentlicht beides auf GitHub Pages
(genau wie beim Zeichentool).

## Aufbau

```
src/engine/   Geschäftslogik (reines Rust, getestet)
  model.rs      Firma, Kunden, Aufträge, Angebote/Rechnungen, Buchungen, Nummernkreise, Storno,
                Zahlungen/Teilzahlungen, Mahnungen und Verzugszinsen
  money.rs      Beträge in Cent, deutsche Zahlenformate, Steuerberechnung
  date.rs       Datumsrechnung (Fälligkeiten, Zeiträume)
  pdf.rs        PDF-Erzeugung ohne Bibliothek: Briefbogen mit Logo, Tabelle, Summen, GiroCode,
                Mahnschreiben mit Rechnungskopie, Jahresübersicht
  report.rs     Jahres-/Monats-/Quartalsauswertung, CSV-Export
  backup.rs     Komplett-Backup, Base64, Beispieldaten
  girocode.rs   GiroCode (EPC-QR-Code) und IBAN-Prüfung
  erechnung.rs  E-Rechnung (CII-XML für ZUGFeRD EN 16931 und XRechnung 3.0), Pflichtangaben-Prüfung
  pdfa.rs       PDF/A-3: sRGB-Farbprofil, XMP-Metadaten mit Factur-X-Erweiterung
  ttf.rs        TrueType-Leser und Teilmengen für eingebettete Schriften (Liberation Sans, OFL)
  termine.rs    Termine, Kalenderwochen, iCalendar-Export
  image.rs      PNG/JPEG für das Logo
src/web/      Oberfläche (wasm-bindgen / web-sys)
  mod.rs        App-Zustand, Navigation, Ereignisse
  views.rs      HTML aller Ansichten
  actions.rs    Klicks, Eingaben, Senden/Speichern
  storage.rs    IndexedDB, Dateien (Logo, Belege), Backup, Android-Brücke
  signature.rs  Unterschriftenfeld (S-Pen, Druck, Handballen-Filter)
  settings.rs   Einstellungen mit Bereichen und Live-Vorschau
  kalender.rs   Wochen-/Monatsansicht, Termin-Dialog
  dnd.rs        Listen per Ziehen umsortieren (Positionen, Seitenleiste)
  cloud.rs      Supabase: Konto, Betrieb/Team, Einladungen, Abgleich, Dateien
  wartung.rs    Anlagen, Wartungsprotokoll, Monteur-Modus
  suche.rs      globale Suche und Schnellbefehle
  heute.rs      „Heute zu tun“ auf der Übersicht
  zeiten.rs     Zeiterfassung im Auftrag (Stoppuhr)
  ablauf.rs     Schrittleiste Angebot → Bestätigung → Rechnung
  navigation.rs Seitenleiste: Menüpunkte, anpassbare Reihenfolge und Sichtbarkeit
assets/fonts/ eingebettete Schriften (Liberation Sans, WinAnsi-Teilmenge) mit Lizenz
web/          index.html, style.css, Manifest, Icons, Service Worker, Schrift Inter (OFL)
android/      Android-Hülle (WebView) mit Brücke für Speichern, PDF öffnen, E-Mail/Teilen und Kamera
```
