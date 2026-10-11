// __is_base_of relates two class types.  A reference is not one, so the trait
// is false when either operand is a reference even though the reference names
// a class -- resolving the operand to its entity would see straight through it
// and answer as if the reference were not there.
//
// libc++ turns on the distinction: its rvalue stream inserter is constrained by
// is_base_of<ios_base, _Stream>, and when _Stream deduces to an lvalue
// reference that false is what removes the overload so the ordinary lvalue
// operator<< is chosen instead.

struct base { };
struct derived : base { };
struct unrelated { };
enum kind { kind0 };

template<bool B> struct expect;
template<> struct expect<true> { typedef int type; };

// True where the relation actually holds, including through cv-qualifiers.
typedef expect<__is_base_of(base, derived)>::type t1;
typedef expect<__is_base_of(const base, derived)>::type t2;
typedef expect<__is_base_of(base, const derived)>::type t3;
typedef expect<__is_base_of(base, base)>::type t4;

// False for a reference on either side.
typedef expect<!__is_base_of(base, derived&)>::type t5;
typedef expect<!__is_base_of(base&, derived)>::type t6;
typedef expect<!__is_base_of(base, derived&&)>::type t7;
typedef expect<!__is_base_of(base&, derived&)>::type t8;

// And false for the other non-class shapes, which the same rule decides.
typedef expect<!__is_base_of(base, derived*)>::type t9;
typedef expect<!__is_base_of(derived, base)>::type t10;
typedef expect<!__is_base_of(base, unrelated)>::type t11;
typedef expect<!__is_base_of(int, int)>::type t12;
typedef expect<!__is_base_of(kind, kind)>::type t13;

int main() { return 0; }
