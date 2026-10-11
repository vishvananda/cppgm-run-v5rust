// N3485 focus: [temp.dep.expr] dependent member-value expressions.
// A qualified-id such as trait<T>::value may also parse as a dependent type;
// non-type template argument resolution must keep structured expression syntax.
template<bool B>
struct enable_if {};

template<>
struct enable_if<true> {
  typedef int type;
};

template<class T>
struct trait {
  static const bool value = true;
};

template<class T, class = typename enable_if<trait<T>::value>::type>
struct uses_member_value {
  enum { value = 7 };
};

int main()
{
  return uses_member_value<int>::value == 7 ? 0 : 1;
}
