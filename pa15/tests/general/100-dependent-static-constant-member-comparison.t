template<class T>
struct X {
  static constexpr const bool is_signed = T(-1) < T(0);
};

int main() {
  return X<long long int>::is_signed ? 0 : 1;
}
