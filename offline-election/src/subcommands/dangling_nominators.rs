use crate::subcommands::staking::slashing_span_of;
use crate::{primitives::AccountId, storage, Client, Opt, LOG_TARGET};
use pallet_staking::Nominations;

/// Main run function of the sub-command.
pub async fn run(client: &Client, opt: Opt) {
	let at = opt.at.unwrap();
	let nominators: Vec<(AccountId, Nominations<AccountId>)> =
		storage::enumerate_map::<AccountId, Nominations<AccountId>>(
			b"Staking",
			b"Nominators",
			client,
			at,
		)
		.await
		.expect("Staking::nominators should be enumerable");

	let count = nominators.len();
	let mut ok = 0;
	let mut nok = 0;
	for (idx, (who, n)) in nominators.into_iter().enumerate() {
		// retain only targets who have not been yet slashed recently. This is highly dependent
		// on the staking implementation.
		let submitted_in = n.submitted_in;
		let targets = n.targets;
		let mut filtered_targets = vec![];
		// TODO: move back to closures and retain, but async-std::block_on can't work well here for
		// whatever reason. Or move to streams?
		for target in targets.iter() {
			let maybe_slashing_spans = slashing_span_of(&target, client, at).await;
			if maybe_slashing_spans.map_or(true, |spans| submitted_in >= spans.last_nonzero_slash())
			{
				filtered_targets.push(target.clone());
			}
		}

		if filtered_targets.len() == targets.len() {
			log::debug!(
				target: LOG_TARGET,
				"[{}/{}] Nominator {:?} Ok. Retaining all {} votes.",
				idx,
				count,
				who,
				targets.len()
			);
			ok += 1;
		} else {
			log::warn!(
				target: LOG_TARGET,
				"[{}/{}] Retaining {}/{} of votes for {:?}.",
				idx,
				count,
				filtered_targets.len(),
				targets.len(),
				who
			);
			nok += 1;
		}
	}

	log::info!(
		target: LOG_TARGET,
		"✅ {} nominators have effective votes.",
		ok
	);
	log::info!(
		target: LOG_TARGET,
		"❌ {} nominators have dangling votes.",
		nok
	);
}
