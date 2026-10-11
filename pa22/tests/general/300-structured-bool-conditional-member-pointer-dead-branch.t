// Reduced from Boost.Xpressive `has_fold_case` selecting between member
// function pointers. A structured-bool false condition must not materialize
// output for the non-selected conditional branch.

template<bool B>
struct bool_
{
  static const bool value = B;
  typedef bool_ type;
  constexpr operator bool() const { return B; }
};

template<bool B>
bool const bool_<B>::value;

template<class T>
struct disabled
  : bool_<false>
{
};

template<class T>
struct holder
{
  typedef int (holder::*fn_type)() const;

  holder()
    : fn_(disabled<T>() ? &holder::bad : &holder::good)
  {
  }

  int good() const
  {
    return 0;
  }

  int bad() const
  {
    typedef char valid_local_array[1];
    return sizeof(valid_local_array);
  }

  fn_type fn_;
};

int main()
{
  holder<int> h;
  return 0;
}
