#![allow(dead_code)]
use litesvm::{
    types::{FailedTransactionMetadata, TransactionMetadata},
    LiteSVM,
};
use solana_address::Address;
use solana_compute_budget_interface::ComputeBudgetInstruction;
use solana_instruction::{account_meta::AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_rent::Rent;
use solana_signer::Signer;
use solana_transaction::Transaction;

pub const ID: Address = Address::new_from_array([41; 32]);
pub const CONSUMER: Address = Address::new_from_array([42; 32]);
pub const HEADER: usize = 45;
pub const PROOF: usize = 6752;
pub const SIZE: usize = HEADER + PROOF;

pub fn fixture() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/blake3.json")).unwrap();
    let bytes =
        |key: &str| hex::decode(value[key].as_str().unwrap().trim_start_matches("0x")).unwrap();
    (bytes("programVK"), bytes("publicValues"), bytes("proofBytes"))
}
pub fn elf(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/../target/deploy/{name}.so", env!("CARGO_MANIFEST_DIR")))
        .expect("build the actual SBF ELF first")
}

pub struct Harness {
    pub svm: LiteSVM,
    pub payer: Keypair,
    pub buffer: Keypair,
    pub receipt: Keypair,
    pub sizes: Vec<usize>,
}
impl Harness {
    pub fn new() -> Self {
        let mut svm = LiteSVM::new();
        let payer = Keypair::new();
        svm.add_program(ID, &elf("zisk_solana_program")).unwrap();
        svm.airdrop(&payer.pubkey(), 1_000_000_000).unwrap();
        Self { svm, payer, buffer: Keypair::new(), receipt: Keypair::new(), sizes: vec![] }
    }
    pub fn send(
        &mut self,
        instructions: Vec<Instruction>,
        sign_buffer: bool,
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        let mut all = vec![ComputeBudgetInstruction::set_compute_unit_limit(1_400_000)];
        all.extend(instructions);
        let mut signers = vec![&self.payer];
        if sign_buffer {
            signers.push(&self.buffer);
        }
        if all
            .iter()
            .any(|ix| ix.accounts.iter().any(|a| a.pubkey == self.receipt.pubkey() && a.is_signer))
        {
            signers.push(&self.receipt);
        }
        let tx = Transaction::new(
            &signers,
            Message::new(&all, Some(&self.payer.pubkey())),
            self.svm.latest_blockhash(),
        );
        let size = bincode::serialize(&tx).unwrap().len();
        assert!(size <= 1232, "oversized transaction: {size}");
        self.sizes.push(size);
        let result = self.svm.send_transaction(tx);
        self.svm.expire_blockhash();
        result
    }
    pub fn instruction(&self, data: Vec<u8>) -> Instruction {
        let verify = data[0] == 3;
        let mut accounts = vec![if verify {
            AccountMeta::new_readonly(self.buffer.pubkey(), false)
        } else {
            AccountMeta::new(self.buffer.pubkey(), data[0] == 0)
        }];
        if !verify {
            accounts.push(AccountMeta::new(self.payer.pubkey(), true));
        }
        Instruction { program_id: ID, accounts, data }
    }
    pub fn create(&mut self) -> u64 {
        let lamports = self.svm.get_sysvar::<Rent>().minimum_balance(SIZE);
        let create = solana_system_interface::instruction::create_account(
            &self.payer.pubkey(),
            &self.buffer.pubkey(),
            lamports,
            SIZE as u64,
            &ID,
        );
        let init = self.instruction([vec![0], (PROOF as u32).to_le_bytes().to_vec()].concat());
        self.send(vec![create, init], true).expect("initialize buffer").compute_units_consumed
    }
    pub fn append(
        &mut self,
        offset: usize,
        chunk: &[u8],
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        self.send(
            vec![self.instruction(
                [vec![1], (offset as u32).to_le_bytes().to_vec(), chunk.to_vec()].concat(),
            )],
            false,
        )
    }
    pub fn seal(&mut self) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        self.send(vec![self.instruction(vec![2])], false)
    }
    pub fn upload(&mut self, proof: &[u8]) {
        self.create();
        for (i, chunk) in proof.chunks(900).enumerate() {
            self.append(i * 900, chunk).unwrap();
        }
        self.seal().unwrap();
    }
    pub fn verify(
        &mut self,
        guest: &[u8],
        outputs: &[u8],
    ) -> Result<TransactionMetadata, FailedTransactionMetadata> {
        self.send(
            vec![self.instruction([vec![3], guest.to_vec(), outputs.to_vec()].concat())],
            false,
        )
    }
}
