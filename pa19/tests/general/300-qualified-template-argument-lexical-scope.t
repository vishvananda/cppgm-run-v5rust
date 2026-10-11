namespace meta {
template<bool B, class T = void> struct enable_if {};
template<class T> struct enable_if<true, T> { typedef T type; };
template<class A, class B> struct same { static const bool value = false; };
template<class A> struct same<A, A> { static const bool value = true; };
template<class T> struct identity { typedef T type; };
}

template<class T>
typename meta::enable_if<!meta::same<typename meta::identity<T>::type, bool>::value>::type choose(T value)
{
  (void)value;
}

int main()
{
  choose(1u);
  return 0;
}
