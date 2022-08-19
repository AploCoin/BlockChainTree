
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

#[macro_export]
macro_rules! report {
    ($ctx:expr, $msg:expr) => {
        return Err(
            Report::new($ctx)
            .attach_printable($msg)
        );
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

    TokenError: "Error ocurred while operating on a token or token action" {
        CreationError,
        DecodeError,
        VerifyError,
        DumpError,
        NotImplementedYet
    },

    ToolsError : "Error ocurred while calling a tool function" {
        BiguintError,
        ZstdError
    }

];
