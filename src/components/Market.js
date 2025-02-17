import React, { useEffect, useState } from 'react';
import { loadEverything, loadMarket } from '../state/views';
import { contractId, marketId, GAS } from '../state/near';
import {Frame} from './Frame';

const NUM_PER_PAGE = 4

export const Market = ({ dispatch, views, account }) => {

	const [page, setPage] = useState(0)

	const { market } = views;
	
	useEffect(() => {
		dispatch(loadMarket());
	}, []);

	const handleOffer = async (item) => {
		if (!account) {
			return alert('Must sign in with NEAR Wallet to purchase');
		}
		const { token_id } = item;
		await account.functionCall(marketId, 'offer', {
			nft_contract_id: contractId,
			token_id,
		}, GAS, item.sales[0].conditions.near);
	};

	return <>
		<div className="gallery">
			<Frame {...{ dispatch, items: market, handleOffer }} />
		</div>
	</>;
};

