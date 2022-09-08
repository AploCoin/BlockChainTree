
use thiserror::Error;
use colored::Colorize;

use crate::Block::SummarizeBlock;

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
        BiguintError(BiguintErrorKind),
        ZstdError(ZstdErrorKind)
    },

    TransactionError : "Error ocurred while operating on a transaction" {
        TxError(TxErrorKind)
    },

    MerkleTreeError : "Error ocurred while operating on the merkletree" {
        TreeError(MerkleTreeErrorKind)
    },

    BlockError : "Error ocurred while operating on a block" {
        BasicInfoError(BasicInfoErrorKind),
        TransactionTokenError(TxTokenErrorKind),
        TransactionBlockError(TxBlockErrorKind),
        TokenBlockError(TokenBlockErrorKind),
        SummarizeBlockError(SummarizeBlockErrorKind)
    }
];


sub_errors![
    BiguintErrorKind {
        DumpError : "failed to dump biguint, amount of bunches larger than 255",
        LoadError : "failed to load biguint (data length < bytes)"
    },
    ZstdErrorKind {
        CompressingFileError : "failed to compress file with zstd",
        DecompressingFileError : "failed to decompress file with zstd"
    },


    TxErrorKind {
        VerifyError : "failed to verify transaction",
        DumpError : "failed to dump transaction (amount)",
        ParseError : "failed to parse transaction"
    },


    MerkleTreeErrorKind {
        GettingProofError : "failed to get proof"
    },


    BasicInfoErrorKind {
        DumpError : "failed to dump basic info",
        ParseError : "failed to parse basic info"
    },
    TxTokenErrorKind {
        SettingTxError : "failed to set transaction (already set)",
        SettingTokenError : "failed to set token (already set)",
        DumpError : "failed to dump (token or transaction)"

    },
    TxBlockErrorKind {
        BuildingMerkleTreeError : "failed to build merkle tree",
        DumpError : "failed to dump",
        ParseError : "failed to parse"
    },
    TokenBlockErrorKind {
        DumpError : "failed to dump",
        ParseError : "failed to parse"
    },
    SummarizeBlockErrorKind {
        DumpError : "failed to dump",
        ParseError : "failed to parse",
        HashError : "failed to hash (couldn't dump)"
    }
];