use super::modell::ErklaerterWert;
use crate::{
    typen::*,
    welt::{Planet, F_DEUTERIUM, F_ERZ, F_KRISTALL, F_NAHRUNG, F_SOLAR},
    Regelwerk,
};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct WerksBilanz {
    pub gebaeude: Gebaeude,
    pub stufe: u8,
    pub arbeiter_bedarf: i64,
    pub fachkraefte_bedarf: i64,
    pub arbeiter_zugewiesen: i64,
    pub fachkraefte_zugewiesen: i64,
    pub besetzung_vor_integritaet_promille: i64,
    pub integritaet_promille: i64,
    pub wirksame_besetzung_promille: i64,
    pub eingangsdeckung_promille: i64,
    pub energiebedarf: i64,
    pub energie_vor_versorgungsnetz: i64,
    pub produktion: [i64; GUETER],
    pub verbrauch: [i64; GUETER],
    pub ausstoss: Option<ErklaerterWert>,
    pub engpaesse: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct WirtschaftErgebnis {
    pub rate: [i64; GUETER],
    pub energie_erzeugung: i64,
    pub energie_verbrauch: i64,
    pub arbeit_bedarf: i64,
    pub arbeit_verfuegbar: i64,
    pub fk_bedarf: i64,
    pub fk_verfuegbar: i64,
    pub nahrung_deckung: i64,
    pub konsum_deckung: i64,
    pub fp_rate: i64,
    pub wohnraum: i64,
    pub produktion: [i64; GUETER],
    pub verbrauch: [i64; GUETER],
    pub verbrauch_bevoelkerung: [i64; GUETER],
    pub energie_bevoelkerung: i64,
    pub energie_deckung_promille: i64,
    pub produktivitaet_promille: i64,
    pub automatisierung_faktor: f64,
    /// Dysonsphäre (nur in Dyson-Welten auf Planeten der Eigner-Allianz im System; sonst nicht serialisiert).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dyson: Option<serde_json::Value>,
    pub energie_erklaerung: ErklaerterWert,
    pub werke: Vec<WerksBilanz>,
}

pub fn berechnen(
    r: &Regelwerk,
    p: &Planet,
    forschung: &[u8; FORSCHUNGEN],
    volk: Volk,
    integritaet: &[i64; GEBAEUDE],
) -> WirtschaftErgebnis {
    berechnen_mit_personen(r,p,forschung,volk,integritaet,None)
}
pub fn berechnen_mit_personen(r:&Regelwerk,p:&Planet,forschung:&[u8;FORSCHUNGEN],volk:Volk,integritaet:&[i64;GEBAEUDE],personen:Option<&crate::ausbau::Personen>) -> WirtschaftErgebnis {
    berechnen_geregelt(r,p,forschung,volk,integritaet,personen,&[100;GEBAEUDE],true,None)
}
/// Personal erst ab Zivilisationsstufe III (Karl 23.10.2026: „vor Stufe III will ich damit nichts zu tun haben … die
/// Produktion darf nicht gedrosselt werden, weil ich keine Leute habe … ab III sehr soft“). Gilt für alle Welten mit Personen.
pub const PERSONAL_AB_STUFE: u8 = 3;
/// Vor Stufe III: Bevölkerungswachstum, Wohnraum je Wohnblock, Grundausbildung und Forschung mal diesem Faktor.
pub const FRUEHPHASE_FAKTOR: i64 = 2;
/// Ab Stufe III: Ein Werk läuft trotz Personalmangel mit mindestens diesem Anteil (Promille), der Rest folgt der Deckung.
pub const PERSONAL_SANFT_BODEN_PROMILLE: i64 = 750;
/// Ohne Zivilisationsstufe (Tests, Simulator): wie ab Stufe III.
pub fn berechnen_geregelt(r:&Regelwerk,p:&Planet,forschung:&[u8;FORSCHUNGEN],volk:Volk,integritaet:&[i64;GEBAEUDE],personen:Option<&crate::ausbau::Personen>,regler:&[u8;GEBAEUDE],xeno_natuerlich:bool,dyson:Option<crate::dyson::DysonWirkung>) -> WirtschaftErgebnis {
    berechnen_geregelt_stufe(r,p,forschung,volk,integritaet,personen,regler,xeno_natuerlich,dyson,PERSONAL_AB_STUFE)
}
#[allow(clippy::too_many_arguments)]
pub fn berechnen_geregelt_stufe(r:&Regelwerk,p:&Planet,forschung:&[u8;FORSCHUNGEN],volk:Volk,integritaet:&[i64;GEBAEUDE],personen:Option<&crate::ausbau::Personen>,regler:&[u8;GEBAEUDE],xeno_natuerlich:bool,dyson:Option<crate::dyson::DysonWirkung>,zivilisationsstufe:u8) -> WirtschaftErgebnis {
    let anschluss=dyson.filter(|d|d.anschluss);
    let geregelt=personen.is_some();
    let fruehphase=geregelt && zivilisationsstufe<PERSONAL_AB_STUFE;
    let output=|g:Gebaeude| if geregelt && crate::industrie::PRODUKTION.contains(&g) {regler[g.idx()] as i64*10}else{1000};
    let demand=|g:Gebaeude| if geregelt && crate::industrie::PRODUKTION.contains(&g) {crate::industrie::bedarf_prozent(regler[g.idx()])*10}else{1000};
    let synthesis=geregelt && !xeno_natuerlich;
    let faktoren=if personen.is_some() {crate::habitabilitaet::faktoren(r,p)} else {p.faktor};
    let hp = |g: Gebaeude| integritaet[g.idx()];
    let mut werke: Vec<WerksBilanz> = Gebaeude::ALLE
        .iter()
        .map(|g| WerksBilanz {
            gebaeude: *g,
            stufe: p.gebaeude[g.idx()],
            arbeiter_bedarf: 0,
            fachkraefte_bedarf: 0,
            arbeiter_zugewiesen: 0,
            fachkraefte_zugewiesen: 0,
            besetzung_vor_integritaet_promille: 1000,
            integritaet_promille: hp(*g),
            wirksame_besetzung_promille: 1000,
            eingangsdeckung_promille: 1000,
            energiebedarf: 0,
            energie_vor_versorgungsnetz: 0,
            produktion: [0; GUETER],
            verbrauch: [0; GUETER],
            ausstoss: None,
            engpaesse: Vec::new(),
        })
        .collect();
    let w = &r.wirtschaft;
    let st = &r.stabilitaet;
    let volk_art = volk;
    let volk = r.volk(volk);
    let t = |f: Forschung| forschung[f.idx()] as f64;
    let n = |g: Gebaeude| p.gebaeude[g.idx()];

    // Arbeitskräfte und Fachkräfte nach Priorität verteilen.
    let auto_f = (1.0 - w.automatisierung_bonus * t(Forschung::Automatisierung)).max(r.kw().produktion.automatisierung_untergrenze);
    let arbeit_verfuegbar = mal(p.bevoelkerung, w.arbeitsquote);
    let mut frei = arbeit_verfuegbar;
    let fk_verfuegbar = (w.fachkraefte_basis * M
        + anteil(
            r.stufenwert(w.fachkraefte_je_akademie, n(Gebaeude::Akademie)),
            hp(Gebaeude::Akademie),
            1000,
        ))
    .min(frei);
    let fk_verfuegbar=personen.map(|p|mal(p.fachkraefte(),w.arbeitsquote)).unwrap_or(fk_verfuegbar);
    let mut fk_frei = fk_verfuegbar;
    let mut forscher_frei=personen.map(|p|mal(p.forscher(),w.arbeitsquote)).unwrap_or(0);
    let mut allgemein_frei=(frei-fk_frei-forscher_frei).max(0);
    let mut besetzung = [1000i64; GEBAEUDE];
    let mut arbeit_bedarf = 0i64;
    let mut fk_bedarf = 0i64;
    for g in crate::Welt::reihenfolge(p) {
        let stufe = n(g);
        if stufe == 0 {
            continue;
        }
        let gr = r.geb(g);
        let b_a = anteil(mal(r.stufenwert(gr.arbeiter, stufe), auto_f),demand(g),1000);
        let b_f = anteil(r.stufenwert(gr.fachkraefte, stufe),demand(g),1000);
        arbeit_bedarf += b_a + b_f;
        fk_bedarf += b_f;
        let (hat_f,hat_a)=if personen.is_some(){
            let pool=if g==Gebaeude::Labor{&mut forscher_frei}else{&mut fk_frei};
            let qualified=b_f.min(*pool).min(frei);*pool-=qualified;frei-=qualified;
            let general=b_a.min(frei);let mut needed=general;
            for pool in [&mut allgemein_frei,&mut fk_frei,&mut forscher_frei]{let take=needed.min(*pool);*pool-=take;needed-=take;}
            frei-=general;(qualified,general)
        }else{
            let qualified=b_f.min(fk_frei).min(frei);fk_frei-=qualified;frei-=qualified;
            let general=b_a.min(frei);frei-=general;(qualified,general)
        };
        let q_a = if b_a > 0 { hat_a * 1000 / b_a } else { 1000 };
        let q_f = if b_f > 0 { hat_f * 1000 / b_f } else { 1000 };
        // Vor Stufe III läuft jedes Werk voll; ab III mindestens PERSONAL_SANFT_BODEN_PROMILLE (ohne Personen: wie bisher).
        let (q_a, q_f) = if fruehphase { (1000, 1000) } else if geregelt {
            let sanft = |q: i64| PERSONAL_SANFT_BODEN_PROMILLE + anteil(1000 - PERSONAL_SANFT_BODEN_PROMILLE, q, 1000);
            (sanft(q_a), sanft(q_f))
        } else { (q_a, q_f) };
        besetzung[g.idx()] = anteil(q_a.min(q_f), hp(g), 1000);
        let werk = &mut werke[g.idx()];
        werk.arbeiter_bedarf = b_a;
        werk.fachkraefte_bedarf = b_f;
        werk.arbeiter_zugewiesen = hat_a;
        werk.fachkraefte_zugewiesen = hat_f;
        werk.besetzung_vor_integritaet_promille = q_a.min(q_f);
        werk.wirksame_besetzung_promille = besetzung[g.idx()];
        if q_a < 1000 {
            werk.engpaesse.push("arbeiter".into());
        }
        if q_f < 1000 {
            werk.engpaesse.push("fachkraefte".into());
        }
        if hp(g) < 1000 {
            werk.engpaesse.push("integritaet".into());
        }
    }

    // Produktivität aus Stabilität.
    let a_pm = milli(st.produktivitaet_basis)
        + anteil(milli(st.produktivitaet_spanne), p.stabilitaet, 100 * M);
    let voll = |g: Gebaeude| {
        ErklaerterWert::neu(
            r.stufenwert(r.geb(g).ertrag, n(g)),
            "tausendstel_gut_je_spielstunde",
        )
        .promille("wirksame_besetzung_inkl_integritaet", besetzung[g.idx()])
        .promille("produktionsregler",output(g))
    };

    // Energie: Erzeugung gegen Verbrauch, alle Verbraucher laufen mit demselben Faktor.
    let e_f = (1.0 + w.energietechnik_bonus * t(Forschung::Energietechnik)) * volk.energie;
    let mut solar_trace = voll(Gebaeude::Solarkraftwerk)
        .promille("planet_solar", faktoren[F_SOLAR]);
    if let Some(d) = dyson { solar_trace = solar_trace.faktor("dyson_sonnenfaktor", d.sonnenfaktor); }
    let solar_trace = solar_trace
        .faktor("energietechnik_und_volk", e_f)
        .in_einheit("tausendstel_energie");
    let solar = solar_trace.wert;
    werke[Gebaeude::Solarkraftwerk.idx()].ausstoss = Some(solar_trace);
    werke[Gebaeude::Solarkraftwerk.idx()].energie_vor_versorgungsnetz = solar;
    let fus_bedarf = anteil(
        anteil(r.stufenwert(w.fusion_deuterium, n(Gebaeude::Fusionskraftwerk)),demand(Gebaeude::Fusionskraftwerk),1000),
        besetzung[Gebaeude::Fusionskraftwerk.idx()],
        1000,
    );
    let fus_q = if fus_bedarf > 0 {
        (p.bestand[Gut::Deuterium.idx()] * 1000 / fus_bedarf).min(1000)
    } else {
        1000
    };
    // Fusion mit eigenem Volksfaktor (Technik v1, `fusion_volk`; sonst der allgemeine Energiefaktor des Volkes).
    let e_fusion = (1.0 + w.energietechnik_bonus * t(Forschung::Energietechnik)) * r.fusion_volk_faktor(volk_art);
    let fusion_trace = voll(Gebaeude::Fusionskraftwerk)
        .promille("deuteriumvorrat", fus_q)
        .faktor("energietechnik_und_volk", e_fusion)
        .in_einheit("tausendstel_energie");
    let fusion = fusion_trace.wert;
    werke[Gebaeude::Fusionskraftwerk.idx()].ausstoss = Some(fusion_trace);
    werke[Gebaeude::Fusionskraftwerk.idx()].energie_vor_versorgungsnetz = fusion;
    if fus_q < 1000 {
        werke[Gebaeude::Fusionskraftwerk.idx()]
            .engpaesse
            .push("deuterium".into());
    }
    let fus_verbrauch = anteil(fus_bedarf, fus_q, 1000);
    // Mondgeneration: Solarsonden nach Sonnenabstand (nur wenn vorhanden; alte Erklärungen bleiben gleich).
    let sonden = p.einheiten[Einheit::Solarsonde.idx()];
    let sonden_energie = match (&r.monde, sonden > 0) {
        (Some(m), true) => sonden * crate::monde::solarsonde_energie_an(r, m, p.koord, forschung[Forschung::Energietechnik.idx()], volk.energie, 0.0),
        _ => 0,
    };
    let sonden_energie = match dyson { Some(d) if sonden_energie > 0 => mal(sonden_energie, d.sonnenfaktor), _ => sonden_energie };
    let mut energie_erklaerung = ErklaerterWert::neu(0, "tausendstel_energie")
        .plus("solar", solar)
        .plus("fusion", fusion);
    if sonden_energie > 0 { energie_erklaerung = energie_erklaerung.plus("solarsonden", sonden_energie); }
    let energie_erklaerung = energie_erklaerung
        .faktor(
            "versorgungsnetz_inkl_integritaet",
            1.0 + r.zusatz.versorgungsnetz_energie_bonus
                * n(Gebaeude::Versorgungsnetz) as f64
                * hp(Gebaeude::Versorgungsnetz) as f64
                / 1000.0,
        );
    let energie_erklaerung = match anschluss { Some(d) => energie_erklaerung.plus("dyson_einspeisung", d.einspeisung), None => energie_erklaerung };
    let energie_erzeugung = energie_erklaerung.wert;
    let mut energie_verbrauch = 0i64;
    for g in Gebaeude::ALLE {
        if n(g) > 0 {
            let bedarf = anteil(
                anteil(r.stufenwert(r.geb(g).energie, n(g)), mal(demand(g), if g==Gebaeude::Xenoextraktor && synthesis {r.kw().produktion.xeno_synthese_energie}else{1.0}),1000),
                besetzung[g.idx()],
                1000,
            );
            energie_verbrauch += bedarf;
            werke[g.idx()].energiebedarf = bedarf;
        }
    }
    let energie_bevoelkerung = if let Some(people)=personen {
        mal(people.gesamt()-people.organisch(),w.energie_je_1000_syntheten)/1000
    } else if volk.ohne_nahrung {
        mal(p.bevoelkerung, w.energie_je_1000_syntheten) / 1000
    } else {
        0
    };
    energie_verbrauch += energie_bevoelkerung;
    let e_pm = if anschluss.is_some() || energie_verbrauch <= energie_erzeugung {
        1000
    } else {
        energie_erzeugung * 1000 / energie_verbrauch
    };

    // Rohproduktion.
    let leistung = |g: Gebaeude| {
        voll(g)
            .promille("stabilitaet", a_pm)
            .promille("energiedeckung", e_pm)
    };
    let mut prod = [0i64; GUETER];
    let mut verb = [0i64; GUETER];
    for (g, gut, factor) in [
        (Gebaeude::Erzmine, Gut::Erz, F_ERZ),
        (Gebaeude::Kristallmine, Gut::Kristall, F_KRISTALL),
        (Gebaeude::Deuteriumsynthesizer, Gut::Deuterium, F_DEUTERIUM),
        (Gebaeude::Farm, Gut::Nahrung, F_NAHRUNG),
    ] {
        let mut trace = leistung(g).promille("planet", faktoren[factor]).faktor("produktionsausbau",if geregelt {r.kw().produktion.produktionsausbau}else{1.0});
        if g == Gebaeude::Deuteriumsynthesizer {
            if let Some(d) = anschluss { trace = trace.faktor("dyson_strombad", d.strombad); }
        }
        if g == Gebaeude::Farm {
            trace = trace.faktor(
                "volk_und_agrarwissenschaft",
                volk.nahrung * (1.0 + w.agrar_bonus * t(Forschung::Agrarwissenschaft)),
            );
        }
        // Technik v1: Plasmatechnik erhöht Erz, Kristall und Deuterium je Stufe (ohne Stufe keine Erklärungszeile).
        if let Some(f) = r.plasma_faktor(gut, forschung[Forschung::Plasmatechnik.idx()]) {
            trace = trace.faktor("plasmatechnik", f);
        }
        prod[gut.idx()] = trace.wert;
        werke[g.idx()].produktion[gut.idx()] = trace.wert;
        werke[g.idx()].ausstoss = Some(trace);
    }
    verb[Gut::Deuterium.idx()] += fus_verbrauch;
    werke[Gebaeude::Fusionskraftwerk.idx()].verbrauch[Gut::Deuterium.idx()] = fus_verbrauch;

    // Verarbeitung: nur so viel, wie Bestand plus Stundenproduktion an Eingängen hergeben.
    let mut verf = [0i64; GUETER];
    for g in 0..GUETER {
        verf[g] = p.bestand[g] + prod[g];
    }
    verf[Gut::Deuterium.idx()] = (verf[Gut::Deuterium.idx()] - fus_verbrauch).max(0);
    for (werk, gut) in [
        (Gebaeude::Giesserei, Gut::Legierung),
        (Gebaeude::Elektronikwerk, Gut::Elektronik),
        (Gebaeude::Konsumgueterwerk, Gut::Konsumgut),
    ] {
        let trace = leistung(werk).faktor("produktionsausbau",if geregelt {r.kw().produktion.produktionsausbau}else{1.0});
        let aus = trace.wert;
        if aus == 0 {
            werke[werk.idx()].ausstoss = Some(trace);
            continue;
        }
        let rezept = &w.rezepte[&gut];
        let overload=if output(werk)>0 {demand(werk) as f64/output(werk) as f64}else{0.0};
        let mut q = 1000i64;
        for (ein, menge) in rezept {
            let bedarf = mal(aus, *menge*overload);
            if bedarf > 0 {
                if verf[ein.idx()] < bedarf {
                    werke[werk.idx()]
                        .engpaesse
                        .push(format!("eingang_{}", ein.name()));
                }
                q = q
                    .min((verf[ein.idx()].max(0) as i128 * 1000 / bedarf as i128).min(1000) as i64);
            }
        }
        for (ein, menge) in rezept {
            let v = anteil(mal(aus, *menge*overload), q, 1000);
            verf[ein.idx()] -= v;
            verb[ein.idx()] += v;
            werke[werk.idx()].verbrauch[ein.idx()] += v;
        }
        werke[werk.idx()].eingangsdeckung_promille = q;
        let mut trace = trace.promille("gemeinsame_eingangsdeckung", q);
        if gut == Gut::Legierung {
            trace = trace.faktor(
                "werkstoffkunde",
                1.0 + w.werkstoffkunde_bonus * t(Forschung::Werkstoffkunde),
            );
        }
        prod[gut.idx()] += trace.wert;
        if geregelt {verf[gut.idx()] += trace.wert;}
        werke[werk.idx()].produktion[gut.idx()] = trace.wert;
        werke[werk.idx()].ausstoss = Some(trace);
    }

    // Natural deposits and costly synthesis share output, power and input accounting.
    let mut xeno=leistung(Gebaeude::Xenoextraktor)
        .faktor("produktionsausbau",if geregelt {r.kw().produktion.produktionsausbau}else{1.0})
        .faktor("xenomaterialkunde",1.0+w.xeno_bonus*t(Forschung::Xenomaterialkunde));
    if let Some(d)=anschluss { xeno=xeno.faktor("dyson_strombad",d.strombad); }
    if synthesis {
        xeno=xeno.faktor("synthese_statt_vorkommen",if anschluss.is_some_and(|d|d.xeno_ohne_malus){1.0}else{r.kw().produktion.xeno_synthese_ausbeute});
        let g=Gebaeude::Xenoextraktor;
        let overhead=if output(g)>0 {demand(g) as f64/output(g) as f64}else{0.0};
        let rf=anschluss.map_or(1.0,|d|d.rezept_faktor);
        let recipe=r.kw().produktion.rezept().map(|(g,n)|(g,n*rf));
        let mut q=1000;
        for (gut,n) in recipe {let need=mal(xeno.wert,n*overhead);if need>0 {q=q.min((verf[gut.idx()].max(0) as i128*1000/need as i128).min(1000) as i64);}}
        for (gut,n) in recipe {let used=anteil(mal(xeno.wert,n*overhead),q,1000);verb[gut.idx()]+=used;verf[gut.idx()]-=used;werke[g.idx()].verbrauch[gut.idx()]+=used;}
        if q<1000 {werke[g.idx()].engpaesse.push("syntheserohstoffe".into());}
        werke[g.idx()].eingangsdeckung_promille=q;xeno=xeno.promille("syntheseeingangsdeckung",q);
    }
    prod[Gut::Xenokristall.idx()]=xeno.wert;
    if let Some(d)=anschluss {
        // Strahlsynthese der Dysonsphäre: feste Erträge unabhängig von Gebäuden.
        prod[Gut::Deuterium.idx()]+=d.deuterium;
        prod[Gut::Xenokristall.idx()]+=d.xenokristall;
    }
    werke[Gebaeude::Xenoextraktor.idx()].produktion[Gut::Xenokristall.idx()]=xeno.wert;
    werke[Gebaeude::Xenoextraktor.idx()].ausstoss=Some(xeno);

    // Verbrauch der Bevölkerung und Deckungsgrade.
    let deckung = |gut: Gut, bedarf: i64| {
        if bedarf <= 0 {
            1000
        } else {
            ((p.bestand[gut.idx()] + prod[gut.idx()]) as i128 * 1000 / bedarf as i128).min(1000)
                as i64
        }
    };
    let nahrung_bedarf = if let Some(people)=personen {
        mal(people.organisch(),w.nahrung_je_1000)/1000
    } else if volk.ohne_nahrung {
        0
    } else {
        mal(p.bevoelkerung, w.nahrung_je_1000) / 1000
    };
    let nahrung_deckung = if let Some(people)=personen {
        let food=deckung(Gut::Nahrung,nahrung_bedarf);
        if people.gesamt()>people.organisch(){food.min(e_pm)}else{food}
    } else if volk.ohne_nahrung {
        e_pm
    } else {
        deckung(Gut::Nahrung, nahrung_bedarf)
    };
    // Vor Zivilisationsstufe III verbraucht die Bevölkerung keine Konsumgüter (Karl 23.10.2026); Deckung gilt dann als voll.
    let konsum_bedarf = if fruehphase { 0 } else { mal(p.bevoelkerung, w.konsum_je_1000) / 1000 };
    let konsum_deckung = deckung(Gut::Konsumgut, konsum_bedarf);
    verb[Gut::Nahrung.idx()] += nahrung_bedarf;
    verb[Gut::Konsumgut.idx()] += konsum_bedarf;

    let mut rate = [0i64; GUETER];
    for g in 0..GUETER {
        rate[g] = prod[g] - verb[g];
    }
    let fp_trace = leistung(Gebaeude::Labor).faktor(
        "volk_und_forschungsarchiv_inkl_integritaet",
        volk.forschung
            * (1.0
                + r.zusatz.archiv_fp_bonus
                    * n(Gebaeude::Forschungsarchiv) as f64
                    * hp(Gebaeude::Forschungsarchiv) as f64
                    / 1000.0),
    );
    let fp_trace = if fruehphase { fp_trace.faktor("vor_zivilisationsstufe_iii", FRUEHPHASE_FAKTOR as f64) } else { fp_trace };
    let fp_rate = fp_trace.wert;
    werke[Gebaeude::Labor.idx()].ausstoss =
        Some(fp_trace.in_einheit("tausendstel_fp_je_spielstunde"));
    let wohnraum = p.grundwohnraum
        + anteil(
            r.stufenwert(r.geb(Gebaeude::Wohnblock).ertrag, n(Gebaeude::Wohnblock)),
            hp(Gebaeude::Wohnblock),
            1000,
        ) * if fruehphase { FRUEHPHASE_FAKTOR } else { 1 }
        + anteil(
            r.zusatz.orbitalring_wohnraum * M * n(Gebaeude::Orbitalring) as i64,
            hp(Gebaeude::Orbitalring),
            1000,
        );

    let mut verbrauch_bevoelkerung = [0; GUETER];
    verbrauch_bevoelkerung[Gut::Nahrung.idx()] = nahrung_bedarf;
    verbrauch_bevoelkerung[Gut::Konsumgut.idx()] = konsum_bedarf;
    for werk in &mut werke {
        if e_pm < 1000
            && werk.ausstoss.is_some()
            && !matches!(
                werk.gebaeude,
                Gebaeude::Solarkraftwerk | Gebaeude::Fusionskraftwerk
            )
        {
            werk.engpaesse.push("energie".into());
        }
    }
    werke.retain(|w| w.stufe > 0);
    WirtschaftErgebnis {
        rate,
        energie_erzeugung,
        energie_verbrauch,
        arbeit_bedarf,
        arbeit_verfuegbar,
        fk_bedarf,
        fk_verfuegbar,
        nahrung_deckung,
        konsum_deckung,
        fp_rate,
        wohnraum,
        produktion: prod,
        verbrauch: verb,
        verbrauch_bevoelkerung,
        energie_bevoelkerung,
        energie_deckung_promille: e_pm,
        produktivitaet_promille: a_pm,
        automatisierung_faktor: auto_f,
        energie_erklaerung,
        werke,
        dyson: dyson.map(|d| serde_json::json!({"sonnenfaktor":d.sonnenfaktor,"anschluss":d.anschluss,"einspeisung_milli":d.einspeisung,
            "strahlsynthese_deuterium_milli":d.deuterium,"strahlsynthese_xenokristall_milli":d.xenokristall,"strombad":d.strombad})),
    }
}
