// Reduced from Boost.Xpressive static_xpression::match.
// The object member access has already preselected Matcher's member templates;
// qualified syntax must not re-run lookup and lose that synthetic scope.

struct state {};

struct next
{
  bool run(state &) const
  {
    return true;
  }
};

struct matcher
{
  template<class State, class Next>
  static bool match(State &s, Next const &n)
  {
    return n.run(s);
  }
};

template<class Matcher, class Next>
struct static_xpression : Matcher
{
  Next next_;

  bool run(state &s) const
  {
    return this->Matcher::match(s, this->next_);
  }
};

int main()
{
  state s;
  static_xpression<matcher, next> x;
  return x.run(s) ? 0 : 1;
}
