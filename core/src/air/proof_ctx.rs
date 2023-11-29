// use crate::trace::trace_layout::TraceLayout;

// pub struct AirInstance {
//     subproof_id: usize,
//     air_id: usize,
//     instance_id: usize,
//     //proof: Vec<T>,
//     trace_layout: TraceLayout,
//     //tmp_pol: Vec<T>,
// }

// impl AirInstance {
//     pub fn new(subproof_id: usize, air_id: usize, instance_id: usize, trace_layout: &TraceLayout) -> AirInstance {
//         AirInstance {
//             subproof_id,
//             air_id,
//             instance_id,
//             //proof: Vec::new(),
//             trace_layout: trace_layout.clone(),
//             // tmp_pol: Vec::new(),
//         }
//     }
// }

// pub struct ProoCtx<T> {
//     name: String,
//     publics: Vec<T>,
//     subproof_values: Vec<Vec<T>>,
//     challenges: Vec<Vec<T>>,
//     air_instances: Vec<AirInstance>,
//     num_instances: usize,
//     //transcript: Transcript,
//     //airout: Airout,
// }

// impl<T> ProoCtx<T> {
//     pub fn new(name: String) -> ProoCtx<T> {
//         ProoCtx {
//             name,
//             publics: Vec::new(),
//             subproof_values: Vec::new(),
//             challenges: Vec::new(),
//             air_instances: Vec::new(),
//             num_instances: 0,
//             //transcript: Transcript,
//             //airout: Airout,
//         }
//     }

//     // pub fn initialize(&mut self, publics: Vec<T>) {
//     //     self.publics = publics;
//     //     self.air_instances = Vec::new();
//     // }

//     // pub fn add_challenge_to_transcript(&mut self, challenge: T) {
//     //     //addTranscriptStark(self.transcript, challenge);
//     // }

//     // pub fn compute_global_challenge(&mut self, stage_id: usize) {
//     //     if self.challenges[stage_id].len() == 0 {
//     //         return;
//     //     }

//     //     for i in 0..self.challenges[stage_id].len() {
//     //         //self.challenges[stage_id][i] = getChallengeStark(self.transcript);
//     //     }
//     // }

//     // pub fn get_challenge(&self, stage_id: usize) -> &Vec<T> {
//     //     if stage_id >= self.challenges.len() {
//     //         panic!("The requested challenge is not within the valid bounds of proof challenges.");
//     //     }

//     //     &self.challenges[stage_id]
//     // }

//     // pub fn get_airout(&self) -> &Vec<AirInstance> {
//     //     &self.air_instances
//     // }
// }
// // const log = require("../logger.js");

// // const { newCommitPolsArrayPil2 } = require("pilcom2/src/polsarray.js");
// // const { addTranscriptStark, getChallengeStark } = require("pil2-stark-js/src/stark/stark_gen_helpers.js");
// // const { buildPoseidonGL, Transcript } = require("pil2-stark-js");

// // class ProofCtx {
// //     /**
// //      * Creates a new ProofCtx
// //      * @constructor
// //      */
// //     constructor(name, finiteField) {
// //         this.name = name;
// //         this.F = finiteField;

// //         this.resetProofCtx();
// //     }

// //     resetProofCtx() {
// //         this.publics = [];
// //         this.subproofValues = [];
// //         this.challenges = [];
// //         this.airInstances = [];
// //         this.numInstances = 0;
// //     }

// //     async initialize(publics) {
// //         this.publics = publics;

// //         const poseidon = await buildPoseidonGL();
// //         this.transcript = new Transcript(poseidon);

// //         this.airInstances = [];
// //     }

// //     addChallengeToTranscript(challenge) {
// //         addTranscriptStark(this.transcript, challenge);
// //     }

// //     computeGlobalChallenge(stageId) {
// //         if(this.challenges[stageId].length === 0) return;

// //         for(let i = 0; i< this.challenges[stageId].length; i++) {
// //             this.challenges[stageId][i] = getChallengeStark(this.transcript);
// //         }
// //     }

// //     getChallenge(stageId) {
// //         if (stageId >= this.challenges.length) {
// //             log.error(`The requested challenge is not within the valid bounds of proof challenges.`);
// //             throw new Error(`The requested challenge is not within the valid bounds of proof challenges.`);
// //         }

// //         return this.challenges[stageId];
// //     }

// //     getAirout() {
// //         return this.airout;
// //     }

// //     // Allocate a new buffer for the given subproofId and airId with the given numRows.
// //     addAirInstance(subproofId, airId, numRows) {
// //         const air = this.airout.getAirBySubproofIdAirId(subproofId, airId);

// //         if (air === undefined) return { result: false, data: undefined };

// //         const instanceId = this.numInstances++;
// //         const layout = { numRows };
// //         const airInstance = new AirInstance(subproofId, airId, instanceId, layout);
// //         this.airInstances[instanceId] = airInstance;

// //         airInstance.wtnsPols = newCommitPolsArrayPil2(air.symbols, air.numRows, this.F);

// //         return { result: true, airInstance};
// //     }   

// //     // Proof API
// //     getAirInstancesBySubproofIdAirId(subproofId, airId) {
// //         const airInstances = this.airInstances.filter(airInstance => airInstance.subproofId === subproofId && airInstance.airId === airId);

// //         return airInstances.sort((a, b) => a.instanceId - b.instanceId);
// //     }

// //     // getAirCols(subproofId, airId)
// //     //

// //     static createProofCtxFromAirout(name, airout, finiteField) {
// //         const proofCtx = new ProofCtx(name, finiteField);
// //         proofCtx.airout = airout;

// //         const zero = finiteField.zero;
// //         const one = finiteField.one;

// //         if (airout.numChallenges !== undefined) {
// //             for (let i = 0; i < airout.numChallenges.length; i++) {
// //                 if (airout.numChallenges[i] === undefined) continue;

// //                 proofCtx.challenges.push(new Array(airout.numChallenges[i]).fill(null));
// //             }
// //         } else {
// //             proofCtx.challenges.push([]);
// //         }

// //         // qStage, evalsStage and friStage
// //         proofCtx.challenges.push(new Array(1).fill(null));
// //         proofCtx.challenges.push(new Array(1).fill(null));
// //         proofCtx.challenges.push(new Array(2).fill(null));
        
// //         // TODO: Calculate friStages
// //         proofCtx.challenges.push(new Array(1).fill(null));
// //         proofCtx.challenges.push(new Array(1).fill(null));
// //         proofCtx.challenges.push(new Array(1).fill(null));
// //         proofCtx.challenges.push(new Array(1).fill(null));
        
// //         for(let i = 0; i < airout.subproofs.length; i++) {
// //             proofCtx.subproofValues[i] = [];
// //             for(let j = 0; j < airout.subproofs[i].subproofvalues?.length; j++) {
// //                 const aggType = airout.subproofs[i].subproofvalues[j].aggType;
// //                 proofCtx.subproofValues[i][j] = aggType === 0 ? zero : one;
// //             }
// //         }
// //         return proofCtx;
// //     }
// // }

// // class AirInstance {
// //     constructor(subproofId, airId, instanceId, layout) {
// //         this.subproofId = subproofId;
// //         this.airId = airId;
// //         this.instanceId = instanceId;
// //         this.proof = {};
// //         this.layout = layout;

// //         this.tmpPol = [];
// //     }
// // }

// // module.exports = ProofCtx;
