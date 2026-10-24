// PA29 GNU typeof: named function type-ids differ from sizeof's functional
// casts; expression queries retain cv/array/function types and remove refs.
template<class A, class B> struct same { static const bool value = false; };
template<class A> struct same<A, A> { static const bool value = true; };
const int value = 3;
int array[4];
int function(double);
typedef int scalar;
typedef __typeof__(scalar()) function_type;
typedef __typeof__(int&) reference_type;
typedef __typeof__((value)) const_type;
typedef __typeof__(array) array_type;
typedef __typeof__ function fn_type;
static_assert(same<function_type, int()>::value, "named function type-id");
static_assert(same<reference_type, int&>::value, "reference type-id");
static_assert(same<const_type, const int>::value, "cv expression");
static_assert(same<array_type, int[4]>::value, "no decay");
static_assert(same<fn_type, int(double)>::value, "function expression");
static_assert(same<__typeof__(static_cast<const int&>(value)), const int>::value, "no reference on expression");
int main() { __typeof(value) local = 3; return local - 3; }
