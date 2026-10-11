// VALIDATION: compile-pass
// Hosted character-trait reducer with a complete primary definition.
// User character conversions are instantiated through ordinary template facts.

namespace std
{
  template<class T>
  struct char_traits
  {
    typedef unsigned int int_type;
    static T to_char_type(int_type value) { return T(value); }
    static int_type to_int_type(T value) { return static_cast<int_type>(value); }
  };
}

struct UChar
{
  UChar(unsigned int code = 0) : code_(code) {}

  operator unsigned int() const
  {
    return code_;
  }

  unsigned int code_;
};

template<class Char>
struct probe
{
  typedef typename std::char_traits<Char>::int_type int_type;

  static Char to_char(int_type value)
  {
    return std::char_traits<Char>::to_char_type(value);
  }

  static int_type to_int(Char value)
  {
    return std::char_traits<Char>::to_int_type(value);
  }
};

int main()
{
  UChar ch = probe<UChar>::to_char(7);
  return probe<UChar>::to_int(ch) == 7 ? 0 : 1;
}
