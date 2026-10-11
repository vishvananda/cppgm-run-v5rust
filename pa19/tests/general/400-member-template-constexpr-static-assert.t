template<class A, class B> struct same { static constexpr bool value = false; };
template<class A> struct same<A, A> { static constexpr bool value = true; };

template<class T1, class T2>
struct check_pair_construction {
  template<class U1, class U2>
  static constexpr bool is_pair_constructible() {
    return same<T1, U1>::value && same<T2, U2>::value;
  }
};

static_assert(
    check_pair_construction<unsigned long, unsigned long>::template
        is_pair_constructible<unsigned long, unsigned long>(),
    "bad");

int main() { return 0; }
