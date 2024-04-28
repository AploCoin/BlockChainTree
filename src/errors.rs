//use colored::Colorize;
use thiserror::Error;

macro_rules! root_errors {
    [$(
        $errname:ident : $msg:literal {
            $(
                //nested should not exist because all the errors are $var + "Kind"
                //but concat_ident!() isn't stable and doesn't work well
                $var:ident($nested:ident)
            ),*
        }

    ),*] => {
        $(
            #[derive(Debug, Error)]
            pub enum $errname {
                $(
                    #[error("{} -> {}: {0}", $msg, stringify!($var))]
                    $var($nested),
                )*
            }
        )*
    };
}

macro_rules! sub_errors {
    [$(
        $name:ident {
            $(
                $var:ident : $msg:literal
            ),*
        }
    ),*] => {
        $(
            #[derive(Debug, Error)]
            pub enum $name {
                $(
                    #[error($msg)]
                    $var,
                )*
            }
        )*
    }
}

// #[macro_export]
// macro_rules! report {
//     ($ctx:expr, $msg:expr) => {
//         return Err(
//             Report::new($ctx)
//             .attach_printable($msg)
//         )
//     };
// }

root_errors![

    ToolsError : "Error ocurred while using a tool function" {
        Biguint(BiguintErrorKind),
        Zstd(ZstdErrorKind)
    },

    TransactionError : "Error ocurred while operating on a transaction" {
        Tx(TxErrorKind)
    },

    MerkleTreeError : "Error ocurred while operating on the merkletree" {
        TreeError(MerkleTreeErrorKind)
    },

    BlockError : "Error ocurred while operating on a block" {
        BasicInfo(BasicInfoErrorKind),
        TransactionToken(TxTokenErrorKind),
        TransactionBlock(TxBlockErrorKind),
        DerivativeBlock(DerivativeBlockErrorKind),
        SummarizeBlock(SummarizeBlockErrorKind),
        HeaderError(DumpHeadersErrorKind),
        NotImplemented(NotImplementedKind)
    },

    BlockChainTreeError : "Error ocurred while operating on the blockchain tree" {
        Chain(ChainErrorKind),
        DerivativeChain(DerivChainErrorKind),
        BlockChainTree(BCTreeErrorKind)
    },

    DumpHeadersError : "Error with dump header"{
        DumpHeadersError(DumpHeadersErrorKind)
    }
];

sub_errors![
    NotImplementedKind {
        Token: "Token is not implemented yet"
    },
    DumpHeadersErrorKind {
        UknownHeader: "Uknown header",
        WrongHeader: "Wrong header"
    },
    BiguintErrorKind {
        Dump: "failed to dump biguint, amount of bunches larger than 255",
        Load: "failed to load biguint (data length < bytes)"
    },
    ZstdErrorKind {
        CompressingFile: "failed to compress file with zstd",
        DecompressingFile: "failed to decompress file with zstd"
    },
    TxErrorKind {
        Verify: "failed to verify transaction",
        Dump: "failed to dump transaction (amount)",
        Parse: "failed to parse transaction"
    },
    MerkleTreeErrorKind {
        GettingProof: "failed to get proof"
    },
    BasicInfoErrorKind {
        Dump: "failed to dump basic info",
        Parse: "failed to parse basic info"
    },
    TxTokenErrorKind {
        SettingTx: "failed to set transaction (already set)",
        SettingToken: "failed to set token (already set)",
        Dump: "failed to dump (token or transaction)"
    },
    TxBlockErrorKind {
        BuildingMerkleTree: "failed to build merkle tree",
        Dump: "failed to dump",
        Parse: "failed to parse"
    },
    DerivativeBlockErrorKind {
        Dump: "failed to dump",
        Parse: "failed to parse"
    },
    SummarizeBlockErrorKind {
        Dump: "failed to dump",
        Parse: "failed to parse",
        Hash: "failed to hash (couldn't dump)"
    },
    ChainErrorKind {
        Init: "failed to create a new chain",
        AddingBlock: "failed to add block",
        AddingTransaction: "failed to add transaction",
        FindByHeight: "failed to find block by height",
        FindByHashE: "failed to find by hash",
        DumpConfig: "failed to dump config",
        InitWithoutConfig: "failed to create a new chain without config",
        FindTransaction: "failed to find transaction",
        FailedToVerify: "failed to verify block",
        FailedToHashBlock: "failed to hash block",
        FailedToRemoveHeighReference: "failed to remove height reference",
        FailedToRemoveTransaction: "failed to remove transaction"
    },
    DerivChainErrorKind {
        Init: "failed to create a new derivative chain",
        AddingBlock: "failed to add block",
        FindByHeight: "failed to find block by height",
        FindByHash: "failed to find by hash",
        DumpConfig: "failed to dump config",
        InitWithoutConfig: "failed to create a new chain without config"
    },
    BCTreeErrorKind {
        Init: "failed to init the blockchain tree (with config)",
        InitWithoutConfig: "failed to init the blockchain tree (with config)",
        DumpPool: "failed to dump pool",
        DumpDb: "failed to dump database",
        GetDerivChain: "failed to get the derivative chain",
        CreateDerivChain: "failed to create the derivative chain",
        CheckMainFolders: "failed to check and fix the main folders",
        AddFunds: "failed to add funds",
        DecreaseFunds: "failed to decrease funds",
        GetFunds: "failed to get funds",
        GetOldFunds: "failed to get funds from old summary db",
        MoveSummaryDB: "failed to move summary database",
        NewTransaction: "failed to create new transaction",
        CreateMainChainBlock: "failed to create new block for the main chain",
        WrongPow: "supplied pow does not satisfy requirements"
    }
];
