template<class T>
struct lambda_functor
{
};

template<int I>
struct placeholder
{
};

typedef const lambda_functor<placeholder<1> > placeholder1_type;

placeholder1_type free1 = placeholder1_type();
placeholder1_type & _1 = free1;

template<class Arg, class B>
int operator+=(const lambda_functor<Arg> &, const B &)
{
  return 7;
}

int main()
{
  return (_1 += ' ') == 7 ? 0 : 1;
}
