
use thiserror::Error;
use colored::Colorize;

macro_rules! root_errors {
    [$(
        $name:ident : $msg:literal {
            $(
                $vars:ident($sub:ident)
            ),*
        }
        
    ),*] => {

        $(
            #[derive(Debug, Error)]
            pub enum $name {
                $(
                    #[error("{} -> {}: {0}", $msg, stringify!($vars))]
                    $vars($sub),
                )*
            }
        )*
    };
}

macro_rules! sub_errors {
    [$(
        $name:ident {
            $(
                $vars:ident : $msg:literal
            ),*
        }
    ),*] => {
        $(
            #[derive(Debug, Error)]
            pub enum $name {
                $(
                    #[error($msg)]
                    $vars,
                )*
            }
        )*
    };
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
    
    BlockError : "Error ocurred while operating with a block: " {
        BasicInfoError(BasicInfoErrorKind),
        TransactionTokenError(TransactionTokenErrorKind),
        TransactionBlockError(TransactionBlockErrorKind),
        TokenBlockError(TokenBlockErrorKind),
        SummarizeBlockError(SummarizeBlockErrorKind),
        SumTransactionBlockError(SumTransactionBlockErrorKind)
    },

    TransactionError : "Error ocurred while operating on a transaction" {
        VerifyError(VerifyTransactionErrorKind),
        DumpError(DumpTransactionErrorKind),
        ParseError(ParseTransactionErrorKind)
    },

    MerkleTreeError : "Error ocurred while operating on the merkel tree" {
        NoHashFoundError(NoHashFoundErrorKind)
    },
    // TokenError: "Error ocurred while operating on a token or token action" {
    //     TokenCreationError(TokenCreationErrorKind),
    //     DecodeError(DecodeErrorKind),
    //     VerifyError(VerifyErrorKind),
    //     DumpError(DumpErrorKind)
    // },

    ToolsError : "Error ocurred while calling a tool function" {
        BiguintError(BiguintErrorKind),
        ZstdError(ZstdErrorKind)
    },
    BlockChainTreeError : "Error ocurred while operating with the blockchain tree" {
        ChainError(ChainErrorKind)
    }

];

//Block
sub_errors![
    BasicInfoErrorKind {
        DumpingPowError: "failed to dump PoW (biguint)",
        ParsingPowError: "failed to parse PoW (biguint)",
        DataTooLargeError: "failed to parse: data length is bigger than 112"

    },
    TransactionTokenErrorKind {
        SettingTransactionError: "failed to set tx in transaction token: tx already set",
        SettingTokenError: "failed to set token in transaction token: token already set",
        DumpingTransactionError: "failed to dump transaction token (transaction)",
        DumpingTokenError: "failed to dump transaction token (token)"
    },
    TransactionBlockErrorKind {
        BuildingMerkleTreeError: "failed to build merkle tree",
        DumpingDefaultInfoError : "failed to dump default info",
        DumpingFeeError : "failed to dump fee",
        TooManyTransactionsError : "failed to dump: too many transactions (> 0xFFFF)",
        ParsingDefaultInfoError : "failed to parse default info",
        ParsingFeeError : "failed to parse fee",
        ParsingTxError : "failed to parse transaction",
        SettingTxError : "failed to set transaction",
        ParsingTkError : "failed to parse token",
        SettingTkError : "failed to set token",
        InvalidTypeError: "failed to parse: type doesn't exist",
        ParsingBlockError : "failed to parse block (offset != block size)"
    },
    TokenBlockErrorKind {
        DumpingTransactionError : "failed to dump: couldn't dump payment transaction",
        DumpingDefaultInfoError : "failed to dump: couldn't dump default info",
        TransactionNotFoundError : "faied to parse: couldn't find transaction",
        ParsingTransactionError : "failed to parse: couldn't parse transaction",
        ParsingDefaultInfoError : "failed to parse: couldn't parse basic info",
        ParsingError : "failed to parse: offset != block size"
    },
    SummarizeBlockErrorKind {
        DumpingTransactionError : "failed to dump: couldn't dump transaction",
        ParsingTransactionError : "failed to parse: couldn't parse transaction",
        NotEnoughDataError : "failed to parse: not enough data",
        SizeMismatchError : "failed to parse: data length < tx size + 8",
        HeadersNotFoundError : "failed to parse: couldn't find headers",
        HashingError : "failed to hash (dump)"
    },
    SumTransactionBlockErrorKind {

    }
];

//Transaction
sub_errors! [
    VerifyTransactionErrorKind {
        VerifySenderError : "failed to load/verify sender",
        VerifyMessageError : "failed to load/verify message",
        VerifySignatureError : "failed to load/verify signature"
    },
    ParseTransactionErrorKind {
        ParseAmountError : "failed to load amount",
        BadSizeError : "failed to parse due to an invalid size (<= 138)",
        InvalidTransactionError : "error parsing transaction due to a data mismatch"
    },
    DumpTransactionErrorKind {
        DumpAmountError : "failed to dump amount"
    }
];

//Merkle tree
sub_errors! [
    NoHashFoundErrorKind {
        NoHashFoundError : "no such hash found"
    }
];

//Tools
sub_errors! [
    BiguintErrorKind {
        DumpError : "failed to dump due to wrong amount of bunches (larger than 255)",
        LoadError : "failed to load due to wrong amount of bunches"
    },
    ZstdErrorKind {
        CreatingFileError : "failed to create file",
        ReadingFileError : "failed reading from decoder",
        EncoderCreationError : "failed to create encoder",
        DecoderCreationError : "failed to create decoder",
        EncodingError : "failed to encode data",
        ClosingFileError : "failed to close file"
    }
];