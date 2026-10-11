using size_t = decltype(sizeof(0));

template<class T, size_t N>
using simd_vector __attribute__((__ext_vector_type__(N))) = T;

template<class Vec>
constexpr size_t simd_size_v = 0;

template<class T, size_t N>
constexpr size_t simd_size_v<simd_vector<T, N>> = N;

template<class T, size_t N>
bool vector_any_of(simd_vector<T, N> value) noexcept {
  return __builtin_reduce_or(
      __builtin_convertvector(value, simd_vector<bool, N>));
}

template<size_t... Indices>
struct index_sequence {};

int load_scalarized_vector() {
  return []<size_t... Indices>(index_sequence<Indices...>) noexcept {
    return int{Indices...};
  }(index_sequence<0>{});
}

template<class T>
T scalarized_mask_bits(bool value) {
  return __builtin_bit_cast(T, value);
}

static_assert(simd_size_v<simd_vector<char, 1>> == 1, "vector size");

int main() {
  simd_vector<char, 1> lhs{};
  simd_vector<char, 1> rhs{};
  auto equal = lhs == rhs;
  return vector_any_of(equal) &&
                 scalarized_mask_bits<unsigned char>(true) == 1 &&
                 load_scalarized_vector() == 0 ?
             0 :
             1;
}
