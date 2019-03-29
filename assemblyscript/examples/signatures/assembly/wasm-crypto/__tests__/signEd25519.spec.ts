describe("Ed25529 signature", (): void => {
    it("should sign and verify using Ed25519", (): void => {
        let seed = new Uint8Array(32);
        for (let i = 0; i < 32; i++) {
            seed[i] = ~i;
        }
        let kp = signEdKeypairFromSeed(seed);
        let msg = new Uint8Array(42);
        for (let i = 0; i < 32; i++) {
            msg[i] = i;
        }
        let signature = signEd(msg, kp);
        expect<Uint8Array>(signature).toHaveLength(SIGN_ED_BYTES);

        let pk = signEdPublicKey(kp);
        expect<Uint8Array>(pk).toHaveLength(SIGN_ED_PUBLICKEYBYTES);

        let verified = signEdVerify(signature, msg, pk);
        expect<bool>(verified).toBeTruthy();

        let noise = new Uint8Array(32);
        for (let i = 0; i < 32; i++) {
            noise[i] = i;
        }
        let signature2 = signEd(msg, kp, noise);
        expect<bool>(equals(signature, signature2)).toBeFalsy();

        let verified2 = signEdVerify(signature2, msg, pk);
        expect<bool>(verified2).toBeTruthy();

        pk[0]++;
        let verified3 = signEdVerify(signature2, msg, pk);
        expect<bool>(verified3).toBeFalsy();

        pk[0]--;
        msg[0]++;
        let verified4 = signEdVerify(signature2, msg, pk);
        expect<bool>(verified4).toBeFalsy();
    });
});
