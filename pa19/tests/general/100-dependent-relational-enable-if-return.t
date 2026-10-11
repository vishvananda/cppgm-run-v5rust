namespace dtl
{
  template<bool B, class T>
  struct enable_if_c
  {
    typedef T type;
  };
}

template<int N, class F, class G>
inline typename dtl::enable_if_c<(N <= 4) && false, void>::type
deep_swap_like(F, G)
{
}

int main()
{
  return 0;
}
