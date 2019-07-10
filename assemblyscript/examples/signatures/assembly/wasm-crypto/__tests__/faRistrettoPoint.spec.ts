describe("Ristretto arithmetic", (): void => {
    it("should perform operations on Ristretto", (): void => {
        let uniform = new Uint8Array(64);
        for (let i = 0; i < 64; i++) {
            uniform[i] = i;
        }
        let p = faPointFromHash(uniform);
        expect<bool>(faPointValidate(p)).toBeTruthy();
        p[0]++;
        expect<bool>(faPointValidate(p)).toBeFalsy();
        p[0]--;

        let uniform2 = new Uint8Array(64);
        for (let i = 0; i < 64; i++) {
            uniform2[i] = ~i;
        }
        let p2 = faPointFromHash(uniform2);
        expect<bool>(faPointValidate(p2)).toBeTruthy();
        p2[0]++;
        expect<bool>(faPointValidate(p2)).toBeFalsy();
        p2[0]--;

        let p3 = faPointAdd(p, p);
        p3 = faPointAdd(p3, p3);

        let scalar3 = new Uint8Array(32);
        scalar3[0] = 4;
        let p4 = faPointMult(scalar3, p);
        expect<bool>(equals(p3, p4)).toBeTruthy();

        p3 = faPointSub(p3, p);
        p3 = faPointSub(p3, p);
        p3 = faPointSub(p3, p);
        expect<bool>(equals(p3, p)).toBeTruthy();

        let zero = faPointSub(p3, p);
        expect<bool>(faPointValidate(zero)).toBeFalsy();
    });
});
