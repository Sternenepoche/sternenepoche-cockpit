'use strict';
// Shared prerequisite source for the game and public website. No decorative unlocks.
window.TechnologyRules={
 rule(c,kind,key){return kind==='research'?c.forschung[key]:kind==='buildings'?c.gebaeude[key]:['ships','defenses'].includes(kind)?c.einheiten[key]?.regel:kind==='civilization'?c.stufen?.[Number(key)-2]:null;},
 dependencies(c,kind,key,scope='local'){
  const r=this.rule(c,kind,key),deps=[],add=(kind,key,level,scope='local')=>deps.push({kind,key,level,scope});if(!r)return deps;
  if(kind==='civilization'){
   if(Number(key)>2)add('civilization',String(Number(key)-1),Number(key)-1);
   for(const[k,n]of Object.entries(r.gebaeude||{}))add('buildings',k,n,'realm');
   for(const[k,n]of Object.entries(r.forschung||{}))add('research',k,n);
  }else{
   if(r.ab_stufe>1)add('civilization',String(r.ab_stufe),r.ab_stufe);
   if(kind==='research'){
    add('buildings','labor',r.labor,'home');const gates=c.forschung_voraussetzungen?.[key];
    for(const[k,n]of Object.entries(gates?.forschung||{}))add('research',k,n);
    for(const[k,n]of Object.entries(gates?.gebaeude_reich||{}))add('buildings',k,n,'realm');
   }else{
    if(kind==='ships')add('buildings','werft',Math.max(1,r.werft||0));
    for(const[k,n]of Object.entries(c.einheiten[key]?.taktik?.voraussetzungen||{}))add('research',k,n);
    for(const[k,n]of Object.entries(r.braucht||{}))add('buildings',k,n,scope);
   }
  }return deps;
 },
 graph(c,target){
  const nodes=new Map(),edges=[],visiting=new Set(),cycles=[];
  const idFor=n=>[n.kind,n.key,n.kind==='buildings'&&n.scope==='realm'?'realm':'local'].join(':');
  const visit=n=>{const id=idFor(n);if(visiting.has(id)){cycles.push(id);return id;}
   const old=nodes.get(id);if(old){old.level=Math.max(old.level,n.level);if(!old.levels.includes(n.level))old.levels.push(n.level);return id;}
   const node={...n,id,levels:[n.level],rule:this.rule(c,n.kind,n.key)};nodes.set(id,node);visiting.add(id);
   for(const dep of this.dependencies(c,n.kind,n.key,n.scope)){const from=visit(dep);if(from!==id&&!edges.some(e=>e.from===from&&e.to===id))edges.push({from,to:id,level:dep.level});}
   visiting.delete(id);return id;
  };
  const targetId=visit({...target,scope:target.scope||'local'}),depth=new Map();
  const rank=id=>{if(depth.has(id))return depth.get(id);depth.set(id,0);const parents=edges.filter(e=>e.to===id);const n=parents.length?1+Math.max(...parents.map(e=>rank(e.from))):0;depth.set(id,n);return n;};
  for(const n of nodes.values()){n.depth=rank(n.id);n.levels.sort((a,b)=>a-b);}
  return {nodes:[...nodes.values()],edges,targetId,cycles};
 }
};
