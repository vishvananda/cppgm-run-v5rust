using raw_unsigned_char = __make_unsigned(char);
using raw_signed_uchar = __make_signed(unsigned char);

template<class T>
using __make_unsigned_t = __make_unsigned(T);

template<class T>
using __make_signed_t = __make_signed(T);

template<class A, class B>
struct same {
  static constexpr bool value = __is_same(A, B);
};

static_assert(same<raw_unsigned_char, unsigned char>::value, "");
static_assert(same<raw_signed_uchar, signed char>::value, "");
static_assert(same<__make_unsigned_t<char>, unsigned char>::value, "");
static_assert(same<__make_signed_t<unsigned char>, signed char>::value, "");
static_assert(same<__make_unsigned_t<wchar_t>, unsigned int>::value, "");
static_assert(same<__make_unsigned_t<char16_t>, unsigned short>::value, "");
static_assert(same<__make_signed_t<char32_t>, int>::value, "");

int main() {
  return 0;
}
