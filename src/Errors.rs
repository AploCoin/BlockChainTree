
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

sub_errors![
    BasicInfoErrorKind {

    },
    TransactionTokenErrorKind {

    },
    TransactionBlockErrorKind {

    },
    TokenBlockErrorKind {

    },
    SummarizeBlockErrorKind {

    },
    SumTransactionBlockErrorKind {

    },


    VerifyTransactionErrorKind {
        VerifySenderError : "failed to load/verify sender",
        VerifyMessageError : "failed to load/verify message",
        VerifySignatureError : "failed to load/verify signature"
    },
    // DecodeErrorKind {

    // },
    ParseTransactionErrorKind {
        ParseAmountError : "failed to load amount",
        BadSizeError : "failed to parse due to an invalid size (<= 138)",
        InvalidTransactionError : "error parsing transaction due to a data mismatch"
    },
    DumpTransactionErrorKind {
        DumpAmountError : "failed to dump amount"
    },


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
    },


    ChainErrorKind {

    }

];
