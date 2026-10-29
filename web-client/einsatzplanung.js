'use strict';
// Abbau- und Bergungsdialog: seit 0.4.7 Teil der Einsatzplanung (web-client/einsatz.js). Diese Datei hält nur die
// bisherigen Einstiege (3D-Galaxie, Karte, Berichte) und leitet sie auf window.Einsatz um.
document.addEventListener('click',e=>{
 const b=e.target.closest('[data-work-site],[data-g3-abbau],[data-map-salvage],[data-report-salvage]');if(!b||!window.Einsatz)return;
 const mission=b.dataset.workMission||'recyceln';// data-g3-abbau ist das Trümmerfeld der 3D-Karteif(!['abbau','recyceln'].includes(mission))return;
 e.preventDefault();e.stopImmediatePropagation();const ziel=b.dataset.workSite||b.dataset.g3Abbau||b.dataset.mapSalvage||b.dataset.reportSalvage;
 b.closest('dialog')?.close();window.Einsatz.oeffnen({ziel,mission});
},true);
