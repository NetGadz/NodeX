#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;
use alloc::vec::Vec;
use borsh::{BorshDeserialize, BorshSerialize};

pub type Address = [u8; 32];
pub type Hash = [u8; 32];

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct NDX20Token {
    pub name: Vec<u8>,
    pub symbol: Vec<u8>,
    pub decimals: u8,
    pub total_supply: u128,
}
