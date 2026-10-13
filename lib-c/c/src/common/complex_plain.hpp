#ifndef COMPLEX_PLAIN_HPP
#define COMPLEX_PLAIN_HPP

// Multiplication of complex numbers x + y*i, with i^2 = -1, for an ffiasm field, with the first
// operand plain (not Montgomery) and the second one in Montgomery form. field.mul(r, u, v) is the
// Montgomery product u*v*R^-1, so a plain u times a Montgomery v = w*R gives the plain product
// u*w, and the result is plain. Karatsuba needs 3 products instead of 4:
//   x3 = x1*x2 - y1*y2
//   y3 = (x1 + y1)*(x2 + y2) - x1*x2 - y1*y2
// where (x2 + y2)*R is just the sum of x2*R and y2*R. With the conversion of the second operand,
// that is 5 Montgomery multiplications instead of 6.

// (x3 + y3*i) = (x1 + y1*i)*(x2 + y2*i), with x1, y1 plain and x2m = x2*R, y2m = y2*R;
// x3 and y3 are plain and must not overlap the inputs
template <typename Field>
inline void complex_mul_plain (Field &field,
    const typename Field::Element &x1, const typename Field::Element &y1,
    const typename Field::Element &x2m, const typename Field::Element &y2m,
    typename Field::Element &x3, typename Field::Element &y3)
{
    typename Field::Element s1, s2, a, b, c;

    field.add(s1, x1, y1);
    field.add(s2, x2m, y2m);

    field.mul(a, x1, x2m); // x1*x2
    field.mul(b, y1, y2m); // y1*y2
    field.mul(c, s1, s2);  // (x1 + y1)*(x2 + y2)

    field.sub(x3, a, b);
    field.sub(c, c, a);
    field.sub(y3, c, b);
}

#endif
