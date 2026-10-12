//! Native Rust multi-player training environment with reset/step semantics.
//!
//! A joint step contains every configured player, including explicit empty no-ops.
//! Envelope/participant errors are rejected before any world mutation. Valid game
//! actions may still be rejected by the ordinary engine and appear in action results.
//! Only currently due roles may act, with the normal per-call action limit. Roles
//! execute in the engine's seeded player order. Uncontrolled players are explicitly
//! inert: their due calls close with no actions. This is not a Python Gym registration.
use crate::{Regelwerk, Rolle, SimZeit, SpielerId, Welt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RollenAktionen {
    pub rolle: Rolle,
    pub aktionen: Vec<Value>,
}
pub type GemeinsameAktion = BTreeMap<SpielerId, Vec<RollenAktionen>>;
pub type Beobachtungen = BTreeMap<SpielerId, Value>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Aktionsresultat {
    pub rolle: Rolle,
    pub index: usize,
    pub angenommen: bool,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UmgebungInfo {
    pub seed: u64,
    pub schritt: u64,
    pub sekunden: SimZeit,
    pub faellige_rollen: BTreeMap<SpielerId, Vec<Rolle>>,
    pub aktionsresultate: BTreeMap<SpielerId, Vec<Aktionsresultat>>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResetErgebnis {
    pub beobachtungen: Beobachtungen,
    pub info: UmgebungInfo,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchrittErgebnis {
    pub beobachtungen: Beobachtungen,
    /// Exact engine point delta, no floating-point reward shaping or hidden-state bonus.
    pub rewards: BTreeMap<SpielerId, i64>,
    /// The engine's normal epoch end was reached.
    pub terminated: bool,
    /// The configured training step limit was reached before the epoch ended.
    pub truncated: bool,
    pub info: UmgebungInfo,
}

pub struct Umgebung {
    regeln: Regelwerk,
    welt: Welt,
    spielerzahl: usize,
    kontrolliert: BTreeSet<SpielerId>,
    max_schritte: u64,
    schritte: u64,
    fertig: bool,
}
impl Umgebung {
    pub fn neu(
        regeln: Regelwerk,
        seed: u64,
        spielerzahl: usize,
        kontrolliert: BTreeSet<SpielerId>,
        max_schritte: u64,
    ) -> Result<Self, String> {
        if kontrolliert.is_empty() || kontrolliert.iter().any(|id| *id as usize >= spielerzahl) {
            return Err("Kontrollierte Spieler müssen gültig und nicht leer sein".into());
        }
        if max_schritte == 0 {
            return Err("max_schritte muss positiv sein".into());
        }
        let mut welt = Welt::neu(regeln.clone(), seed, spielerzahl)?;
        welt.punkte_neu();
        welt.fenster_vorbereiten();
        Ok(Self {
            regeln,
            welt,
            spielerzahl,
            kontrolliert,
            max_schritte,
            schritte: 0,
            fertig: false,
        })
    }
    /// Rebuilds the exact initial world for a seed; configuration is retained.
    pub fn reset(&mut self, seed: u64) -> Result<ResetErgebnis, String> {
        let mut welt = Welt::neu(self.regeln.clone(), seed, self.spielerzahl)?;
        welt.punkte_neu();
        welt.fenster_vorbereiten();
        self.welt = welt;
        self.schritte = 0;
        self.fertig = false;
        Ok(self.beobachten())
    }
    /// Player-filtered observations only; never returns `Welt` or global latent data.
    pub fn beobachten(&self) -> ResetErgebnis {
        ResetErgebnis {
            beobachtungen: self.sichten(),
            info: self.info(BTreeMap::new()),
        }
    }
    pub fn zustand_hash(&self) -> String {
        self.welt.hash()
    }
    pub fn no_op(&self) -> GemeinsameAktion {
        self.kontrolliert
            .iter()
            .map(|&id| (id, Vec::new()))
            .collect()
    }
    fn sichten(&self) -> Beobachtungen {
        self.kontrolliert
            .iter()
            .map(|&id| (id, self.welt.sicht(id, Rolle::Alle)))
            .collect()
    }
    fn info(&self, resultate: BTreeMap<SpielerId, Vec<Aktionsresultat>>) -> UmgebungInfo {
        let mut rollen: BTreeMap<_, Vec<_>> = self
            .kontrolliert
            .iter()
            .map(|&id| (id, Vec::new()))
            .collect();
        for f in &self.welt.faellig {
            if let Some(v) = rollen.get_mut(&f.spieler) {
                v.push(f.rolle);
            }
        }
        UmgebungInfo {
            seed: self.welt.startwert,
            schritt: self.schritte,
            sekunden: self.welt.zeit,
            faellige_rollen: rollen,
            aktionsresultate: resultate,
        }
    }
    pub fn step(&mut self, aktionen: GemeinsameAktion) -> Result<SchrittErgebnis, String> {
        if self.fertig {
            return Err("Episode beendet; reset erforderlich".into());
        }
        // Everything before this boundary is read-only: malformed joint calls cannot
        // spend resources, advance clocks, append logs, or consume a role opportunity.
        if aktionen.keys().copied().collect::<BTreeSet<_>>() != self.kontrolliert {
            return Err("Gemeinsame Aktion muss exakt alle kontrollierten Spieler enthalten (leere Liste = No-op)".into());
        }
        let due: BTreeSet<_> = self
            .welt
            .faellig
            .iter()
            .map(|f| (f.spieler, f.rolle))
            .collect();
        for (&sid, batches) in &aktionen {
            let mut rollen = BTreeSet::new();
            for batch in batches {
                if batch.rolle == Rolle::Alle || !rollen.insert(batch.rolle) {
                    return Err(
                        "Rollen müssen eindeutig sein; Alle ist kein ausführbarer Agent".into(),
                    );
                }
                if batch.aktionen.len() > self.regeln.agenten.aktionen_je_aufruf {
                    return Err("Zu viele Aktionen für einen Rollenaufruf".into());
                }
                if !batch.aktionen.is_empty() && !due.contains(&(sid, batch.rolle)) {
                    return Err(format!(
                        "Rolle {} von Spieler {sid} ist nicht fällig",
                        batch.rolle
                    ));
                }
                if batch
                    .aktionen
                    .iter()
                    .any(|a| !a.is_object() || a["typ"].as_str().is_none())
                {
                    return Err("Jede Aktion braucht ein Objekt mit Stringfeld typ".into());
                }
            }
        }
        let vorher: BTreeMap<_, _> = self
            .kontrolliert
            .iter()
            .map(|&sid| (sid, self.welt.spieler[sid as usize].punkte.gesamt()))
            .collect();
        let mut resultate: BTreeMap<_, Vec<_>> = self
            .kontrolliert
            .iter()
            .map(|&id| (id, Vec::new()))
            .collect();
        for f in self.welt.faellig.clone() {
            let batch = aktionen
                .get(&f.spieler)
                .and_then(|b| b.iter().find(|a| a.rolle == f.rolle));
            let mut hinweise = Vec::new();
            if let Some(batch) = batch {
                for (index, a) in batch.aktionen.iter().enumerate() {
                    let (angenommen, text) = self.welt.handeln(f.spieler, f.rolle, a);
                    if !angenommen {
                        hinweise.push(text.clone());
                    }
                    resultate
                        .get_mut(&f.spieler)
                        .unwrap()
                        .push(Aktionsresultat {
                            rolle: f.rolle,
                            index,
                            angenommen,
                            text,
                        });
                }
            }
            self.welt
                .aufruf_ende(f.spieler, f.rolle, None, None, &hinweise);
        }
        self.welt.schritt_wenn_bereit()?;
        self.welt.punkte_neu();
        self.schritte += 1;
        let terminated = self.welt.beendet();
        let truncated = !terminated && self.schritte >= self.max_schritte;
        self.fertig = terminated || truncated;
        let rewards = self
            .kontrolliert
            .iter()
            .map(|&sid| {
                (
                    sid,
                    self.welt.spieler[sid as usize].punkte.gesamt() - vorher[&sid],
                )
            })
            .collect();
        Ok(SchrittErgebnis {
            beobachtungen: self.sichten(),
            rewards,
            terminated,
            truncated,
            info: self.info(resultate),
        })
    }
}
