// VALIDATION: compile-pass
// The substituted qualifier for Next::has_key must remain a semantic type.

template<class T, T V>
struct integral_constant
{
  static const T value = V;
};

template<bool B>
using bool_constant = integral_constant<bool, B>;

namespace support {

typedef char (&no_tag)[2];

struct empty_list
{
  template<class Key>
  static no_tag has_key(Key *);
};

}

template<class Next>
struct arg_list : Next
{
  typedef int key_type;

  using unique_key = bool_constant<
      sizeof(Next::has_key(static_cast<key_type *>(0))) ==
      sizeof(support::no_tag)>;

  static_assert(unique_key::value, "expected unique key");
};

arg_list<support::empty_list> value;

int main()
{
  return 0;
}
