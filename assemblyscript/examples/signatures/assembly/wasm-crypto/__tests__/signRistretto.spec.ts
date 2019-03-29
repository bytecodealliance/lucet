describe("Ristretto signature", (): void => {
    it("should sign and verify using Ristretto", (): void => {
        let seed = new Uint8Array(32);
        for (let i = 0; i < 32; i++) {
            seed[i] = ~i;
        }
        let kp = signKeypairFromSeed(seed);
        let msg = new Uint8Array(42);
        for (let i = 0; i < 32; i++) {
            msg[i] = i;
        }
        let signature = sign(msg, kp);
        expect<Uint8Array>(signature).toHaveLength(SIGN_BYTES);

        let pk = signPublicKey(kp);
        expect<Uint8Array>(pk).toHaveLength(SIGN_PUBLICKEYBYTES);

        let verified = signVerify(signature, msg, pk);
        expect<bool>(verified).toBeTruthy();

        let noise = new Uint8Array(32);
        for (let i = 0; i < 32; i++) {
            noise[i] = i;
        }
        let signature2 = sign(msg, kp, noise);
        expect<bool>(equals(signature, signature2)).toBeFalsy();

        let verified2 = signVerify(signature2, msg, pk);
        expect<bool>(verified2).toBeTruthy();

        pk[0]++;
        let verified3 = signVerify(signature2, msg, pk);
        expect<bool>(verified3).toBeFalsy();

        pk[0]--;
        msg[0]++;
        let verified4 = signVerify(signature2, msg, pk);
        expect<bool>(verified4).toBeFalsy();
    });
});
