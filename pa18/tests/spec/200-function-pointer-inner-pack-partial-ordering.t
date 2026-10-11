// N3485 focus: 14.8.2.4 [temp.deduct.partial], 14.8.3 [temp.over]
// A function-pointer pattern with an inner pack beats a generic parameter.
template<class R, class... P, class... A>
int choose(R (*)(P...), A const &...);

template<class F, class... A>
char choose(F, A const &...);

int unary(int);

static_assert(sizeof(choose(&unary, 0)) == sizeof(int), "");

int main()
{
  return 0;
}
