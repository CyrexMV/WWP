# WWP Büro

Kunden, Aufträge, Angebote, Rechnungen und Finanzen an einem Ort – **komplett in Rust geschrieben**
(WebAssembly) und als **Android-APK** sowie installierbare Web-App verfügbar. Optimiert für das
**Galaxy Tab S12 Ultra** (Quer- und Hochformat, Touch, helles und dunkles Design).

👉 Web-App: **https://cyrexmv.github.io/WWP/** · APK: **https://cyrexmv.github.io/WWP/wwp.apk**
(sobald GitHub Pages eingerichtet ist, siehe unten – die APK liegt außerdem bei jedem Build unter
*Actions → Lauf → Artifacts → wwp-apk*).

## Funktionen

**Kunden- und Auftragsverwaltung (CRM)**
- Kunden mit Kundennummer (K-1001 …), Anschrift, Ansprechpartner, E-Mail, Telefon, USt-IdNr., Notizen
- Suche, Umsatz (bezahlt) und offene Beträge je Kunde, alle Aufträge und Dokumente des Kunden auf einen Blick
- Direkt aus dem Kunden: E-Mail schreiben, anrufen, Anfrage/Angebot/Rechnung anlegen
- **Auftrags-Board** von der Anfrage bis zur Abrechnung: *Anfrage → Angebot gesendet → Beauftragt → In Arbeit →
  Erledigt → Abgerechnet* (plus *Abgelehnt*), mit Termin, geschätztem Wert und „Weiter“-Knopf auf jeder Karte
- Verlauf je Auftrag (Statuswechsel und eigene Notizen mit Datum)
- Der Status läuft automatisch mit: Angebot versendet → „Angebot gesendet“, angenommen → „Beauftragt“,
  Rechnung versendet → „Erledigt“, bezahlt → „Abgerechnet“

**Angebots- und Rechnungsgenerator**
- Fertige PDFs im DIN-5008-Briefbogen: **dein Logo** (PNG/JPG), Absenderzeile, Anschriftfeld, Infoblock
  (Nummer, Datum, Kundennr., Leistungsdatum, Zahlungsziel), Positionstabelle, Summen mit Steuerausweis,
  Fußzeile mit Kontakt, Bankverbindung, Steuernummer/USt-IdNr., Falzmarken, Seitenumbruch mit Seitenzahlen
- Einfach ausfüllen: Kunde wählen → Anschrift und Briefanrede („Sehr geehrte Frau Schön,“) werden übernommen;
  Positionen mit Menge, Einheit, Einzelpreis, USt. (19/7/0 %) – Summen rechnen live mit
- **Leistungskatalog**: häufige Leistungen speichern und mit einem Tipp einfügen
- **Per E-Mail senden**: PDF wird angehängt, Empfänger, Betreff und Text sind vorausgefüllt (Gmail, Outlook,
  Samsung E-Mail …) – oder an WhatsApp/Drive teilen; PDF ansehen oder in *Download/WWP* speichern
- Fortlaufende Nummern je Jahr (AN-2026-0001, RE-2026-0001), Präfixe einstellbar
- Angebot → Rechnung mit einem Tipp, Duplizieren
- **Auftragsbestätigung** und **Lieferschein/Arbeitsnachweis** (ohne Preise) als eigene Dokumente mit Nummernkreis,
  erstellt aus Angebot oder Auftrag; aus dem Lieferschein wird mit einem Tipp die Rechnung
- **Unterschrift per S-Pen**: Der Kunde unterschreibt direkt auf dem Tablet (Druckstufen, Handballen wird
  ignoriert) – Angebot angenommen, Auftrag bestätigt oder Leistung abgenommen; die Unterschrift erscheint als
  Vektor-Tinte mit Ort, Datum und Name im PDF, der Auftrag läuft automatisch mit
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

**Einnahmen- und Ausgaben-Tracker**
- „Als bezahlt markieren“ bucht die Einnahme automatisch (bei gemischten Steuersätzen getrennt)
- Ausgaben mit Kategorie (angelehnt an die Anlage EÜR), Zahlart, Lieferant und **Beleg** (Foto per Kamera oder PDF)
- Jahresübersicht: Einnahmen, Ausgaben, Gewinn, Monatsdiagramm, Kategorien
- **Für die Steuer**: Umsatzsteuer, Vorsteuer und Zahllast je Quartal (für die Voranmeldung), Hinweis auf
  Ausgaben ohne Beleg, Export als **CSV** (Buchungsliste, Rechnungsausgangsbuch – für Excel/Steuerberater)
  und **PDF-Jahresübersicht**
- Privatentnahmen, Steuerzahlungen ans Finanzamt u. Ä. sind „gewinnneutral“ und verfälschen den Gewinn nicht

**Daten**
- Alles wird automatisch gespeichert – nur auf dem Gerät (IndexedDB), ohne Konto und ohne Cloud
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
  image.rs      PNG/JPEG für das Logo
src/web/      Oberfläche (wasm-bindgen / web-sys)
  mod.rs        App-Zustand, Navigation, Ereignisse
  views.rs      HTML aller Ansichten
  actions.rs    Klicks, Eingaben, Senden/Speichern
  storage.rs    IndexedDB, Dateien (Logo, Belege), Backup, Android-Brücke
  signature.rs  Unterschriftenfeld (S-Pen, Druck, Handballen-Filter)
  settings.rs   Einstellungen mit Bereichen und Live-Vorschau
web/          index.html, style.css, Manifest, Icons, Service Worker
android/      Android-Hülle (WebView) mit Brücke für Speichern, PDF öffnen, E-Mail/Teilen und Kamera
```
