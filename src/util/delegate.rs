#[doc(hidden)]
pub mod delegate_macro_internal {
    pub use std::{
        clone::Clone,
        convert::From,
        fmt,
        marker::{PhantomData, Send, Sync},
        ops::Deref,
        stringify,
        sync::Arc,
    };
}

#[macro_export]
macro_rules! delegate {
	(
		$(#[$attr_meta:meta])*
		$vis:vis fn $name:ident
			$(
				<$($generic:ident),* $(,)?>
				$(<$($fn_lt:lifetime),* $(,)?>)?
			)?
			($($para_name:ident: $para:ty),* $(,)?) $(-> $ret:ty)?
		$(where $($where_token:tt)*)?
	) => {
		$(#[$attr_meta])*
		$vis struct $name $(<$($generic),*>)?
		$(where
			$($where_token)*
		)? {
			_ty: ($($($crate::util::delegate::delegate_macro_internal::PhantomData<fn() -> $generic>,)*)?),
			handler: $crate::util::delegate::delegate_macro_internal::Arc<
				dyn
					$($(for<$($fn_lt),*>)?)?
					Fn($($para),*) $(-> $ret)? +
						$crate::util::delegate::delegate_macro_internal::Send +
						$crate::util::delegate::delegate_macro_internal::Sync
			>,
		}

		impl$(<$($generic),*>)? $name $(<$($generic),*>)?
		$(where
			$($where_token)*
		)? {
			#[allow(unused)]
			pub fn new<Func>(handler: Func) -> Self
			where
				Func: 'static +
					$crate::util::delegate::delegate_macro_internal::Send +
					$crate::util::delegate::delegate_macro_internal::Sync +
					$($(for<$($fn_lt),*>)?)?
						Fn($($para),*) $(-> $ret)?,
			{
				Self {
					_ty: ($($($crate::util::delegate::delegate_macro_internal::PhantomData::<fn() -> $generic>,)*)?),
					handler: $crate::util::delegate::delegate_macro_internal::Arc::new(handler),
				}
			}
		}

		impl<
			Func: 'static +
				$crate::util::delegate::delegate_macro_internal::Send +
				$crate::util::delegate::delegate_macro_internal::Sync +
				$($(for<$($fn_lt),*>)?)?
					Fn($($para),*) $(-> $ret)?
			$(, $($generic),*)?
		> $crate::util::delegate::delegate_macro_internal::From<Func> for $name $(<$($generic),*>)?
		$(where
			$($where_token)*
		)? {
			fn from(handler: Func) -> Self {
				Self::new(handler)
			}
		}

		impl$(<$($generic),*>)? $crate::util::delegate::delegate_macro_internal::Deref for $name $(<$($generic),*>)?
		$(where
			$($where_token)*
		)? {
			type Target = dyn $($(for<$($fn_lt),*>)?)? Fn($($para),*) $(-> $ret)? +
				$crate::util::delegate::delegate_macro_internal::Send +
				$crate::util::delegate::delegate_macro_internal::Sync;

			fn deref(&self) -> &Self::Target {
				&*self.handler
			}
		}

		impl$(<$($generic),*>)? $crate::util::delegate::delegate_macro_internal::fmt::Debug for $name $(<$($generic),*>)?
		$(where
			$($where_token)*
		)? {
			fn fmt(&self, fmt: &mut $crate::util::delegate::delegate_macro_internal::fmt::Formatter) -> $crate::util::delegate::delegate_macro_internal::fmt::Result {
				fmt.write_str("delegate::")?;
				fmt.write_str($crate::util::delegate::delegate_macro_internal::stringify!($name))?;
				fmt.write_str("(")?;
				$(
					fmt.write_str($crate::util::delegate::delegate_macro_internal::stringify!($para))?;
				)*
				fmt.write_str(")")?;

				Ok(())
			}
		}

		impl$(<$($generic),*>)? $crate::util::delegate::delegate_macro_internal::Clone for $name $(<$($generic),*>)?
		$(where
			$($where_token)*
		)? {
			fn clone(&self) -> Self {
				Self {
					_ty: ($($($crate::util::delegate::delegate_macro_internal::PhantomData::<fn() -> $generic>,)*)?),
					handler: $crate::util::delegate::delegate_macro_internal::Clone::clone(&self.handler),
				}
			}
		}
	};
}

pub use delegate;
