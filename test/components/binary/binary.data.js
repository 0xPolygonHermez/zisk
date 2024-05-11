module.exports = { ok: [
    /////////
    // ADD
    /////////

    // w=0
    {
        a: "0FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "0FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "1FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFE",
        carry: 0,
        operation: "0",
        type: 1,
    },

    // w=1
    {
        a: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFE",
        carry: 1,
        operation: "0",
        type: 1,
    },
    // w=2
    {
        a: "b01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        b: "b486e735789b55a76376c3478ae4bc588d0740184aa0873dd0386392daed8db5",
        c: "649b4c45bb034df66329b0c327023b1eec4d927d75c3ef2525820401441a42f4",
        carry: 1,
        operation: "0",
        type: 1,
    },
    /////////
    // SUB
    /////////
    // w=3
    {
        a: "2",
        b: "1",
        c: "1",
        carry: 0,
        operation: "1",
        type: 1,
    },
    // w=4
    {
        a: "0",
        b: "1",
        c: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        carry: 1,
        operation: "1",
        type: 1,
    },
    // w=5
    {
        a: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "0",
        carry: 0,
        operation: "1",
        type: 1,
    },
    // w=6
    {
        a: "b486e735789b55a76376c3478ae4bc588d0740184aa0873dd0386392daed8db5",
        b: "a01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        c: "1472822536335d5863c3d5cbeec73d922dc0edb31f7d1f567aeec32471c0d876",
        carry: 0,
        operation: "1",
        type: 1,
    },
    // w=7
    {
        a: "a01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        b: "b486e735789b55a76376c3478ae4bc588d0740184aa0873dd0386392daed8db5",
        c: "eb8d7ddac9cca2a79c3c2a341138c26dd23f124ce082e0a985113cdb8e3f278a",
        carry: 1,
        operation: "1",
        type: 1,
    },

    /////////
    // LT
    /////////
    // w=8
    {
        a: "0",
        b: "1",
        c: "1",
        carry: 1,
        operation: "2",
        type: 1,
    },
    // w=9
    {
        a: "1",
        b: "0",
        c: "0",
        carry: 0,
        operation: "2",
        type: 1,
    },
    // w=10
    {
        a: "0",
        b: "0",
        c: "0",
        carry: 0,
        operation: "2",
        type: 1,
    },
    // w=11
    {
        a: "b01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        b: "0",
        c: "0",
        carry: 0,
        operation: "2",
        type: 1,
    },
    // w=12
    {
        a: "b01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        b: "b01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        c: "0",
        carry: 0,
        operation: "2",
        type: 1,
    },
    // w=13
    {
        a: "a01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        b: "b01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        c: "1",
        carry: 1,
        operation: "2",
        type: 1,
    },
    // w=14
    {
        a: "b01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        b: "a01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        c: "0",
        carry: 0,
        operation: "2",
        type: 1,
    },
    // w=15
    {
        a: "FFFF",
        b: "00FF",
        c: "0",
        carry: 0,
        operation: "2",
        type: 1,
    },
    /////////
    // SLT
    /////////
    // w=16
    {
        a: "8000000000000000000000000000000000000000000000000000000000000000",
        b: "0000000000000000000000000000000000000000000000000000000000000000",
        c: "1",
        carry: 1,
        operation: "3",
        type: 1,
    },

    // w=17
    {
        a: "0",
        b: "0",
        c: "0",
        carry: 0,
        operation: "3",
        type: 1,
    },
    // w=18
    {
        a: "1",
        b: "0",
        c: "0",
        carry: 0,
        operation: "3",
        type: 1,
    },
    // w=19
    {
        a: "0",
        b: "1",
        c: "1",
        carry: 1,
        operation: "3",
        type: 1,
    },
    // w=20
    {
        a: "FF00FF",
        b: "00FF00",
        c: "0",
        carry: 0,
        operation: "3",
        type: 1,
    },
    // w=21
    {
        a: "00FF00",
        b: "FF00FF",
        c: "1",
        carry: 1,
        operation: "3",
        type: 1,
    },
    // w=22
    {
        a: "FFEEDDCCBBAA",
        b: "FFEEDDCCBBAA",
        c: "0",
        carry: 0,
        operation: "3",
        type: 1,
    },
    // w=23
    {
        a: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "0",
        c: "1",
        carry: 1,
        operation: "3",
        type: 1,
    },
    // w=24
    {
        a: "0",
        b: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "0",
        carry: 0,
        operation: "3",
        type: 1,
    },
    // w=25
    {
        a: "FF00FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "FEFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "0",
        carry: 0,
        operation: "3",
        type: 1,
    },
    // w=26
    {
        a: "800FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "0",
        carry: 0,
        operation: "3",
        type: 1,
    },
    // w=27
    {
        a: "FF00FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "1",
        carry: 1,
        operation: "3",
        type: 1,
    },
    // w=28
    {
        a: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "8000FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "0",
        carry: 0,
        operation: "3",
        type: 1,
    },
    // w=29
    {
        a: "80FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "FF00FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "1",
        carry: 1,
        operation: "3",
        type: 1,
    },
    // w=30
    {
        a: "FF00FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "1",
        carry: 1,
        operation: "3",
        type: 1,
    },

    // w=31
    {
        a: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "FF00FFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFFFFFFFFFFF000000",
        c: "0",
        carry: 0,
        operation: "3",
        type: 1,
    },
    // w=32
    {
        a: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFFFFFFFFFFF000000",
        b: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF000000",
        c: "1",
        carry: 1,
        operation: "3",
        type: 1,
    },
    /////////
    // EQ
    /////////
    // w=33
    {
        a: "3e9",
        b: "3e9",
        c: "1",
        carry: 1,
        operation: "4",
        type: 1,
    },
    // w=34
    {
        a: "b01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        b: "0",
        c: "0",
        carry: 0,
        operation: "4",
        type: 1,
    },
    // w=35
    {
        a: "b01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        b: "b01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        c: "1",
        carry: 1,
        operation: "4",
        type: 1,
    },
    // w=36
    {
        a: "3f",
        b: "3f",
        c: "1",
        carry: 1,
        operation: "4",
        type: 1,
    },
    // w=37
    {
        a: "FF00",
        b: "FF00",
        c: "1",
        carry: 1,
        operation: "4",
        type: 1,
    },
    // w=38
    {
        a: "FF00",
        b: "00FF",
        c: "0",
        carry: 0,
        operation: "4",
        type: 1,
    },
    // w=39
    {
        a: "FF00",
        b: "FFF00",
        c: "00",
        carry: 0,
        operation: "4",
        type: 1,
    },

    /////////
    // AND
    /////////
    // w=40
    {
        a: "0F01",
        b: "0F01",
        c: "0F01",
        carry: 1,
        operation: "5",
        type: 1,
    },
    // w=41
    {
        a: "0E0E",
        b: "0101",
        c: "0000",
        carry: 0,
        operation: "5",
        type: 1,
    },
    // w=42
    {
        a: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        carry: 1,
        operation: "5",
        type: 1,
    },
    // w=43
    {
        a: "0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F",
        b: "0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F",
        c: "0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F",
        carry: 1,
        operation: "5",
        type: 1,
    },
    /////////
    // OR
    /////////
    // w=44
    {
        a: "b01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        b: "b486e735789b55a76376c3478ae4bc588d0740184aa0873dd0386392daed8db5",
        c: "b496e7357afffdeffff6ef7f9efdfededf47527d6ba3e7ffd579e3fefbedbdbf",
        carry: 0,
        operation: "6",
        type: 1,
    },

    /////////
    // XOR
    /////////
    // w=45
    {
        a: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "0",
        carry: 0,
        operation: "7",
        type: 1,
    },
    // w=46
    {
        a: "0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F",
        b: "F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0",
        c: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        carry: 0,
        operation: "7",
        type: 1,
    },
    // w=47
    {
        a: "b01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        b: "b486e735789b55a76376c3478ae4bc588d0740184aa0873dd0386392daed8db5",
        c: "49282253afcade99cc42e3c16f9c29ed241127d6183e0da8571c3fcb3c1388a",
        carry: 0,
        operation: "7",
        type: 1,
    },
    /////////
    // LT4
    /////////
    // w=48 (+0)
    {
        a: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "FFFFFFFF00000001FFFFFFFF00000001FFFFFFFF00000001FFFFFFFF00000001",
        c: "0",
        carry: 0,
        operation: "8",
        type: 1,
    },
    // w=49 (+1)
    {
        a: "FFFFFFFF00000000FFFFFFFF00000000FFFFFFFF00000000FFFFFFFF00000000",
        b: "FFFFFFFF00000001FFFFFFFF00000001FFFFFFFF00000001FFFFFFFF00000001",
        c: "1",
        carry: 1,
        operation: "8",
        type: 1,
    },
    // w=50 (+2)
    {
        a: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "000000000000000000000000000000000000000000000000FFFFFFFF00000001",
        c: "0",
        carry: 0,
        operation: "8",
        type: 1,
    },
    // w=51 (+3)
    {
        a: "FFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFF",
        b: "FFFEFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "1",
        carry: 1,
        operation: "8",
        type: 1,
    },
    // w=52 (+4)
    {
        a: "FFEFFFFFFFFFFFFF000000000000000000000000000000000000000000000000",
        b: "FFFEFFFFFFFFFFFF000000000000000000000000000000000000000000000000",
        c: "0",
        carry: 0,
        operation: "8",
        type: 1,
    },
    // w=53 (+5)
    {
        a: "FFEFFFFFFFFFFFFF00000000000000000000000000000000FFEFFFFFFFFFFFFF",
        b: "FFFEFFFFFFFFFFFF00000000000000000000000000000000FFFEFFFFFFFFFFFF",
        c: "0",
        carry: 0,
        operation: "8",
        type: 1,
    },
    // w=54 (+6)
    {
        a: "FFEFFFFFFFFFFFFF0000000000000000FFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFF",
        b: "FFFEFFFFFFFFFFFF0000000000000000FFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFF",
        c: "0",
        carry: 0,
        operation: "8",
        type: 1,
    },
    // w=55 (+7)
    {
        a: "FFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFF0000000000000000FFEFFFFFFFFFFFFF",
        b: "FFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFF0000000000000000FFFEFFFFFFFFFFFF",
        c: "0",
        carry: 0,
        operation: "8",
        type: 1,
    },
    // w=56 (+8)
    {
        a: "FFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFF0000000000000000",
        b: "FFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFF0000000000000000",
        c: "0",
        carry: 0,
        operation: "8",
        type: 1,
    },
    // w=57 (+9)
    {
        a: "0000000000000000FFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFF",
        b: "0000000000000000FFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFF",
        c: "0",
        carry: 0,
        operation: "8",
        type: 1,
    },
    // w=58 (+10)
    {
        a: "FFFFFFFF00000000FFFFFFFF00000000FFFFFFFF00000000FFFFFFFF00000000",
        b: "FFFFFFFF00000001FFFFFFFF00000001FFFFFFFF00000001FFFFFFFF00000001",
        c: "1",
        carry: 1,
        operation: "8",
        type: 2,
    }
],
error: [
    // w=0 Binary.w=[0..15]
    {
        a: "0FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "0FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "0FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFE",
        carry: 0n,
        operation: "0", // ADD
        type: 1,
    },
    // w=1 Binary.w=[16..31]
    {
        a: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "1",
        carry: 0n,
        operation: "1", // SUB
        type: 1,
    },
    // w=2 Binary.w=[32..47]
    {
        a: "0",
        b: "1",
        c: "0",
        carry: 1n,
        operation: "2", // LT
        type: 1,
    },
    // w=3 Binary.w=[48..63]
    {
        a: "1",
        b: "0",
        c: "1",
        carry: 0n,
        operation: "2", // LT
        type: 1,
    },
    // w=4 Binary.w=[64..79]
    {
        a: "8000000000000000000000000000000000000000000000000000000000000000",
        b: "0000000000000000000000000000000000000000000000000000000000000000",
        c: "0",
        carry: 0n,
        operation: "3", // SLT
        type: 1,
    },
    // w=5 Binary.w=[80..95]
    {
        a: "0000000000000000000000000000000000000000000000000000000000000000",
        b: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "1",
        carry: 0n,
        operation: "3", // SLT
        type: 1,
    },
    // w=6 Binary.w=[96..111]
    {
        a: "FF00",
        b: "FF00",
        c: "0",
        carry: 1n,
        operation: "4", // EQ
        type: 1,
    },
    // w=7 Binary.w=[112..127]
    {
        a: "FF00",
        b: "00FF",
        c: "1",
        carry: 0n,
        operation: "4", // EQ
        type: 1,
    },
    // w=8 Binary.w=[128..143]
    {
        a: "FF00",
        b: "FFF00",
        c: "100",
        carry: 0n,
        operation: "4", // EQ
        type: 1,
    },
    // w=9 Binary.w=[144,159]
    {
        a: "0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F",
        b: "0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F",
        c: "0E0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F",
        carry: 1n,
        operation: "5", // AND
        type: 1,
    },
    // w=10 Binary.w=[160..175]
    {
        a: "b01465104267f84effb2ed7b9c1d7ec65f4652652b2367e75549a06e692cb53f",
        b: "b486e735789b55a76376c3478ae4bc588d0740184aa0873dd0386392daed8db5",
        c: "a496e7357afffdeffff6ef7f9efdfededf47527d6ba3e7ffd579e3fefbedbdbf",
        carry: 0n,
        operation: "6", // OR
        type: 1,
    },
    // w=11 Binary.w=[176..191]
    {
        a: "0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F",
        b: "F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0F0",
        c: "EFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        carry: 0n,
        operation: "7",  // XOR
        type: 1,
    },
    // w=12 Binary.w=[192..207]
    {
        a: "00FF",
        b: "FF00",
        c: "1000000000000000000000000000000000000000000000000000000000000001",
        carry: 1n,
        operation: "2",  // LT
        type: 1,
    },
    // w=13 Binary.w=[208..223]
    {
        a: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "FFFFFFFF00000001FFFFFFFF00000001FFFFFFFF00000001FFFFFFFF00000001",
        c: "0",
        carry: 1,
        operation: "8", // LT4
        type: 1,
    },
    // w=14 Binary.w=[224..239]
    {
        a: "FFFFFFFF00000000FFFFFFFF00000000FFFFFFFF00000000FFFFFFFF00000000",
        b: "FFFFFFFF00000001FFFFFFFF00000001FFFFFFFF00000001FFFFFFFF00000001",
        c: "1000000000000000000000000000000000000000000000000000000000000001",
        carry: 1,
        operation: "8", // LT4
        type: 1,
    },
    // w=15 Binary.w=[240..255]
    {
        a: "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        b: "000000000000000000000000000000000000000000000000FFFFFFFF00000001",
        c: "1",
        carry: 0,
        operation: "8",
        type: 1,
    },
    // w=16 Binary.w=[256..271]
    {
        a: "FFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFF",
        b: "FFFEFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "0",
        carry: 1,
        operation: "8",
        type: 1,
    },
    // w=17 Binary.w=[272..287]
    {
        a: "FFEFFFFFFFFFFFFF000000000000000000000000000000000000000000000000",
        b: "FFFEFFFFFFFFFFFF000000000000000000000000000000000000000000000000",
        c: "0000000000000000000000000000000000000000000000000000000000000001",
        carry: 0,
        operation: "8",
        type: 1,
    },
    // w=18 Binary.w=[288..303]
    {
        a: "FFEFFFFFFFFFFFFF00000000000000000000000000000000FFEFFFFFFFFFFFFF",
        b: "FFFEFFFFFFFFFFFF00000000000000000000000000000000FFFEFFFFFFFFFFFF",
        c: "1",
        carry: 1,
        operation: "8",
        type: 1,
    },
    // w=19 Binary.w=[304..320]
    {
        a: "FFEFFFFFFFFFFFFF0000000000000000FFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFF",
        b: "FFFEFFFFFFFFFFFF0000000000000000FFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFF",
        c: "1",
        carry: 0,
        operation: "8",
        type: 1,
    },
    // w=20
    {
        a: "FFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFF0000000000000000FFEFFFFFFFFFFFFF",
        b: "FFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFF0000000000000000FFFEFFFFFFFFFFFF",
        c: "1",
        carry: 0,
        operation: "8",
        type: 1,
    },
    // w=21
    {
        a: "FFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFF0000000000000000",
        b: "FFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFF0000000000000000",
        c: "1",
        carry: 1,
        operation: "8",
        type: 1,
    },
    // w=22
    {
        a: "0000000000000000FFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFF",
        b: "0000000000000000FFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFF",
        c: "1",
        carry: 1,
        operation: "8",
        type: 1,
    },
    // w=23
    {
        a: "FFFFFFFF00000000FFFFFFFF00000000FFFFFFFF00000000FFFFFFFF00000000",
        b: "FFFFFFFF00000001FFFFFFFF00000001FFFFFFFF00000001FFFFFFFF00000001",
        c: "1000000000000000000000000000000000000000000000000000000000000001",
        carry: 1,
        operation: "8",
        type: 2,
    },
    // w=24 Binary.w=[256..271]
    {
        a: "FFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFF",
        b: "FFFEFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF",
        c: "1000000000000000000000000000000000000000000000000000000000000000",
        carry: 1,
        operation: "8",
        type: 1,
    },
    // w=25
    {
        a: "FFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFF0000000000000000",
        b: "FFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFFFFFEFFFFFFFFFFFF0000000000000000",
        c: "1000000000000000000000000000000000000000000000000000000000000000",
        carry: 1,
        operation: "8",
        type: 1,
    },
]
};