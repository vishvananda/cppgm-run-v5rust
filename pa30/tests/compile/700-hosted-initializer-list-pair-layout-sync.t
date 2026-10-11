#include <initializer_list>
#include <map>
#include <utility>

template<class T>
struct static_constant {
  static T value;
};

template<class T>
T static_constant<T>::value;

namespace named {

template<class Id, bool Required = false>
struct keyword {
  typedef Id id;
};

template<class T, class Id, class RefType = T const&>
struct named_parameter {
  typedef Id id;
  typedef T data_type;
  typedef RefType ref_type;

  explicit named_parameter(ref_type value) : value_(value) {}

  ref_type operator[](keyword<Id, false>) const { return value_; }

private:
  ref_type value_;
};

template<class T, class Id, bool Required = false>
struct typed_keyword : keyword<Id, Required> {
  named_parameter<T const, Id>
  operator=(T const& value) const
  {
    return named_parameter<T const, Id>(value);
  }
};

}  // namespace named

namespace domain {

template<class CharT>
class basic_cstring {
public:
  typedef CharT value_type;
  typedef value_type * iterator;
  typedef value_type const * const_iterator;

  basic_cstring() : first_(0), last_(0) {}
  basic_cstring(iterator first, iterator last) : first_(first), last_(last) {}
  basic_cstring(iterator first) : first_(first), last_(first) {}

  const_iterator begin() const { return first_; }
  const_iterator end() const { return last_; }

private:
  iterator first_;
  iterator last_;
};

typedef basic_cstring<char const> cstring;

enum output_format {
  of_invalid,
  of_text,
  of_xml
};

bool operator<(cstring lhs, cstring rhs)
{
  return lhs.begin() < rhs.begin() ||
         (!(rhs.begin() < lhs.begin()) && lhs.end() < rhs.end());
}

}  // namespace domain

namespace runtime {

template<class EnumType>
using enum_values = static_constant<
    named::typed_keyword<
        std::initializer_list<std::pair<const domain::cstring, EnumType> >,
        struct enum_values_t> >;

template<class EnumType>
struct value_interpreter {
  template<class Modifiers>
  explicit value_interpreter(Modifiers const& modifiers)
      : names(modifiers[enum_values<EnumType>::value])
  {
  }

  std::map<domain::cstring, EnumType> names;
};

}  // namespace runtime

int main()
{
  runtime::value_interpreter<domain::output_format> interpreter{
      runtime::enum_values<domain::output_format>::value = {
          {"text", domain::of_text},
          {"xml", domain::of_xml}
      }};
  return interpreter.names.size() == 2 ? 0 : 1;
}
