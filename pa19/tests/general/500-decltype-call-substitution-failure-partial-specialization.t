struct Nat {
  Nat() = delete;
  Nat(const Nat &) = delete;
  Nat & operator=(const Nat &) = delete;
  ~Nat() = delete;
};

template<class T, class U, class = void>
struct core_convertible {
  static constexpr bool value = false;
};

template<class T, class U>
struct core_convertible<T, U,
    decltype(static_cast<void (*)(U)>(0)(static_cast<T (*)()>(0)()))> {
  static constexpr bool value = true;
};

static_assert(!core_convertible<Nat, bool>::value,
              "Nat should not convert to bool");

int main()
{
  return core_convertible<int, bool>::value ? 0 : 1;
}
