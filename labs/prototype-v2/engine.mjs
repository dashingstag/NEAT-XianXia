export const clamp=(v,lo=0,hi=1)=>Math.max(lo,Math.min(hi,v));
export function rng(seed){let a=seed>>>0;return()=>{a+=0x6D2B79F5;let t=a;t=Math.imul(t^t>>>15,t|1);t^=t+Math.imul(t^t>>>7,t|61);return((t^t>>>14)>>>0)/4294967296;};}
export const STATES={Mortal:{power:.18,body:true,cultivate:false},Cultivator:{power:.55,body:true,cultivate:true},Immortal:{power:1,body:true,cultivate:true},Spirit:{power:.4,body:false,cultivate:true},Sealed:{power:.06,body:false,cultivate:false}};
export const ACTIONS=['Aid','Trade','Attack','Retreat','Cultivate','Rest','Observe'];
export const FEATURES=['need','benefit','risk','affinity','resentment','compassion','ambition','aggression','caution','commitment','switchCost','aid','trade','attack','retreat','cultivate','rest','observe'];
export const PRESETS={
 'Roadside ambush':{danger:.8,qi:.25,food:3,spacing:1,hostile:1,injury:.35},
 'Village famine':{danger:.12,qi:.15,food:0,spacing:1,hostile:0,injury:.55},
 'Spirit spring':{danger:.1,qi:.95,food:3,spacing:1,hostile:0,injury:.1},
 'Distant rival':{danger:.45,qi:.5,food:2,spacing:5,hostile:1,injury:.15},
 'Quiet recovery':{danger:0,qi:0,food:2,spacing:1,hostile:0,injury:.45}
};
const node=(id,weights,bias=0)=>({id,bias,weights:Object.fromEntries(FEATURES.map(k=>[k,weights[k]||0]))});
export function genome(){return {generation:0,nodes:[
 node('seed:opportunity',{need:1.2,benefit:1.2,risk:-1.3},-.2),
 node('seed:social',{affinity:.8,resentment:-.8,compassion:.5,aid:.7,trade:.3,attack:-.7}),
 node('seed:risk',{risk:-1.1,caution:-.5,retreat:.7,rest:.4,attack:-.3}),
 node('seed:ambition',{ambition:.7,cultivate:1,benefit:.3},-.6),
 node('seed:conflict',{aggression:1,resentment:.8,attack:1,affinity:-.6},-.7)
 ],out:{'seed:opportunity':1.3,'seed:social':.5,'seed:risk':.6,'seed:ambition':.4,'seed:conflict':.3},direct:{commitment:.7,switchCost:-.6,observe:-.1},bias:0};}
export function world(preset='Roadside ambush'){
 const p=PRESETS[preset];const npcs=[['Lin Yue','Mortal',.85,.6,.15,.7],['Shen Wu','Cultivator',.25,.85,.8,.35],['Bai He','Immortal',.7,.5,.2,.65],['Ash Wisp','Spirit',.5,.7,.35,.6]].map(([name,state,compassion,ambition,aggression,caution],i)=>({id:`npc-${i}`,name,state,traits:{compassion,ambition,aggression,caution},hp:i===2?1:1-p.injury,energy:.8,food:p.food+(i===2?3:0),progress:0,exposure:1,x:i*p.spacing,hostile:i===1&&!!p.hostile,relations:{},grudges:{},memory:[],goal:null,genome:genome()}));
 return {preset,tick:0,env:{danger:p.danger,qi:p.qi},npcs,journal:[],metrics:{aid:0,trades:0,damage:0,progress:0,switches:0,decisions:0,invalid:0}};
}
export function needs(n){return {recovery:1-n.hp,rest:1-n.energy,hunger:STATES[n.state].body?clamp((2-n.food)/2):0,growth:n.traits.ambition};}
export function perceive(w,n){const cap=STATES[n.state];return {needs:needs(n),danger:w.env.danger*(n.exposure??1),qi:w.env.qi,targets:w.npcs.filter(t=>t.id!==n.id&&t.hp>0&&Math.abs(t.x-n.x)<=6).map(t=>({id:t.id,name:t.name,distance:Math.abs(t.x-n.x),injury:1-t.hp,food:t.food,hostile:t.hostile,relativePower:clamp(STATES[t.state].power/(cap.power+.01),0,6),trust:(n.relations[t.id]||0)/100,grudge:(n.grudges[t.id]||0)/100}))};}
function candidate(action,target,label,valid,reason,context){return {action,target,label,key:`${action}:${target||'self'}`,valid,reason:valid?'Available':reason,context};}
export function candidates(w,n){const p=perceive(w,n),cap=STATES[n.state],alive=n.hp>0,options=[];
 for(const t of p.targets){const reach=t.distance<=2,physical=cap.body&&STATES[w.npcs.find(n=>n.id===t.id).state].body;
 options.push(candidate('Aid',t.id,t.name,alive&&physical&&reach&&n.food>=1&&n.energy>=.12&&t.injury>=.05,'Needs living bodies, range ≤2, one food, energy ≥12%, and an injured target',t));
 options.push(candidate('Trade',t.id,t.name,alive&&physical&&reach&&n.food>=1&&t.food>=1&&n.energy>=.08,'Needs living bodies, range ≤2, one food on both sides, and energy ≥8%',t));
 options.push(candidate('Attack',t.id,t.name,alive&&physical&&reach&&n.energy>=.18,'Needs living bodies, range ≤2, and energy ≥18%',t));}
 options.push(candidate('Retreat',null,'away from danger',alive&&n.energy>=.04&&(p.danger>.05||p.targets.some(t=>t.hostile)), 'Needs a perceived threat and energy ≥4%',{}));
 options.push(candidate('Cultivate',null,'self',alive&&cap.cultivate&&w.env.qi>.05&&n.energy>=.18,'Needs cultivation access, qi >5%, and energy ≥18%',{}));
 options.push(candidate('Rest',null,'self',alive,'Actor is dead',{}));options.push(candidate('Observe',null,'surroundings',alive,'Actor is dead',{}));return {perception:p,options};
}
export function features(w,n,c){const p=perceive(w,n),t=c.context,a=c.action;let need=0,benefit=0,risk=0;
 if(a==='Aid'){need=t.injury;benefit=Math.min(.22,t.injury);risk=p.danger*.25;}
 if(a==='Trade'){need=p.needs.hunger;benefit=.08;risk=t.hostile?.6:.05;}
 if(a==='Attack'){need=t.hostile?p.danger:.05;benefit=clamp((1-t.relativePower/6)*.3);risk=clamp(t.relativePower/3+p.danger*.25);}
 if(a==='Retreat'){need=p.danger;benefit=.2*p.danger;risk=.04;}
 if(a==='Cultivate'){need=p.needs.growth;benefit=.12*w.env.qi;risk=p.danger;}
 if(a==='Rest'){const canHeal=!STATES[n.state].body||n.food>0;need=Math.max(canHeal?p.needs.recovery:0,p.needs.rest);benefit=(canHeal?Math.min(.06,p.needs.recovery):0)+Math.min(canHeal?.2:.08,p.needs.rest);risk=p.danger*.6;}
 if(a==='Observe'){need=.1;benefit=Math.min(.02,p.needs.rest);risk=p.danger*.2;}
 const goal=n.goal&&n.goal.remaining>0;return {need,benefit,risk,affinity:t.trust||0,resentment:t.grudge||0,...n.traits,commitment:goal&&n.goal.key===c.key?1:0,switchCost:goal&&n.goal.key!==c.key?1:0,...Object.fromEntries(ACTIONS.map(k=>[k.toLowerCase(),a===k?1:0]))};
}
export function policy(g,x){const nodes=g.nodes.map(n=>({id:n.id,z:n.bias+FEATURES.reduce((sum,k)=>sum+(n.weights[k]||0)*x[k],0)})).map(n=>({...n,activation:Math.tanh(n.z)}));const terms=nodes.map(n=>({...n,weight:g.out[n.id]||0,value:n.activation*(g.out[n.id]||0)}));const direct=Object.entries(g.direct).map(([id,weight])=>({id:`input:${id}`,weight,activation:x[id],value:weight*x[id]}));return {score:g.bias+[...terms,...direct].reduce((sum,t)=>sum+t.value,0),terms:[...terms,...direct],bias:g.bias};}
export function decide(w,n){const {perception,options}=candidates(w,n);const scored=options.map(c=>{const x=features(w,n,c);return {...c,features:x,...policy(n.genome,x)};});const valid=scored.filter(c=>c.valid).sort((a,b)=>b.score-a.score);const max=valid[0]?.score??0,den=valid.reduce((sum,c)=>sum+Math.exp(c.score-max),0);scored.forEach(c=>c.share=c.valid?Math.exp(c.score-max)/den:0);return {perception,options:scored.sort((a,b)=>Number(b.valid)-Number(a.valid)||b.score-a.score),choice:valid[0]||null};}
function remember(n,event){n.memory.unshift(event);n.memory=n.memory.slice(0,12);}
export function execute(w,n,requested){
 // Recheck after earlier actors: an opportunity may have disappeared this tick.
 const current=candidates(w,n).options.find(c=>c.key===requested.key);if(!current?.valid){return {executed:false,text:'Opportunity changed; action cancelled',delta:0};}
 const t=w.npcs.find(t=>t.id===current.target),a=current.action;let delta=0,text='';
 const switched=n.goal&&n.goal.remaining>0&&n.goal.key!==current.key;if(switched)w.metrics.switches++;
 n.goal={key:current.key,remaining:n.goal?.key===current.key?Math.max(0,n.goal.remaining-1):2};
 if(a==='Aid'){n.food--;n.energy-=.12;delta=Math.min(.22,1-t.hp);t.hp=clamp(t.hp+delta);t.relations[n.id]=clamp((t.relations[n.id]||0)+8,-100,100);w.metrics.aid+=delta;text=`restored ${Math.round(delta*100)} health to ${t.name}`;remember(t,{tick:w.tick,kind:'helped',actor:n.id,value:delta});}
 if(a==='Trade'){n.energy-=.08;n.food--;t.food--;delta=.08;n.energy=clamp(n.energy+.12);t.energy=clamp(t.energy+.12);t.relations[n.id]=clamp((t.relations[n.id]||0)+4,-100,100);n.relations[t.id]=clamp((n.relations[t.id]||0)+4,-100,100);w.metrics.trades++;text=`shared provisions with ${t.name}; both spent one food`;remember(t,{tick:w.tick,kind:'traded',actor:n.id,value:delta});}
 if(a==='Attack'){n.energy-=.18;delta=Math.min(t.hp,.16*STATES[n.state].power/(STATES[t.state].power+.15));t.hp=clamp(t.hp-delta);const retaliation=Math.min(n.hp,.08*STATES[t.state].power/(STATES[n.state].power+.15));n.hp=clamp(n.hp-retaliation);t.relations[n.id]=clamp((t.relations[n.id]||0)-20,-100,100);t.grudges[n.id]=clamp((t.grudges[n.id]||0)+18,0,100);w.metrics.damage+=delta;text=`dealt ${Math.round(delta*100)} damage to ${t.name}; took ${Math.round(retaliation*100)} retaliation`;remember(t,{tick:w.tick,kind:'attacked',actor:n.id,value:delta});}
 if(a==='Retreat'){const enemies=w.npcs.filter(t=>t.id!==n.id&&t.hostile&&t.hp>0).sort((a,b)=>Math.abs(a.x-n.x)-Math.abs(b.x-n.x));n.x+=enemies.length?(n.x<=enemies[0].x?-3:3):3;n.energy=clamp(n.energy-.04);n.exposure=(n.exposure??1)*.5;text='moved three units away from the nearest hostile';}
 if(a==='Cultivate'){n.energy-=.18;delta=.12*w.env.qi;n.progress+=delta;w.metrics.progress+=delta;text=`gained ${delta.toFixed(3)} cultivation progress`;}
 if(a==='Rest'){const embodied=STATES[n.state].body,food=embodied&&n.food>0;if(food)n.food--;const before=n.hp;n.energy=clamp(n.energy+(food||!embodied?.2:.08));n.hp=clamp(n.hp+(food||!embodied?.06:0));delta=n.hp-before;text=!embodied?'recovered spiritual energy and integrity':food?'consumed one food, recovered energy and health':'recovered a little energy; no food for healing';}
 if(a==='Observe'){n.energy=clamp(n.energy+.02);text='surveyed surroundings; recovered 2% energy';}
 n.energy=clamp(n.energy);w.metrics.decisions++;remember(n,{tick:w.tick,kind:a,target:current.target,value:delta});return {executed:true,text,delta,action:a,target:current.target};
}
export function tick(w){w.tick++;const offset=(w.tick-1)%w.npcs.length,events=[];for(let k=0;k<w.npcs.length;k++){const n=w.npcs[(offset+k)%w.npcs.length];if(n.hp<=0)continue;const d=decide(w,n);if(!d.choice)continue;const result=execute(w,n,d.choice);events.push({tick:w.tick,actor:n.id,name:n.name,action:d.choice.action,target:d.choice.label,score:d.choice.score,result,trace:d.choice.terms,features:d.choice.features});}
 for(const n of w.npcs){if(n.hp<=0)continue;const sheltered=n.memory[0]?.kind==='Retreat';const exposure=w.env.danger*(n.exposure??1)*(sheltered?.002:.008);n.hp=clamp(n.hp-exposure);for(const k of Object.keys(n.grudges))n.grudges[k]*=.99;}
 w.journal.unshift(...events);w.journal=w.journal.slice(0,80);return events;
}
export function mutate(g,seed,rate=.12){const r=rng(seed),c=structuredClone(g);const m=v=>clamp(v+(r()<rate?(r()-.5)*.5:0),-3,3);c.nodes.forEach(n=>{n.bias=m(n.bias);FEATURES.forEach(k=>n.weights[k]=m(n.weights[k]||0));c.out[n.id]=m(c.out[n.id]||0);});Object.keys(c.direct).forEach(k=>c.direct[k]=m(c.direct[k]));c.bias=m(c.bias);return c;}
export function cross(a,b,seed,rate=.12){const r=rng(seed),c=structuredClone(a);c.generation=Math.max(a.generation,b.generation)+1;
 for(const n of c.nodes){const other=b.nodes.find(o=>o.id===n.id);if(!other)continue;FEATURES.forEach(k=>{if(r()<.5)n.weights[k]=other.weights[k];});if(r()<.5)n.bias=other.bias;if(r()<.5)c.out[n.id]=b.out[n.id];}
 Object.keys(c.direct).forEach(k=>{if(k in b.direct&&r()<.5)c.direct[k]=b.direct[k];});if(r()<.5)c.bias=b.bias;return mutate(c,seed^0xA341316C,rate);
}
export function addNode(g,seed){if(g.nodes.length>=10)return false;const r=rng(seed),source=g.nodes[Math.floor(r()*g.nodes.length)];
 // Stable identity from its ancestry. Independent identical additions are homologous.
 const id=`branch:${source.id}`;if(g.nodes.some(n=>n.id===id))return false;
 g.nodes.push({...structuredClone(source),id});const weight=g.out[source.id]||0;g.out[source.id]=weight/2;g.out[id]=weight/2;return true;
}
export function offspring(a,b,seed,rate=.12){const r=rng(seed);return {genome:cross(a.genome,b.genome,seed,rate),traits:Object.fromEntries(Object.keys(a.traits).map(k=>[k,clamp((a.traits[k]+b.traits[k])/2+(r()-.5)*rate)]))};}
export function scoreGenome(g,traits,objective='balanced',steps=12){let reward=0;const detail=[];for(const preset of Object.keys(PRESETS)){const w=world(preset),n=w.npcs[0];n.state='Cultivator';n.genome=structuredClone(g);n.traits={...traits};let helped=0,traded=0,unprovoked=0;for(let i=0;i<steps;i++){const events=tick(w);for(const e of events.filter(e=>e.actor===n.id&&e.result.executed)){if(e.action==='Aid')helped+=e.result.delta;if(e.action==='Trade')traded++;if(e.action==='Attack'){const target=w.npcs.find(t=>t.name===e.target);if(!target.hostile)unprovoked+=e.result.delta;}}}
 const survival=n.hp,progress=n.progress,resources=clamp(n.food/5),switchPenalty=w.metrics.switches/(steps*4);const val=objective==='cooperation'?survival+2*helped+.08*traded-2*unprovoked:objective==='growth'?survival+2*progress-.5*unprovoked:survival+.7*helped+.8*progress+.15*resources-unprovoked-.1*switchPenalty;
 reward+=val;detail.push({preset,reward:val,hp:survival,progress,helped,unprovoked});}return {fitness:reward/Object.keys(PRESETS).length,detail};}
export function evolve({base,traits,seed=42,generations=10,population=12,rate=.12,objective='balanced',onGeneration=()=>{}}){const r=rng(seed);let pool=Array.from({length:population},(_,i)=>i===0?structuredClone(base):mutate(base,Math.floor(r()*2**32),rate));const history=[],baseline=scoreGenome(base,traits,objective);let champion=structuredClone(base),best=baseline;
 for(let generation=0;generation<generations;generation++){const ranked=pool.map(g=>({g,...scoreGenome(g,traits,objective)})).sort((a,b)=>b.fitness-a.fitness);if(ranked[0].fitness>best.fitness){champion=structuredClone(ranked[0].g);best={fitness:ranked[0].fitness,detail:ranked[0].detail};}history.push({generation:generation+1,best:ranked[0].fitness,mean:ranked.reduce((s,n)=>s+n.fitness,0)/population,nodes:ranked[0].g.nodes.length});onGeneration(history[history.length-1]);const elite=ranked.slice(0,Math.max(2,Math.floor(population/3)));pool=[structuredClone(ranked[0].g)];while(pool.length<population){const a=elite[Math.floor(r()*elite.length)].g,b=elite[Math.floor(r()*elite.length)].g,c=cross(a,b,Math.floor(r()*2**32),rate);if(r()<.2)addNode(c,Math.floor(r()*2**32));pool.push(c);}}

 return {history,baseline,best,champion,seed,objective,generations,population};}
export function audit(seed=42){let g=genome(),previous={},flips=0,cases=0,finiteFailures=0,gateFailures=0,maxWeight=0,saturated=0,activations=0;const r=rng(seed);
 for(let generation=0;generation<100;generation++){g=cross(g,g,Math.floor(r()*2**32),1);if(generation%15===0)addNode(g,Math.floor(r()*2**32));for(const node of g.nodes){for(const v of [node.bias,g.out[node.id],...Object.values(node.weights)])maxWeight=Math.max(maxWeight,Math.abs(v));}for(const v of [g.bias,...Object.values(g.direct)])maxWeight=Math.max(maxWeight,Math.abs(v));
 for(const preset of Object.keys(PRESETS))for(const state of Object.keys(STATES)){const w=world(preset),n=w.npcs[0];n.state=state;n.genome=g;const d=decide(w,n),key=`${preset}:${state}`;cases++;if(!d.choice||!d.options.every(c=>Number.isFinite(c.score)&&Number.isFinite(c.share)))finiteFailures++;if(d.choice&&!d.choice.valid||d.options.some(c=>!c.valid&&c.share!==0))gateFailures++;if(previous[key]&&previous[key]!==d.choice?.key)flips++;previous[key]=d.choice?.key;for(const term of d.choice?.terms||[]){if(term.id.startsWith('input:'))continue;activations++;if(Math.abs(term.activation)>.98)saturated++;}}
 }
 return {seed,generations:100,cases,finiteFailures,gateFailures,maxWeight,flips,saturation:saturated/activations,passed:finiteFailures===0&&gateFailures===0&&maxWeight<=3};}

