// When two function templates have the same fixed parameter shape, the
// fixed-arity template is more specialized than a generic trailing pack that
// consumes zero arguments.
template<unsigned long... I>
struct index_sequence
{
};

template<class R, class Context, class Tuple, unsigned long... I>
R dispatch(Context &, index_sequence<I...>, Tuple &&)
{
  return 1;
}

template<class R, class Context, class Tuple, class... Rest, unsigned long... I>
R dispatch(Context &, index_sequence<I...>, Tuple &&, Rest &&...)
{
  return 2;
}

int main()
{
  int context = 0;
  int tuple = 0;
  return dispatch<int>(context, index_sequence<0, 1>(), tuple) == 1 ? 0 : 1;
}
