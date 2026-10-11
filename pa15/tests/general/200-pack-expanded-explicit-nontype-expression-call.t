// Each expansion of an explicit non-type template argument expression must
// carry the corresponding value-pack binding into its structured expression.
template<int I>
struct tag
{
};

template<int I>
tag<I> make_tag()
{
  return tag<I>();
}

void consume(tag<1>, tag<2>)
{
}

template<int... I>
struct sequence
{
};

template<int... I>
void expand(sequence<I...>)
{
  consume(make_tag<I + 1>()...);
}

int main()
{
  expand(sequence<0, 1>());
}
