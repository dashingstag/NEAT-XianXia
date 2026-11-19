export const traits=['compassion','ambition','aggression','caution'];
export const inputs=['danger','scarcity','qi','distress','trust','grudge','compassion','ambition','aggression','caution','power'];
export const hidden=['Survival','Empathy','Aspiration','Dominance','Attachment'];
export const actions=['Aid','Trade','Cultivate','Attack','Retreat','Observe'];
export const states={Mortal:{power:.15,cultivate:false,body:true},Cultivator:{power:.55,cultivate:true,body:true},Immortal:{power:1,cultivate:true,body:true},Spirit:{power:.4,cultivate:true,body:false},Sealed:{power:.05,cultivate:false,body:false}};
export const scenarios={
 'Roadside ambush':{danger:.8,scarcity:.3,qi:.25,distress:.85},
 'Spirit spring':{danger:.15,scarcity:.2,qi:.95,distress:.15},
 'Village famine':{danger:.35,scarcity:.95,qi:.1,distress:.9},
 'Sect rivalry':{danger:.65,scarcity:.55,qi:.7,distress:.35}
};
export function rng(seed){let n=seed>>>0;return()=>{n+=0x6D2B79F5;let t=n;t=Math.imul(t^t>>>15,t|1);t^=t+Math.imul(t^t>>>7,t|61);return((t^t>>>14)>>>0)/4294967296;};}
export function genome(){return {generation:0,hidden:[...hidden],ih:[
 [2,1,0,0,-.5,.4,0,0,0,1,-1],
 [0,0,0,1,.5,-.7,1.8,0,-.4,0,0],
 [-.2,0,1.8,0,0,0,0,1.6,0,-.4,.2],
 [.4,.5,0,0,-.7,1,0,.5,1.7,-.7,.5],
 [0,0,0,.4,1.8,-1,.5,0,0,0,0]],
 ho:[[ -.4,1.8,0,-.6,.8],[-.3,.4,0,-.3,1],[-.6,0,2,0,0],[-.5,-1,0,2,-1],[2,0,-.4,-.7,0],[.4,.1,.2,-.3,0]],hb:[-.8,-1,-1,-1,-.5],ob:[-.3,.1,-.5,-.6,-.4,.2]};}
export function initial(){return {seed:42,tick:0,selected:0,target:1,scenario:'Roadside ambush',env:{...scenarios['Roadside ambush']},disabled:[],npcs:[
 {name:'Lin Yue',state:'Mortal',compassion:.85,ambition:.6,aggression:.15,caution:.65},
 {name:'Shen Wu',state:'Cultivator',compassion:.2,ambition:.9,aggression:.8,caution:.3},
 {name:'Bai He',state:'Immortal',compassion:.7,ambition:.4,aggression:.2,caution:.8},
 {name:'Ash Wisp',state:'Spirit',compassion:.4,ambition:.7,aggression:.5,caution:.6}
 ].map((n,i)=>({...n,id:i,relationships:{0:0,1:i===0?-45:0,2:55,3:-15},grudges:{},genome:genome()}))};}
export function evaluate(npc,target,env,disabled=[]){
 const g=npc.genome,cap=states[npc.state],trust=(npc.relationships[target.id]||0)/100;
 const x=[env.danger,env.scarcity,env.qi,env.distress,trust,(npc.grudges[target.id]||0)/100,...traits.map(k=>npc[k]),cap.power].map((v,i)=>disabled.includes(inputs[i])?0:v);
 const h=g.ih.map((row,j)=>Math.tanh(g.hb[j]+row.reduce((s,w,i)=>s+w*x[i],0)));
 const logits=g.ho.map((row,j)=>g.ob[j]+row.reduce((s,w,i)=>s+w*h[i],0));
 const blocked=actions.map(a=>a==='Cultivate'&&!cap.cultivate?'No cultivation access':(['Aid','Trade','Attack'].includes(a)&&!cap.body)?'No physical body':null);
 const max=Math.max(...logits.filter((_,i)=>!blocked[i]));const ex=logits.map((v,i)=>blocked[i]?0:Math.exp(v-max));const sum=ex.reduce((a,b)=>a+b,0);
 const scores=ex.map(v=>v/sum);const winner=scores.indexOf(Math.max(...scores));
 return {x,h,logits,scores,blocked,winner,action:actions[winner],contributions:g.ho[winner].map((w,i)=>({node:g.hidden[i],weight:w,activation:h[i],value:w*h[i]})).sort((a,b)=>Math.abs(b.value)-Math.abs(a.value)),bias:g.ob[winner]};
}
export function step(s){const events=s.npcs.map((n,i)=>{const target=s.npcs[i===s.selected?s.target:(i+1)%s.npcs.length];return {npc:n,target,result:evaluate(n,target,s.env,s.disabled)};});for(const {npc,target,result}of events){let d={Aid:8,Trade:4,Attack:-16}[result.action]||0;target.relationships[npc.id]=Math.max(-100,Math.min(100,(target.relationships[npc.id]||0)+d));if(d<0)target.grudges[npc.id]=Math.min(100,(target.grudges[npc.id]||0)+12);}s.tick++;return events;}
export function breed(a,b,seed,rate=.15){const r=rng(seed),g=structuredClone(a.genome);g.generation=Math.max(a.genome.generation,b.genome.generation)+1;
 for(const key of ['ih','ho','hb','ob']){const cross=(v,w)=>Array.isArray(v)?v.map((n,i)=>cross(n,w?.[i] ?? n)):(r()<.5?v:(w ?? v))+(r()<rate?(r()-.5)*.8:0);g[key]=cross(g[key],b.genome[key]);}
 return {...a,name:`Descendant ${g.generation}`,genome:g,...Object.fromEntries(traits.map(k=>[k,Math.max(0,Math.min(1,(a[k]+b[k])/2+(r()-.5)*rate))])),relationships:{},grudges:{}};
}
export function splitConnection(g,seed){const r=rng(seed),j=Math.floor(r()*g.hidden.length),i=Math.floor(r()*inputs.length),w=g.ih[j][i];g.ih[j][i]=0;const row=inputs.map((_,k)=>k===i?1:0);g.ih.push(row);g.hb.push(0);g.hidden.push(`Evolved ${g.hidden.length-4}`);g.ho.forEach((row,k)=>row.push(g.ho[k][j]*w*.3));}

