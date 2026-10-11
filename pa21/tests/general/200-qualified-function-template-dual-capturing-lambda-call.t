namespace N {

template <class If, class Else>
int helper(bool cond, If if_fn, Else else_fn)
{
  if(cond) {
    return if_fn();
  }
  return else_fn();
}

struct Vec {
  int add(bool cond)
  {
    int n = 7;
    return N::helper(cond,
                     [n] { return n + 1; },
                     [n] { return n - 1; });
  }
};

}

int main()
{
  N::Vec v;
  return v.add(true) - 8;
}
