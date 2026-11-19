// Jobs are preferences; duty and current task are independent state.
export const RESOURCE_KEYS=['food','wood','stone','coin','jade','herb','tool','medicine'];
export const JOBS={
 Forager:{inputs:'Wild plants, time, provisions',outputs:'Food and herbs'},
 Woodcutter:{inputs:'Standing trees, tools, provisions',outputs:'Timber'},
 Miner:{inputs:'Stone deposits, tools, provisions',outputs:'Stone'},
 Hauler:{inputs:'Existing goods, carrying time',outputs:'Goods delivered to storage'},
 Builder:{inputs:'Reserved timber and stone, labor',outputs:'Structures and paid construction labor'},
 Artisan:{inputs:'Timber + stone, or herbs + food',outputs:'Tools and medicine'},
 Healer:{inputs:'One actual medicine, treatment time',outputs:'Health restored to a living patient'},
 Merchant:{inputs:'Existing inventory and coin',outputs:'Market exchanges and distribution'},
 Hunter:{inputs:'Equipment, provisions, health risk',outputs:'Retrieved creature loot'}
};
export const RECIPES={tool:{inputs:{wood:2,stone:1},seconds:7,output:1},medicine:{inputs:{herb:2,food:1},seconds:6,output:1}};
const CAMP={x:490,y:417},live=w=>w.npcs.filter(n=>n.alive),near=(a,b,r)=>Math.hypot(a.x-b.x,a.y-b.y)<=r;
const message=(w,text,tone='trade')=>{w.events.unshift({time:w.time,text,tone});w.events=w.events.slice(0,60);};
const amount=(stock,k)=>stock[k]||0,move=(from,to,k,q)=>{from[k]-=q;to[k]=(to[k]||0)+q;};
const blank=()=>Object.fromEntries(RESOURCE_KEYS.map(k=>[k,0]));
const marketOpen=w=>w.village&&w.buildings.some(b=>b.type==='Market');
export function initializeEconomy(w){
 w.council={leaderId:null,quartermasterId:null,legitimacy:.7,priority:'balanced',levyAt:-60,muster:null,notice:'A council will form when the village is founded.'};
 w.market={orders:[],nextId:1,refreshAt:0,history:[]};
 w.ledger.produced=blank();
 Object.assign(w.stats,{crafted:0,treatments:0,wages:0,levies:0,mobilized:0,refusals:0,guardSeconds:0});
 for(const n of w.npcs)initializeResident(n);
 for(let i=0;i<8;i++){const v=Math.sin((w.seed+i)*127.1)*43758.54,f=v-Math.floor(v);w.nodes.push({id:`herb${i}`,type:'herb',x:110+(i%4)*51+f*15,y:150+Math.floor(i/4)*110+f*20,stock:3,max:3,rate:.008,growth:0,age:0});w.ledger.initial.herb+=3;}
}
export function initializeResident(n){
 const jobs=['Forager','Woodcutter','Miner','Builder','Artisan','Hunter','Forager','Healer','Hauler','Merchant','Forager','Hunter','Woodcutter','Builder'];
 n.job=jobs[Number(n.id.slice(1))%jobs.length];n.duty=null;n.taxPaid=false;n.toolWear=0;n.workSeconds=0;n.contributed=0;
 for(const k of RESOURCE_KEYS)n.inventory[k]??=0;
}
export function economyStock(w){const out=blank();for(const o of w.market.orders)for(const k of RESOURCE_KEYS)out[k]+=amount(o.escrow,k);for(const n of w.npcs)for(const k of RESOURCE_KEYS)out[k]+=amount(n.task?.escrow||{},k);return out;}
function owner(w,id){return id==='public'?w.warehouse:w.npcs.find(n=>n.id===id)?.inventory;}
export function cancelOrder(w,id){const o=w.market.orders.find(o=>o.id===id);if(!o)return false;const stock=owner(w,o.owner);if(stock)for(const k of RESOURCE_KEYS)move(o.escrow,stock,k,amount(o.escrow,k));w.market.orders=w.market.orders.filter(v=>v.id!==id);return true;}
export function placeOrder(w,{side,owner:who,resource,quantity,price,ttl=35}){
 if(!marketOpen(w)||!['bid','ask'].includes(side)||!RESOURCE_KEYS.includes(resource)||resource==='coin'||!Number.isInteger(quantity)||quantity<1||!Number.isInteger(price)||price<1||!Number.isFinite(ttl)||ttl<=0)return null;
 if(who!=='public'&&!live(w).some(n=>n.id===who))return null;
 const stock=owner(w,who);if(!stock)return null;const k=side==='bid'?'coin':resource,q=side==='bid'?quantity*price:quantity;if(amount(stock,k)<q)return null;
 const escrow=blank();move(stock,escrow,k,q);const o={id:`order${w.market.nextId++}`,side,owner:who,resource,remaining:quantity,quantity,price,expires:w.time+ttl,escrow};w.market.orders.push(o);return o;
}
export function fillOrder(w,n,id){const o=w.market.orders.find(o=>o.id===id);if(!n.alive||!o||o.owner===n.id||o.expires<=w.time||!marketOpen(w)||!near(n,CAMP,20))return false;
 const counterparty=o.owner==='public'?null:w.npcs.find(v=>v.id===o.owner);if(counterparty&&(!counterparty.alive||!near(counterparty,CAMP,150)))return false;
 const other=owner(w,o.owner);if(!other)return false;
 if(o.side==='ask'){if(n.inventory.coin<o.price||o.escrow[o.resource]<1)return false;move(n.inventory,other,'coin',o.price);move(o.escrow,n.inventory,o.resource,1);}
 else{if(amount(n.inventory,o.resource)<1||o.escrow.coin<o.price)return false;move(n.inventory,other,o.resource,1);move(o.escrow,n.inventory,'coin',o.price);}
 o.remaining--;w.stats.trades++;n.tradeAt=w.time+3;w.market.history.unshift({time:w.time,actor:n.name,side:o.side,resource:o.resource,price:o.price,owner:o.owner});w.market.history=w.market.history.slice(0,30);
 message(w,`${n.name} ${o.side==='ask'?'bought':'sold'} 1 ${o.resource} for ${o.price} coins (${o.owner==='public'?'public store':'resident offer'}).`);
 if(o.remaining===0)cancelOrder(w,id);return true;
}
export function cancelWork(w,n,death=false){if(n.task?.escrow){const stock=death?n.inventory:w.warehouse;for(const k of RESOURCE_KEYS)move(n.task.escrow,stock,k,amount(n.task.escrow,k));}n.task=null;n.actionAge=0;}
export function residentDied(w,n){for(const o of [...w.market.orders])if(o.owner===n.id)cancelOrder(w,o.id);cancelWork(w,n,true);n.duty=null;}
export function electLeader(w){if(!w.village)return false;const candidates=live(w).filter(n=>n.assembled&&!n.duty).sort((a,b)=>(b.care+.15*b.bravery+.005*b.contributed)-(a.care+.15*a.bravery+.005*a.contributed)||a.id.localeCompare(b.id));if(!candidates.length)return false;
 w.council.leaderId=candidates[0].id;w.council.quartermasterId=candidates[1]?.id||null;w.council.notice=`${candidates[0].name} leads the council; ${candidates[1]?.name||'no one'} manages supplies.`;message(w,w.council.notice,'milestone');return true;}
export function collectLevy(w,founding=false){if(!w.village||!founding&&w.time-w.council.levyAt<60)return 0;let collected=0;for(const n of live(w).filter(n=>n.assembled)){if(founding&&n.taxPaid)continue;const q=Math.min(founding?2:1,n.inventory.coin);move(n.inventory,w.warehouse,'coin',q);collected+=q;if(founding)n.taxPaid=true;}
 if(!founding)w.council.levyAt=w.time;w.stats.levies+=collected;if(collected)message(w,`Council collected ${collected} existing coins as ${founding?'founding contributions':'a public levy'}. No money was created.`,'council');return collected;}
export function setJob(w,n,id){if(!n?.alive||!JOBS[id])return false;n.job=id;if(!n.duty)cancelWork(w,n);return true;}
export function payWork(w,n,units=1){const q=Math.min(Math.floor(units),w.warehouse.coin);if(q>0){move(w.warehouse,n.inventory,'coin',q);w.stats.wages+=q;}return q;}
export function wearTool(w,n){if(n.inventory.tool<1)return;n.toolWear++;if(n.toolWear>=12){n.toolWear=0;n.inventory.tool--;if(n.duty?.issued.tool>0)n.duty.issued.tool--;w.ledger.consumed.tool++;message(w,`${n.name}'s tool wore out after 12 work uses.`,'work');}}
export function roleBonus(n,task,w){const type=w.nodes.find(v=>v.id===task.id)?.type;
 const fits={Forager:task.kind==='harvest'&&['food','herb'].includes(type),Woodcutter:task.kind==='harvest'&&type==='wood',Miner:task.kind==='harvest'&&type==='stone',Hauler:['deposit','salvage'].includes(task.kind),Builder:task.kind==='build',Hunter:task.kind==='hunt',Artisan:task.kind==='craft',Healer:task.kind==='heal',Merchant:task.kind==='market'};
 return fits[n.job]?1.1:0;
}
function exemptIds(w){const people=live(w).filter(n=>n.assembled),ids=new Set([w.council.leaderId,w.council.quartermasterId]);for(const n of people.filter(n=>n.job==='Forager').slice(0,2))ids.add(n.id);for(const job of ['Healer','Artisan']){const n=people.find(n=>n.job===job);if(n)ids.add(n.id);}return ids;}
export function requestMuster(w,count,duration=90){if(!w.village||!Number.isInteger(count)||count<1||count>4||!Number.isFinite(duration)||duration<=0||duration>180)return false;
 if(w.council.muster){w.council.notice='Stand down the current call-up before issuing another.';return false;}
 w.council.muster={requested:count,until:w.time+duration,refused:[],started:w.time};w.council.notice=`Call-up: ${count} militia for ${duration}s. Essential workers and officers are exempt; equipment and food must exist.`;message(w,w.council.notice,'council');return true;}
export function demobilize(w){let count=0;for(const n of live(w))if(n.duty?.status==='active'){n.duty.status='returning';cancelWork(w,n);count++;}w.council.muster=null;w.council.notice=count?`${count} militia stood down. Unused issued supplies must be returned at camp.`:'Call-up cancelled. No active militia.';if(count)message(w,w.council.notice,'council');return count;}
function muster(w){const call=w.council.muster;if(!call)return;if(w.time>=call.until){demobilize(w);return;}
 let active=live(w).filter(n=>n.duty?.status==='active').length;const excluded=exemptIds(w),candidates=live(w).filter(n=>n.assembled&&!n.duty&&!excluded.has(n.id)&&n.hp>=65&&n.energy>=35&&n.hunger<65&&!call.refused.includes(n.id)).sort((a,b)=>b.bravery-a.bravery);
 for(const n of candidates){if(active>=call.requested)break;const acceptance=.5*w.council.legitimacy+.35*n.bravery+.15*n.care;if(acceptance<.48){call.refused.push(n.id);w.stats.refusals++;message(w,`${n.name} refused the call-up. Leadership and willingness were insufficient.`,'council');continue;}
 const tool=n.inventory.tool>0?0:1,food=Math.max(0,2-n.inventory.food);if(w.warehouse.tool<tool||w.warehouse.food<food)continue;
 cancelWork(w,n);move(w.warehouse,n.inventory,'tool',tool);move(w.warehouse,n.inventory,'food',food);n.duty={status:'active',issued:{tool,food},seconds:0};active++;w.stats.mobilized++;message(w,`${n.name} left civilian work for militia duty. Issued ${tool} tool and ${food} food.`,'council');
 }
 w.council.notice=`${active}/${call.requested} mobilized; ${call.refused.length} refused. Vacancies may reflect exemptions, health, or missing supplies.`;
}
export function supplyTargets(w){const people=live(w).length,call=w.council.muster;return {food:Math.max(6,people*(w.council.priority==='food'?3:2)),wood:24+(w.warehouse.tool<3?6:0),stone:8+(w.warehouse.tool<3?3:0),herb:6,tool:3+(call?.requested||0),medicine:3};}
function refreshOrders(w){if(!marketOpen(w))return;
 const targets=supplyTargets(w),prices={food:2,wood:1,stone:2,herb:1};
 for(const resource of ['food','wood','stone','herb']){if(w.market.orders.some(o=>o.owner==='public'&&o.side==='bid'&&o.resource===resource))continue;const gap=targets[resource]-w.warehouse[resource],quantity=Math.min(3,gap,Math.floor(w.warehouse.coin/prices[resource]));if(quantity>0)placeOrder(w,{owner:'public',side:'bid',resource,quantity,price:prices[resource]});}
 for(const[resource,price]of [['tool',5],['medicine',4]])if(w.warehouse[resource]>0&&!w.market.orders.some(o=>o.owner==='public'&&o.side==='ask'&&o.resource===resource))placeOrder(w,{owner:'public',side:'ask',resource,quantity:1,price});
 for(const n of live(w))if(!n.duty&&near(n,CAMP,150)&&n.inventory.food>=5&&n.hunger<40&&!w.market.orders.some(o=>o.owner===n.id))placeOrder(w,{owner:n.id,side:'ask',resource:'food',quantity:1,price:2,ttl:25});
}
export function updateEconomy(w,dt){if(!w.village)return;if(!live(w).some(n=>n.id===w.council.leaderId))electLeader(w);collectLevy(w,true);
 if(!live(w).some(n=>n.id===w.council.quartermasterId))w.council.quartermasterId=live(w).find(n=>n.id!==w.council.leaderId&&!n.duty)?.id||null;
 for(const o of [...w.market.orders]){const n=o.owner==='public'?null:w.npcs.find(n=>n.id===o.owner);if(o.expires<=w.time||n&&(!n.alive||!near(n,CAMP,150)||n.hunger>55)||!marketOpen(w))cancelOrder(w,o.id);}
 muster(w);if(w.time>=w.market.refreshAt){w.market.refreshAt=w.time+4;refreshOrders(w);}
}
export function economyCandidates(w,n){const out=[],add=(kind,score,reason,data={})=>out.push({kind,id:null,x:CAMP.x,y:CAMP.y,score,reason,...data});if(!n.alive||!n.assembled||!w.village)return out;
 if(n.duty?.status==='returning'){add('returnGear',40,'Return unused public equipment and provisions, then resume your civilian job.');return out;}
 if(n.duty?.status==='active'){if(n.hp<35||n.energy<20||n.hunger>45)add('rest',30,'Militia recovery: civilian work is suspended, basic needs remain.');else{const slot=Number(n.id.slice(1))%4,points=[{x:600,y:360},{x:550,y:480},{x:400,y:450},{x:440,y:340}],p=points[slot];add('guard',15,'Temporary militia patrol. Defense consumes time and energy; no goods are produced.',{x:p.x,y:p.y});const threat=w.monsters.find(m=>(m.rare||m.raiding)&&m.hp>0&&near(m,CAMP,180));if(threat)add('hunt',18,'Defend the settlement against a nearby predator.',{id:threat.id,x:threat.x,y:threat.y,defense:true});}return out;}
 if(n.inventory.medicine>0&&n.hp<75)add('medicate',12,'Consume one carried medicine to restore up to 30 health.');
 const targets=supplyTargets(w);for(const[resource,recipe]of Object.entries(RECIPES)){const inProgress=live(w).filter(v=>v.task?.kind==='craft'&&v.task.recipe===resource).length;if(w.warehouse[resource]+inProgress<targets[resource]&&Object.entries(recipe.inputs).every(([k,q])=>w.warehouse[k]>=q)&&n.energy>20){const score=n.job==='Artisan'?5.8:1.3;add('craft',score,`Make 1 ${resource}: ${Object.entries(recipe.inputs).map(([k,q])=>q+' '+k).join(' + ')} and ${recipe.seconds}s labor.`,{recipe:resource});}}
 if(w.warehouse.medicine>0&&n.energy>15){const patients=live(w).filter(t=>t.id!==n.id&&t.hp<70).sort((a,b)=>a.hp-b.hp);if(patients.length)add('heal',n.job==='Healer'?8:1.5,'Treat a living patient using one medicine from the public store.',{id:patients[0].id,x:patients[0].x,y:patients[0].y});}
 if(marketOpen(w)&&w.time>n.tradeAt){for(const o of w.market.orders){if(o.owner===n.id)continue;const reserve=o.resource==='food'?3:o.resource==='tool'?1:0;
 if(o.side==='bid'&&amount(n.inventory,o.resource)>reserve)add('market',5.5+(n.job==='Merchant'?1:0),`Fulfil a funded order: sell 1 ${o.resource} for ${o.price} coins.`,{order:o.id});
 if(o.side==='ask'&&n.inventory.coin>=o.price){const wanted=o.resource==='food'&&n.inventory.food<3||o.resource==='tool'&&n.inventory.tool===0&&['Woodcutter','Miner','Artisan','Hunter'].includes(n.job)||o.resource==='medicine'&&n.hp<75&&n.inventory.medicine===0;if(wanted)add('market',o.resource==='food'?7:4.5,`Buy 1 reserved ${o.resource} for ${o.price} coins.`,{order:o.id});}
 }}return out;
}
export function economyTarget(w,n,task){if(['craft','market','medicate','returnGear'].includes(task.kind))return CAMP;if(task.kind==='guard')return task;if(task.kind==='heal'){const p=w.npcs.find(p=>p.id===task.id);return p?.alive?p:null;}return null;}
export function performEconomyTask(w,n,dt){const t=n.task;if(!t||!['craft','market','heal','medicate','guard','returnGear'].includes(t.kind))return null;
 if(t.kind==='market'){if(n.actionAge<1)return false;fillOrder(w,n,t.order);return true;}
 if(t.kind==='craft'){
 const recipe=RECIPES[t.recipe];if(!recipe)return true;if(!t.escrow){if(!Object.entries(recipe.inputs).every(([k,q])=>w.warehouse[k]>=q))return true;t.escrow=blank();for(const[k,q]of Object.entries(recipe.inputs))move(w.warehouse,t.escrow,k,q);n.actionAge=0;}
 n.energy=Math.max(0,n.energy-dt*.6);if(n.actionAge<recipe.seconds)return false;for(const[k,q]of Object.entries(recipe.inputs)){t.escrow[k]-=q;w.ledger.consumed[k]+=q;}w.warehouse[t.recipe]+=recipe.output;w.ledger.produced[t.recipe]+=recipe.output;w.stats.crafted++;payWork(w,n,1);message(w,`${n.name} crafted 1 ${t.recipe} from actual inputs.`,'work');return true;
 }
 if(t.kind==='medicate'){if(n.actionAge<1.5)return false;if(n.inventory.medicine>0){n.inventory.medicine--;w.ledger.consumed.medicine++;n.hp=Math.min(100,n.hp+30);w.stats.treatments++;}return true;}
 if(t.kind==='heal'){const p=w.npcs.find(v=>v.id===t.id);if(!p?.alive||p.hp>=90)return true;if(!t.escrow){if(w.warehouse.medicine<1)return true;t.escrow=blank();move(w.warehouse,t.escrow,'medicine',1);n.actionAge=0;}if(n.actionAge<3)return false;t.escrow.medicine--;w.ledger.consumed.medicine++;const benefit=Math.min(30,100-p.hp);p.hp+=benefit;w.stats.treatments++;payWork(w,n,1);message(w,`${n.name} treated ${p.name} (+${Math.round(benefit)} health), consuming 1 medicine.`,'work');return true;}
 if(t.kind==='returnGear'){for(const k of ['tool','food']){const q=Math.min(amount(n.duty?.issued||{},k),n.inventory[k]);move(n.inventory,w.warehouse,k,q);}n.duty=null;return true;}
 if(t.kind==='guard'){if(n.duty?.status!=='active')return true;n.energy=Math.max(0,n.energy-dt*.5);n.duty.seconds+=dt;w.stats.guardSeconds+=dt;
 const threat=w.monsters.find(m=>(m.rare||m.raiding)&&near(m,CAMP,150)&&near(m,n,70));if(threat){t.x=threat.x;t.y=threat.y;return true;}
 if(n.duty.seconds>=20){payWork(w,n,1);n.duty.seconds-=20;}return n.actionAge>=5;}
 return true;
}
export function startThreatDrill(w){if(!w.village)return false;const m=w.monsters.find(m=>m.rare&&m.hp>0&&!m.raiding)||w.monsters.find(m=>m.hp>0&&!m.raiding);if(!m)return false;m.raiding=true;message(w,`Threat test: an existing ${m.kind} is approaching the village. No creature or loot was created.`,'danger');return true;}

