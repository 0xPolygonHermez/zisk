#ifndef CURVE_PLAIN_HPP
#define CURVE_PLAIN_HPP

#include <stdio.h>
#include <string.h>

// Affine point addition and doubling on short Weierstrass curves y^2 = x^3 + a*x + b, for
// a = 0 or a = -3, working on plain (not Montgomery) canonical field elements to minimize the
// number of Montgomery multiplications. With M(u, v) = u*v*R^-1 the Montgomery product, and for
// the ffiasm fields:
//   - field.mul(r, u, v) computes M(u, v), so for plain u, v it gives u*v*R^-1, and for a plain
//     u and a Montgomery v = w*R it gives the plain product u*w
//   - field.inv(r, v) computes M(v^-1, R^3) = v^-1*R^2
// So the slope is obtained directly in Montgomery form, lambda*R = M(dy, dx^-1*R^2), and every
// other product mixes it with a plain value, which gives plain results.
// An addition takes 5 Montgomery multiplications (one of them inside inv()) and a doubling 7,
// instead of 10 and 9 when converting inputs and outputs to and from Montgomery form.

// (x3, y3) = (x1, y1) + (x2, y2); returns -1, leaving x3 and y3 undefined, if x1 = x2.
// x3 and y3 must not overlap the inputs.
template <typename Field>
inline int curve_add_plain (Field &field, const char * name,
    const typename Field::Element &x1, const typename Field::Element &y1,
    const typename Field::Element &x2, const typename Field::Element &y2,
    typename Field::Element &x3, typename Field::Element &y3)
{
    typename Field::Element dx, dy, t, lambda, lambda_r;

    // lambda = (y2 - y1)/(x2 - x1)
    field.sub(dx, x2, x1);
    if (field.isZero(dx))
    {
        printf("%s got denominator=0 2\n", name);
        return -1;
    }
    field.sub(dy, y2, y1);
    field.inv(t, dx);                      // t = dx^-1*R^2
    field.mul(lambda_r, dy, t);            // lambda*R
    field.fromMontgomery(lambda, lambda_r);

    // x3 = lambda^2 - x1 - x2
    field.mul(t, lambda_r, lambda);        // lambda^2
    field.sub(t, t, x1);
    field.sub(x3, t, x2);

    // y3 = lambda*(x1 - x3) - y1
    field.sub(t, x1, x3);
    field.mul(t, lambda_r, t);
    field.sub(y3, t, y1);

    return 0;
}

// (x3, y3) = 2*(x1, y1), for a = 0, or a = -3 if a_minus_3; returns -1, leaving x3 and y3
// undefined, if y1 = 0. x3 and y3 must not overlap the inputs.
template <typename Field>
inline int curve_dbl_plain (Field &field, const char * name, bool a_minus_3,
    const typename Field::Element &x1, const typename Field::Element &y1,
    typename Field::Element &x3, typename Field::Element &y3)
{
    typename Field::Element num, den, t, lambda, lambda_r;

    // lambda = (3*x1^2 + a)/(2*y1)
    field.add(den, y1, y1);
    if (field.isZero(den))
    {
        printf("%s got denominator=0 1\n", name);
        return -1;
    }
    field.toMontgomery(t, x1);
    field.mul(num, x1, t);                 // x1^2
    if (a_minus_3)
    {
        typename Field::Element one;
        memset(&one, 0, sizeof(one));
        one.v[0] = 1;
        field.sub(num, num, one);          // x1^2 - 1, so 3*(x1^2 - 1) = 3*x1^2 - 3
    }
    field.add(t, num, num);
    field.add(num, t, num);                // 3*x1^2 + a
    field.inv(t, den);                     // t = den^-1*R^2
    field.mul(lambda_r, num, t);           // lambda*R
    field.fromMontgomery(lambda, lambda_r);

    // x3 = lambda^2 - 2*x1
    field.mul(t, lambda_r, lambda);        // lambda^2
    field.sub(t, t, x1);
    field.sub(x3, t, x1);

    // y3 = lambda*(x1 - x3) - y1
    field.sub(t, x1, x3);
    field.mul(t, lambda_r, t);
    field.sub(y3, t, y1);

    return 0;
}

#endif
