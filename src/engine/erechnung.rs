//! E-Rechnung nach EN 16931 in der Syntax UN/CEFACT CII (Cross Industry Invoice, D16B):
//! * ZUGFeRD 2 / Factur-X, Profil EN 16931 – als XML im PDF/A-3 eingebettet
//! * XRechnung 3.0 (CII) – als reine XML-Datei, z. B. für öffentliche Auftraggeber mit Leitweg-ID
//!
//! Außerdem eine Prüfung der Pflichtangaben vor dem Versand.

use super::date;
use super::girocode::{iban_clean, iban_valid};
use super::model::{Data, DokArt, Dokument, Kunde};
use super::money::Cents;
use std::fmt::Write as _;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Profil {
    /// ZUGFeRD 2 / Factur-X EN 16931 (im PDF eingebettet)
    Zugferd,
    /// XRechnung 3.0 in CII-Syntax
    XRechnung,
}

/// Ergebnis der Prüfung: (Pflicht?, Hinweis)
pub type Befund = (bool, String);

fn xml(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&apos;"),
            c if (c as u32) < 0x20 && c != '\n' && c != '\t' => {}
            c => o.push(c),
        }
    }
    o
}

/// Betrag „1234.56“ (immer zwei Nachkommastellen).
fn amt(c: Cents) -> String {
    let neg = c < 0;
    let a = c.unsigned_abs();
    format!("{}{}.{:02}", if neg { "-" } else { "" }, a / 100, a % 100)
}

fn qty(q: f64) -> String {
    let s = format!("{:.4}", q);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".into()
    } else {
        s.to_string()
    }
}

fn d102(iso: &str) -> String {
    iso.replace('-', "")
}

/// Einheit → UN/ECE-Code (Recommendation 20).
pub fn einheit_code(e: &str) -> &'static str {
    match e.trim().trim_end_matches('.').to_lowercase().as_str() {
        "std" | "h" | "stunde" | "stunden" => "HUR",
        "min" | "minute" | "minuten" => "MIN",
        "tag" | "tag(e)" | "tage" | "tg" => "DAY",
        "woche" | "wochen" => "WEE",
        "monat" | "monate" => "MON",
        "m" | "meter" | "lfm" => "MTR",
        "m²" | "m2" | "qm" => "MTK",
        "m³" | "m3" | "cbm" => "MTQ",
        "km" => "KMT",
        "kg" => "KGM",
        "t" => "TNE",
        "l" | "liter" => "LTR",
        "pausch" | "psch" | "pauschal" | "pauschale" => "LS",
        "stk" | "stück" | "st" => "H87",
        _ => "C62",
    }
}

/// Ländername/Code → ISO-3166-Code (Standard DE).
pub fn land_code(land: &str) -> String {
    let l = land.trim().to_lowercase();
    if l.len() == 2 && l.chars().all(|c| c.is_ascii_alphabetic()) {
        return l.to_uppercase();
    }
    match l.as_str() {
        "" | "deutschland" | "germany" => "DE",
        "österreich" | "oesterreich" | "austria" => "AT",
        "schweiz" | "switzerland" => "CH",
        "frankreich" | "france" => "FR",
        "niederlande" | "netherlands" | "holland" => "NL",
        "belgien" | "belgium" => "BE",
        "luxemburg" | "luxembourg" => "LU",
        "italien" | "italy" => "IT",
        "spanien" | "spain" => "ES",
        "polen" | "poland" => "PL",
        "tschechien" | "czech republic" => "CZ",
        "dänemark" | "denmark" => "DK",
        _ => "DE",
    }
    .into()
}

/// „80331 München“ → („80331“, „München“)
fn plz_ort(s: &str) -> (String, String) {
    let s = s.trim();
    match s.split_once(' ') {
        Some((a, b)) if a.chars().any(|c| c.is_ascii_digit()) => (a.trim().to_string(), b.trim().to_string()),
        _ => (String::new(), s.to_string()),
    }
}

/// Leistungsdatum aus Freitext („12.11.2026“) lesen.
fn leistungsdatum(s: &str) -> Option<String> {
    let t = s.trim();
    let mut it = t.split('.');
    let d: u32 = it.next()?.trim().parse().ok()?;
    let m: u32 = it.next()?.trim().parse().ok()?;
    let y: i32 = it.next()?.trim().get(..4)?.parse().ok()?;
    let iso = date::iso(y, m, d);
    date::parse(&iso).map(|_| iso)
}

/// Käuferangaben: aus dem Kundenkonto, sonst aus der Anschrift im Dokument.
struct Kaeufer {
    name: String,
    strasse: String,
    plz: String,
    ort: String,
    land: String,
    email: String,
    ust_id: String,
    referenz: String,
}

fn kaeufer(doc: &Dokument, k: Option<&Kunde>) -> Kaeufer {
    if let Some(k) = k {
        return Kaeufer {
            name: k.name(),
            strasse: k.strasse.trim().to_string(),
            plz: k.plz.trim().to_string(),
            ort: k.ort.trim().to_string(),
            land: land_code(&k.land),
            email: k.email.trim().to_string(),
            ust_id: k.ust_id.trim().to_string(),
            referenz: if !k.leitweg_id.trim().is_empty() { k.leitweg_id.trim().to_string() } else { k.nr.clone() },
        };
    }
    let lines: Vec<&str> = doc.empfaenger.lines().map(|l| l.trim()).filter(|l| !l.is_empty()).collect();
    let (plz, ort) = lines.last().map(|l| plz_ort(l)).unwrap_or_default();
    Kaeufer {
        name: lines.first().map(|s| s.to_string()).unwrap_or_default(),
        strasse: if lines.len() >= 3 { lines[lines.len() - 2].to_string() } else { String::new() },
        plz,
        ort,
        land: "DE".into(),
        email: String::new(),
        ust_id: String::new(),
        referenz: String::new(),
    }
}

/// Pflichtangaben prüfen. Einträge mit `true` verhindern eine gültige E-Rechnung.
pub fn pruefen(data: &Data, doc: &Dokument, profil: Profil) -> Vec<Befund> {
    let f = &data.firma;
    let mut v: Vec<Befund> = Vec::new();
    if doc.art != DokArt::Rechnung {
        v.push((true, "Nur Rechnungen können als E-Rechnung ausgegeben werden".into()));
        return v;
    }
    if f.name.trim().is_empty() {
        v.push((true, "Firmenname fehlt (Einstellungen → Firma)".into()));
    }
    let (fplz, fort) = plz_ort(&f.plz_ort);
    if f.strasse.trim().is_empty() || fplz.is_empty() || fort.is_empty() {
        v.push((true, "Firmenanschrift unvollständig – Straße sowie PLZ und Ort (Einstellungen → Firma)".into()));
    }
    if f.steuernummer.trim().is_empty() && f.ust_id.trim().is_empty() {
        v.push((true, "Steuernummer oder USt-IdNr. fehlt (Einstellungen → Bank & Steuern)".into()));
    }
    if f.telefon.trim().is_empty() || f.email.trim().is_empty() {
        v.push((profil == Profil::XRechnung, "Telefon und E-Mail der Firma fehlen (für XRechnung Pflicht)".into()));
    }
    if !iban_valid(&f.iban) {
        v.push((false, "Keine gültige IBAN – die Zahlungsart wird nicht übermittelt".into()));
    }
    let k = data.kunde(doc.kunde_id);
    let b = kaeufer(doc, k);
    if b.name.is_empty() {
        v.push((true, "Kundenname fehlt".into()));
    }
    if b.strasse.is_empty() || b.plz.is_empty() || b.ort.is_empty() {
        v.push((true, "Kundenanschrift unvollständig (Straße, PLZ, Ort)".into()));
    }
    if profil == Profil::XRechnung {
        if b.email.is_empty() {
            v.push((true, "E-Mail des Kunden fehlt (für XRechnung Pflicht)".into()));
        }
        if b.referenz.is_empty() {
            v.push((true, "Leitweg-ID bzw. Käuferreferenz fehlt".into()));
        }
    }
    if !doc.positionen.iter().any(|p| p.zaehlt() && !p.text.trim().is_empty()) {
        v.push((true, "Keine berechneten Positionen".into()));
    }
    if leistungsdatum(&doc.leistung).is_none() {
        v.push((false, "Leistungsdatum nicht als Datum (TT.MM.JJJJ) erkennbar – es wird das Rechnungsdatum verwendet".into()));
    }
    v
}

/// Gibt es Hinweise, die eine E-Rechnung verhindern?
pub fn gueltig(data: &Data, doc: &Dokument, profil: Profil) -> bool {
    !pruefen(data, doc, profil).iter().any(|b| b.0)
}

/// (Kategorie, Satz) einer Position bzw. eines Steuersatzes.
fn kategorie(doc: &Dokument, rate: u8) -> (&'static str, u8) {
    if doc.kleinunternehmer {
        ("E", 0)
    } else if rate == 0 {
        ("Z", 0)
    } else {
        ("S", rate)
    }
}

const KU_GRUND: &str = "Kein Ausweis von Umsatzsteuer, da Kleinunternehmer gemäß § 19 UStG";

fn tax_xml(o: &mut String, ind: &str, cat: &str, rate: u8) {
    let _ = write!(
        o,
        "{ind}<ram:ApplicableTradeTax>\n{ind}  <ram:TypeCode>VAT</ram:TypeCode>\n{ind}  <ram:CategoryCode>{cat}</ram:CategoryCode>\n{ind}  <ram:RateApplicablePercent>{rate}</ram:RateApplicablePercent>\n{ind}</ram:ApplicableTradeTax>\n"
    );
}

/// Rechnung als CII-XML.
pub fn cii_xml(data: &Data, doc: &Dokument, profil: Profil) -> String {
    let f = &data.firma;
    let k = data.kunde(doc.kunde_id);
    let b = kaeufer(doc, k);
    let sum = doc.summen();
    let storno = doc.storno_von.is_some();
    let mut o = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    o += "<rsm:CrossIndustryInvoice xmlns:rsm=\"urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100\" xmlns:ram=\"urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100\" xmlns:qdt=\"urn:un:unece:uncefact:data:standard:QualifiedDataType:100\" xmlns:udt=\"urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100\">\n";
    o += "  <rsm:ExchangedDocumentContext>\n";
    if profil == Profil::XRechnung {
        o += "    <ram:BusinessProcessSpecifiedDocumentContextParameter>\n      <ram:ID>urn:fdc:peppol.eu:2017:poacc:billing:01:1.0</ram:ID>\n    </ram:BusinessProcessSpecifiedDocumentContextParameter>\n";
    }
    let guideline = match profil {
        Profil::Zugferd => "urn:cen.eu:en16931:2017",
        Profil::XRechnung => "urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0",
    };
    let _ = write!(o, "    <ram:GuidelineSpecifiedDocumentContextParameter>\n      <ram:ID>{guideline}</ram:ID>\n    </ram:GuidelineSpecifiedDocumentContextParameter>\n  </rsm:ExchangedDocumentContext>\n");

    // --- Kopf
    let _ = write!(
        o,
        "  <rsm:ExchangedDocument>\n    <ram:ID>{}</ram:ID>\n    <ram:TypeCode>{}</ram:TypeCode>\n    <ram:IssueDateTime>\n      <udt:DateTimeString format=\"102\">{}</udt:DateTimeString>\n    </ram:IssueDateTime>\n",
        xml(&doc.nummer),
        if storno { "384" } else { "380" },
        d102(&doc.datum)
    );
    let mut notes: Vec<String> = Vec::new();
    if !doc.betreff.trim().is_empty() {
        notes.push(doc.betreff.trim().to_string());
    }
    if doc.kleinunternehmer {
        notes.push("Gemäß § 19 UStG wird keine Umsatzsteuer berechnet.".into());
    }
    for n in notes {
        let _ = write!(o, "    <ram:IncludedNote>\n      <ram:Content>{}</ram:Content>\n    </ram:IncludedNote>\n", xml(&n));
    }
    o += "  </rsm:ExchangedDocument>\n  <rsm:SupplyChainTradeTransaction>\n";

    // --- Positionen
    let mut line = 0;
    for p in doc.positionen.iter().filter(|p| p.zaehlt()) {
        line += 1;
        let mut zeilen = p.text.trim().lines().map(|l| l.trim()).filter(|l| !l.is_empty());
        let name = zeilen.next().unwrap_or("Position").to_string();
        let rest: Vec<&str> = zeilen.collect();
        let (cat, rate) = kategorie(doc, p.ust);
        let _ = write!(
            o,
            "    <ram:IncludedSupplyChainTradeLineItem>\n      <ram:AssociatedDocumentLineDocument>\n        <ram:LineID>{line}</ram:LineID>\n      </ram:AssociatedDocumentLineDocument>\n      <ram:SpecifiedTradeProduct>\n        <ram:Name>{}</ram:Name>\n",
            xml(&name)
        );
        if !rest.is_empty() {
            let _ = writeln!(o, "        <ram:Description>{}</ram:Description>", xml(&rest.join(" ")));
        }
        let _ = write!(
            o,
            "      </ram:SpecifiedTradeProduct>\n      <ram:SpecifiedLineTradeAgreement>\n        <ram:NetPriceProductTradePrice>\n          <ram:ChargeAmount>{}</ram:ChargeAmount>\n        </ram:NetPriceProductTradePrice>\n      </ram:SpecifiedLineTradeAgreement>\n      <ram:SpecifiedLineTradeDelivery>\n        <ram:BilledQuantity unitCode=\"{}\">{}</ram:BilledQuantity>\n      </ram:SpecifiedLineTradeDelivery>\n      <ram:SpecifiedLineTradeSettlement>\n",
            amt(p.preis),
            einheit_code(&p.einheit),
            qty(p.menge)
        );
        tax_xml(&mut o, "        ", cat, rate);
        let rabatt = p.rabatt_betrag();
        if rabatt != 0 {
            let _ = write!(
                o,
                "        <ram:SpecifiedTradeAllowanceCharge>\n          <ram:ChargeIndicator>\n            <udt:Indicator>false</udt:Indicator>\n          </ram:ChargeIndicator>\n          <ram:ActualAmount>{}</ram:ActualAmount>\n          <ram:Reason>Rabatt</ram:Reason>\n        </ram:SpecifiedTradeAllowanceCharge>\n",
                amt(rabatt.abs())
            );
        }
        let _ = write!(
            o,
            "        <ram:SpecifiedTradeSettlementLineMonetarySummation>\n          <ram:LineTotalAmount>{}</ram:LineTotalAmount>\n        </ram:SpecifiedTradeSettlementLineMonetarySummation>\n      </ram:SpecifiedLineTradeSettlement>\n    </ram:IncludedSupplyChainTradeLineItem>\n",
            amt(p.netto())
        );
    }

    // --- Vereinbarung: Verkäufer und Käufer
    o += "    <ram:ApplicableHeaderTradeAgreement>\n";
    let referenz = if b.referenz.is_empty() { "-".to_string() } else { b.referenz.clone() };
    let _ = writeln!(o, "      <ram:BuyerReference>{}</ram:BuyerReference>", xml(&referenz));
    let (fplz, fort) = plz_ort(&f.plz_ort);
    o += "      <ram:SellerTradeParty>\n";
    // ohne USt-IdNr. dient die Steuernummer als Verkäuferkennung (BT-29, Regel BR-CO-26)
    if f.ust_id.trim().is_empty() && !f.steuernummer.trim().is_empty() {
        let _ = writeln!(o, "        <ram:ID>{}</ram:ID>", xml(f.steuernummer.trim()));
    }
    let _ = writeln!(o, "        <ram:Name>{}</ram:Name>", xml(f.name.trim()));
    if !f.telefon.trim().is_empty() || !f.email.trim().is_empty() {
        let person = if f.zusatz.trim().is_empty() { f.name.trim() } else { f.zusatz.trim() };
        let _ = write!(o, "        <ram:DefinedTradeContact>\n          <ram:PersonName>{}</ram:PersonName>\n", xml(person));
        if !f.telefon.trim().is_empty() {
            let _ = write!(
                o,
                "          <ram:TelephoneUniversalCommunication>\n            <ram:CompleteNumber>{}</ram:CompleteNumber>\n          </ram:TelephoneUniversalCommunication>\n",
                xml(f.telefon.trim())
            );
        }
        if !f.email.trim().is_empty() {
            let _ = write!(
                o,
                "          <ram:EmailURIUniversalCommunication>\n            <ram:URIID>{}</ram:URIID>\n          </ram:EmailURIUniversalCommunication>\n",
                xml(f.email.trim())
            );
        }
        o += "        </ram:DefinedTradeContact>\n";
    }
    let _ = write!(
        o,
        "        <ram:PostalTradeAddress>\n          <ram:PostcodeCode>{}</ram:PostcodeCode>\n          <ram:LineOne>{}</ram:LineOne>\n          <ram:CityName>{}</ram:CityName>\n          <ram:CountryID>DE</ram:CountryID>\n        </ram:PostalTradeAddress>\n",
        xml(&fplz),
        xml(f.strasse.trim()),
        xml(&fort)
    );
    if !f.email.trim().is_empty() {
        let _ = write!(o, "        <ram:URIUniversalCommunication>\n          <ram:URIID schemeID=\"EM\">{}</ram:URIID>\n        </ram:URIUniversalCommunication>\n", xml(f.email.trim()));
    }
    if !f.ust_id.trim().is_empty() {
        let _ = write!(o, "        <ram:SpecifiedTaxRegistration>\n          <ram:ID schemeID=\"VA\">{}</ram:ID>\n        </ram:SpecifiedTaxRegistration>\n", xml(&f.ust_id.replace(' ', "")));
    }
    if !f.steuernummer.trim().is_empty() {
        let _ = write!(o, "        <ram:SpecifiedTaxRegistration>\n          <ram:ID schemeID=\"FC\">{}</ram:ID>\n        </ram:SpecifiedTaxRegistration>\n", xml(f.steuernummer.trim()));
    }
    o += "      </ram:SellerTradeParty>\n";
    let _ = write!(
        o,
        "      <ram:BuyerTradeParty>\n        <ram:Name>{}</ram:Name>\n        <ram:PostalTradeAddress>\n          <ram:PostcodeCode>{}</ram:PostcodeCode>\n          <ram:LineOne>{}</ram:LineOne>\n          <ram:CityName>{}</ram:CityName>\n          <ram:CountryID>{}</ram:CountryID>\n        </ram:PostalTradeAddress>\n",
        xml(&b.name),
        xml(&b.plz),
        xml(&b.strasse),
        xml(&b.ort),
        xml(&b.land)
    );
    if !b.email.is_empty() {
        let _ = write!(o, "        <ram:URIUniversalCommunication>\n          <ram:URIID schemeID=\"EM\">{}</ram:URIID>\n        </ram:URIUniversalCommunication>\n", xml(&b.email));
    }
    if !b.ust_id.is_empty() {
        let _ = write!(o, "        <ram:SpecifiedTaxRegistration>\n          <ram:ID schemeID=\"VA\">{}</ram:ID>\n        </ram:SpecifiedTaxRegistration>\n", xml(&b.ust_id.replace(' ', "")));
    }
    o += "      </ram:BuyerTradeParty>\n    </ram:ApplicableHeaderTradeAgreement>\n";

    // --- Lieferung
    let liefer = leistungsdatum(&doc.leistung).unwrap_or_else(|| doc.datum.clone());
    let _ = write!(
        o,
        "    <ram:ApplicableHeaderTradeDelivery>\n      <ram:ActualDeliverySupplyChainEvent>\n        <ram:OccurrenceDateTime>\n          <udt:DateTimeString format=\"102\">{}</udt:DateTimeString>\n        </ram:OccurrenceDateTime>\n      </ram:ActualDeliverySupplyChainEvent>\n    </ram:ApplicableHeaderTradeDelivery>\n",
        d102(&liefer)
    );

    // --- Abrechnung
    let _ = write!(
        o,
        "    <ram:ApplicableHeaderTradeSettlement>\n      <ram:PaymentReference>{}</ram:PaymentReference>\n      <ram:InvoiceCurrencyCode>EUR</ram:InvoiceCurrencyCode>\n",
        xml(&doc.nummer)
    );
    if iban_valid(&f.iban) {
        let _ = write!(
            o,
            "      <ram:SpecifiedTradeSettlementPaymentMeans>\n        <ram:TypeCode>58</ram:TypeCode>\n        <ram:PayeePartyCreditorFinancialAccount>\n          <ram:IBANID>{}</ram:IBANID>\n          <ram:AccountName>{}</ram:AccountName>\n        </ram:PayeePartyCreditorFinancialAccount>\n",
            iban_clean(&f.iban),
            xml(f.name.trim())
        );
        let bic: String = f.bic.chars().filter(|c| !c.is_whitespace()).collect();
        if bic.len() == 8 || bic.len() == 11 {
            let _ = write!(o, "        <ram:PayeeSpecifiedCreditorFinancialInstitution>\n          <ram:BICID>{}</ram:BICID>\n        </ram:PayeeSpecifiedCreditorFinancialInstitution>\n", xml(&bic.to_uppercase()));
        }
        o += "      </ram:SpecifiedTradeSettlementPaymentMeans>\n";
    }
    // Steuern je Satz (inkl. 0 %-Anteil)
    let mut saetze: Vec<(u8, Cents, Cents)> = sum.steuern.clone();
    let null = sum.netto - sum.steuern.iter().map(|t| t.1).sum::<Cents>();
    if null != 0 || saetze.is_empty() {
        saetze.insert(0, (0, null, 0));
    }
    for (rate, basis, steuer) in &saetze {
        let (cat, r) = kategorie(doc, *rate);
        let _ = write!(o, "      <ram:ApplicableTradeTax>\n        <ram:CalculatedAmount>{}</ram:CalculatedAmount>\n        <ram:TypeCode>VAT</ram:TypeCode>\n", amt(*steuer));
        if cat == "E" {
            let _ = writeln!(o, "        <ram:ExemptionReason>{}</ram:ExemptionReason>", xml(KU_GRUND));
        }
        let _ = write!(
            o,
            "        <ram:BasisAmount>{}</ram:BasisAmount>\n        <ram:CategoryCode>{cat}</ram:CategoryCode>\n",
            amt(*basis)
        );
        if cat == "E" {
            o += "        <ram:ExemptionReasonCode>VATEX-EU-O</ram:ExemptionReasonCode>\n";
        }
        let _ = write!(o, "        <ram:RateApplicablePercent>{r}</ram:RateApplicablePercent>\n      </ram:ApplicableTradeTax>\n");
    }
    // Gesamtrabatt je Steuersatz
    for (rate, betrag) in &sum.rabatt_saetze {
        if *betrag == 0 {
            continue;
        }
        let (cat, r) = kategorie(doc, *rate);
        let _ = write!(
            o,
            "      <ram:SpecifiedTradeAllowanceCharge>\n        <ram:ChargeIndicator>\n          <udt:Indicator>false</udt:Indicator>\n        </ram:ChargeIndicator>\n        <ram:ActualAmount>{}</ram:ActualAmount>\n        <ram:Reason>Rabatt</ram:Reason>\n        <ram:CategoryTradeTax>\n          <ram:TypeCode>VAT</ram:TypeCode>\n          <ram:CategoryCode>{cat}</ram:CategoryCode>\n          <ram:RateApplicablePercent>{r}</ram:RateApplicablePercent>\n        </ram:CategoryTradeTax>\n      </ram:SpecifiedTradeAllowanceCharge>\n",
            amt(*betrag)
        );
    }
    // Zahlungsbedingungen
    let mut bed = String::new();
    if !doc.faellig.is_empty() && !storno {
        bed = format!("Zahlbar bis {} ohne Abzug.", date::german(&doc.faellig));
    }
    if doc.skonto_betrag() > 0 && !storno {
        // strukturierte Skonto-Angabe nach XRechnung-Konvention
        let _ = write!(
            bed,
            "\n#SKONTO#TAGE={}#PROZENT={:.2}#\n",
            doc.skonto_tage,
            doc.skonto_prozent
        );
    }
    if !bed.is_empty() || (!doc.faellig.is_empty() && !storno) {
        o += "      <ram:SpecifiedTradePaymentTerms>\n";
        if !bed.is_empty() {
            let _ = writeln!(o, "        <ram:Description>{}</ram:Description>", xml(&bed));
        }
        if !doc.faellig.is_empty() && !storno {
            let _ = write!(o, "        <ram:DueDateDateTime>\n          <udt:DateTimeString format=\"102\">{}</udt:DateTimeString>\n        </ram:DueDateDateTime>\n", d102(&doc.faellig));
        }
        o += "      </ram:SpecifiedTradePaymentTerms>\n";
    }
    let _ = write!(
        o,
        "      <ram:SpecifiedTradeSettlementHeaderMonetarySummation>\n        <ram:LineTotalAmount>{}</ram:LineTotalAmount>\n",
        amt(sum.positionen)
    );
    if sum.rabatt != 0 {
        let _ = writeln!(o, "        <ram:AllowanceTotalAmount>{}</ram:AllowanceTotalAmount>", amt(sum.rabatt));
    }
    let _ = write!(
        o,
        "        <ram:TaxBasisTotalAmount>{}</ram:TaxBasisTotalAmount>\n        <ram:TaxTotalAmount currencyID=\"EUR\">{}</ram:TaxTotalAmount>\n        <ram:GrandTotalAmount>{}</ram:GrandTotalAmount>\n        <ram:DuePayableAmount>{}</ram:DuePayableAmount>\n      </ram:SpecifiedTradeSettlementHeaderMonetarySummation>\n",
        amt(sum.netto),
        amt(sum.ust),
        amt(sum.brutto),
        amt(sum.brutto)
    );
    if let Some(orig) = data.dokument(doc.storno_von) {
        let _ = write!(
            o,
            "      <ram:InvoiceReferencedDocument>\n        <ram:IssuerAssignedID>{}</ram:IssuerAssignedID>\n        <ram:FormattedIssueDateTime>\n          <qdt:DateTimeString format=\"102\">{}</qdt:DateTimeString>\n        </ram:FormattedIssueDateTime>\n      </ram:InvoiceReferencedDocument>\n",
            xml(&orig.nummer),
            d102(&orig.datum)
        );
    }
    o += "    </ram:ApplicableHeaderTradeSettlement>\n  </rsm:SupplyChainTradeTransaction>\n</rsm:CrossIndustryInvoice>\n";
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::*;

    pub fn beispiel() -> (Data, Id) {
        let mut d = Data::new();
        let f = &mut d.firma;
        f.name = "Muster Haustechnik".into();
        f.zusatz = "Max Muster".into();
        f.strasse = "Werkstraße 5".into();
        f.plz_ort = "80331 München".into();
        f.telefon = "089 1234567".into();
        f.email = "info@muster.de".into();
        f.iban = "DE89 3704 0044 0532 0130 00".into();
        f.bic = "COBADEFFXXX".into();
        f.steuernummer = "143/123/45678".into();
        f.ust_id = "DE123456789".into();
        let k = d.neuer_kunde("2026-11-12");
        let kd = d.kunde_mut(k).unwrap();
        kd.firma = "Bäckerei Schön & Söhne GmbH".into();
        kd.strasse = "Am Markt 3".into();
        kd.plz = "80333".into();
        kd.ort = "München".into();
        kd.email = "rechnung@schoen.de".into();
        let r = d.neues_dokument(DokArt::Rechnung, Some(k), None, "2026-11-12");
        let x = d.dokument_mut(r).unwrap();
        x.positionen = vec![
            Position { text: "Wartung\nmit Prüfprotokoll".into(), menge: 1.0, einheit: "Pausch.".into(), preis: 18_900, ust: 19, ..Default::default() },
            Position { text: "Arbeitszeit".into(), menge: 2.5, einheit: "Std.".into(), preis: 6_500, ust: 19, rabatt: 10.0, ..Default::default() },
            Position { art: PosArt::Optional, text: "Option".into(), menge: 1.0, preis: 99_900, ust: 19, ..Default::default() },
            Position { text: "Fachbuch".into(), menge: 1.0, einheit: "Stk.".into(), preis: 3_000, ust: 7, ..Default::default() },
        ];
        x.rabatt_prozent = 5.0;
        x.skonto_prozent = 2.0;
        x.skonto_tage = 7;
        (d, r)
    }

    #[test]
    fn xml_summen_und_aufbau() {
        let (d, r) = beispiel();
        let doc = d.dokument(Some(r)).unwrap();
        assert!(pruefen(&d, doc, Profil::Zugferd).iter().all(|b| !b.0), "{:?}", pruefen(&d, doc, Profil::Zugferd));
        let x = cii_xml(&d, doc, Profil::Zugferd);
        let s = doc.summen();
        // Positionen: 189,00 + 146,25 + 30,00 = 365,25; Rabatt 5 % = 18,26
        assert_eq!(s.positionen, 36_525);
        assert!(x.contains("<ram:LineTotalAmount>365.25</ram:LineTotalAmount>"));
        assert!(x.contains(&format!("<ram:AllowanceTotalAmount>{}</ram:AllowanceTotalAmount>", amt(s.rabatt))));
        assert!(x.contains(&format!("<ram:GrandTotalAmount>{}</ram:GrandTotalAmount>", amt(s.brutto))));
        assert!(x.contains("Bäckerei Schön &amp; Söhne GmbH"));
        assert!(x.contains("unitCode=\"HUR\">2.5<"));
        assert!(x.contains("<ram:ActualAmount>16.25</ram:ActualAmount>"));
        assert!(x.contains("#SKONTO#TAGE=7#PROZENT=2.00#"));
        assert!(!x.contains("Option"), "optionale Positionen gehören nicht in die Rechnung");
        // Steuern je Satz stimmen mit den Rabatt-Anteilen überein
        let basis: Cents = s.steuern.iter().map(|t| t.1).sum();
        assert_eq!(basis, s.netto);
        // XRechnung verlangt E-Mail des Käufers und Referenz
        let xr = cii_xml(&d, doc, Profil::XRechnung);
        assert!(xr.contains("xrechnung_3.0"));
        assert!(xr.contains("schemeID=\"EM\">rechnung@schoen.de"));
        assert!(xr.contains("<ram:BuyerReference>K-1001</ram:BuyerReference>"));
    }

    #[test]
    fn ohne_ust_id_steuernummer_als_kennung() {
        let (mut d, r) = beispiel();
        d.firma.ust_id.clear();
        let x = cii_xml(&d, d.dokument(Some(r)).unwrap(), Profil::Zugferd);
        assert!(x.contains("<ram:SellerTradeParty>\n        <ram:ID>143/123/45678</ram:ID>"));
    }

    #[test]
    fn pruefung_findet_luecken() {
        let mut d = Data::new();
        let r = d.neues_dokument(DokArt::Rechnung, None, None, "2026-11-12");
        let doc = d.dokument(Some(r)).unwrap();
        let b = pruefen(&d, doc, Profil::XRechnung);
        assert!(b.iter().filter(|x| x.0).count() >= 5);
        assert!(!gueltig(&d, doc, Profil::Zugferd));
    }

    #[test]
    fn kleinunternehmer_und_codes() {
        let (mut d, r) = beispiel();
        d.dokument_mut(r).unwrap().kleinunternehmer = true;
        let x = cii_xml(&d, d.dokument(Some(r)).unwrap(), Profil::Zugferd);
        assert!(x.contains("<ram:CategoryCode>E</ram:CategoryCode>"));
        assert!(x.contains("§ 19 UStG"));
        assert_eq!(einheit_code("m²"), "MTK");
        assert_eq!(land_code("Österreich"), "AT");
        assert_eq!(leistungsdatum("12.11.2026"), Some("2026-11-12".into()));
        assert_eq!(leistungsdatum("Oktober 2026"), None);
    }
}
