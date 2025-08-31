export default {
	preset: "ts-jest/presets/default-esm",
	extensionsToTreatAsEsm: [".ts"],
	testEnvironment: "node",
	transform: {
		"^.+\\.ts$": [
			"ts-jest",
			{
				useESM: true,
				tsconfig: "./tsconfig.json",
			},
		],
	},
	testMatch: ["**/*.test.ts"],
	verbose: true,
};