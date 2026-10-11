template<class A, class B> struct is_same { static constexpr bool value = false; };
template<class A> struct is_same<A, A> { static constexpr bool value = true; };

static_assert(!is_same<char*&, char* const&>::value, "char*& vs char* const&");
static_assert(!is_same<const char*&, const char* const&>::value,
              "const char*& vs const char* const&");

int main() { return 0; }
