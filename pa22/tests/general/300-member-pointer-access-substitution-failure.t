struct base
{
public:
  void public_function() {}

protected:
  void protected_function() {}

private:
  void private_function() {}
};

struct derived : base
{
  template<class T, void (T::*)()>
  struct check {};

  template<class T>
  static char has_protected(check<T, &T::protected_function> *);

  template<class>
  static long has_protected(...);

  template<class T>
  static char has_private(check<T, &T::private_function> *);

  template<class>
  static long has_private(...);
};

int main()
{
  return sizeof(derived::has_protected<base>(0)) == sizeof(long) &&
         sizeof(derived::has_private<base>(0)) == sizeof(long) ? 0 : 1;
}
