# Roadmap – Zento

Geplante Erweiterungen. Jede Funktion hat ein eigenes Issue mit Beschreibung und Umfang;
der Stand wird im GitHub-Project-Board verfolgt.

## Erledigt

| # | Funktion | Bereich |
|---|----------|---------|
| [#3](https://github.com/CyrexMV/WWP/issues/3) | ✅ GiroCode (QR-Code zum Bezahlen) auf Rechnungen | Rechnungen |
| [#4](https://github.com/CyrexMV/WWP/issues/4) | ✅ Mahnwesen mit Mahnstufen und Teilzahlungen | Rechnungen |
| [#2](https://github.com/CyrexMV/WWP/issues/2) | ✅ E-Rechnung: ZUGFeRD / XRechnung erzeugen | Rechnungen |
| [#9](https://github.com/CyrexMV/WWP/issues/9) | ✅ Rabatt und Skonto | Rechnungen |
| [#12](https://github.com/CyrexMV/WWP/issues/12) | ✅ Positionsgruppen, Zwischensummen und optionale Positionen | Rechnungen |
| [#14](https://github.com/CyrexMV/WWP/issues/14) | ✅ Termine und Kalenderansicht | Aufträge |
| [#11](https://github.com/CyrexMV/WWP/issues/11) | ✅ Unterschrift per S-Pen, Auftragsbestätigung und Lieferschein | Rechnungen, Aufträge |
| [#28](https://github.com/CyrexMV/WWP/issues/28) | ✅ Buttons modernisieren | Design |
| – | ✅ Einstellungen neu gestaltet (Bereiche, Live-Vorschau, Einrichtungsstand) | Design |
| – | ✅ Eigene Menüpunkte für Auftragsbestätigungen, Mahnwesen und Leistungskatalog | Design |
| – | ✅ Seitenleiste anpassbar: Reihenfolge per Ziehen/Pfeilen, Menüpunkte ein- und ausblenden | Design |
| [#26](https://github.com/CyrexMV/WWP/issues/26) | ✅ Globale Suche und Schnellbefehle (Strg+K) | Technik |
| [#7](https://github.com/CyrexMV/WWP/issues/7) | ✅ Zeiterfassung pro Auftrag → direkt in die Rechnung | Aufträge |
| [#20](https://github.com/CyrexMV/WWP/issues/20) | ✅ Steuerrücklage-Rechner und Umsatzprognose | Finanzen |
| – | ✅ „Heute zu tun“: automatische Aufgaben inkl. Angebote nachfassen (Teil von #15) und Backup-Erinnerung (Teil von #5) | Assistent |
| – | ✅ Vereinfacht: nur noch Kunde → Bestätigung → Rechnung; Angebote und Aufträge ausgeblendet, Rechnungen mit Reitern (Bestätigungen, Mahnwesen) | Design |
| – | ✅ Leistungskatalog: Vorlage Heizungsbau mit 50 Leistungen und Artikeln (Richtpreise) | Rechnungen |
| – | ✅ Neues Design: Indigo/Violett, dunkle Seitenleiste, Schrift Inter, Gewinn-Übersicht mit Verlauf, Symbole an Kennzahlen | Design |
| – | ✅ Ablauf vereinfacht: Schrittleiste Angebot → Bestätigung → Rechnung, aufgeräumte Aktionen; Lieferscheine entfernt | Design |

## Priorität hoch

| # | Funktion | Bereich |
|---|----------|---------|
| [#1](https://github.com/CyrexMV/WWP/issues/1) | Praxistest auf dem Galaxy Tab S12 Ultra (Android-Funktionen) | Technik |
| [#5](https://github.com/CyrexMV/WWP/issues/5) | Automatisches Backup (Erinnerung ✅) | Technik |
| [#6](https://github.com/CyrexMV/WWP/issues/6) | App-Sperre mit PIN und Fingerabdruck | Technik |

## Priorität mittel

| # | Funktion | Bereich |
|---|----------|---------|
| [#8](https://github.com/CyrexMV/WWP/issues/8) | Abschlags- und Schlussrechnungen | Rechnungen |
| [#10](https://github.com/CyrexMV/WWP/issues/10) | Wiederkehrende Rechnungen (z. B. Wartungsverträge) | Rechnungen |
| [#13](https://github.com/CyrexMV/WWP/issues/13) | Fotos, Dateien und Skizzen am Auftrag | Aufträge |
| [#15](https://github.com/CyrexMV/WWP/issues/15) | Eigene Wiedervorlagen und Aufgaben (automatische ✅) | Aufträge |
| [#16](https://github.com/CyrexMV/WWP/issues/16) | Kunden: mehrere Ansprechpartner und Adressen, Import aus CSV/Kontakten | Kunden |
| [#17](https://github.com/CyrexMV/WWP/issues/17) | Bankumsätze importieren und mit Rechnungen abgleichen | Finanzen |
| [#18](https://github.com/CyrexMV/WWP/issues/18) | Wiederkehrende Ausgaben (Miete, Versicherungen, Abos) | Finanzen |
| [#19](https://github.com/CyrexMV/WWP/issues/19) | DATEV-Export und Zuordnung zur Anlage EÜR | Finanzen |

## Priorität niedrig

| # | Funktion | Bereich |
|---|----------|---------|
| [#21](https://github.com/CyrexMV/WWP/issues/21) | Belegerkennung: Betrag und Datum aus dem Foto lesen | Finanzen |
| [#22](https://github.com/CyrexMV/WWP/issues/22) | Fahrtenbuch und Kilometerpauschale | Finanzen |
| [#23](https://github.com/CyrexMV/WWP/issues/23) | Anlagenverzeichnis und Abschreibungen (AfA) | Finanzen |
| [#24](https://github.com/CyrexMV/WWP/issues/24) | EU-Kunden: Reverse-Charge und Rechnungen auf Englisch | Rechnungen |
| [#25](https://github.com/CyrexMV/WWP/issues/25) | Synchronisation zwischen Geräten (verschlüsselt) | Technik |
| [#27](https://github.com/CyrexMV/WWP/issues/27) | Automatische Oberflächentests im Build | Technik |

## Vorgeschlagene Reihenfolge

1. **Absichern:** #1 Praxistest, #5 automatisches Backup, #6 App-Sperre
2. ~~**Schneller bezahlt werden:** #3 GiroCode, #4 Mahnwesen~~ ✅
3. ~~**Rechtssicher bleiben:** #2 E-Rechnung (vor der B2B-Pflicht)~~ ✅
4. **Arbeit vor Ort:** ~~#7 Zeiterfassung~~ ✅, ~~#11 Unterschrift~~ ✅, #13 Fotos, ~~#14 Termine~~ ✅
5. **Weniger Tipparbeit in den Finanzen:** #17 Bankimport, #18 wiederkehrende Ausgaben, #21 Belegerkennung
6. Danach nach Bedarf
