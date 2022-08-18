
use thiserror::Error;
use colored::Colorize;

macro_rules! root_errors {
    [$($name:ident : $msg:tt {$($vars:ident),*}),*] => {
        $(
            #[derive(Debug, Error)]
            #[error($msg)]
            pub enum $name {
                $(
                    $vars,
                )*
            }
        )*
    };
}

root_errors![
    
    BlockError : "Error ocurred while operating with a block" {
        BasicInfoError,
        TransactionTokenError,
        TransactionBlockError,
        TokenBlockError,
        SummarizeBlockError,
        SumTransactionBlockError
    },

    TransactionError : "Error ocurred while operating on a transaction" {
        VerifyError,
        DumpError,
        ParseError
    },

    ToolsError : "Error ocurred while calling a tool function" {
        BiguintError,
        ZstdError
    }

];
