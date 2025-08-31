import {
	createSolanaRpc,
	createKeyPairFromBytes,
	lamports,
	generateKeyPairSigner,
	getProgramDerivedAddress,
	getAddressFromPublicKey,
	sendAndConfirmTransactionFactory,
	createSolanaRpcSubscriptions,
	pipe,
	AccountRole,
	createTransactionMessage,
	setTransactionMessageLifetimeUsingBlockhash,
	appendTransactionMessageInstruction,
	signTransactionMessageWithSigners,
	setTransactionMessageFeePayerSigner,
	assertIsTransactionWithinSizeLimit,
	type SolanaRpcApi,
	type SolanaRpcSubscriptionsApi,
	type Address,
	type KeyPairSigner,
	type RpcSubscriptions,
	type Rpc,
	airdropFactory,
	fetchEncodedAccount,
	assertAccountExists,
	getStructCodec,
	getAddressCodec,
	getU64Codec,
	getU8Codec,
	none,
	type ProgramDerivedAddress,
} from "@solana/kit";
import {
	TOKEN_PROGRAM_ADDRESS,
	ASSOCIATED_TOKEN_PROGRAM_ADDRESS,
	findAssociatedTokenPda,
} from "@solana-program/token";
import { SYSTEM_PROGRAM_ADDRESS } from "@solana-program/system";
import {
	estimateComputeUnitLimitFactory,
	getSetComputeUnitLimitInstruction,
} from "@solana-program/compute-budget";

describe("Pinocchio Rating tests", () => {
	let rpc: Rpc<SolanaRpcApi>;
	let rpcSubscriptions: RpcSubscriptions<SolanaRpcSubscriptionsApi>;
	let programId: Address;
	let unitsPerRatingToken: number;
	let LAMPORTS_PER_SOL: number;
	let adminAuthority: KeyPairSigner;
	let user: KeyPairSigner;
	let ratingMint: KeyPairSigner;
	let adminPDA: Address;
	let adminPDABump: number;
	let adminATA: Address;
	let adminATABump: number;

	beforeAll(async () => {
		rpc = createSolanaRpc("http://127.0.0.1:8899");
		rpcSubscriptions = createSolanaRpcSubscriptions("ws://127.0.0.1:8900");
		console.log("RPC initialized");

		programId = await getAddressFromPublicKey(
			(
				await createKeyPairFromBytes(
					new Uint8Array([
						135, 184, 85, 32, 28, 131, 112, 139, 146, 132, 244, 241,
						2, 47, 94, 201, 66, 8, 218, 153, 206, 128, 249, 81, 31,
						179, 36, 21, 217, 78, 203, 64, 140, 255, 194, 33, 146,
						45, 72, 116, 122, 130, 229, 197, 8, 112, 69, 144, 218,
						130, 17, 87, 137, 101, 31, 81, 99, 98, 61, 84, 114, 77,
						100, 212,
					])
				)
			).publicKey
		);
		console.log("Program ID: ", programId);

		unitsPerRatingToken = 1_000_000_000;
		LAMPORTS_PER_SOL = 1_000_000_000;

		adminAuthority = await generateKeyPairSigner();
		console.log("Admin authority: ", adminAuthority.address);
		user = await generateKeyPairSigner();
		console.log("User: ", user.address);

		const airdrop = airdropFactory({ rpc, rpcSubscriptions });

		await airdrop({
			commitment: "confirmed",
			recipientAddress: adminAuthority.address,
			lamports: lamports(BigInt(10 * LAMPORTS_PER_SOL)),
		});

		await airdrop({
			commitment: "confirmed",
			recipientAddress: user.address,
			lamports: lamports(BigInt(10 * LAMPORTS_PER_SOL)),
		});

		console.log(
			"Airdropped to admin authority, balance: ",
			(await rpc.getBalance(adminAuthority.address).send()).value /
				BigInt(LAMPORTS_PER_SOL)
		);
		console.log(
			"Airdropped to user, balance: ",
			(await rpc.getBalance(user.address).send()).value /
				BigInt(LAMPORTS_PER_SOL)
		);

		ratingMint = await generateKeyPairSigner();
		console.log("Rating mint: ", ratingMint.address);
	});

	it("Init program admin", async () => {
		const ixDiscriminator = 0;
		const ratingReward = BigInt(10 * unitsPerRatingToken);
		const initAdminPayload = Buffer.alloc(9); // Discriminator + reward amount
		initAdminPayload.writeUInt8(ixDiscriminator, 0);
		initAdminPayload.writeBigUint64LE(ratingReward, 1);

		// derive required PDAs
		[adminPDA, adminPDABump] = await getProgramDerivedAddress({
			programAddress: programId,
			seeds: [Buffer.from("ratings_admin")],
		});
		console.log("Admin PDA: ", adminPDA);

		[adminATA, adminATABump] = await findAssociatedTokenPda({
			owner: adminPDA,
			tokenProgram: TOKEN_PROGRAM_ADDRESS,
			mint: ratingMint.address,
		});
		console.log("Admin ATA: ", adminATA);

		const initAdminIx = {
			programAddress: programId,
			accounts: [
				{
					address: adminAuthority.address,
					role: AccountRole.WRITABLE_SIGNER,
					signer: adminAuthority,
				},
				{ address: adminPDA, role: AccountRole.WRITABLE },
				{
					address: ratingMint.address,
					role: AccountRole.WRITABLE_SIGNER,
					signer: ratingMint,
				},
				{ address: adminATA, role: AccountRole.WRITABLE },
				{ address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
				{ address: TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
				{
					address: ASSOCIATED_TOKEN_PROGRAM_ADDRESS,
					role: AccountRole.READONLY,
				},
			],
			data: initAdminPayload,
		};

		let blockhash = (await rpc.getLatestBlockhash().send()).value;

		let transactiontx = pipe(
			createTransactionMessage({ version: 0 }),
			(tx) => setTransactionMessageFeePayerSigner(adminAuthority, tx),
			(tx) => setTransactionMessageLifetimeUsingBlockhash(blockhash, tx),
			(tx) => appendTransactionMessageInstruction(initAdminIx, tx)
		);

		const signedTx = await signTransactionMessageWithSigners(transactiontx);

		const sendAndConfirm = sendAndConfirmTransactionFactory({
			rpc,
			rpcSubscriptions,
		});

		assertIsTransactionWithinSizeLimit(signedTx);

		console.log("Sending transaction...");
		try {
			await sendAndConfirm(signedTx, {
				commitment: "confirmed",
			});
			console.log("✅ Transaction confirmed successfully");
		} catch (error: any) {
			console.error("Transaction failed with detailed error:");
			console.dir(error.context, { depth: 10 });

			throw error;
		}

		const sizeOfAdminState = 80; // size of admin state + padding

		const adminAccountInfo = await fetchEncodedAccount(rpc, adminPDA);
		assertAccountExists(adminAccountInfo);

		expect(adminAccountInfo.data.byteLength).toEqual(sizeOfAdminState);
		expect(adminAccountInfo.programAddress).toEqual(programId);

		const adminCodec = getStructCodec([
			["authority", getAddressCodec()],
			["tokenMint", getAddressCodec()],
			["rewardAmount", getU64Codec()],
			["bump", getU8Codec()],
		]);

		const adminState = adminCodec.decode(adminAccountInfo.data);
		console.log(
			"Admin state:",
			JSON.stringify(
				adminState,
				(key, value) =>
					typeof value === "bigint" ? value.toString() : value,
				2
			)
		);
	});

	it("Init rating", async () => {
		let ixDiscriminator = 1;
		let movieTitle = "Top Gun: Maverick";
		let rating = 8;
		// Calculate total size needed
		const discriminatorSize = 1; // u8
		const titleSize = movieTitle.length; // string length
		const ratingSize = 1; // u8
		const totalSize = discriminatorSize + titleSize + ratingSize;

		// Pre-allocate buffer
		let initRatingPayload = Buffer.alloc(totalSize);

		// Write values at correct offsets
		initRatingPayload.writeUInt8(ixDiscriminator, 0);
		// Write the title and capture how many bytes were actually written
		const titleBytesWritten = initRatingPayload.write(movieTitle, 1);

		// Then write the rating at the correct offset
		initRatingPayload.writeUInt8(rating, 1 + titleBytesWritten);

		let [ratingPDA, bump] = await getProgramDerivedAddress({
			programAddress: programId,
			seeds: [getAddressCodec().encode(user.address), Buffer.from(movieTitle, 'utf-8')],
		});
		console.log("Rating PDA: ", ratingPDA);

		let [userATA, userATABump] = await findAssociatedTokenPda({
			owner: user.address,
			tokenProgram: TOKEN_PROGRAM_ADDRESS,
			mint: ratingMint.address,
		});
		console.log("User ATA: ", userATA);
        
		const initRatingAccounts = [
			{
				address: user.address,
				role: AccountRole.WRITABLE_SIGNER,
				signer: user,
			},
			{ address: ratingPDA, role: AccountRole.WRITABLE },
			{ address: userATA, role: AccountRole.WRITABLE },
			{ address: adminPDA, role: AccountRole.READONLY },
			{ address: adminATA, role: AccountRole.WRITABLE },
			{ address: ratingMint.address, role: AccountRole.READONLY },
			{ address: SYSTEM_PROGRAM_ADDRESS, role: AccountRole.READONLY },
			{ address: TOKEN_PROGRAM_ADDRESS, role: AccountRole.READONLY },
			{
				address: ASSOCIATED_TOKEN_PROGRAM_ADDRESS,
				role: AccountRole.READONLY,
			},
		];

		const initRatingIx = {
			programAddress: programId,
			accounts: initRatingAccounts,
			data: initRatingPayload,
		};

		let recentBlockHash = (await rpc.getLatestBlockhash().send()).value;

		let computeLimitFactory = estimateComputeUnitLimitFactory({ rpc });

		let initRatingPipe = pipe(
			createTransactionMessage({ version: 0 }),
			(tx) => setTransactionMessageFeePayerSigner(user, tx),
			(tx) =>
				setTransactionMessageLifetimeUsingBlockhash(
					recentBlockHash,
					tx
				),
			(tx) => appendTransactionMessageInstruction(initRatingIx, tx)
		);

		let signedTx = await signTransactionMessageWithSigners(initRatingPipe);
		assertIsTransactionWithinSizeLimit(signedTx);

		let sendAndConfirmFactory = sendAndConfirmTransactionFactory({
			rpc,
			rpcSubscriptions,
		});

		try {
			await sendAndConfirmFactory(signedTx, { commitment: "confirmed" });
			console.log("Transaction successful");
		} catch (error: any) {
			console.log(`Transaction failed with error: ${JSON.stringify(error.context, (key, value) =>
                typeof value === "bigint" ? value.toString() : value,
            2)}`);
		}
	});
});
