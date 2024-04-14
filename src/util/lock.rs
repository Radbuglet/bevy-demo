use std::sync::LockResult;

pub trait UnpoisonExt: Sized {
    type Guard;

    fn unpoison(self) -> Self::Guard;
}

impl<G> UnpoisonExt for LockResult<G> {
    type Guard = G;

    fn unpoison(self) -> Self::Guard {
        match self {
            Ok(guard) => guard,
            Err(err) => err.into_inner(),
        }
    }
}
