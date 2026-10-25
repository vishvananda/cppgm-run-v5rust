using I = int __attribute__((vector_size(16)));
using F = float __attribute__((vector_size(16)));
using B = bool __attribute__((ext_vector_type(4)));
using S = short __attribute__((ext_vector_type(4)));

template<class T, class V>
auto converted(V v) -> decltype(__builtin_convertvector(v, T)) {
  return __builtin_convertvector(v, T);
}
template<class T, class V>
auto reinterpreted(V v) -> decltype(__builtin_bit_cast(T, v)) {
  return __builtin_bit_cast(T, v);
}

static_assert(__is_same(decltype(converted<F>(I{})), F), "dependent vector result");
static_assert(__is_same(decltype(reinterpreted<unsigned>(1.0f)), unsigned), "dependent bit result");
static_assert(__is_same(decltype(__builtin_reduce_or(B{})), bool), "bool lane result");
static_assert(__is_same(decltype(__builtin_reduce_add(S{})), short), "unpromoted result");
static_assert(__is_same(decltype(__builtin_reduce_min(F{})), float), "floating result");
I input() noexcept;
I throwing();
static_assert(noexcept(__builtin_convertvector(input(), F)), "intrinsic is nonthrowing");
static_assert(!noexcept(__builtin_convertvector(throwing(), F)), "operand can throw");
static_assert(noexcept(__builtin_bit_cast(unsigned, 1.0f)), "representation is nonthrowing");

template<class T> auto select_vector(T v) -> decltype(__builtin_convertvector(v, I));
char select_vector(...);
template<class T> auto select_bits(T v) -> decltype(__builtin_bit_cast(unsigned, v));
char select_bits(...);
static_assert(__is_same(decltype(select_vector(I{})), I), "valid vector substitution");
static_assert(__is_same(decltype(select_vector(0)), char), "scalar substitution fallback");
static_assert(__is_same(decltype(select_bits(0.0f)), unsigned), "valid bit substitution");
static_assert(__is_same(decltype(select_bits(0.0)), char), "size substitution fallback");
int main() {
  I input{};
  F output = converted<F>(input);
  return reinterpreted<unsigned>(output[0]);
}
