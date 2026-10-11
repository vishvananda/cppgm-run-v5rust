namespace ns {
template<bool B>
struct stable {
  static const bool value = B;
};

template<class T, class Option>
struct heap {
  static const bool value = Option::value;
};
}

template<bool stable>
int f()
{
  typedef ns::heap<int, ns::stable<stable> > type;
  return type::value ? 1 : 0;
}

int main()
{
  return f<true>() - 1;
}
