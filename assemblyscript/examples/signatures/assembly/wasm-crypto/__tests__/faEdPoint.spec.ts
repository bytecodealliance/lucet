describe("Ed25519 arithmetic", (): void => {
    it("should perform operations on Ed25519", (): void => {
        let scalar = new Uint8Array(32);
        for (let i = 0; i < 32; i++) {
            scalar[i] = i;
        }
        let p = faEdBasePointMult(scalar);
        expect<bool>(faEdPointValidate(p)).toBeTruthy();
        p[0]++;
        expect<bool>(faEdPointValidate(p)).toBeFalsy();
        p[0]--;

        let scalar2 = new Uint8Array(32);
        for (let i = 0; i < 32; i++) {
            scalar2[i] = ~i;
        }
        let p2 = faEdBasePointMult(scalar);
        expect<bool>(faEdPointValidate(p2)).toBeTruthy();
        p2[0]++;
        expect<bool>(faEdPointValidate(p2)).toBeFalsy();
        p2[0]--;

        let p3 = faEdPointAdd(p, p);
        p3 = faEdPointAdd(p3, p3);

        let scalar3 = new Uint8Array(32);
        scalar3[0] = 4;
        let p4 = faEdPointMult(scalar3, p);
        expect<bool>(equals(p3, p4)).toBeTruthy();

        p3 = faEdPointSub(p3, p);
        p3 = faEdPointSub(p3, p);
        p3 = faEdPointSub(p3, p);
        expect<bool>(equals(p3, p)).toBeTruthy();

        let zero = faEdPointSub(p3, p);
        expect<bool>(faEdPointValidate(zero)).toBeFalsy();
    });

    it("should perform operations on Ed25519 (with clamping)", (): void => {
        let scalar = new Uint8Array(32);
        for (let i = 0; i < 32; i++) {
            scalar[i] = i;
        }
        let p = faEdBasePointMultClamp(scalar);
        expect<bool>(faEdPointValidate(p)).toBeTruthy();
        p[0]++;
        expect<bool>(faEdPointValidate(p)).toBeTruthy();
        p[0]--;

        let scalar2 = new Uint8Array(32);
        for (let i = 0; i < 32; i++) {
            scalar2[i] = ~i;
        }
        let p2 = faEdBasePointMultClamp(scalar);
        expect<bool>(faEdPointValidate(p2)).toBeTruthy();
        p2[0]++;
        expect<bool>(faEdPointValidate(p2)).toBeTruthy();
        p2[0]--;

        let p3 = faEdPointAdd(p, p);
        p3 = faEdPointAdd(p3, p3);

        let scalar3 = new Uint8Array(32);
        scalar3[0] = 4;
        let p4 = faEdPointMultClamp(scalar3, p);
        expect<bool>(equals(p3, p4)).toBeFalsy();

        p3 = faEdPointSub(p3, p);
        p3 = faEdPointSub(p3, p);
        p3 = faEdPointSub(p3, p);
        expect<bool>(equals(p3, p)).toBeTruthy();

        let zero = faEdPointSub(p3, p);
        expect<bool>(faEdPointValidate(zero)).toBeFalsy();
    });
});