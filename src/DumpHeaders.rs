#[repr(u8)]
pub enum Headers{
    Transaction = 0,
    Token = 1,
    TransactionBlock = 2,
    TokenBlock = 3,
    SummarizeBlock = 4

}