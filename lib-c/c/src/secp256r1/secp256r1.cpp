
#include <gmpxx.h>
#include "secp256r1.hpp"
#include "../ffiasm/psecp256r1.hpp"
#include "../ffiasm/nsecp256r1.hpp"
#include "../common/utils.hpp"
#include "../common/globals.hpp"
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

int inline secp256r1_add_point_ec_fe (bool dbl, const RawpSecp256r1::Element &x1, const RawpSecp256r1::Element &y1, const RawpSecp256r1::Element &x2, const RawpSecp256r1::Element &y2, RawpSecp256r1::Element &x3, RawpSecp256r1::Element &y3)
{
    // Check if results are buffered
#ifdef ENABLE_EXPERIMENTAL_CODE
    if(ctx.ecRecoverPrecalcBuffer.filled == true){
        if(ctx.ecRecoverPrecalcBuffer.pos < 2){
            zklog.error("ecRecoverPrecalcBuffer.buffer buffer is not filled, but pos < 2 (pos=" + to_string(ctx.ecRecoverPrecalcBuffer.pos) + ")");
            exitProcess();
        }
        x3 = ctx.ecRecoverPrecalcBuffer.buffer[ctx.ecRecoverPrecalcBuffer.pos-2];
        y3 = ctx.ecRecoverPrecalcBuffer.buffer[ctx.ecRecoverPrecalcBuffer.pos-1];
        return ZKR_SUCCESS;
    }
#endif

    RawpSecp256r1::Element aux1, aux2, s;

    if (dbl)
    {
        // s = (3*x1*x1 + (p-3))/2*y1 = 3*(x1^2 - 1)/2*y1
        secp256r1.mul(aux1, x1, x1);
        secp256r1.fromUI(aux2, 3);
        secp256r1.add(aux1, aux1, secp256r1.negOne());
        secp256r1.mul(aux1, aux1, aux2);
        secp256r1.add(aux2, y1, y1);
        if (secp256r1.isZero(aux2))
        {
            printf("secp256r1_add_point_ec_fe() got denominator=0 1\n");
            return -1;
        }
        secp256r1.div(s, aux1, aux2);

        // Required for x3 calculation
        secp256r1.add(aux2, x1, x1);
    }
    else
    {
        // s = (y2-y1)/(x2-x1)
        secp256r1.sub(aux1, y2, y1);
        secp256r1.sub(aux2, x2, x1);
        if (secp256r1.isZero(aux2))
        {
            printf("secp256r1_add_point_ec_fe() got denominator=0 2\n");
            return -1;
        }
        secp256r1.div(s, aux1, aux2);

        // Required for x3 calculation
        secp256r1.add(aux2, x1, x2);
    }

    // x3 = s*s - (x1+x2)
    secp256r1.mul(aux1, s, s);
    // aux2 was calculated before
    secp256r1.sub(x3, aux1, aux2);

    // y3 = s*(x1-x3) - y1
    secp256r1.sub(aux1, x1, x3);;
    secp256r1.mul(aux1, aux1, s);
    secp256r1.sub(y3, aux1, y1);

    return 0;
}

int inline secp256r1_add_point_ec_dbl_fe (RawpSecp256r1::Element &x1, RawpSecp256r1::Element &y1)
{
    // Check if results are buffered
#ifdef ENABLE_EXPERIMENTAL_CODE
    if(ctx.ecRecoverPrecalcBuffer.filled == true){
        if(ctx.ecRecoverPrecalcBuffer.pos < 2){
            zklog.error("ecRecoverPrecalcBuffer.buffer buffer is not filled, but pos < 2 (pos=" + to_string(ctx.ecRecoverPrecalcBuffer.pos) + ")");
            exitProcess();
        }
        x3 = ctx.ecRecoverPrecalcBuffer.buffer[ctx.ecRecoverPrecalcBuffer.pos-2];
        y3 = ctx.ecRecoverPrecalcBuffer.buffer[ctx.ecRecoverPrecalcBuffer.pos-1];
        return ZKR_SUCCESS;
    }
#endif

    RawpSecp256r1::Element aux1, aux2, aux3, s;

    // s = 3*x1*x1/2*y1
    secp256r1.mul(aux1, x1, x1);
    secp256r1.fromUI(aux2, 3);
    secp256r1.mul(aux1, aux1, aux2);
    secp256r1.add(aux2, y1, y1);
    if (secp256r1.isZero(aux2))
    {
        printf("secp256r1_add_point_ec_dbl_fe() got denominator=0 1\n");
        return -1;
    }
    secp256r1.div(s, aux1, aux2);

    // Required for x3 calculation
    secp256r1.add(aux2, x1, x1);

    // x3 = s*s - (x1+x2)
    secp256r1.mul(aux1, s, s);
    // aux2 was calculated before
    
    secp256r1.sub(aux3, aux1, aux2);

    // y3 = s*(x1-x3) - y1
    secp256r1.sub(aux1, x1, aux3);
    x1 = aux3;
    secp256r1.mul(aux1, aux1, s);
    secp256r1.sub(y1, aux1, y1);

    return 0;
}

int secp256r1_add_point_ec (uint64_t _dbl, const uint64_t * _x1, const uint64_t * _y1, const uint64_t * _x2, const uint64_t * _y2, uint64_t * _x3, uint64_t * _y3)
{
    bool dbl = _dbl;

    RawpSecp256r1::Element x1, y1, x2, y2, x3, y3;
    array2fe(_x1, x1);
    array2fe(_y1, y1);
    if (!dbl)
    {
        array2fe(_x2, x2);
        array2fe(_y2, y2);
    }

    int result = secp256r1_add_point_ec_fe (dbl, x1, y1, x2, y2, x3, y3);
    
    fe2array(x3, _x3);
    fe2array(y3, _y3);

    return result;
}

int secp256r1_add_point_ec_dbl (uint64_t * _x1, uint64_t * _y1)
{
    RawpSecp256r1::Element x1, y1;
    array2fe(_x1, x1);
    array2fe(_y1, y1);

    int result = secp256r1_add_point_ec_dbl_fe (x1, y1);
    
    fe2array(x1, _x1);
    fe2array(y1, _y1);

    return result;
}

int secp256r1_add_point_ecp (uint64_t _dbl, const uint64_t * p1, const uint64_t * p2, uint64_t * p3)
{
    bool dbl = _dbl;

    RawpSecp256r1::Element x1, y1, x2, y2, x3, y3;
    array2fe(p1, x1);
    array2fe(p1 + 4, y1);
    if (!dbl)
    {
        array2fe(p2, x2);
        array2fe(p2 + 4, y2);
    }

    // printf("secp256r1_add_point_ecp() x1=%s\n", secp256r1.toString(x1, 16).c_str());
    // printf("secp256r1_add_point_ecp() y1=%s\n", secp256r1.toString(y1, 16).c_str());
    // printf("secp256r1_add_point_ecp() x2=%s\n", secp256r1.toString(x2, 16).c_str());
    // printf("secp256r1_add_point_ecp() y2=%s\n", secp256r1.toString(y2, 16).c_str());

    int result = secp256r1_add_point_ec_fe (dbl, x1, y1, x2, y2, x3, y3);

    fe2array(x3, p3);
    fe2array(y3, p3 + 4);

    return result;
}

uint64_t SECP256R1_G[8] = {
    0xF4A13945D898C296,
    0x77037D812DEB33A0,
    0xF8BCE6E563A440F2,
    0x6B17D1F2E12C4247,
    0xCBB6406837BF51F5,
    0x2BCE33576B315ECE,
    0x8EE7EB4A7C0F9E16,
    0x4FE342E2FE1A7F9B
};

int secp256r1_ecdsa_verify (
    const uint64_t * pk,     // 8 x 64 bits
    const uint64_t * _z,      // 4 x 64 bits
    const uint64_t * _r,      // 4 x 64 bits
    const uint64_t * _s,      // 4 x 64 bits
          uint64_t * result  // 8 x 64 bits
)
{
    // Convert z, r, s inputs to field elements
    RawnSecp256r1::Element z, r, s;
    array2fe(_z, z);
    array2fe(_r, r);
    array2fe(_s, s);

    // Given the public key pk and the signature (r, s) over the message hash z:
    // 1. Computes s_inv = s⁻¹ mod n
    // 2. Computes u1 = z·s_inv mod n
    // 3. Computes u2 = r·s_inv mod n
    // 4. Computes and returns the curve point p = u1·G + u2·PK
    
    // s_inv = s⁻¹ mod n
    RawnSecp256r1::Element s_inv;
    secp256r1n.inv(s_inv, s);

    // u1 = z·s_inv mod n
    RawnSecp256r1::Element u1;
    secp256r1n.mul(u1, z, s_inv);

    // u2 = r·s_inv mod n
    RawnSecp256r1::Element u2;
    secp256r1n.mul(u2, r, s_inv);
    uint64_t u1_array[4];
    uint64_t u2_array[4];
    fe2array(u1, u1_array);
    fe2array(u2, u2_array);

    secp256r1_curve_dbl_scalar_mul(u1_array, SECP256R1_G, u2_array, pk, result);

    return 0;
}

const uint64_t SECP256R1_IDENTITY[8] = {0,0,0,0,0,0,0,0};

void secp256r1_curve_add(
    const uint64_t * p, // 8 x 64 bits
    const uint64_t * q, // 8 x 64 bits
          uint64_t * r  // 8 x 64 bits
)
{
    // Get the 2 points coordinates
    const uint64_t * x1 = &p[0];
    const uint64_t * y1 = &p[4];
    const uint64_t * x2 = &q[0];
    const uint64_t * y2 = &q[4];

    // If p==q return dbl(p)
    if (x1[0] == x2[0] &&
        x1[1] == x2[1] &&
        x1[2] == x2[2] &&
        x1[3] == x2[3])
    {
        if (y1[0] == y2[0] &&
            y1[1] == y2[1] &&
            y1[2] == y2[2] &&
            y1[3] == y2[3]) {
            secp256r1_curve_dbl(p, r);
            return;
        } else {
            for (int i = 0; i < 8; i++) {
                r[i] = SECP256R1_IDENTITY[i];
            }
            return;
        }
    }

    // If p==0 return q
    if ( p[0] == SECP256R1_IDENTITY[0] &&
         p[1] == SECP256R1_IDENTITY[1] &&
         p[2] == SECP256R1_IDENTITY[2] &&
         p[3] == SECP256R1_IDENTITY[3] &&
         p[4] == SECP256R1_IDENTITY[4] &&
         p[5] == SECP256R1_IDENTITY[5] &&
         p[6] == SECP256R1_IDENTITY[6] &&
         p[7] == SECP256R1_IDENTITY[7] )
    {
        for (int i = 0; i < 8; i++)
        {
            r[i] = q[i];
        }
        return;
    }
    // if q == 0 return p
    else if (  q[0] == SECP256R1_IDENTITY[0] &&
               q[1] == SECP256R1_IDENTITY[1] &&
               q[2] == SECP256R1_IDENTITY[2] &&
               q[3] == SECP256R1_IDENTITY[3] &&
               q[4] == SECP256R1_IDENTITY[4] &&
               q[5] == SECP256R1_IDENTITY[5] &&
               q[6] == SECP256R1_IDENTITY[6] &&
               q[7] == SECP256R1_IDENTITY[7] )
    {
        for (int i = 0; i < 8; i++)
        {
            r[i] = p[i];
        }
        return;
    }

    // Convert coordinates to field elements
    RawpSecp256r1::Element x1_fe, y1_fe, x2_fe, y2_fe;
    array2fe(x1, x1_fe);
    array2fe(y1, y1_fe);
    array2fe(x2, x2_fe);
    array2fe(y2, y2_fe);

    // Calculate lambda = (y2 - y1) / (x2 - x1)
    RawpSecp256r1::Element y2_minus_y1;
    secp256r1.sub(y2_minus_y1, y2_fe, y1_fe);
    RawpSecp256r1::Element x2_minus_x1;
    secp256r1.sub(x2_minus_x1, x2_fe, x1_fe);
    RawpSecp256r1::Element x2_minus_x1_inv;
    secp256r1.inv(x2_minus_x1_inv, x2_minus_x1);
    RawpSecp256r1::Element lambda;
    secp256r1.mul(lambda, y2_minus_y1, x2_minus_x1_inv);

    // Calculate x3 = lambda^2 - (x1 + x2)
    RawpSecp256r1::Element x3_fe;
    RawpSecp256r1::Element lambda_sq;
    secp256r1.square(lambda_sq, lambda);
    RawpSecp256r1::Element x1_plus_x2;
    secp256r1.add(x1_plus_x2, x1_fe, x2_fe);
    secp256r1.sub(x3_fe, lambda_sq, x1_plus_x2);

    // Calculate y3 = lambda * (x1 - x3) - y1
    RawpSecp256r1::Element y3_fe;
    RawpSecp256r1::Element x1_minus_x3;
    secp256r1.sub(x1_minus_x3, x1_fe, x3_fe);
    RawpSecp256r1::Element lambda_x1_minus_x3;
    secp256r1.mul(lambda_x1_minus_x3, lambda, x1_minus_x3);
    secp256r1.sub(y3_fe, lambda_x1_minus_x3, y1_fe);

    // Convert to result
    fe2array(x3_fe, r);
    fe2array(y3_fe, r + 4);
}

void secp256r1_curve_dbl(
    const uint64_t * p, // 8 x 64 bits
          uint64_t * r  // 8 x 64 bits
)
{
    // If p==0 return p
    if ( p[0] == SECP256R1_IDENTITY[0] &&
         p[1] == SECP256R1_IDENTITY[1] &&
         p[2] == SECP256R1_IDENTITY[2] &&
         p[3] == SECP256R1_IDENTITY[3] &&
         p[4] == SECP256R1_IDENTITY[4] &&
         p[5] == SECP256R1_IDENTITY[5] &&
         p[6] == SECP256R1_IDENTITY[6] &&
         p[7] == SECP256R1_IDENTITY[7] )
    {
        for (int i = 0; i < 8; i++)
        {
            r[i] = p[i];
        }
        return;
    }

    // Convert coordinates to field elements
    uint64_t * x = (uint64_t *)&p[0];
    uint64_t * y = (uint64_t *)&p[4];
    RawpSecp256r1::Element x_fe, y_fe;
    array2fe(x, x_fe);
    array2fe(y, y_fe);

    // Calculate lambda = (3*x1^2) / (2*y1)
    RawpSecp256r1::Element x1_sq;
    secp256r1.square(x1_sq, x_fe);
    secp256r1.add(x1_sq, x1_sq, secp256r1.negOne());
    RawpSecp256r1::Element three;
    secp256r1.fromUI(three, 3);
    RawpSecp256r1::Element three_x1_sq;
    secp256r1.mul(three_x1_sq, x1_sq, three);
    RawpSecp256r1::Element two_y1;
    secp256r1.add(two_y1, y_fe, y_fe);
    RawpSecp256r1::Element two_y1_inv;
    secp256r1.inv(two_y1_inv, two_y1);
    RawpSecp256r1::Element lambda;
    secp256r1.mul(lambda, three_x1_sq, two_y1_inv);

    // Calculate x3 = lambda^2 - 2*x1
    RawpSecp256r1::Element lambda_sq;
    secp256r1.square(lambda_sq, lambda);
    RawpSecp256r1::Element two_x1;
    secp256r1.add(two_x1, x_fe, x_fe);
    RawpSecp256r1::Element x3_fe;
    secp256r1.sub(x3_fe, lambda_sq, two_x1);

    // Calculate y3 = lambda * (x1 - x3) - y1
    RawpSecp256r1::Element x1_minus_x3;
    secp256r1.sub(x1_minus_x3, x_fe, x3_fe);
    RawpSecp256r1::Element lambda_x1_minus_x3;
    secp256r1.mul(lambda_x1_minus_x3, lambda, x1_minus_x3);
    RawpSecp256r1::Element y3_fe;
    secp256r1.sub(y3_fe, lambda_x1_minus_x3, y_fe);

    // Convert to result
    fe2array(x3_fe, r);
    fe2array(y3_fe, r + 4);
}

int secp256r1_curve_dbl_scalar_mul(
    const uint64_t * k1, // 4 x 64 bits
    const uint64_t * p1, // 8 x 64 bits
    const uint64_t * k2, // 4 x 64 bits
    const uint64_t * p2, // 8 x 64 bits
    uint64_t * r // 8 x 64 bits
)
{
    for (uint64_t i = 0; i < 8; i++) {
        r[i] = 0;
    }

    for (int64_t ii=255; ii>=0; ii--) {
        uint64_t i = ii;

        // r = r + r
        secp256r1_curve_dbl(r, r);

        // If k1[i] == 1 then r = r + p1
        uint64_t k1_bit = (k1[i / 64] >> (i % 64)) & 1;
        if (k1_bit == 1)
        {
            secp256r1_curve_add(r, p1, r);
        }

        // If k2[i] == 1 then r = r + p2
        uint64_t k2_bit = (k2[i / 64] >> (i % 64)) & 1;
        if (k2_bit == 1)
        {
            secp256r1_curve_add(r, p2, r);
        }
    }

    return 0;
}

#ifdef __cplusplus
} // extern "C"
#endif