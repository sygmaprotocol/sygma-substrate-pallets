#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

// #[cfg(test)]
mod mock;

#[allow(unused_variables)]
#[allow(clippy::large_enum_variant)]
#[frame_support::pallet]
pub mod pallet {
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::StorageVersion};
	use frame_system::pallet_prelude::*;

	const STORAGE_VERSION: StorageVersion = StorageVersion::new(0);

	/// Mapping signature of extrinsic to account has access
	/// (pallet_index, extrinsic_index) => account
	#[pallet::storage]
	#[pallet::getter(fn extrinsic_access)]
	pub type ExtrinsicAccess<T: Config> = StorageMap<_, Twox64Concat, (u8, u32), T::AccountId>;

	#[pallet::pallet]
	#[pallet::generate_store(pub (super) trait Store)]
	#[pallet::storage_version(STORAGE_VERSION)]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Origin used to administer the pallet
		type BridgeCommitteeOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Current pallet index defined in runtime
		type PalletIndex: Get<u8>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub (super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Extrinsic access grant to someone
		/// args: [pallet_index, extrinsic_index, who]
		AccessGranted { pallet_index: u8, extrinsic_index: u32, who: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Function unimplemented
		Unimplemented,
		/// Failed to grant extrinsic access permission to an account
		GrantAccessFailed,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Grants access to an account for a extrinsic.
		#[pallet::weight(195_000_000)]
		pub fn grant_access(
			origin: OriginFor<T>,
			pallet_index: u8,
			extrinsic_index: u32,
			who: T::AccountId,
		) -> DispatchResult {
			// Ensure bridge committee or the account that has permisson to grant access to an
			// extrinsic
			if T::BridgeCommitteeOrigin::ensure_origin(origin.clone()).is_err() {
				let who = ensure_signed(origin)?;
				// 0 is extrinsc index of `grant_access` by default
				let extrinsic_index = frame_system::Pallet::<T>::extrinsic_index().unwrap_or(0);
				ensure!(
					ExtrinsicAccess::<T>::get((T::PalletIndex::get(), extrinsic_index)).is_some(),
					Error::<T>::GrantAccessFailed
				);
			}

			// Apply access
			ExtrinsicAccess::<T>::insert((pallet_index, extrinsic_index), &who);

			// Emit AccessGranted event
			Self::deposit_event(Event::AccessGranted { pallet_index, extrinsic_index, who });
			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn has_access(pallet_index: u8, extrinsic_index: u32, caller: T::AccountId) -> bool {
			ExtrinsicAccess::<T>::get((pallet_index, extrinsic_index))
				.map_or(false, |who| who == caller)
		}
	}

	#[cfg(test)]
	mod test {
		use crate as sygma_access_segregator;
		use crate::{
			mock::{
				assert_events, new_test_ext, AccessSegregator, PalletIndex, RuntimeEvent as Event,
				RuntimeOrigin as Origin, Test, ALICE, BOB, CHARLIE,
			},
			Event as AccessSegregatorEvent,
		};
		use frame_support::{assert_noop, assert_ok};

		#[test]
		fn should_work() {
			new_test_ext().execute_with(|| {
				// (PalletIndex:get(), 0) indicates extrinsic: `grant_access` of this pallet
				assert_noop!(
					AccessSegregator::grant_access(Some(ALICE).into(), PalletIndex::get(), 0, BOB),
					sygma_access_segregator::Error::<Test>::GrantAccessFailed
				);
				assert!(!AccessSegregator::has_access(PalletIndex::get(), 0, ALICE));
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					PalletIndex::get(),
					0,
					ALICE
				));
				assert!(AccessSegregator::has_access(PalletIndex::get(), 0, ALICE));

				// ALICE grants access permission to BOB for an extrinsic (100, 100)
				assert_ok!(AccessSegregator::grant_access(Some(ALICE).into(), 100, 100, BOB));
				assert!(!AccessSegregator::has_access(100, 100, ALICE));
				assert!(AccessSegregator::has_access(100, 100, BOB));

				assert_events(vec![
					Event::AccessSegregator(AccessSegregatorEvent::AccessGranted {
						pallet_index: PalletIndex::get(),
						extrinsic_index: 0,
						who: ALICE,
					}),
					Event::AccessSegregator(AccessSegregatorEvent::AccessGranted {
						pallet_index: 100,
						extrinsic_index: 100,
						who: BOB,
					}),
				]);
			})
		}

		#[test]
		fn should_not_work_but_works() {
			new_test_ext().execute_with(|| {
				println!("ALICE: {ALICE:?}");
				println!("BOB: {BOB:?}");
				println!("CHARLIE: {CHARLIE:?}");

				// ALICE grants BOB access, should fail because in ExtrinsicAccess,
				// there is no value for key == (T::PalletIndex::get(), 0) at this moment
				// should get GrantAccessFailed error
				assert_noop!(
					AccessSegregator::grant_access(Some(ALICE).into(), PalletIndex::get(), 0, BOB),
					sygma_access_segregator::Error::<Test>::GrantAccessFailed
				);
				// neither ALICE nor BOB should have the access
				assert!(!AccessSegregator::has_access(PalletIndex::get(), 0, ALICE));
				assert!(!AccessSegregator::has_access(PalletIndex::get(), 0, BOB));

				// Root origin grants access to BOB, not ALICE
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					PalletIndex::get(),
					0,
					BOB
				));
				// BOB has access, but ALICE does not
				assert!(AccessSegregator::has_access(PalletIndex::get(), 0, BOB));
				assert!(!AccessSegregator::has_access(PalletIndex::get(), 0, ALICE));

				// At this moment, there is value for key == (T::PalletIndex::get(), 0) in
				// ExtrinsicAccess so anyone is able to grant any extrinsic access to anybody

				// ALICE grants access to CHARLIE
				assert_ok!(AccessSegregator::grant_access(
					Some(ALICE).into(),
					PalletIndex::get(),
					0,
					CHARLIE
				));
				// Now CHARLIE has access of 0 extrinsic but BOB not
				assert!(AccessSegregator::has_access(PalletIndex::get(), 0, CHARLIE));
				assert!(!AccessSegregator::has_access(PalletIndex::get(), 0, BOB));

				// ALICE grants access to CHARLIE with access of extrinsic 100
				assert_ok!(AccessSegregator::grant_access(
					Some(ALICE).into(),
					PalletIndex::get(),
					100,
					CHARLIE
				));
				// Now CHARLIE has access to extrinsic 100 but BOB not
				assert!(AccessSegregator::has_access(PalletIndex::get(), 100, CHARLIE));
				assert!(!AccessSegregator::has_access(PalletIndex::get(), 100, BOB));

				// BOB grants access to CHARLIE with access of extrinsic 999
				assert_ok!(AccessSegregator::grant_access(
					Some(BOB).into(),
					PalletIndex::get(),
					999,
					CHARLIE
				));
				// Now CHARLIE has access to extrinsic 999
				assert!(AccessSegregator::has_access(PalletIndex::get(), 999, CHARLIE));
			})
		}
	}
}
