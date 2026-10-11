template<bool B, class T = void>
struct enable_if {};

template<class T>
struct enable_if<true, T>
{
  typedef T type;
};

template<class T>
struct remove_cv
{
  typedef T type;
};

template<class T>
struct trait
{
  static const bool value = false;
};

template<class T>
struct box
{
  template<class Y, class Y2 = typename remove_cv<Y>::type>
  typename enable_if<!trait<Y2>::value>::type accept(Y *)
  {
  }

  box(T * p)
  {
    accept(p);
  }
};

int main()
{
  int value = 0;
  box<int> b(&value);
  return 0;
}
