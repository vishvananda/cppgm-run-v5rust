// VALIDATION: compile-pass
// N3485 focus: 14.5.2 [temp.mem], 14.8 [temp.fct.spec]

template<bool Condition, class T>
struct enable_if
{
};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

template<class T>
struct is_pointer
{
  static const bool value = false;
};

template<class T>
struct is_pointer<T *>
{
  static const bool value = true;
};

template<class Owner>
struct selector
{
  template<class T>
  typename enable_if<is_pointer<T>::value, int>::type select(T);

  template<class T>
  typename enable_if<!is_pointer<T>::value, long>::type select(T);
};

template<class Owner>
template<class T>
auto selector<Owner>::select(T) ->
    typename enable_if<is_pointer<T>::value, int>::type
{
  return 11;
}

template<class Owner>
template<class T>
auto selector<Owner>::select(T) ->
    typename enable_if<!is_pointer<T>::value, long>::type
{
  return 22;
}

int main()
{
  selector<void> value;
  int object = 0;
  return value.select(&object) == 11 && value.select(object) == 22 ? 0 : 1;
}
