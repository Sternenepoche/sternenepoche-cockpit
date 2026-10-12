// Local TOTP and the optional, disabled-by-default email registration flow.
(() => {
  'use strict';
  let challenge=null,grant=null,owner=null,server=null,reauth='',expiry,recovery=[];
  function clearPrivate(){
    reauth='';recovery=[];clearTimeout(expiry);
    $('authenticator-qr').removeAttribute('src');$('authenticator-link').removeAttribute('href');
    $('authenticator-manual').textContent='';$('authenticator-codes').textContent='';
    $('authenticator-setup').hidden=true;$('authenticator-recovery').hidden=true;
    $('authenticator-settings').reset();$('authenticator-confirm').reset();
  }
  async function availability(){
    try{
      const status=await api('/api/registration/status',undefined,false);
      $('email-send').disabled=!status.enabled;
      $('registration-availability').textContent=status.enabled?'1 · E-Mail bestätigen   2 · Reich einrichten   3 · Losspielen':'Neue Konten werden derzeit vom Betreiber angelegt. Mit einem vorhandenen Testkonto kannst du dich direkt anmelden.';
    }catch{$('email-send').disabled=true;$('registration-availability').textContent='Registrierung derzeit nicht erreichbar.';}
  }
  function render(me){
    if(owner!==session?.name||server!==base){clearPrivate();challenge=null;grant=null;for(const id of ['email-start','email-verify','registration-profile'])$(id).reset();$('email-start').hidden=false;$('email-verify').hidden=true;$('registration-profile').hidden=true;owner=session?.name;server=base;}
    const enabled=!!me?.authenticator_enabled;
    $('authenticator-status').textContent=enabled?'Verbunden · Beim nächsten Anmelden brauchst du zusätzlich einen Code aus der App oder einen Wiederherstellungscode.':'Noch nicht verbunden · Die Einrichtung ist freiwillig.';
    $('authenticator-create').hidden=enabled;$('authenticator-remove').hidden=!enabled;$('authenticator-remove-code').hidden=!enabled;
    if(base)void availability();
  }
  $('email-start').addEventListener('submit',guard(async e=>{
    const result=await api('/api/registration/start',Object.fromEntries(new FormData(e.target)),false);
    challenge=result.challenge_id;grant=null;server=base;$('email-verify').hidden=false;$('registration-profile').hidden=true;
    show('Bestätigungscode versendet. Prüfe auch deinen Spamordner.');$('email-verify').elements.code.focus();
  }));
  $('email-verify').addEventListener('submit',guard(async e=>{
    if(!challenge||server!==base)throw Error('Fordere zuerst einen neuen E-Mail-Code an.');
    const result=await api('/api/registration/verify',{challenge_id:challenge,code:e.target.elements.code.value},false);
    grant=result.registration_token;$('email-verify').hidden=true;$('email-start').hidden=true;$('registration-profile').hidden=false;e.target.reset();
    $('registration-profile').elements.name.focus();
  }));
  $('registration-profile').addEventListener('submit',guard(async e=>{
    if(!grant||server!==base)throw Error('Bestätige zuerst deine E-Mail-Adresse.');
    const fields=Object.fromEntries(new FormData(e.target));
    if(fields.password!==fields.confirm_password)throw Error('Die Passwörter stimmen nicht überein.');
    delete fields.confirm_password;
    session=await api('/api/register',{...fields,registration_token:grant,world_id:world.world_id},false);
    grant=null;challenge=null;sessionStorage.setItem('sternenepoche-session',JSON.stringify({server:base,...session}));e.target.reset();await refresh();
  }));
  $('authenticator-settings').addEventListener('submit',guard(async e=>{
    const fields=Object.fromEntries(new FormData(e.target));
    if(e.submitter.value==='disable'){
      await api('/api/authenticator/disable',fields);clearPrivate();await refresh();show('Authenticator entfernt.');
    }else{
      const result=await api('/api/authenticator/setup',{password:fields.password});
      clearPrivate();reauth=fields.password;$('authenticator-qr').src=/^data:image\/png;base64,[A-Za-z0-9+/=]+$/.test(String(result.qr_image))?result.qr_image:'';
      $('authenticator-link').href=/^otpauth:\/\//i.test(String(result.otpauth_uri))?result.otpauth_uri:'#';$('authenticator-manual').textContent=result.manual_key;
      $('authenticator-setup').hidden=false;expiry=setTimeout(()=>{clearPrivate();show('Einrichtung abgelaufen. Erzeuge bei Bedarf einen neuen QR-Code.');},result.expires_in*1000);
      $('authenticator-confirm').elements.code.focus();
    }
  }));
  $('authenticator-confirm').addEventListener('submit',guard(async e=>{
    const result=await api('/api/authenticator/enable',{password:reauth,code:e.target.elements.code.value});
    clearPrivate();recovery=result.recovery_codes;
    $('authenticator-codes').textContent=recovery.join('\n');$('authenticator-recovery').hidden=false;
    await refresh();show('Authenticator verbunden. Sichere jetzt deine Wiederherstellungscodes.');
  }));
  $('authenticator-download').addEventListener('click',()=>{
    if(!recovery.length)return;
    const text=`STERNENEPOCHE · WIEDERHERSTELLUNGSCODES\nKonto: ${session.name}\nServer: ${base}\n\nJeder Code funktioniert genau einmal anstelle des Authenticator-Codes. Geheim und getrennt vom Handy aufbewahren.\n\n${recovery.join('\n')}\n`;
    const url=URL.createObjectURL(new Blob([text],{type:'text/plain;charset=utf-8'}));const a=document.createElement('a');a.href=url;a.download='Sternenepoche-Wiederherstellungscodes.txt';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000);
  });
  window.addEventListener('pagehide',clearPrivate);
  function fitQR(){
    const qr=$('authenticator-qr'),container=$('authenticator-setup');
    if(!qr.naturalWidth||container.hidden)return;
    // The local PNG encoder draws each QR module at eight pixels. Keep whole
    // modules on screen rather than resampling a dense code to an arbitrary size.
    const modules=qr.naturalWidth/8;
    const scale=Math.max(1,Math.min(8,Math.floor((Math.min(360,container.clientWidth)-32)/modules)));
    qr.style.width=(modules*scale+32)+'px';
  }
  $('authenticator-qr').addEventListener('load',fitQR);
  new ResizeObserver(fitQR).observe($('authenticator-setup'));
  window.SternenSecurity={render};
})();
