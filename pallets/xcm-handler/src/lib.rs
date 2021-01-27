// Copyright 2020 Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

//! A pallet which implements the message-broker APIs for handling incoming XCM:
//! * `DownwardMessageHandler`
//! * `HrmpMessageHandler`
//!
//! Also provides an implementation of `SendXcm` to handle outgoing XCM.

#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use cumulus_primitives::{
	DownwardMessageHandler, HrmpMessageHandler, HrmpMessageSender, InboundDownwardMessage,
	InboundHrmpMessage, OutboundHrmpMessage, ParaId, UpwardMessageSender,
};
use frame_support::{decl_error, decl_event, decl_module, sp_runtime::traits::Hash};
use frame_system::ensure_signed;
use sp_std::convert::{TryFrom, TryInto};
use xcm::{
	v0::{Error as XcmError, ExecuteXcm, Junction, MultiLocation, SendXcm, Xcm},
	VersionedXcm,
};
use xcm_executor::traits::LocationConversion;

pub trait Config: frame_system::Config {
	type Event: From<Event<Self>> + Into<<Self as frame_system::Config>::Event>;
	/// Something to execute an XCM message.
	type XcmExecutor: ExecuteXcm;
	/// Something to send an upward message.
	type UpwardMessageSender: UpwardMessageSender;
	/// Something to send an HRMP message.
	type HrmpMessageSender: HrmpMessageSender;
	/// Convert AccountId to MultiLocation
	type AccountIdConverter: LocationConversion<Self::AccountId>;
}

decl_event! {
	pub enum Event<T> where Hash = <T as frame_system::Config>::Hash {
		/// Some XCM was executed ok.
		Success(Hash),
		/// Some XCM failed.
		Fail(Hash, XcmError),
		/// Bad XCM version used.
		BadVersion(Hash),
		/// Bad XCM format used.
		BadFormat(Hash),
		/// An upward message was sent to the relay chain.
		UpwardMessageSent(Hash),
		/// An HRMP message was sent to a sibling parachainchain.
		HrmpMessageSent(Hash),
	}
}

decl_error! {
	pub enum Error for Module<T: Config> {
		/// Failed to send XCM message.
		FailedToSend,
		/// Failed to execute XCM message
		FailedToExecute,
	}
}

decl_module! {
	pub struct Module<T: Config> for enum Call where origin: T::Origin {
		fn deposit_event() = default;

		#[weight = 1_000]
		fn execute_xcm(origin, message: Xcm) {
			frame_support::debug::RuntimeLogger::init();

			let who = ensure_signed(origin)?;

			frame_support::debug::info!("----------- Execute xcm -------------");
			let xcm_origin = T::AccountIdConverter::try_into_location(who).map_err(|_| Error::<T>::FailedToExecute)?;
			let msg = message.try_into().map_err(|_| Error::<T>::FailedToExecute)?;
			T::XcmExecutor::execute_xcm(xcm_origin, msg).map_err(|_| Error::<T>::FailedToExecute)?;
		}

		#[weight = 1_000]
		fn send_plain_xcm(origin, dest: MultiLocation, message: Xcm) {
			ensure_signed(origin)?;
			Self::send_xcm(dest, message).map_err(|_| Error::<T>::FailedToSend)?;
		}

		#[weight = 1_000]
		fn send_upward_xcm(origin, message: VersionedXcm) {
			ensure_signed(origin)?;
			let data = message.encode();
			T::UpwardMessageSender::send_upward_message(data).map_err(|_| Error::<T>::FailedToSend)?;
		}

		#[weight = 1_000]
		fn send_hrmp_xcm(origin, recipient: ParaId, message: VersionedXcm) {
			ensure_signed(origin)?;
			let data = message.encode();
			let outbound_message = OutboundHrmpMessage {
				recipient,
				data,
			};
			T::HrmpMessageSender::send_hrmp_message(outbound_message).map_err(|_| Error::<T>::FailedToSend)?;
		}
	}
}

impl<T: Config> DownwardMessageHandler for Module<T> {
	fn handle_downward_message(msg: InboundDownwardMessage) {

		frame_support::debug::RuntimeLogger::init();

		frame_support::debug::info!("----------- xcm-handler: handle_downward_message -------------");
		frame_support::debug::info!(
			">>> inbound downward msg: {:?}",
			msg
		);
		let hash = msg.using_encoded(T::Hashing::hash);
		frame_support::debug::info!("Processing Downward XCM: {:?}", &hash);
		match VersionedXcm::decode(&mut &msg.msg[..]).map(Xcm::try_from) {
			Ok(Ok(xcm)) => {
				match T::XcmExecutor::execute_xcm(Junction::Parent.into(), xcm) {
					Ok(..) => RawEvent::Success(hash),
					Err(e) => RawEvent::Fail(hash, e),
				};
			}
			Ok(Err(..)) => Self::deposit_event(RawEvent::BadVersion(hash)),
			Err(..) => Self::deposit_event(RawEvent::BadFormat(hash)),
		}
	}
}

impl<T: Config> HrmpMessageHandler for Module<T> {
	fn handle_hrmp_message(sender: ParaId, msg: InboundHrmpMessage) {

		frame_support::debug::RuntimeLogger::init();

		frame_support::debug::info!("----------- xcm-handler: handle_hrmp_message -------------");
		frame_support::debug::info!(
			">>> paraId: {:?}, inbound hrmp msg: {:?}",
			sender,
			msg
		);
		let hash = msg.using_encoded(T::Hashing::hash);
		frame_support::debug::info!("Processing HRMP XCM: {:?}", &hash);
		match VersionedXcm::decode(&mut &msg.data[..]).map(Xcm::try_from) {
			Ok(Ok(xcm)) => {
				match T::XcmExecutor::execute_xcm(
					Junction::Parachain { id: sender.into() }.into(),
					xcm,
				) {
					Ok(..) => RawEvent::Success(hash),
					Err(e) => RawEvent::Fail(hash, e),
				};
			}
			Ok(Err(..)) => Self::deposit_event(RawEvent::BadVersion(hash)),
			Err(..) => Self::deposit_event(RawEvent::BadFormat(hash)),
		}
	}
}

impl<T: Config> SendXcm for Module<T> {
	fn send_xcm(dest: MultiLocation, msg: Xcm) -> Result<(), XcmError> {
		let msg: VersionedXcm = msg.into();
		frame_support::debug::info!("----------- xcm-handler: send_xcm -------------");
		frame_support::debug::info!(
			">>> dest: {:?}, dest.first(): {:?} msg: {:?}",
			dest,
			dest.first(),
			msg
		);
		match dest.first() {
			// An upward message for the relay chain.
			Some(Junction::Parent) if dest.len() == 1 => {
				frame_support::debug::info!("---------------------- Destionation is Parent, send upward message");

				let data = msg.encode();
				let hash = T::Hashing::hash(&data);

				T::UpwardMessageSender::send_upward_message(data)
					.map_err(|_| XcmError::Undefined)?;
				Self::deposit_event(RawEvent::UpwardMessageSent(hash));

				Ok(())
			}
			// An HRMP message for a sibling parachain.
			Some(Junction::Parachain { id }) => {
				frame_support::debug::info!("---------------- Destionation is Parachain, send hrmp message ");

				let data = msg.encode();
				let hash = T::Hashing::hash(&data);
				let message = OutboundHrmpMessage {
					recipient: (*id).into(),
					data,
				};
				// TODO: Better error here
				T::HrmpMessageSender::send_hrmp_message(message)
					.map_err(|_| XcmError::Undefined)?;
				Self::deposit_event(RawEvent::HrmpMessageSent(hash));
				Ok(())
			}

			_ => {
				frame_support::debug::info!("----------------- Unhandled xcm message");
				/* TODO: Handle other cases, like downward message */
				Err(XcmError::UnhandledXcmMessage)
			}
		}
	}
}

/// Origin for the parachains module.
#[derive(PartialEq, Eq, Clone, Encode, Decode)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum Origin {
	/// It comes from the (parent) relay chain.
	Relay,
	/// It comes from a (sibling) parachain.
	SiblingParachain(ParaId),
}

impl From<ParaId> for Origin {
	fn from(id: ParaId) -> Origin {
		Origin::SiblingParachain(id)
	}
}
impl From<u32> for Origin {
	fn from(id: u32) -> Origin {
		Origin::SiblingParachain(id.into())
	}
}
