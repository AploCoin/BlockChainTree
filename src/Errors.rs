#[derive(Debug)]
pub enum NodeError {
    CannotCreateFile(&'static str),
}

#[derive(Debug)]
pub enum BlockError {
    CannotCreateFile(&'static str),
}

#[derive(Debug)]
pub enum TreeError {
    CannotCreateFile(&'static str),
}

#[derive(Debug)]
pub enum TokenError {
    CannotCreateFile(&'static str),
}