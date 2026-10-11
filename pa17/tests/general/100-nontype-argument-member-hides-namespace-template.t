// VALIDATION: a non-type template argument spelled as a bare name finds the
// class's own static constant, which hides a namespace-scope class template
// of the same name (libc++'s numeric_limits passes its is_signed and digits
// members while std::is_signed is a class template).

template <class T>
struct is_signed
{
  static const bool value = false;
};

template <class T>
struct digits
{
  static const int value = 0;
};

template <class T, int Digits, bool IsSigned>
struct compute_min
{
  static const T value = T(T(1) << Digits);
};

template <class T, int Digits>
struct compute_min<T, Digits, false>
{
  static const T value = T(0);
};

template <class T>
class limits
{
protected:
  typedef T type;
  static const bool is_signed = type(-1) < type(0);
  static const int digits = static_cast<int>(sizeof(type) * 8 - is_signed);
  static const type lowest = compute_min<type, digits, is_signed>::value;

public:
  static type min() { return lowest; }
  static int width() { return digits; }
};

int main()
{
  if (limits<unsigned char>::min() != 0) return 1;
  if (limits<unsigned char>::width() != 8) return 2;
  if (limits<signed char>::width() != 7) return 3;
  if (limits<signed char>::min() != -128) return 4;
  if (is_signed<int>::value) return 5;
  return 0;
}
