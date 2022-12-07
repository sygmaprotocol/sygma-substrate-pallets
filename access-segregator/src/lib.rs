#![cfg_attr(not(feature = "std"), no_std)]

pub use self::pallet::*;

#[cfg(test)]
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
					Self::has_access(T::PalletIndex::get(), extrinsic_index, who),
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
		fn pure_grant_access_test() {
			new_test_ext().execute_with(|| {
				// ALICE grants BOB access, should fail because AlICE does not have access to
				// extrinsic 0 yet should get GrantAccessFailed error
				assert_noop!(
					AccessSegregator::grant_access(Some(ALICE).into(), PalletIndex::get(), 0, BOB),
					sygma_access_segregator::Error::<Test>::GrantAccessFailed
				);
				// neither ALICE nor BOB should have the access
				assert!(!AccessSegregator::has_access(PalletIndex::get(), 0, ALICE));
				assert!(!AccessSegregator::has_access(PalletIndex::get(), 0, BOB));

				// Root origin grants access to BOB of the access extrinsic 0, not ALICE
				// so that BOB is able to grant other accounts just like Root origin
				assert_ok!(AccessSegregator::grant_access(
					Origin::root(),
					PalletIndex::get(),
					0,
					BOB
				));
				// BOB has access, but ALICE does not
				assert!(AccessSegregator::has_access(PalletIndex::get(), 0, BOB));
				assert!(!AccessSegregator::has_access(PalletIndex::get(), 0, ALICE));

				// BOB grants access to CHARLIE of access to extrinsic 100, should work
				// check if CHARLIE already has access to extrinsic 100
				assert!(!AccessSegregator::has_access(PalletIndex::get(), 100, CHARLIE));
				assert_ok!(AccessSegregator::grant_access(
					Some(BOB).into(),
					PalletIndex::get(),
					100,
					CHARLIE
				));
				// BOB has access of extrinsic 0
				assert!(AccessSegregator::has_access(PalletIndex::get(), 0, BOB));

				// CHARLIE should not have access to any extrinsic other then extrinsic 100
				assert!(!AccessSegregator::has_access(PalletIndex::get(), 0, CHARLIE));
				assert!(!AccessSegregator::has_access(PalletIndex::get(), 999, CHARLIE));
				assert!(AccessSegregator::has_access(PalletIndex::get(), 100, CHARLIE));

				// AlICE does not have access to extrinsic 100 at this moment
				assert!(!AccessSegregator::has_access(PalletIndex::get(), 100, ALICE));
				// Since CHARLIE has the access to extrinsic 100, CHARLIE tries to grant access to
				// ALICE of extrinsic 100, should not work
				assert_noop!(
					AccessSegregator::grant_access(
						Some(CHARLIE).into(),
						PalletIndex::get(),
						100,
						ALICE
					),
					sygma_access_segregator::Error::<Test>::GrantAccessFailed
				);
			})
		}
	}
}
