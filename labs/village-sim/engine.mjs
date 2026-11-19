import {RESOURCE_KEYS,initializeEconomy,initializeResident,economyStock,roleBonus,economyCandidates,economyTarget,performEconomyTask,updateEconomy,residentDied,cancelWork,payWork,wearTool} from './economy.mjs';
export const WIDTH=1000,HEIGHT=650,CAMP={x:490,y:417};
export const RES=RESOURCE_KEYS;
const empty=()=>Object.fromEntries(RES.map(k=>[k,0]));
export const clamp=(x,lo=0,hi=1)=>Math.max(lo,Math.min(hi,x));
export const distance=(a,b)=>Math.hypot(a.x-b.x,a.y-b.y);
export function rng(seed){let a=seed>>>0;return()=>{a+=0x6D2B79F5;let t=a;t=Math.imul(t^t>>>15,t|1);t^=t+Math.imul(t^t>>>7,t|61);return((t^t>>>14)>>>0)/4294967296;};}
const NAMES=['Lin Yue','Shen Wu','Bai He','Mei Lin','Han Tao','Su Yan','Yun Fei','Qin Lan','Mo Chen','Li Hua','Wei Ning','An Rui','Xiao Yu','Ren Song'];
export const COSTS={House:{wood:16,stone:4},Market:{wood:24,stone:8}};
export function log(w,text,tone='normal'){w.events.unshift({time:w.time,text,tone});w.events=w.events.slice(0,60);}
function villager(w,id,x,y){const r=w.random;return {id:`v${id}`,name:NAMES[id],x,y,hp:100,hunger:r()*8,energy:90,inventory:{...empty(),food:5,coin:8},power:.7+r()*.5,bravery:.15+r()*.75,care:.3+r()*.6,task:null,trace:[],gathered:0,kills:0,assembled:false,alive:true,decisionAt:0,actionAge:0,tradeAt:0};}
function monster(w,initial=false){const rare=w.random()<.22;const m={id:`m${w.nextMonster++}`,x:755+w.random()*155,y:120+w.random()*145,hp:rare?65:30,maxHp:rare?65:30,power:rare?1.8:.65,rare,kind:rare?'Jadefang':'Wild boar',loot:{...empty(),food:rare?10:6,jade:rare?1:0},cooldown:0};w.monsters.push(m);for(const k of RES)(initial?w.ledger.initial:w.ledger.renewed)[k]+=m.loot[k];return m;}
export function createWorld(seed=42){const w={seed,random:rng(seed),time:0,nextMonster:0,spawnTimer:0,arrivals:0,nextArrival:18,village:false,formedAt:null,threshold:8,productivity:1,hillPeril:1,riskAppetite:.5,habitat:30,maxHabitat:40,npcs:[],nodes:[],monsters:[],drops:[],buildings:[],project:null,warehouse:empty(),events:[],ledger:{initial:empty(),imported:empty(),renewed:empty(),consumed:empty(),built:empty()},stats:{trades:0,kills:0,deaths:0,foodEaten:0,arrived:6},tradeFlash:0,day:0};
 const r=w.random;
 for(let i=0;i<18;i++){const n={id:`berry${i}`,type:'food',x:95+r()*235,y:120+r()*220,stock:7,max:7,rate:.012,growth:0,age:0};w.nodes.push(n);w.ledger.initial.food+=n.stock;}
 for(let i=0;i<24;i++){const n={id:`tree${i}`,type:'wood',x:70+r()*290,y:100+r()*255,stock:6,max:6,rate:.003,growth:0,age:0};w.nodes.push(n);w.ledger.initial.wood+=n.stock;}
 for(let i=0;i<7;i++){const n={id:`rock${i}`,type:'stone',x:420+r()*230,y:190+r()*65,stock:12,max:12,rate:0,growth:0,age:0};w.nodes.push(n);w.ledger.initial.stone+=n.stock;}
 for(let i=0;i<6;i++){const n=villager(w,i,430+(i%3)*36,420+Math.floor(i/3)*32);w.npcs.push(n);for(const k of RES)w.ledger.initial[k]+=n.inventory[k];}
 for(let i=0;i<4;i++)monster(w,true);
 // Guarantee one visibly rare inhabitant without inventing loot after the ledger is initialized.
 if(!w.monsters.some(m=>m.rare)){const m=w.monsters[3];m.rare=true;m.kind='Jadefang';m.hp=m.maxHp=65;m.power=1.8;m.loot.food+=4;m.loot.jade=1;w.ledger.initial.food+=4;w.ledger.initial.jade++;}
 initializeEconomy(w);
 log(w,'Two small caravans have stopped in the clearing. Eight assembled villagers can found a village.','milestone');return w;
}
export function alive(w){return w.npcs.filter(n=>n.alive);}
export function nodeStock(w,type){return w.nodes.filter(n=>n.type===type).reduce((s,n)=>s+n.stock,0);}
export function stored(w,type){return w.warehouse[type]+alive(w).reduce((s,n)=>s+n.inventory[type],0);}
export function total(w){const result=empty();for(const n of w.nodes)result[n.type]+=n.stock;for(const n of w.npcs)for(const k of RES)result[k]+=n.inventory[k];for(const m of w.monsters)for(const k of RES)result[k]+=m.loot[k];for(const d of w.drops)for(const k of RES)result[k]+=d.inventory[k];const escrow=economyStock(w);for(const k of RES)result[k]+=w.warehouse[k]+escrow[k];return result;}
export function balance(w){const stock=total(w);return Object.fromEntries(RES.map(k=>[k,{initial:w.ledger.initial[k],imported:w.ledger.imported[k],renewed:w.ledger.renewed[k],produced:w.ledger.produced[k],consumed:w.ledger.consumed[k],built:w.ledger.built[k],stock:stock[k],error:w.ledger.initial[k]+w.ledger.imported[k]+w.ledger.renewed[k]+w.ledger.produced[k]-w.ledger.consumed[k]-w.ledger.built[k]-stock[k]}]));}
function target(w,task){if(!task)return null;const economic=economyTarget(w,null,task);if(economic)return economic;if(task.kind==='harvest')return w.nodes.find(n=>n.id===task.id);if(task.kind==='hunt')return w.monsters.find(m=>m.id===task.id);if(task.kind==='salvage')return w.drops.find(d=>d.id===task.id);if(task.kind==='build')return w.project;if(['deposit','rest','eat','assemble'].includes(task.kind))return CAMP;return null;}
const carry=n=>n.inventory.wood+n.inventory.stone+n.inventory.jade+n.inventory.herb+Math.max(0,n.inventory.food-3);
export function perceive(w,n){return {hungry:n.hunger/100,wounded:(100-n.hp)/100,tired:(100-n.energy)/100,carry:carry(n),stock:{food:stored(w,'food'),wood:stored(w,'wood'),stone:stored(w,'stone')},nearestThreat:w.monsters.filter(m=>distance(n,m)<90).sort((a,b)=>distance(n,a)-distance(n,b))[0]||null};}
export function decisions(w,n){if(!n.alive)return[];const p=perceive(w,n),out=[],add=(kind,t,score,reason)=>out.push({kind,id:t?.id||null,x:t?.x??CAMP.x,y:t?.y??CAMP.y,score,reason});
 if(!n.assembled){add('assemble',null,100,'Join the caravan clearing before taking a job.');return out;}
 if(n.hunger>28&&n.inventory.food>0)add('eat',null,4+n.hunger/12,'Consume one carried food; meals are never free.');
 if(n.hunger>35&&w.warehouse.food>0)add('rest',null,3+n.hunger/16,'Go to the real food stockpile and eat.');
 if(carry(n)>0&&(carry(n)>=4||n.inventory.wood+n.inventory.stone>0||n.inventory.jade>0))add('deposit',null,3+carry(n)*.55,'Deliver carried resources to the shared stockpile.');
 if(n.hp<90||n.energy<65)add('rest',null,1+(100-n.hp)*.08+(100-n.energy)*.055,'Recover energy at camp; healing also requires a meal.');
 const foodPressure=clamp((alive(w).length*4-p.stock.food)/(alive(w).length*4));
 const desired=w.project?null:nextBuilding(w),woodNeed=(desired?COSTS[desired].wood:8)+(w.warehouse.tool<3?6:0)-w.warehouse.wood,stoneNeed=(desired?COSTS[desired].stone:4)+(w.warehouse.tool<3?3:0)-w.warehouse.stone;
 if(carry(n)<8&&n.energy>8){for(const type of ['food','wood','stone','herb']){if(type==='food'&&n.inventory.food>=7)continue;if(type!=='food'&&!w.village)continue;if(type==='wood'&&woodNeed<=0||type==='stone'&&stoneNeed<=0||type==='herb'&&w.warehouse.herb>=6)continue;const nodes=w.nodes.filter(t=>t.type===type&&t.stock>=1).sort((a,b)=>distance(n,a)-distance(n,b));if(nodes.length){const t=nodes[0];const need=type==='food'?foodPressure*3+(n.inventory.food<2?2:0):type==='wood'?2:type==='herb'?(w.warehouse.medicine<2?2:1):1.6;add('harvest',t,1+need-distance(n,t)/500-(type==='stone'?.1:0),`Harvest finite ${type} (${Math.floor(t.stock)} left); ${type==='food'?'food demand':desired+' construction'} drives this trip.`);}}}
 if(w.project&&distance(n,w.project)<120&&n.energy>12)add('build',w.project,4+n.care,'Materials already paid from the stockpile; contribute labor.');
 if(n.energy>25&&n.hp>35&&carry(n)<6){const monsters=w.monsters.filter(m=>m.hp>0).sort((a,b)=>distance(n,a)-distance(n,b));for(const m of monsters.slice(0,2)){const risk=clamp(m.power*w.hillPeril/(n.power*2)+(100-n.hp)/150);const reward=m.rare?1.8:1;add('hunt',m,.5+n.bravery*2+w.riskAppetite*2+foodPressure*2+reward-risk*(3.5-n.bravery)-distance(n,m)/600,`${m.kind}: risk ${Math.round(risk*100)}%, power ${m.power.toFixed(1)} vs ${n.power.toFixed(1)}. Meat and ${m.rare?'jade':'experience'} are potential rewards.`);}}
 if(carry(n)<7){const caches=w.drops.filter(d=>RES.some(k=>d.inventory[k]>0)).sort((a,b)=>distance(n,a)-distance(n,b));if(caches.length)add('salvage',caches[0],3-distance(n,caches[0])/450,'Recover dropped goods; resources remain on the ground until collected.');}
 add('rest',null,.2,'Wait at camp when no useful opportunity remains.');let choices=[...out,...economyCandidates(w,n)];if(n.duty)choices=choices.filter(c=>['eat','rest','guard','returnGear','hunt'].includes(c.kind)&&!(c.kind==='hunt'&&!c.defense));for(const c of choices){c.score+=roleBonus(n,c,w);if(w.council.priority==='food'&&c.kind==='harvest'&&w.nodes.find(t=>t.id===c.id)?.type==='food')c.score+=1.5;if(w.council.priority==='build'&&['build','deposit'].includes(c.kind))c.score+=1;}return choices.sort((a,b)=>b.score-a.score);
}
export function nextBuilding(w){if(!w.village)return null;const houses=w.buildings.filter(b=>b.type==='House').length,market=w.buildings.some(b=>b.type==='Market');if(houses===0)return'House';if(!market)return'Market';if(houses*4<alive(w).length)return'House';return null;}
function startProject(w){if(w.project)return;const type=nextBuilding(w);if(!type)return;const cost=COSTS[type];if(!Object.entries(cost).every(([k,v])=>w.warehouse[k]>=v))return;for(const[k,v]of Object.entries(cost)){w.warehouse[k]-=v;w.ledger.built[k]+=v;}const index=w.buildings.length;const spots=[{x:423,y:364},{x:541,y:381},{x:425,y:483},{x:559,y:477},{x:490,y:335}];w.project={id:'project',type,...spots[index%spots.length],progress:0,required:type==='Market'?36:24,cost:{...cost}};log(w,`${type} funded: ${cost.wood} timber + ${cost.stone} stone reserved. Villagers can now build.`,'build');}
function die(w,n){if(!n.alive)return;residentDied(w,n);n.alive=false;n.hp=0;w.stats.deaths++;w.drops.push({id:`grave-${n.id}`,x:n.x,y:n.y,inventory:{...n.inventory},kind:'Lost pack'});n.inventory=empty();n.task=null;log(w,`${n.name} died. Their pack remains where they fell.`,'danger');}
function move(n,t,dt){const d=distance(n,t);if(d<=9)return true;const speed=28*(.65+n.energy/285),step=Math.min(d-8,speed*dt);n.x+=(t.x-n.x)/d*step;n.y+=(t.y-n.y)/d*step;n.energy=Math.max(0,n.energy-dt*.1);return false;}
function eat(w,n){if(n.inventory.food<=0)return false;n.inventory.food--;if(n.duty?.issued.food>0)n.duty.issued.food--;w.ledger.consumed.food++;w.stats.foodEaten++;n.hunger=Math.max(0,n.hunger-45);n.hp=Math.min(100,n.hp+2);return true;}
function finish(n){n.task=null;n.actionAge=0;n.decisionAt=0;}
function act(w,n,dt){const task=n.task,t=target(w,task);if(!task)return;if(!t){cancelWork(w,n);return;}
 if(task.kind==='eat'){eat(w,n);finish(n);return;}
 if(!move(n,t,dt))return;n.actionAge+=dt;const economic=performEconomyTask(w,n,dt);if(economic!==null){if(economic)cancelWork(w,n);return;}
 if(task.kind==='assemble'){n.assembled=true;finish(n);return;}
 if(task.kind==='deposit'){let delivered=0;for(const k of ['wood','stone','jade','herb']){delivered+=n.inventory[k];n.contributed+=n.inventory[k];w.warehouse[k]+=n.inventory[k];n.inventory[k]=0;}const extra=Math.max(0,n.inventory.food-3);w.warehouse.food+=extra;n.inventory.food-=extra;n.contributed+=extra;delivered+=extra;if(delivered>=3)payWork(w,n,1);finish(n);return;}
 if(task.kind==='rest'){const beds=w.buildings.filter(b=>b.type==='House').length*4,sheltered=alive(w).filter(v=>v.task?.kind==='rest'&&distance(v,CAMP)<20).slice(0,beds).some(v=>v.id===n.id);n.energy=Math.min(100,n.energy+dt*(sheltered?6:5));if(n.hunger>30){if(n.inventory.food===0&&w.warehouse.food>0){w.warehouse.food--;n.inventory.food++;}eat(w,n);}if(n.hunger<35)n.hp=Math.min(100,n.hp+dt*(sheltered?1:.65));if(n.actionAge>5||n.hp>=100&&n.energy>=98)finish(n);return;}
 if(task.kind==='build'){if(!w.project){finish(n);return;}w.project.progress+=dt;n.workSeconds+=dt;if(n.workSeconds>=5){n.workSeconds-=5;payWork(w,n,1);}n.energy=Math.max(0,n.energy-dt*.4);if(w.project.progress>=w.project.required){const b={...w.project,id:`building${w.buildings.length}`};w.buildings.push(b);w.project=null;log(w,`${b.type} completed. ${b.type==='House'?'Four permanent beds are available.':'The market now supports funded procurement and reserved offers.'}`,'build');}if(n.actionAge>5||n.energy<15)finish(n);return;}
 if(task.kind==='harvest'){const period=(t.type==='food'||t.type==='herb'?1.3:t.type==='wood'?2.2:2.5)/(n.inventory.tool>0?1.35:1);if(t.stock<1||carry(n)>=8||t.type==='food'&&n.inventory.food>=7){finish(n);return;}if(n.actionAge>=period){t.stock--;t.age=0;n.inventory[t.type]++;n.gathered++;wearTool(w,n);n.energy=Math.max(0,n.energy-1);n.actionAge=0;if(carry(n)>=6||t.type==='food'&&n.inventory.food>=7||n.energy<8)finish(n);}return;}
 if(task.kind==='salvage'){for(const k of RES){const cap=k==='food'?Math.max(0,7-n.inventory.food):k==='coin'?999:Math.max(0,8-carry(n));const take=Math.min(t.inventory[k],cap);n.inventory[k]+=take;t.inventory[k]-=take;}finish(n);return;}
 if(task.kind==='hunt'){if(t.hp<=0){finish(n);return;}t.hp-=dt*5*n.power*(n.inventory.tool>0?1.2:1);n.hp-=dt*2.4*t.power*w.hillPeril;n.energy=Math.max(0,n.energy-dt*.75);if(t.hp<=0){n.kills++;wearTool(w,n);w.stats.kills++;n.power=Math.min(2.5,n.power+.07);w.drops.push({id:`loot-${t.id}`,x:t.x,y:t.y,inventory:{...t.loot},kind:t.kind+' carcass'});t.loot=empty();w.monsters=w.monsters.filter(m=>m.id!==t.id);log(w,`${n.name} defeated a ${t.kind}. Its finite loot is on the hill.`,t.rare?'rare':'hunt');finish(n);if(n.hp<=0)die(w,n);}else if(n.hp<=0){die(w,n);return;}else if(n.hp<25||n.energy<15){log(w,`${n.name} broke off the fight with ${t.kind}.`,'danger');finish(n);n.task={kind:'rest',id:null,reason:'Wounded: retreat to camp.'};}return;}
}
function arrive(w){const remaining=14-w.npcs.length;if(remaining<=0)return;const count=Math.min(2,remaining);for(let i=0;i<count;i++){const n=villager(w,w.npcs.length,90+i*20,575+i*14);initializeResident(n);w.npcs.push(n);for(const k of RES)w.ledger.imported[k]+=n.inventory[k];}w.stats.arrived+=count;w.arrivals++;w.nextArrival=w.time+65;log(w,`A caravan of ${count} arrived with ${count*5} food and ${count*8} coins. Imports recorded.`,'arrival');}
export function update(w,dt){if(dt<=0||!Number.isFinite(dt))return;dt=Math.min(dt,.25);w.time+=dt;w.day=Math.floor(w.time/180)+1;
 if(w.time>=w.nextArrival&&w.npcs.length<14)arrive(w);
 for(const node of w.nodes){if(node.rate===0||node.stock>=node.max)continue;node.age+=dt;if(node.type==='wood'&&node.stock===0&&node.age<150)continue;node.growth+=node.rate*w.productivity*dt;if(node.growth>=1){const amount=Math.min(node.max-node.stock,Math.floor(node.growth));node.stock+=amount;node.growth-=amount;w.ledger.renewed[node.type]+=amount;}}
 w.habitat=Math.min(w.maxHabitat,w.habitat+dt*.035*w.productivity);w.spawnTimer+=dt;if(w.spawnTimer>=85&&w.monsters.length<4&&w.habitat>=10){w.spawnTimer=0;w.habitat-=10;const m=monster(w);log(w,`Hill wildlife replenished: ${m.kind}. Reproduction used 10 habitat forage.`,'nature');}
 for(const m of w.monsters)if(m.raiding&&distance(m,CAMP)>45){const d=distance(m,CAMP),step=Math.min(d-45,dt*7);m.x+=(CAMP.x-m.x)/d*step;m.y+=(CAMP.y-m.y)/d*step;}
 for(const n of w.npcs){if(!n.alive)continue;n.hunger=Math.min(110,n.hunger+dt*1.15);if(n.hunger>85)n.hp-=dt*(n.hunger-85)*.12;const predator=w.monsters.find(m=>(m.rare||m.raiding)&&distance(n,m)<65);if(predator&&n.task?.kind!=='hunt')n.hp-=dt*2*predator.power*w.hillPeril;if(n.hp<=0){die(w,n);continue;}
 if(n.hunger>55&&n.inventory.food>0)eat(w,n);
 if(!n.task){n.trace=decisions(w,n).slice(0,5);n.task=n.trace[0]?{...n.trace[0]}:null;n.actionAge=0;}
 act(w,n,dt);
 }
 if(!w.village){const assembled=alive(w).filter(n=>n.assembled&&distance(n,CAMP)<140).length;if(assembled>=w.threshold){w.village=true;w.formedAt=w.time;log(w,`${assembled} villagers assembled. Willow Crossing is now a village. Permanent construction is unlocked.`,'milestone');}}
 updateEconomy(w,dt);if(w.village)startProject(w);
 w.drops=w.drops.filter(d=>RES.some(k=>d.inventory[k]>0));
}
export function advance(w,seconds,step=.2){for(let t=0;t<seconds-1e-9;t+=step)update(w,Math.min(step,seconds-t));return w;}





