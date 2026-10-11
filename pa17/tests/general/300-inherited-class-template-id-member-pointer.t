struct base {
  template<class T>
  struct prop_fns {};
};

template<class T>
struct any_executor : base {
  const prop_fns<any_executor>* prop_fns_;
};

int main()
{
  any_executor<int> ex;
  (void)ex;
  return 0;
}
