namespace std {
template<class If, class Else>
void helper(bool cond, If if_fn, Else else_fn)
{
  if(cond) {
    if_fn();
  } else {
    else_fn();
  }
}
}

template<class T>
struct LambdaBox {
  template<class... Args>
  int run(Args&&... args)
  {
    int value = 0;
    std::helper(true, [&] { value = 1; }, [&] { value = 2; });
    return value;
  }
};
