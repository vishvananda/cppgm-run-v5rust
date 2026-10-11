namespace sequence
{
  template<class T, T... I>
  struct integer_sequence
  {
  };

  template<unsigned long... I>
  using index_sequence = integer_sequence<unsigned long, I...>;
}

struct context
{
};

struct handler
{
};

template<class R, class Context, class Handler, unsigned long... I>
R dispatch(Context &, sequence::index_sequence<I...>, Handler &&)
{
  return sizeof...(I);
}

int main()
{
  context ctx;
  handler h;
  sequence::integer_sequence<unsigned long, 0, 1> indices;
  return dispatch<int>(ctx, indices, static_cast<handler &&>(h)) == 2 ? 0 : 1;
}
