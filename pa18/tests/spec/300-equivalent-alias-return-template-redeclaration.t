// N3485 focus: 14.5.6 [temp.fct], 14.5.7 [temp.alias]

namespace compat {

template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<bool Condition, class T = void>
using __enable_if_t = typename enable_if<Condition, T>::type;

template<class...>
struct __and_ {
  static const bool value = true;
};

template<class Head, class... Tail>
struct __and_<Head, Tail...> {
  static const bool value = Head::value && __and_<Tail...>::value;
};

template<class Condition>
struct __not_ {
  static const bool value = !Condition::value;
};

template<class>
struct __is_tuple_like {
  static const bool value = false;
};

template<class>
struct is_move_constructible {
  static const bool value = true;
};

template<class>
struct is_move_assignable {
  static const bool value = true;
};

template<class... _Cond>
using _Require = __enable_if_t<__and_<_Cond...>::value>;

template<class _Tp>
_Require<__not_<__is_tuple_like<_Tp> >,
         is_move_constructible<_Tp>,
         is_move_assignable<_Tp> >
swap(_Tp&, _Tp&);

template<class _Tp>
typename enable_if<__and_<__not_<__is_tuple_like<_Tp> >,
                          is_move_constructible<_Tp>,
                          is_move_assignable<_Tp> >::value>::type
swap(_Tp& left, _Tp& right) {
  _Tp tmp = left;
  left = right;
  right = tmp;
}

}  // namespace compat

int main() {
  int left = 1;
  int right = 2;
  compat::swap(left, right);
  return left - 2;
}
